// LAB-H1 E2a/E2b: UEFI frame ownership and gated ELF staging in a pinned QEMU profile.
// NO ExitBootServices, kernel-CR3 write or freestanding kernel entry.
// This is EFI preflight only, intentionally returning to firmware afterwards.
use core::{ptr, slice};
use unios_bootmap::{Reservation, EMPTY, MAP_CAP, PAGE};
use unios_elf::Image;

const EFI_BOOT_SERVICES_SIGNATURE: u64 = 0x5652_4553_544f_4f42;
const ALLOCATE_ANY_PAGES: u32 = 0;
const EFI_LOADER_DATA: u32 = 2;
const DESCRIPTOR_CAP: usize = 256;

#[repr(C)]
struct Header {
    signature: u64,
    revision: u32,
    size: u32,
    crc32: u32,
    reserved: u32,
}

#[repr(C)]
pub struct BootServices {
    header: Header,
    raise_tpl: usize,
    restore_tpl: usize,
    allocate_pages: unsafe extern "efiapi" fn(u32, u32, usize, *mut u64) -> usize,
    free_pages: usize,
    get_memory_map: unsafe extern "efiapi" fn(
        *mut usize, *mut u8, *mut usize, *mut usize, *mut u32
    ) -> usize,
    // EFI_BOOT_SERVICES offset 64..232, verified x64 layout.
    // No use of the intervening services in E2p.
    #[cfg(feature="lab-e2p-actual-exitbs")]
    unused_boot_services: [usize;21],
    #[cfg(feature="lab-e2p-actual-exitbs")]
    exit_boot_services: unsafe extern "efiapi" fn(usize,usize)->usize,
}

#[repr(align(4096))]
struct AlignedMap([u8; MAP_CAP]);

static mut MAP_BUFFER: AlignedMap = AlignedMap([0; MAP_CAP]);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    BadServices,
    Allocate,
    ReservedRange,
    TooMany,
    MemoryMap,
    MapInvalid,
    #[cfg(feature = "lab-e2b-staging")]
    OldCr3,
    #[cfg(feature = "lab-e2b-staging")]
    Staging,
    #[cfg(feature = "lab-e2c-tables")]
    Paging,
    #[cfg(feature = "lab-e2d-bridge-audit")]
    Bridge,
    #[cfg(feature = "lab-e2e-bounded-bridge")]
    BoundedBridge,
    #[cfg(feature = "lab-e2f-descriptors")]
    Descriptors,
    #[cfg(feature = "lab-e2h-host-root-proof")]
    HostProof,
    #[cfg(feature = "lab-e2k-emergency-closure")]
    EmergencyCode,
    #[cfg(feature = "lab-e2l-lab-descriptors")]
    ActiveLab,
    #[cfg(feature="lab-e2n-transition-closure")]
    TransitionCut,
    #[cfg(feature="lab-e2o-host-private-witness")]
    NewHostProof,
    #[cfg(feature="lab-e2p-exitbs-contract")]
    FinalMap,
    #[cfg(feature="lab-e2q-cr3-code")]
    SwitchCode,
}

fn reserve(bs: &BootServices, pages: u64, id: u8) -> Result<Reservation, Error> {
    if pages == 0 || pages > (16 * 1024 * 1024 / PAGE) {
        return Err(Error::ReservedRange);
    }
    let pages_uefi = usize::try_from(pages).map_err(|_| Error::ReservedRange)?;
    let mut address = 0u64;
    // Firmware assigns this backing. Treat returned address as PHYSICAL ONLY;
    // converting it into a Rust pointer is forbidden until CR3 proof.
    let status = unsafe {
        (bs.allocate_pages)(ALLOCATE_ANY_PAGES, EFI_LOADER_DATA, pages_uefi, &mut address)
    };
    if status != 0 { return Err(Error::Allocate); }
    if address % PAGE != 0 ||
        address.checked_add(pages.checked_mul(PAGE).ok_or(Error::ReservedRange)?).is_none() {
        return Err(Error::ReservedRange);
    }
    Ok(Reservation { base: address, pages, id })
}

pub fn preflight(services: *mut BootServices, image_handle: usize, image: &Image, payload: &[u8]) -> Result<usize, Error> {
    if services.is_null() { return Err(Error::BadServices); }
    let bs = unsafe { &*services };
    #[cfg(any(feature="lab-e2q-actual-cr3",feature="lab-e2r-active-descriptors"))]
    let mut switch_root=0u64;
    #[cfg(any(feature="lab-e2q-actual-cr3",feature="lab-e2r-active-descriptors"))]
    let mut switch_bridge_top=0u64;
    if bs.header.signature != EFI_BOOT_SERVICES_SIGNATURE ||
        (bs.header.size as usize) < core::mem::size_of::<BootServices>() {
        return Err(Error::BadServices);
    }
    // Cap 8 load-segment physical allocations + 7 fixed reservations.
    // Never allocate on the final GetMemoryMap -> ExitBS path in a future stage.
    let mut ledger = [EMPTY; 16];
    let mut count = 0usize;
    for seg in image.segments.iter().take(image.count) {
        if count >= ledger.len() { return Err(Error::TooMany); }
        let pages = (seg.page_end - seg.page_start) / PAGE;
        ledger[count] = reserve(bs, pages, 1)?;
        count += 1;
    }
    // 512KiB private page-table CAP, bridge/main/IST stacks, emergency
    // descriptor storage, BootContext and COM1 scratch. Virtual guard pages
    // must remain UNMAPPED in E3; reserving stack pages does not map them.
    for (id, pages) in [
        (2u8, 128u64), // 512 KiB PML4+tables (future, precharged only)
        (3, 2), // 8 KiB bridge stack
        (4, 4), // 16 KiB main stack
        (5, 4), // 16 KiB double-fault IST stack
        (6, 1), // 4 KiB emergency GDT/IDT/TSS metadata (future)
        (7, 1), // 4 KiB BootContext header (future)
        (8, 1), // 4 KiB diagnostic scratch (future)
    ] {
        if count >= ledger.len() { return Err(Error::TooMany); }
        ledger[count] = reserve(bs, pages, id)?;
        count += 1;
    }
    // Fixed 64KiB loader-resident buffer. This EFI image is still alive.
    // No pool allocations or print callbacks between map snapshot and audit.
    let mut map_size = MAP_CAP;
    let mut map_key = 0usize;
    let mut descriptor_size = 0usize;
    let mut descriptor_version = 0u32;
    let buffer = unsafe { ptr::addr_of_mut!(MAP_BUFFER.0).cast::<u8>() };
    let status = unsafe {
        (bs.get_memory_map)(
            &mut map_size, buffer, &mut map_key,
            &mut descriptor_size, &mut descriptor_version
        )
    };
    if status != 0 || map_size > MAP_CAP || map_key == 0 ||
        descriptor_size > DESCRIPTOR_CAP {
        return Err(Error::MemoryMap);
    }
    let map = unsafe { slice::from_raw_parts(buffer, MAP_CAP) };
    let report = unios_bootmap::validate(
        map, map_size, descriptor_size, descriptor_version, &ledger[..count]
    ).map_err(|_| Error::MapInvalid)?;
    if report.reservations != count || report.reserved_pages == 0 {
        return Err(Error::MapInvalid);
    }
    #[cfg(feature = "lab-e2b-staging")]
    {
        // This tests same-linear physical pages *under the current old CR3*.
        // It does not establish their post-ExitBootServices provenance.
        // Fail closed on any unsupported or mismatched translation.
        // Map and reservation ownership validated without any old-root deref.
        let root_gate=crate::oldmap::inspect_current_root(
            map,map_size,descriptor_size,&ledger[..count],
        ).map_err(|_|Error::OldCr3)?;
        #[cfg(feature="lab-e2h-host-root-proof")]
        if !crate::host_checkpoint::host_checkpoint(&ledger[..count]) {
            return Err(Error::HostProof);
        }
        // The first volatile physical-root pointer access must occur
        // ONLY after the independent paused-QEMU host authorized COM2.
        crate::oldmap::check_owned_identity(&ledger[..count],root_gate)
            .map_err(|_| Error::OldCr3)?;
        for (segment, reserved) in image.segments.iter().take(image.count)
            .zip(ledger.iter().take(image.count))
        {
            let size = usize::try_from(reserved.pages * PAGE).map_err(|_| Error::Staging)?;
            // SAFETY: frames were allocated by EFI and shown to be RW and
            // identity-mapped by the current firmware CR3 page walk.
            let dest = unsafe { slice::from_raw_parts_mut(reserved.base as *mut u8, size) };
            unios_staging::stage(segment, payload, dest).map_err(|_| Error::Staging)?;
            unios_staging::verify(segment, payload, dest).map_err(|_| Error::Staging)?;
        }
            #[cfg(feature = "lab-e2c-tables")]
        {
            use unios_paging::{Builder, Rights, TABLE_WORDS, MAIN_STACK_VA, IST_STACK_VA,
                              DESCRIPTORS_VA, CONTEXT_VA};
            let backing = ledger[image.count];
            if backing.id != 2 || backing.pages != unios_paging::TABLE_PAGES as u64 {
                return Err(Error::Paging);
            }
            // We already checked all 512 KiB of these exact owned frames
            // under the firmware CR3. This pointer is NEVER used post-cut.
            let frames=unsafe {
                slice::from_raw_parts_mut(backing.base as *mut u64, TABLE_WORDS)
            };
            let mut tables=Builder::new(frames,backing.base).map_err(|_|Error::Paging)?;
            for (seg, reserved) in image.segments.iter().take(image.count)
                .zip(ledger.iter().take(image.count))
            {
                tables.map_kernel_segment(seg,reserved.base).map_err(|_|Error::Paging)?;
                let rights=if seg.flags & 1 != 0 {Rights::Rx}
                    else if seg.flags & 2 != 0 {Rights::Rw}
                    else {Rights::Ro};
                for n in 0..reserved.pages {
                    tables.verify(seg.page_start + n*PAGE,reserved.base + n*PAGE,rights)
                        .map_err(|_|Error::Paging)?;
                }
            }
            // Fixed, nonoverlapping VM-only guard addresses; nothing maps
            // these pages. No direct-map / writable kernel RX frame aliases.
            let aux=&ledger[image.count..count];
            if aux.len() != 7 || aux[1].id != 3 || aux[2].id != 4 ||
                aux[3].id != 5 || aux[4].id != 6 || aux[5].id != 7 {
                return Err(Error::Paging);
            }
            tables.map_owned_low_bridge_stack(aux[1].base).map_err(|_|Error::Paging)?;
            tables.map_stack(MAIN_STACK_VA,aux[2].base,4).map_err(|_|Error::Paging)?;
            tables.map_stack(IST_STACK_VA,aux[3].base,4).map_err(|_|Error::Paging)?;
            tables.map4k(DESCRIPTORS_VA,aux[4].base,Rights::Rw)
                .map_err(|_|Error::Paging)?;
            tables.map4k(CONTEXT_VA,aux[5].base,Rights::Ro)
                .map_err(|_|Error::Paging)?;
            tables.audit_guard_pages().map_err(|_|Error::Paging)?;
            // Defensive positive and negative check of supervisor leaf rights.
            for (va,pa,rights) in [
                (aux[1].base,aux[1].base,Rights::Rw),
                (MAIN_STACK_VA,aux[2].base,Rights::Rw),
                (IST_STACK_VA,aux[3].base,Rights::Rw),
                (DESCRIPTORS_VA,aux[4].base,Rights::Rw),
                (CONTEXT_VA,aux[5].base,Rights::Ro),
            ] {
                tables.verify(va,pa,rights).map_err(|_|Error::Paging)?;
            }
            #[cfg(feature = "lab-e2d-bridge-audit")]
            {
                // An executable anchor page, not actual CR3-switch code.
                // Check CPU *capability* and current firmware profile, but
                // do not change WP/NXE or claim enforcement at future cut.
                let _audit = crate::cpuprobe::inspect().map_err(|_|Error::Bridge)?;
                let (code_va,code_pa)=crate::oldmap::audit_efi_code_page(
                    root_gate, crate::bridge_anchor::linear_address(),map,map_size,descriptor_size,
                ).map_err(|_|Error::Bridge)?;
                tables.map_efi_executable_anchor(code_va,code_pa)
                    .map_err(|_|Error::Bridge)?;
                tables.verify(code_va,code_pa,Rights::Rx)
                    .map_err(|_|Error::Bridge)?;
            }
            #[cfg(feature = "lab-e2e-bounded-bridge")]
            {
                use unios_bridgecheck::{Page, Span};
                // The assembly stub is finite and can be inspected by its
                // exact linker-defined range. It is still NOT executed.
                let (begin,end)=crate::bridge_span::bounds();
                let span=Span::new(begin,end).map_err(|_|Error::BoundedBridge)?;
                let cpu=crate::cpuprobe::inspect().map_err(|_|Error::BoundedBridge)?;
                let mut proofs=[Page {
                    linear:0, old_physical:0,new_physical:0,
                    old_executable:false,new_executable:false,
                    old_supervisor:false,new_supervisor:false,efi_loader_code:false,
                };unios_bridgecheck::MAX_CODE_PAGES];
                for n in 0..span.pages {
                    let address=span.page(n).ok_or(Error::BoundedBridge)?;
                    let (va,pa)=crate::oldmap::audit_efi_code_page(
                    root_gate, address,map,map_size,descriptor_size,
                    ).map_err(|_|Error::BoundedBridge)?;
                    // The anchor may occupy the same EFI loader code page.
                    // If so, demand identical PA and rights, not a remap.
                    match tables.lookup(va).map_err(|_|Error::BoundedBridge)? {
                        Some(existing) if existing.pa == pa &&
                            existing.rights == Rights::Rx => {}
                        Some(_) => return Err(Error::BoundedBridge),
                        None => tables.map_efi_executable_anchor(va,pa)
                            .map_err(|_|Error::BoundedBridge)?,
                    }
                    tables.verify(va,pa,Rights::Rx).map_err(|_|Error::BoundedBridge)?;
                    proofs[n]=Page {
                        linear:va,old_physical:pa,new_physical:pa,
                        old_executable:true,new_executable:true,
                        old_supervisor:true,new_supervisor:true,efi_loader_code:true,
                    };
                }
                unios_bridgecheck::verify(
                    span,&proofs[..span.pages],cpu.wp_enabled,cpu.nxe_enabled,
                ).map_err(|_|Error::BoundedBridge)?;
                crate::bridge_span::validate_linked_bytes()
                    .map_err(|_|Error::BoundedBridge)?;
            }
            #[cfg(feature = "lab-e2f-descriptors")]
            {
                // Dormant descriptor image in the already-allocated
                // one-page EFI metadata reservation. No LGDT/LIDT/LTR:
                // current UEFI firmware descriptors stay live.
                let (generic,df)=crate::emergency_handlers::addresses();
                if generic==df {return Err(Error::Descriptors);}
                for handler in [generic,df] {
                    let (va,pa)=crate::oldmap::audit_efi_code_page(
                    root_gate, handler,map,map_size,descriptor_size,
                    ).map_err(|_|Error::Descriptors)?;
                    match tables.lookup(va).map_err(|_|Error::Descriptors)? {
                        Some(existing) if existing.pa==pa &&
                            existing.rights==Rights::Rx => {}
                        Some(_) => return Err(Error::Descriptors),
                        None => tables.map_efi_executable_anchor(va,pa)
                            .map_err(|_|Error::Descriptors)?,
                    }
                    tables.verify(va,pa,Rights::Rx).map_err(|_|Error::Descriptors)?;
                }
                let desc_pa=aux[4].base;
                tables.verify(DESCRIPTORS_VA,desc_pa,Rights::Rw)
                    .map_err(|_|Error::Descriptors)?;
                tables.audit_guard_pages().map_err(|_|Error::Descriptors)?;
                let desc=unsafe {
                    slice::from_raw_parts_mut(desc_pa as *mut u8,unios_descriptors::PAGE)
                };
                let main_top=MAIN_STACK_VA+4*PAGE;
                let df_top=IST_STACK_VA+4*PAGE;
                let image=unios_descriptors::build(
                    desc,DESCRIPTORS_VA,main_top,df_top,generic,df,
                ).map_err(|_|Error::Descriptors)?;
                unios_descriptors::verify(desc,image,main_top,df_top,generic,df)
                    .map_err(|_|Error::Descriptors)?;
                // Validate that both top-of-stack addresses are the
                // ends of their last 4KiB mapped page.
                tables.verify(main_top-1,aux[2].base+4*PAGE-1,Rights::Rw)
                    .map_err(|_|Error::Descriptors)?;
                tables.verify(df_top-1,aux[3].base+4*PAGE-1,Rights::Rw)
                    .map_err(|_|Error::Descriptors)?;
                if tables.lookup(unios_paging::MAIN_GUARD_VA)
                    .map_err(|_|Error::Descriptors)?.is_some() ||
                   tables.lookup(unios_paging::IST_GUARD_VA)
                    .map_err(|_|Error::Descriptors)?.is_some() {
                    return Err(Error::Descriptors);
                }
                // Next stages must independently prove the complete
                // exception handler bytes and machine frame ABI before
                // these descriptor images can ever be installed.
            }
            #[cfg(feature = "lab-e2k-emergency-closure")]
            {
                use unios_bridgecheck::{Page as CodePage, Span};
                // E2k exact-linked emergency code closure, still DORMANT.
                // For each page intersecting both fail-stop handlers prove:
                // old CR3 supervisor executable + UEFI_LOADER_CODE owner,
                // new private table same-PA supervisor RX, CPU WP/NXE.
                let (normal,df)=crate::emergency_handlers::addresses();
                let spans=crate::emergency_handlers::spans();
                if spans[0].0!=normal || spans[1].0!=df ||
                   spans[0].0>=spans[1].0 || spans[0].1>spans[1].0 {
                    return Err(Error::EmergencyCode);
                }
                let cpu=crate::cpuprobe::inspect()
                    .map_err(|_|Error::EmergencyCode)?;
                for (begin,end) in spans {
                    let span=Span::new(begin,end).map_err(|_|Error::EmergencyCode)?;
                    let mut proof=[CodePage {
                        linear:0,old_physical:0,new_physical:0,
                        old_executable:false,new_executable:false,
                        old_supervisor:false,new_supervisor:false,
                        efi_loader_code:false,
                    };unios_bridgecheck::MAX_CODE_PAGES];
                    for n in 0..span.pages {
                        let source_page=span.page(n).ok_or(Error::EmergencyCode)?;
                        let (va,pa)=crate::oldmap::audit_efi_code_page(
                            root_gate,source_page,map,map_size,descriptor_size
                        ).map_err(|_|Error::EmergencyCode)?;
                        match tables.lookup(va).map_err(|_|Error::EmergencyCode)? {
                            Some(existing) if existing.pa==pa && existing.rights==Rights::Rx => {}
                            Some(_) => return Err(Error::EmergencyCode),
                            None => tables.map_efi_executable_anchor(va,pa)
                                .map_err(|_|Error::EmergencyCode)?,
                        }
                        tables.verify(va,pa,Rights::Rx)
                            .map_err(|_|Error::EmergencyCode)?;
                        proof[n]=CodePage {
                            linear:va,old_physical:pa,new_physical:pa,
                            old_executable:true,new_executable:true,
                            old_supervisor:true,new_supervisor:true,
                            efi_loader_code:true,
                        };
                    }
                    unios_bridgecheck::verify(
                        span,&proof[..span.pages],cpu.wp_enabled,cpu.nxe_enabled
                    ).map_err(|_|Error::EmergencyCode)?;
                }
                // Read only AFTER verifying old/new code page provenance.
                crate::emergency_handlers::validate_linked_bytes()
                    .map_err(|_|Error::EmergencyCode)?;
            }
            #[cfg(feature="lab-e2n-transition-closure")]
            {
                use unios_paging::transition_closure::{CutSpec,Witness};
                // No CR3 changes. Audit actual raw private PTE words,
                // separately from Builder::lookup and its high-level
                // verify() method, including *every* stack frame.
                // Never use this check as an ExitBootServices permit.
                let mut entry=None;
                for (seg,res) in image.segments.iter().take(image.count)
                    .zip(ledger.iter().take(image.count))
                {
                    if seg.flags&1!=0 &&
                       image.entry>=seg.vaddr &&
                       image.entry<seg.vaddr+seg.mem_size {
                        entry=Some(Witness{
                            va:image.entry,
                            pa:res.base+(image.entry-seg.page_start),
                        });
                    }
                }
                let entry=entry.ok_or(Error::TransitionCut)?;
                let (bridge_begin,bridge_end)=crate::bridge_span::bounds();
                let (normal,df)=crate::emergency_handlers::addresses();
                if bridge_end<=bridge_begin {
                    return Err(Error::TransitionCut);
                }
                let anchors=[bridge_begin,bridge_end-1,normal,df];
                let mut code=[Witness{va:0,pa:0};4];
                for (i,anchor) in anchors.iter().enumerate() {
                    // Independently recheck firmware executable ownership
                    // and old-CR3 page provenance for each target address.
                    let (va,pa)=crate::oldmap::audit_efi_code_page(
                        root_gate,*anchor,map,map_size,descriptor_size
                    ).map_err(|_|Error::TransitionCut)?;
                    code[i]=Witness{va:*anchor,pa:pa+(*anchor-va)};
                }
                let spec=CutSpec {
                    bridge_stack_pa:aux[1].base,
                    main_stack_pa:aux[2].base,
                    ist_stack_pa:aux[3].base,
                    descriptor_pa:aux[4].base,
                    context_pa:aux[5].base,
                    kernel_entry:entry,code,
                };
                let closure=tables.audit_transition_cut(spec)
                    .map_err(|_|Error::TransitionCut)?;
                if closure.verified_pages!=13 || closure.code_anchors!=4 ||
                   closure.bridge_stack_top!=aux[1].base+2*PAGE ||
                   closure.main_stack_top!=MAIN_STACK_VA+4*PAGE ||
                   closure.ist_stack_top!=IST_STACK_VA+4*PAGE {
                    return Err(Error::TransitionCut);
                }
                tables.audit_guard_pages().map_err(|_|Error::TransitionCut)?;
                #[cfg(feature="lab-e2q-cr3-code")]
                {
                    use unios_bridgecheck::Span;
                    let (start,end)=crate::cr3_terminal::bounds();
                    if end<=start {return Err(Error::SwitchCode);}
                    let span=Span::new(start,end).map_err(|_|Error::SwitchCode)?;
                    // Every machine-code page must be old-CR3 executable,
                    // EFI_LOADER_CODE-owned, and exactly same-PA RX in
                    // the new private PML4 before host checkpoint.
                    for i in 0..span.pages {
                        let page=span.page(i).ok_or(Error::SwitchCode)?;
                        let (va,pa)=crate::oldmap::audit_efi_code_page(
                            root_gate,page,map,map_size,descriptor_size
                        ).map_err(|_|Error::SwitchCode)?;
                        match tables.lookup(va).map_err(|_|Error::SwitchCode)? {
                            Some(x) if x.pa==pa && x.rights==Rights::Rx=>{},
                            Some(_)=>return Err(Error::SwitchCode),
                            None=>tables.map_efi_executable_anchor(va,pa)
                                .map_err(|_|Error::SwitchCode)?,
                        }
                        tables.verify(va,pa,Rights::Rx)
                            .map_err(|_|Error::SwitchCode)?;
                    }
                    if ((end-1)>>12).saturating_sub(start>>12)>1 {
                        // A finite span across at most two executable pages,
                        // EACH independently old/new verified above. The
                        // PE linker may place the same code across a 4K cut.
                        return Err(Error::SwitchCode);
                    }
                    // No active new CR3 yet. Bracket the newly mapped
                    // instruction range in the final host QMP manifest.
                    #[cfg(any(feature="lab-e2q-actual-cr3",feature="lab-e2r-active-descriptors"))]
                    {
                        switch_root=tables.root();
                        switch_bridge_top=aux[1].base+2*PAGE;
                        tables.verify(switch_bridge_top-1,
                            switch_bridge_top-1,Rights::Rw)
                            .map_err(|_|Error::SwitchCode)?;
                    }
                }
                #[cfg(feature="lab-e2r-high-entry-code")]
                {
                    // C1: first validate the *actual linked* high-entry stub
                    // under firmware old CR3 and the dormant private root.
                    // No descriptor activation, jump or ExitBS in this mode.
                    use unios_bridgecheck::Span;
                    let (begin,end)=crate::high_entry::bounds();
                    let span=Span::new(begin,end).map_err(|_|Error::SwitchCode)?;
                    for index in 0..span.pages {
                        let address=span.page(index).ok_or(Error::SwitchCode)?;
                        let (va,pa)=crate::oldmap::audit_efi_code_page(
                            root_gate,address,map,map_size,descriptor_size
                        ).map_err(|_|Error::SwitchCode)?;
                        match tables.lookup(va).map_err(|_|Error::SwitchCode)? {
                            Some(x) if x.pa==pa && x.rights==Rights::Rx=>{},
                            Some(_)=>return Err(Error::SwitchCode),
                            None=>tables.map_efi_executable_anchor(va,pa)
                                .map_err(|_|Error::SwitchCode)?,
                        }
                        tables.verify(va,pa,Rights::Rx)
                            .map_err(|_|Error::SwitchCode)?;
                    }
                }
                #[cfg(feature="lab-e2r-descriptor-code")]
                {
                    // E2r-C4: independently prove every page intersecting
                    // the *linked* descriptor install / CS-reload stub
                    // is EFI_LOADER_CODE supervisor RX under OLD firmware
                    // and equal-physical supervisor RX under dormant NEW CR3.
                    // No LGDT/LIDT/LTR is executed by this preflight.
                    use unios_bridgecheck::Span;
                    let (begin,end)=crate::high_descriptor::bounds();
                    let span=Span::new(begin,end).map_err(|_|Error::SwitchCode)?;
                    for index in 0..span.pages {
                        let address=span.page(index).ok_or(Error::SwitchCode)?;
                        let (va,pa)=crate::oldmap::audit_efi_code_page(
                            root_gate,address,map,map_size,descriptor_size
                        ).map_err(|_|Error::SwitchCode)?;
                        match tables.lookup(va).map_err(|_|Error::SwitchCode)? {
                            Some(x) if x.pa==pa && x.rights==Rights::Rx=>{},
                            Some(_)=>return Err(Error::SwitchCode),
                            None=>tables.map_efi_executable_anchor(va,pa)
                                .map_err(|_|Error::SwitchCode)?,
                        }
                        tables.verify(va,pa,Rights::Rx)
                            .map_err(|_|Error::SwitchCode)?;
                    }
                }
                #[cfg(feature="lab-e2r-active-descriptors")]
                {
                    // A distinct, finite actual-CR3+GDT/IDT/TSS CPU stub.
                    // Both old firmware and new private CR3 execute the
                    // exact same EFI-loader-code-owned physical bytes.
                    // This is a disposable QEMU feature, not default EFI.
                    use unios_bridgecheck::Span;
                    let (begin,end)=crate::live_descriptors::bounds();
                    let span=Span::new(begin,end).map_err(|_|Error::SwitchCode)?;
                    for index in 0..span.pages {
                        let address=span.page(index).ok_or(Error::SwitchCode)?;
                        let (va,pa)=crate::oldmap::audit_efi_code_page(
                            root_gate,address,map,map_size,descriptor_size
                        ).map_err(|_|Error::SwitchCode)?;
                        match tables.lookup(va).map_err(|_|Error::SwitchCode)? {
                            Some(x) if x.pa==pa && x.rights==Rights::Rx=>{},
                            Some(_)=>return Err(Error::SwitchCode),
                            None=>tables.map_efi_executable_anchor(va,pa)
                                .map_err(|_|Error::SwitchCode)?,
                        }
                        tables.verify(va,pa,Rights::Rx)
                            .map_err(|_|Error::SwitchCode)?;
                    }
                    // Never use unvalidated live start/end or loader frame.
                    if end-begin>1024 || ((end-1)>>12)-(begin>>12)>1 {
                        return Err(Error::SwitchCode);
                    }
                    tables.verify(unios_paging::DESCRIPTORS_VA,aux[4].base,Rights::Rw)
                        .map_err(|_|Error::SwitchCode)?;
                    tables.verify(unios_paging::IST_STACK_VA+4*PAGE-1,
                                  aux[3].base+4*PAGE-1,Rights::Rw)
                        .map_err(|_|Error::SwitchCode)?;
                }
                #[cfg(feature="lab-e2o-host-private-witness")]
                {
                    use crate::host_checkpoint::{PrivateTarget,PRIVATE_TARGETS};
                    let mut targets=[PrivateTarget{va:0,pa:0,rights:0};PRIVATE_TARGETS];
                    targets[0]=PrivateTarget{va:entry.va,pa:entry.pa,rights:1};
                    for i in 0..4 {
                        targets[i+1]=PrivateTarget{
                            va:code[i].va,pa:code[i].pa,rights:1,
                        };
                    }
                    for i in 0..2u64 {
                        targets[5+i as usize]=PrivateTarget{
                            va:aux[1].base+i*PAGE,pa:aux[1].base+i*PAGE,rights:2,
                        };
                    }
                    for i in 0..4u64 {
                        targets[7+i as usize]=PrivateTarget{
                            va:MAIN_STACK_VA+i*PAGE,
                            pa:aux[2].base+i*PAGE,rights:2,
                        };
                        targets[11+i as usize]=PrivateTarget{
                            va:IST_STACK_VA+i*PAGE,
                            pa:aux[3].base+i*PAGE,rights:2,
                        };
                    }
                    targets[15]=PrivateTarget{
                        va:DESCRIPTORS_VA,pa:aux[4].base,rights:2,
                    };
                    targets[16]=PrivateTarget{
                        va:CONTEXT_VA,pa:aux[5].base,rights:3,
                    };
                    targets[17]=PrivateTarget{
                        va:unios_paging::MAIN_GUARD_VA,pa:0,rights:0,
                    };
                    targets[18]=PrivateTarget{
                        va:unios_paging::IST_GUARD_VA,pa:0,rights:0,
                    };
                    #[cfg(feature="lab-e2q-cr3-code")]
                    {
                        let (start,end)=crate::cr3_terminal::bounds();
                        targets[19]=PrivateTarget{
                            va:start,pa:start,rights:1,
                        };
                        targets[20]=PrivateTarget{
                            va:end-1,pa:end-1,rights:1,
                        };
                    }
                    #[cfg(feature="lab-e2r-high-entry-code")]
                    {
                        let (start,end)=crate::high_entry::bounds();
                        if end<=start {return Err(Error::SwitchCode);}
                        targets[21]=PrivateTarget{va:start,pa:start,rights:1};
                        targets[22]=PrivateTarget{va:end-1,pa:end-1,rights:1};
                    }
                    #[cfg(feature="lab-e2r-descriptor-code")]
                    {
                        let (begin,end)=crate::high_descriptor::bounds();
                        if end<=begin {return Err(Error::SwitchCode);}
                        targets[23]=PrivateTarget{va:begin,pa:begin,rights:1};
                        targets[24]=PrivateTarget{va:end-1,pa:end-1,rights:1};
                    }
                    #[cfg(feature="lab-e2r-active-descriptors")]
                    {
                        let (begin,end)=crate::live_descriptors::bounds();
                        if end<=begin {return Err(Error::SwitchCode);}
                        targets[25]=PrivateTarget{va:begin,pa:begin,rights:1};
                        targets[26]=PrivateTarget{va:end-1,pa:end-1,rights:1};
                    }
                    // The VM is halted a SECOND time by an independent
                    // host that reads the physical PML4 and all traversed
                    // tables; only host-issued 'O' can unlock the guest.
                    if !crate::host_checkpoint::private_checkpoint(
                        tables.root(),tables.used_tables(),&targets,
                    ) {return Err(Error::NewHostProof);}
                }
            }
            #[cfg(all(feature="lab-e2l-lab-descriptors",not(feature="lab-e2r-active-descriptors")))]
            {
                // Build a second test-only register image at the CURRENT
                // identity-mapped EFI physical VA; dormant high-half GDT
                // cannot be installed under old firmware CR3. This happens
                // after every E2k code-page/provenance proof has succeeded.
                let (ud,ist)=crate::exception_lab::addresses();
                if ud==ist {return Err(Error::ActiveLab);}
                #[cfg(feature="lab-e2m-smoke-real-df")]
                let real_df=crate::exception_lab::real_df_address();
                #[cfg(feature="lab-e2m-smoke-real-df")]
                let executable_anchors=[ud,ist,real_df];
                #[cfg(not(feature="lab-e2m-smoke-real-df"))]
                let executable_anchors=[ud,ist,ist];
                for handler in executable_anchors {
                    let (va,pa)=crate::oldmap::audit_efi_code_page(
                        root_gate,handler,map,map_size,descriptor_size
                    ).map_err(|_|Error::ActiveLab)?;
                    // The active lab still never loads the new PML4.
                    match tables.lookup(va).map_err(|_|Error::ActiveLab)? {
                        Some(existing) if existing.pa==pa && existing.rights==Rights::Rx => {}
                        Some(_) => return Err(Error::ActiveLab),
                        None => tables.map_efi_executable_anchor(va,pa)
                            .map_err(|_|Error::ActiveLab)?,
                    }
                    tables.verify(va,pa,Rights::Rx)
                        .map_err(|_|Error::ActiveLab)?;
                }
                let desc_pa=aux[4].base;
                let main_top=aux[2].base+4*PAGE;
                let ist_top=aux[3].base+4*PAGE;
                let desc=unsafe {
                    slice::from_raw_parts_mut(desc_pa as *mut u8,unios_descriptors::PAGE)
                };
                let (normal,old_df)=crate::emergency_handlers::addresses();
                // Software vector8 is restricted to explicitly named
                // QEMU IST smoke mode, not claimed as a real double fault.
                #[cfg(feature="lab-e2m-smoke-real-df")]
                let df=real_df;
                #[cfg(all(feature="lab-e2l-smoke-ist",not(feature="lab-e2m-smoke-real-df")))]
                let df=ist;
                #[cfg(not(any(feature="lab-e2l-smoke-ist",feature="lab-e2m-smoke-real-df")))]
                let df=old_df;
                #[cfg(feature="lab-e2m-smoke-real-df")]
                let img=unios_descriptors::build_lab_df(
                    desc,desc_pa,main_top,ist_top,normal,df,ud,
                ).map_err(|_|Error::ActiveLab)?;
                #[cfg(not(feature="lab-e2m-smoke-real-df"))]
                let img=unios_descriptors::build_lab_ud(
                    desc,desc_pa,main_top,ist_top,normal,df,ud,
                ).map_err(|_|Error::ActiveLab)?;
                #[cfg(feature="lab-e2m-smoke-real-df")]
                unios_descriptors::verify_lab_df(
                    desc,img,main_top,ist_top,normal,df,ud
                ).map_err(|_|Error::ActiveLab)?;
                #[cfg(not(feature="lab-e2m-smoke-real-df"))]
                unios_descriptors::verify_lab_ud(
                    desc,img,main_top,ist_top,normal,df,ud
                ).map_err(|_|Error::ActiveLab)?;
                // Old CR3 identity rights were already checked for all
                // reservation pages, including physical metadata and IST.
                // Terminal fault tests intentionally never return to EFI.
                #[cfg(feature="lab-e2l-smoke-ud")]
                unsafe {crate::exception_lab::trigger_ud(img);}
                #[cfg(feature="lab-e2l-smoke-ist")]
                unsafe {crate::exception_lab::trigger_ist(
                    img,aux[3].base,ist_top,
                );}
                #[cfg(feature="lab-e2m-smoke-real-df")]
                unsafe {crate::exception_lab::trigger_real_df(
                    img,aux[3].base,ist_top,
                );}
            }
            if tables.used_tables() < 4 {return Err(Error::Paging);}
            // NO write to CR3. Missing shared-linear EFI code, exception
            // descriptors, independent old-CR3 proof and final map cut.
        }
    }
    #[cfg(not(feature = "lab-e2b-staging"))]
    let _ = payload;

    #[cfg(feature="lab-e2p-exitbs-contract")]
    {
        // This is a separately obtained map; the earlier E2a snapshot
        // cannot be reused as a final map key after QEMU checkpoints.
        // All allocations and host acknowledgements have finished.
        // No console, UART or firmware allocation is called between
        // the final map read and any opt-in ExitBootServices call.
        let mut final_size=MAP_CAP;
        let mut final_key=0usize;
        let mut final_stride=0usize;
        let mut final_version=0u32;
        let status=unsafe {
            (bs.get_memory_map)(&mut final_size,buffer,&mut final_key,
                                &mut final_stride,&mut final_version)
        };
        unios_exitbs::validate_map(status,unios_exitbs::Snapshot{
            bytes:final_size,key:final_key,
            stride:final_stride,version:final_version,
        },MAP_CAP).map_err(|_|Error::FinalMap)?;
        unios_bootmap::validate(
            map,final_size,final_stride,final_version,&ledger[..count]
        ).map_err(|_|Error::FinalMap)?;
        #[cfg(feature="lab-e2p-actual-exitbs")]
        {
            // Terminal disposable-QEMU ONLY. After success, even
            // returning through Rust/UEFI is forbidden. No GDT/CR3
            // change is attempted; only a raw debug port write.
            // A stale key may be retried only after a fresh GetMemoryMap.
            for attempt in 0..unios_exitbs::ATTEMPTS {
                // First attempt uses just-validated map. A second
                // attempt MUST fetch and validate a *fresh* key.
                let key=if attempt==0 {final_key} else {
                    let mut n=MAP_CAP;
                    let mut k=0usize;
                    let mut st=0usize;
                    let mut ver=0u32;
                    let ret=unsafe {
                        (bs.get_memory_map)(&mut n,buffer,&mut k,&mut st,&mut ver)
                    };
                    if unios_exitbs::validate_map(ret,unios_exitbs::Snapshot{
                        bytes:n,key:k,stride:st,version:ver,
                    },MAP_CAP).is_err() {
                        unsafe {terminal_debug(0x3a);}
                    }
                    if unios_bootmap::validate(
                        map,n,st,ver,&ledger[..count]
                    ).is_err() {unsafe {terminal_debug(0x3a);}}
                    k
                };
                let status=unsafe {(bs.exit_boot_services)(image_handle,key)};
                match unios_exitbs::after_exit(status,attempt) {
                    Ok(unios_exitbs::ExitDecision::Success)=>{
                        // NO EFI CALLS AFTER SUCCESS; NO return-to-firmware.
                        #[cfg(all(feature="lab-e2q-actual-cr3",not(feature="lab-e2r-active-descriptors")))]
                        unsafe {crate::cr3_terminal::activate(
                            switch_root,switch_bridge_top,
                        );}
                        #[cfg(feature="lab-e2r-active-descriptors")]
                        unsafe {crate::live_descriptors::activate(
                            switch_root,switch_bridge_top,
                            unios_paging::DESCRIPTORS_VA,
                        );}
                        #[cfg(all(not(feature="lab-e2q-actual-cr3"),not(feature="lab-e2r-active-descriptors")))]
                        unsafe {terminal_debug(0x39);}
                    }
                    Ok(unios_exitbs::ExitDecision::Retry)=>{},
                    Err(_)=>unsafe {terminal_debug(0x3a);},
                }
            }
            unsafe {terminal_debug(0x3a);}
        }
    }
    #[cfg(not(feature="lab-e2p-exitbs-contract"))]
    let _=image_handle;
    Ok(report.descriptors)
}

#[cfg(feature="lab-e2p-actual-exitbs")]
#[inline(never)]
unsafe fn terminal_debug(signal:u8)->! {
    core::arch::asm!("cli","out dx,al",in("dx") 0xf4u16,
                     in("al") signal,options(nomem,nostack));
    loop {core::arch::asm!("hlt",options(nomem,nostack));}
}


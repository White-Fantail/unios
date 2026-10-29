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

pub fn preflight(services: *mut BootServices, image: &Image, payload: &[u8]) -> Result<usize, Error> {
    if services.is_null() { return Err(Error::BadServices); }
    let bs = unsafe { &*services };
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
        if !crate::host_checkpoint::host_checkpoint() {
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
            if tables.used_tables() < 4 {return Err(Error::Paging);}
            // NO write to CR3. Missing shared-linear EFI code, exception
            // descriptors, independent old-CR3 proof and final map cut.
        }
    }
    #[cfg(not(feature = "lab-e2b-staging"))]
    let _ = payload;

    // In future E2c, a *new* final map key is required immediately before
    // ExitBootServices. This preflight key is not reusable after console calls.
    Ok(report.descriptors)
}

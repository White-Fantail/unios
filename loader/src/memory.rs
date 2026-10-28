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
        crate::oldmap::check_owned_identity(&ledger[..count])
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

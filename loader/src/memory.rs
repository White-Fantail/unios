// LAB-H1 E2a: reserve physical frames and validate a real UEFI memory map.
// NO writes to unverified physical addresses, no ExitBootServices, no CR3.
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

pub fn preflight(services: *mut BootServices, image: &Image) -> Result<usize, Error> {
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
    let buffer = ptr::addr_of_mut!(MAP_BUFFER.0).cast::<u8>();
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
    // In future E2b, a *new* final map key is required immediately before
    // ExitBootServices. This preflight key is not reusable after console calls.
    Ok(report.descriptors)
}

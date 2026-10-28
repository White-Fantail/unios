// LAB-H1 E2b disposable QEMU/OVMF diagnostic only, not portable firmware mapping.
// Each page is checked in current firmware CR3 before a physical reservation
// is used as a linear staging destination. FIRST root-table read is a narrow
// probe of QEMU/OVMF's old physical->linear mapping. If this is inaccessible,
// the VM may fault, which is HOST FAILURE, never an accepted boot result.
//
// This is not the post-ExitBootServices bridge proof (LC-O02 remains open).
use core::{arch::asm, ptr};
use unios_bootmap::{Reservation, PAGE};

#[derive(Clone, Copy, Debug)]
pub enum Error { Mode, Root, NotMapped, NonIdentity, NotWritable, NotExecutable, NotLoaderCode, Address }

const ADDR_MASK: u64 = 0x000f_ffff_ffff_f000;
const PRESENT: u64 = 1;
const WRITABLE: u64 = 2;
const LARGE: u64 = 1 << 7;

fn read_cr3_cr4() -> (u64, u64) {
    let (cr3, cr4): (u64, u64);
    unsafe {
        asm!("mov {}, cr3", out(reg) cr3, options(nomem, nostack, preserves_flags));
        asm!("mov {}, cr4", out(reg) cr4, options(nomem, nostack, preserves_flags));
    }
    (cr3, cr4)
}

// SAFETY: the first firmware page-table physical root is experimentally read
// at the same linear address. Only the pinned disposable QEMU guest runs this
// probe. A fault means no usable proof and must never produce the lab exit.
unsafe fn translate(root: u64, va: u64) -> Result<(u64,bool,bool), Error> {
    let mut table_pa = root;
    let mut can_write = true;
    let mut can_execute = true;
    for (depth, shift) in [39u32, 30, 21, 12].iter().enumerate() {
        let index = (va >> *shift) & 0x1ff;
        let entry_addr = table_pa.checked_add(index * 8).ok_or(Error::Address)?;
        if entry_addr >= (1u64 << 48) { return Err(Error::Address); }
        let entry = ptr::read_volatile(entry_addr as *const u64);
        if entry & PRESENT == 0 { return Err(Error::NotMapped); }
        can_write &= entry & WRITABLE != 0;
        can_execute &= entry & (1u64 << 63) == 0;
        if depth == 1 && entry & LARGE != 0 {
            let mask = (1u64 << 30) - 1;
            return Ok(((entry & ADDR_MASK & !mask) | (va & mask),can_write,can_execute));
        }
        if depth == 2 && entry & LARGE != 0 {
            let mask = (1u64 << 21) - 1;
            return Ok(((entry & ADDR_MASK & !mask) | (va & mask),can_write,can_execute));
        }
        if depth == 3 { return Ok(((entry & ADDR_MASK) | (va & 0xfff), can_write, can_execute)); }
        table_pa = entry & ADDR_MASK;
        if table_pa == 0 { return Err(Error::Root); }
    }
    Err(Error::Root)
}
pub fn check_owned_identity(reservations: &[Reservation]) -> Result<u64, Error> {
    let (cr3, cr4) = read_cr3_cr4();
    // LAB-H1 four-level, no LA57, no PCID; no 5-level walker shortcut.
    if (cr4 & ((1<<12) | (1<<17))) != 0 { return Err(Error::Mode); }
    let root = cr3 & ADDR_MASK;
    if root == 0 || root % PAGE != 0 { return Err(Error::Root); }
    // Audit root self-access before using firmware PA values as Rust VA.
    let (mapped_root, _, _) = unsafe { translate(root, root)? };
    if mapped_root != root { return Err(Error::NonIdentity); }
    for res in reservations {
        let length = res.pages.checked_mul(PAGE).ok_or(Error::Address)?;
        let end = res.base.checked_add(length).ok_or(Error::Address)?;
        let mut va = res.base;
        while va < end {
            let (mapped, rw, _) = unsafe { translate(root, va)? };
            if mapped != va { return Err(Error::NonIdentity); }
            if !rw { return Err(Error::NotWritable); }
            va = va.checked_add(PAGE).ok_or(Error::Address)?;
        }
    }
    Ok(cr3)
}

pub fn audit_efi_code_page(
    anchor: u64, map: &[u8], used: usize, stride: usize,
) -> Result<(u64,u64),Error> {
    // One-page executable provenance only. This is NOT a byte-bounded
    // trampoline or proof of a live RIP while CR3 is written.
    if anchor < 0x10_0000 || anchor >= (1u64<<32) ||
        used > map.len() || stride < 40 || used%stride != 0 {
        return Err(Error::Address);
    }
    let va=anchor & !(PAGE-1);
    let cr3=read_cr3_cr4().0;
    let root=cr3 & ADDR_MASK;
    let (first,_,exec) = unsafe { translate(root,anchor)? };
    if !exec {return Err(Error::NotExecutable);}
    let (last,_,end_exec) = unsafe { translate(root,va+PAGE-1)? };
    if !end_exec || (first & !(PAGE-1)) != (last & !(PAGE-1)) {
        return Err(Error::NotExecutable);
    }
    let pa=first & !(PAGE-1);
    // Firmware descriptors are variable-stride and end-exclusive. The
    // whole code page must be of type EFI_LOADER_CODE=1 (not arbitrary RAM).
    let mut owner=false;
    for i in 0..used/stride {
        let o=i*stride;
        let kind=u32::from_le_bytes(map[o..o+4].try_into().map_err(|_|Error::Address)?);
        let begin=u64::from_le_bytes(map[o+8..o+16].try_into().map_err(|_|Error::Address)?);
        let pages=u64::from_le_bytes(map[o+24..o+32].try_into().map_err(|_|Error::Address)?);
        let end=begin.checked_add(pages.checked_mul(PAGE).ok_or(Error::Address)?)
            .ok_or(Error::Address)?;
        if kind==1 && pa>=begin && pa.checked_add(PAGE).ok_or(Error::Address)?<=end {
            owner=true;break;
        }
    }
    if !owner {return Err(Error::NotLoaderCode);}
    Ok((va,pa))
}

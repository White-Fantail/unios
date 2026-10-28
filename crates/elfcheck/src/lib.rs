#![no_std]

// LAB-ELF1: one strictly bounded, non-relocating, fixed-high-half kernel image.
// Parsing is deliberately separate from UEFI allocation and memory writes.
pub const BASE: u64 = 0xffff_ffff_8000_0000;
pub const LIMIT: u64 = 0xffff_ffff_8400_0000;
pub const PAGE: u64 = 4096;
pub const MAX_LOAD: usize = 8;
pub const MAX_STAGED: u64 = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Header, Machine, Type, Bounds, HeaderCount, UnsupportedHeader,
    Alignment, Permissions, Overlap, Size, Entry, Relocations,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Segment {
    pub file_offset: u64,
    pub file_size: u64,
    pub mem_size: u64,
    pub vaddr: u64,
    pub page_start: u64,
    pub page_end: u64,
    pub flags: u32,
}
const EMPTY: Segment = Segment {
    file_offset: 0, file_size: 0, mem_size: 0, vaddr: 0,
    page_start: 0, page_end: 0, flags: 0,
};

#[derive(Debug)]
pub struct Image {
    pub entry: u64,
    pub segments: [Segment; MAX_LOAD],
    pub count: usize,
    pub staged_bytes: u64,
}
fn u16at(s: &[u8], o: usize) -> Result<u16, Error> {
    let b = s.get(o..o.checked_add(2).ok_or(Error::Bounds)?).ok_or(Error::Bounds)?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}
fn u32at(s: &[u8], o: usize) -> Result<u32, Error> {
    let b = s.get(o..o.checked_add(4).ok_or(Error::Bounds)?).ok_or(Error::Bounds)?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}
fn u64at(s: &[u8], o: usize) -> Result<u64, Error> {
    let b = s.get(o..o.checked_add(8).ok_or(Error::Bounds)?).ok_or(Error::Bounds)?;
    Ok(u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
}
fn offset(n: u64) -> Result<usize, Error> {
    usize::try_from(n).map_err(|_| Error::Bounds)
}
fn array_item(start: u64, n: usize, stride: u64, len: usize) -> Result<usize, Error> {
    let o = start.checked_add((n as u64).checked_mul(stride).ok_or(Error::Bounds)?)
        .ok_or(Error::Bounds)?;
    let end = o.checked_add(stride).ok_or(Error::Bounds)?;
    if end > len as u64 { return Err(Error::Bounds); }
    offset(o)
}
fn page_up(n: u64) -> Result<u64, Error> {
    Ok(n.checked_add(PAGE - 1).ok_or(Error::Bounds)? & !(PAGE - 1))
}
pub fn validate(bytes: &[u8]) -> Result<Image, Error> {
    if bytes.len() < 64 || &bytes[..4] != b"\x7fELF"
        || bytes[4] != 2 || bytes[5] != 1 || bytes[6] != 1
        || u16at(bytes, 52)? != 64 {
        return Err(Error::Header);
    }
    if u16at(bytes, 16)? != 2 { return Err(Error::Type); } // ET_EXEC only
    if u16at(bytes, 18)? != 62 || u32at(bytes, 20)? != 1 {
        return Err(Error::Machine);
    }
    let entry = u64at(bytes, 24)?;
    let phoff = u64at(bytes, 32)?;
    let shoff = u64at(bytes, 40)?;
    let phentsize = u16at(bytes, 54)? as u64;
    let phnum = u16at(bytes, 56)? as usize;
    let shentsize = u16at(bytes, 58)? as u64;
    let shnum = u16at(bytes, 60)? as usize;
    if phentsize != 56 || phnum == 0 || phnum > 32 {
        return Err(Error::HeaderCount);
    }
    if (shnum > 0 && (shentsize != 64 || shoff == 0))
        || (shnum == 0 && shoff != 0) {
        return Err(Error::Header);
    }
    // Reject any residual relocation sections, including static relocation
    // metadata left in an ET_EXEC image, rather than implementing fixups.
    for i in 0..shnum {
        let o = array_item(shoff, i, shentsize, bytes.len())?;
        let kind = u32at(bytes, o + 4)?;
        if (kind == 4 || kind == 9 || kind == 19) && u64at(bytes, o + 32)? != 0 {
            return Err(Error::Relocations);
        }
    }
    let mut image = Image {
        entry, segments: [EMPTY; MAX_LOAD], count: 0, staged_bytes: 0,
    };
    for i in 0..phnum {
        let o = array_item(phoff, i, phentsize, bytes.len())?;
        let kind = u32at(bytes, o)?;
        let flags = u32at(bytes, o + 4)?;
        if kind == 0 || kind == 4 || kind == 6 || kind == 0x6474_e552 {
            continue; // NULL, NOTE, PHDR, GNU_RELRO (no payload copied)
        }
        if kind == 0x6474_e551 {
            if (flags & 1) != 0 { return Err(Error::Permissions); }
            continue; // non-executable GNU_STACK
        }
        if kind != 1 { return Err(Error::UnsupportedHeader); }
        if image.count == MAX_LOAD { return Err(Error::HeaderCount); }
        let file_offset = u64at(bytes, o + 8)?;
        let vaddr = u64at(bytes, o + 16)?;
        let file_size = u64at(bytes, o + 32)?;
        let mem_size = u64at(bytes, o + 40)?;
        let align = u64at(bytes, o + 48)?;
        if mem_size == 0 || file_size > mem_size
            || file_offset.checked_add(file_size).ok_or(Error::Bounds)? > bytes.len() as u64 {
            return Err(Error::Bounds);
        }
        if align == 0 || !align.is_power_of_two()
            || (file_offset & (align - 1)) != (vaddr & (align - 1)) {
            return Err(Error::Alignment);
        }
        // All loadable pages are supervisor-only, R + X xor W, never W+X.
        if flags & !7 != 0 || flags & 4 == 0 || flags & 3 == 3 {
            return Err(Error::Permissions);
        }
        let end = vaddr.checked_add(mem_size).ok_or(Error::Bounds)?;
        let page_start = vaddr & !(PAGE - 1);
        let page_end = page_up(end)?;
        if page_start < BASE || page_end > LIMIT || page_end <= page_start {
            return Err(Error::Bounds);
        }
        for prev in image.segments.iter().take(image.count) {
            if page_start < prev.page_end && prev.page_start < page_end {
                return Err(Error::Overlap); // reject even compatible shared pages
            }
        }
        image.staged_bytes = image.staged_bytes.checked_add(page_end - page_start)
            .ok_or(Error::Size)?;
        if image.staged_bytes > MAX_STAGED { return Err(Error::Size); }
        image.segments[image.count] = Segment {
            file_offset, file_size, mem_size, vaddr, page_start, page_end, flags,
        };
        image.count += 1;
    }
    if image.count == 0 { return Err(Error::HeaderCount); }
    let mut executable_entry = false;
    for s in image.segments.iter().take(image.count) {
        if s.flags & 1 != 0
            && entry >= s.vaddr
            && entry < s.vaddr + s.mem_size {
            executable_entry = true;
        }
    }
    if !executable_entry { return Err(Error::Entry); }
    Ok(image)
}

#[cfg(test)]
extern crate std;
#[cfg(test)]
mod tests {
    use super::*;
    fn w16(b: &mut [u8], n: usize, x: u16) { b[n..n+2].copy_from_slice(&x.to_le_bytes()); }
    fn w32(b: &mut [u8], n: usize, x: u32) { b[n..n+4].copy_from_slice(&x.to_le_bytes()); }
    fn w64(b: &mut [u8], n: usize, x: u64) { b[n..n+8].copy_from_slice(&x.to_le_bytes()); }
    fn valid() -> std::vec::Vec<u8> {
        let mut b = std::vec![0u8; 8192];
        b[..4].copy_from_slice(b"\x7fELF");
        b[4] = 2; b[5] = 1; b[6] = 1;
        w16(&mut b, 16, 2); w16(&mut b, 18, 62); w32(&mut b, 20, 1);
        w64(&mut b, 24, BASE + 4096);
        w64(&mut b, 32, 64); w16(&mut b, 52, 64);
        w16(&mut b, 54, 56); w16(&mut b, 56, 1);
        w32(&mut b, 64, 1); w32(&mut b, 68, 5);
        w64(&mut b, 72, 4096); w64(&mut b, 80, BASE + 4096);
        w64(&mut b, 96, 4); w64(&mut b, 104, 4096);
        w64(&mut b, 112, 4096);
        b
    }
    #[test] fn accepts_fixed_elf() { assert_eq!(validate(&valid()).unwrap().count, 1); }
    #[test] fn rejects_pie() { let mut b=valid(); w16(&mut b,16,3); assert_eq!(validate(&b).unwrap_err(), Error::Type); }
    #[test] fn rejects_wx() { let mut b=valid(); w32(&mut b,68,7); assert_eq!(validate(&b).unwrap_err(), Error::Permissions); }
    #[test] fn rejects_non_rx_entry() { let mut b=valid(); w64(&mut b,24,BASE+8192); assert_eq!(validate(&b).unwrap_err(), Error::Entry); }
    #[test] fn rejects_bad_range() { let mut b=valid(); w64(&mut b,80,u64::MAX - 4095); assert!(validate(&b).is_err()); }
    #[test] fn rejects_bad_alignment() { let mut b=valid(); w64(&mut b,112,3); assert_eq!(validate(&b).unwrap_err(), Error::Alignment); }
    #[test] fn rejects_short_payload() { let mut b=valid(); w64(&mut b,96,8192); assert_eq!(validate(&b).unwrap_err(), Error::Bounds); }
    #[test] fn rejects_dynamic_phdr() { let mut b=valid(); w32(&mut b,64,2); assert_eq!(validate(&b).unwrap_err(), Error::UnsupportedHeader); }
    #[test] fn rejects_relocations() {
        let mut b=valid(); w64(&mut b,40,128); w16(&mut b,58,64); w16(&mut b,60,1);
        w32(&mut b,132,4); w64(&mut b,160,24);
        assert_eq!(validate(&b).unwrap_err(), Error::Relocations);
    }
}

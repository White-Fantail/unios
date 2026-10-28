#![no_std]

// LAB-H1: bounded, descriptor-stride-aware UEFI memory map preflight.
// The raw map is firmware-owned metadata. It never grants physical access.
pub const PAGE: u64 = 4096;
pub const MAP_CAP: usize = 64 * 1024;
pub const EFI_MEMORY_DESCRIPTOR_VERSION: u32 = 1;
pub const EFI_LOADER_DATA: u32 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    MapSize, Version, DescriptorStride, DescriptorRange,
    DescriptorOverlap, ReservationRange, ReservationOverlap,
    UnownedReservation, TooMany,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reservation {
    pub base: u64,
    pub pages: u64,
    pub id: u8,
}
pub const EMPTY: Reservation = Reservation { base: 0, pages: 0, id: 0 };

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Report {
    pub descriptors: usize,
    pub reservations: usize,
    pub reserved_pages: u64,
}
fn read32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at+1], bytes[at+2], bytes[at+3]])
}
fn read64(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes([
        bytes[at], bytes[at+1], bytes[at+2], bytes[at+3],
        bytes[at+4], bytes[at+5], bytes[at+6], bytes[at+7],
    ])
}
fn extent(base: u64, pages: u64) -> Option<u64> {
    if pages == 0 || base % PAGE != 0 { return None; }
    base.checked_add(pages.checked_mul(PAGE)?)
}
pub fn validate(
    buffer: &[u8],
    used: usize,
    stride: usize,
    version: u32,
    reservations: &[Reservation],
) -> Result<Report, Error> {
    if version != EFI_MEMORY_DESCRIPTOR_VERSION { return Err(Error::Version); }
    // OVMF may return a descriptor stride greater than the 40-byte minimum.
    // Never use size_of::<MemoryDescriptor>() as an array stride.
    if stride < 40 || stride % 8 != 0 || stride > 256 {
        return Err(Error::DescriptorStride);
    }
    if used == 0 || used > MAP_CAP || used > buffer.len() || used % stride != 0 {
        return Err(Error::MapSize);
    }
    if reservations.len() > 16 { return Err(Error::TooMany); }
    let count = used / stride;
    for index in 0..count {
        let start = index * stride;
        let kind = read32(buffer, start);
        let base = read64(buffer, start + 8);
        let pages = read64(buffer, start + 24);
        let end = extent(base, pages).ok_or(Error::DescriptorRange)?;
        if end <= base { return Err(Error::DescriptorRange); }
        // The UEFI type namespace admits vendor ranges. Do not trust an
        // unrecognized type as available RAM; only LOADER_DATA grants ownership.
        let _ = kind;
        for other in 0..index {
            let earlier = other * stride;
            let other_base = read64(buffer, earlier + 8);
            let other_end = extent(other_base, read64(buffer, earlier + 24))
                .ok_or(Error::DescriptorRange)?;
            if base < other_end && other_base < end {
                return Err(Error::DescriptorOverlap);
            }
        }
    }
    let mut sum = 0u64;
    for (i, r) in reservations.iter().enumerate() {
        let end = extent(r.base, r.pages).ok_or(Error::ReservationRange)?;
        sum = sum.checked_add(r.pages).ok_or(Error::ReservationRange)?;
        for prior in &reservations[..i] {
            let prior_end = extent(prior.base, prior.pages)
                .ok_or(Error::ReservationRange)?;
            if r.base < prior_end && prior.base < end {
                return Err(Error::ReservationOverlap);
            }
        }
        // Every page must be covered by the firmware map's LoaderData
        // region; descriptor splitting is accepted only when contiguous.
        let mut cursor = r.base;
        while cursor < end {
            let mut next = cursor;
            for j in 0..count {
                let at = j * stride;
                let kind = read32(buffer, at);
                let start = read64(buffer, at + 8);
                let stop = extent(start, read64(buffer, at + 24))
                    .ok_or(Error::DescriptorRange)?;
                if kind == EFI_LOADER_DATA && start <= cursor && cursor < stop {
                    next = core::cmp::min(end, stop);
                    break;
                }
            }
            if next <= cursor { return Err(Error::UnownedReservation); }
            cursor = next;
        }
    }
    Ok(Report { descriptors: count, reservations: reservations.len(), reserved_pages: sum })
}

#[cfg(test)]
extern crate std;
#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;
    use std::vec::Vec;

    fn map(stride: usize) -> Vec<u8> {
        let mut b = vec![0u8; stride * 3];
        for (i, (kind, addr, pages)) in [
            (EFI_LOADER_DATA, 0x100000u64, 8u64),
            (7u32, 0x108000, 8),
            (EFI_LOADER_DATA, 0x110000, 8),
        ].iter().enumerate() {
            let o = i * stride;
            b[o..o+4].copy_from_slice(&kind.to_le_bytes());
            b[o+8..o+16].copy_from_slice(&addr.to_le_bytes());
            b[o+24..o+32].copy_from_slice(&pages.to_le_bytes());
        }
        b
    }
    fn valid(b: &[u8], stride: usize, reserves: &[Reservation]) -> Result<Report, Error> {
        validate(b, b.len(), stride, 1, reserves)
    }
    const R: Reservation = Reservation { base: 0x100000, pages: 2, id: 1 };

    #[test] fn handles_uefi_padded_stride() {
        assert_eq!(valid(&map(48), 48, &[R]).unwrap().descriptors, 3);
    }
    #[test] fn handles_minimum_stride() { assert!(valid(&map(40), 40, &[R]).is_ok()); }
    #[test] fn rejects_wrong_version() {
        let b = map(48);
        assert_eq!(validate(&b, b.len(), 48, 2, &[R]), Err(Error::Version));
    }
    #[test] fn rejects_truncated_and_bad_stride() {
        let b = map(48);
        assert_eq!(validate(&b, b.len()-1, 48, 1, &[R]), Err(Error::MapSize));
        assert_eq!(validate(&b, b.len(), 41, 1, &[R]), Err(Error::DescriptorStride));
    }
    #[test] fn rejects_wrong_type() {
        let b = map(48);
        let other = Reservation { base: 0x108000, pages: 1, id: 2 };
        assert_eq!(valid(&b, 48, &[other]), Err(Error::UnownedReservation));
    }
    #[test] fn rejects_overlap_of_reserved_ranges() {
        let b = map(48);
        let other = Reservation { base: 0x101000, pages: 1, id: 2 };
        assert_eq!(valid(&b, 48, &[R,other]), Err(Error::ReservationOverlap));
    }
    #[test] fn rejects_overflow_reservation() {
        let b=map(48);
        assert_eq!(valid(&b,48,&[Reservation{base:u64::MAX-4095,pages:2,id:3}]),
                   Err(Error::ReservationRange));
    }
    #[test] fn rejects_overlapping_descriptors() {
        let mut b=map(48);
        b[48+8..48+16].copy_from_slice(&0x101000u64.to_le_bytes());
        assert_eq!(valid(&b,48,&[R]), Err(Error::DescriptorOverlap));
    }
    #[test] fn rejects_unaligned_descriptor() {
        let mut b=map(48);
        b[8..16].copy_from_slice(&0x100001u64.to_le_bytes());
        assert_eq!(valid(&b,48,&[R]), Err(Error::DescriptorRange));
    }
    #[test] fn rejects_missing_reservation() {
        let b=map(48);
        assert_eq!(valid(&b,48,&[Reservation{base:0x120000,pages:1,id:4}]),
                   Err(Error::UnownedReservation));
    }
    #[test] fn rejects_oversized_map_buffer() {
        let b=vec![0u8; MAP_CAP+48];
        assert_eq!(validate(&b,b.len(),48,1,&[R]), Err(Error::MapSize));
    }
}

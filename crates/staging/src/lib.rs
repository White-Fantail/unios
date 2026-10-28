#![no_std]

// L1 E2b: deterministic page-granular ELF copy and BSS fill. The caller
// must independently establish that the destination is an owned, mapped
// and writable frame *before* creating a mutable slice pointing to it.
use unios_elf::{Segment, PAGE};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error { Length, Range, Overflow, Mismatch }

fn layout<'a>(segment: &Segment, bytes: &'a [u8], destination_len: usize)
    -> Result<(usize, &'a [u8], usize), Error>
{
    let span = segment.page_end.checked_sub(segment.page_start)
        .ok_or(Error::Overflow)?;
    if span == 0 || span % PAGE != 0 ||
        usize::try_from(span).map_err(|_| Error::Length)? != destination_len {
        return Err(Error::Length);
    }
    let prefix = usize::try_from(segment.vaddr.checked_sub(segment.page_start)
        .ok_or(Error::Overflow)?).map_err(|_| Error::Overflow)?;
    let file_offset = usize::try_from(segment.file_offset).map_err(|_| Error::Overflow)?;
    let file_len = usize::try_from(segment.file_size).map_err(|_| Error::Overflow)?;
    let mem_len = usize::try_from(segment.mem_size).map_err(|_| Error::Overflow)?;
    let file_end = file_offset.checked_add(file_len).ok_or(Error::Overflow)?;
    let mem_end = prefix.checked_add(mem_len).ok_or(Error::Overflow)?;
    if file_len > mem_len || mem_end > destination_len {
        return Err(Error::Range);
    }
    let source = bytes.get(file_offset..file_end).ok_or(Error::Range)?;
    Ok((prefix, source, mem_end))
}
pub fn stage(segment: &Segment, source: &[u8], dest: &mut [u8])
    -> Result<(), Error>
{
    let (prefix, file, _) = layout(segment, source, dest.len())?;
    // All destination pages are explicitly zeroed, including BSS, page
    // padding before p_vaddr and trailing slack.
    dest.fill(0);
    let end = prefix.checked_add(file.len()).ok_or(Error::Overflow)?;
    dest.get_mut(prefix..end).ok_or(Error::Range)?.copy_from_slice(file);
    Ok(())
}
pub fn verify(segment: &Segment, source: &[u8], dest: &[u8])
    -> Result<(), Error>
{
    let (prefix, file, mem_end) = layout(segment, source, dest.len())?;
    let file_end = prefix.checked_add(file.len()).ok_or(Error::Overflow)?;
    if dest.get(prefix..file_end) != Some(file)
        || dest[..prefix].iter().any(|&b| b != 0)
        || dest[file_end..mem_end].iter().any(|&b| b != 0)
        || dest[mem_end..].iter().any(|&b| b != 0)
    {
        return Err(Error::Mismatch);
    }
    Ok(())
}

#[cfg(test)]
extern crate std;
#[cfg(test)]
mod tests {
    use super::*;
    use std::{vec, vec::Vec};
    fn seg(file_offset: u64, file_size: u64, mem_size: u64, vaddr_offset: u64)
        -> Segment {
        let start = unios_elf::BASE;
        Segment {
            file_offset, file_size, mem_size, vaddr: start + vaddr_offset,
            page_start: start, page_end: start + PAGE * 2, flags: 6,
        }
    }
    #[test] fn exact_copy_and_zeroed_bss_padding() {
        let image: Vec<u8> = (0..64u8).collect();
        let s = seg(4, 20, 32, 39);
        let mut d = vec![0x55; (PAGE * 2) as usize];
        stage(&s, &image, &mut d).unwrap();
        assert_eq!(&d[39..59], &image[4..24]);
        assert!(d[..39].iter().all(|x| *x == 0));
        assert!(d[59..].iter().all(|x| *x == 0));
        assert_eq!(verify(&s, &image, &d), Ok(()));
    }
    #[test] fn rejects_short_source_without_modifying_destination() {
        let mut d = vec![0x55; 8192];
        assert_eq!(stage(&seg(60, 20, 32, 2), &[1u8;64], &mut d), Err(Error::Range));
        assert!(d.iter().all(|x| *x == 0x55));
    }
    #[test] fn rejects_wrong_target_length() {
        let mut d = vec![9u8;4096];
        assert_eq!(stage(&seg(0, 3, 10, 0), &[1,2,3], &mut d), Err(Error::Length));
    }
    #[test] fn rejects_excessive_memory_span() {
        let mut d = vec![7u8;8192];
        assert_eq!(stage(&seg(0, 1, 8193, 1), &[1], &mut d), Err(Error::Range));
    }
    #[test] fn rejects_payload_exceeding_memsz() {
        let mut d = vec![7u8;8192];
        assert_eq!(stage(&seg(0, 4, 3, 0), &[1,2,3,4], &mut d), Err(Error::Range));
    }
    #[test] fn detects_mutated_data() {
        let s = seg(0, 3, 3, 0);
        let mut d=vec![0;8192];
        stage(&s, &[1,2,3], &mut d).unwrap();
        d[2] = 9;
        assert_eq!(verify(&s,&[1,2,3],&d),Err(Error::Mismatch));
    }
    #[test] fn detects_mutated_bss() {
        let s = seg(0, 3, 100, 0);
        let mut d=vec![0;8192];
        stage(&s, &[1,2,3], &mut d).unwrap();
        d[90] = 1;
        assert_eq!(verify(&s,&[1,2,3],&d),Err(Error::Mismatch));
    }
    #[test] fn detects_mutated_page_padding() {
        let s = seg(0, 3, 100, 0);
        let mut d=vec![0;8192];
        stage(&s, &[1,2,3], &mut d).unwrap();
        d[8191] = 0xff;
        assert_eq!(verify(&s,&[1,2,3],&d),Err(Error::Mismatch));
    }
}

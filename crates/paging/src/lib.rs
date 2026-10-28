#![no_std]

// LAB-H1 E2c: 4-level, 4KiB-only, supervisor-only private page-table
// construction. NEVER load this CR3 before the bridge/IDT/IST and
// ExitBootServices contracts are independently verified.
use unios_elf::{Segment, PAGE, BASE, LIMIT};

pub const TABLE_PAGES: usize = 128; // 512KiB hard cap
pub const ENTRIES: usize = 512;
pub const TABLE_WORDS: usize = TABLE_PAGES * ENTRIES;
pub const PHYS_ADDR_MASK: u64 = 0x000f_ffff_ffff_f000;
const P: u64 = 1;
const W: u64 = 2;
const U: u64 = 4;
const PS: u64 = 1 << 7;
const NX: u64 = 1 << 63;

pub const CONTEXT_VA: u64 = LIMIT - 0x30000;
pub const DESCRIPTORS_VA: u64 = LIMIT - 0x40000;
pub const MAIN_GUARD_VA: u64 = LIMIT - 0x20000;
pub const MAIN_STACK_VA: u64 = MAIN_GUARD_VA + PAGE;
pub const IST_GUARD_VA: u64 = LIMIT - 0x10000;
pub const IST_STACK_VA: u64 = IST_GUARD_VA + PAGE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rights { Rx, Ro, Rw }
impl Rights {
    pub fn bits(self) -> u64 {
        match self { Self::Rx => P, Self::Ro => P | NX, Self::Rw => P | W | NX }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Alignment, BadPhysical, BadVirtual, Range, Overlap, Alias, Guard,
    Capacity, Corrupt, Permission,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Leaf { pub pa: u64, pub rights: Rights }

pub fn canonical(va: u64) -> bool {
    let top = va >> 48;
    top == if (va & (1<<47)) == 0 { 0 } else { 0xffff }
}
fn guard(va: u64) -> bool { va == MAIN_GUARD_VA || va == IST_GUARD_VA }
fn addr_ok(pa: u64) -> bool {
    pa & (PAGE - 1) == 0 && pa & !(PHYS_ADDR_MASK) == 0
}
fn index(va: u64, level: usize) -> usize {
    ((va >> [39,30,21,12][level]) & 511) as usize
}
fn rights_of(pte: u64) -> Result<Rights, Error> {
    if pte & P == 0 || pte & (U | PS) != 0 { return Err(Error::Permission); }
    match (pte & W != 0, pte & NX != 0) {
        (false, false) => Ok(Rights::Rx),
        (false, true) => Ok(Rights::Ro),
        (true, true) => Ok(Rights::Rw),
        (true, false) => Err(Error::Permission),
    }
}
pub struct Builder<'a> {
    frames: &'a mut [u64],
    root_pa: u64,
    used: usize,
    depth: [u8; TABLE_PAGES],
}
impl<'a> Builder<'a> {
    // Backing must be the CURRENT-old-CR3 verified RW linear mapping of
    // 512KiB of private EFI-allocated table frames; zeroing happens once.
    pub fn new(backing: &'a mut [u64], root_pa: u64) -> Result<Self, Error> {
        if backing.len() != TABLE_WORDS { return Err(Error::Capacity); }
        if !addr_ok(root_pa) || root_pa == 0 { return Err(Error::BadPhysical); }
        let end = root_pa.checked_add(TABLE_PAGES as u64 * PAGE).ok_or(Error::BadPhysical)?;
        if end > PHYS_ADDR_MASK + PAGE { return Err(Error::BadPhysical); }
        backing.fill(0);
        Ok(Self { frames: backing, root_pa, used: 1, depth: [0; TABLE_PAGES] })
    }
    pub fn root(&self) -> u64 { self.root_pa }
    pub fn used_tables(&self) -> usize { self.used }
    fn table_index(&self, pa: u64) -> Result<usize, Error> {
        let off = pa.checked_sub(self.root_pa).ok_or(Error::Corrupt)?;
        if off % PAGE != 0 || off / PAGE >= self.used as u64 {
            return Err(Error::Corrupt);
        }
        Ok((off / PAGE) as usize)
    }
    fn walk_table(&mut self, va: u64) -> Result<usize, Error> {
        let mut current = 0usize;
        for level in 0..3 {
            let pos = current * ENTRIES + index(va, level);
            let val = self.frames[pos];
            if val & P == 0 {
                if self.used >= TABLE_PAGES { return Err(Error::Capacity); }
                let next = self.used;
                self.used += 1;
                self.depth[next] = (level + 1) as u8;
                let pa = self.root_pa.checked_add(next as u64 * PAGE).ok_or(Error::BadPhysical)?;
                self.frames[pos] = pa | P | W; // effective rights constrained by leaf
                current = next;
            } else {
                if val & (U | PS) != 0 || val & W == 0 || val & NX != 0 {
                    return Err(Error::Corrupt);
                }
                current = self.table_index(val & PHYS_ADDR_MASK)?;
                if self.depth[current] != (level + 1) as u8 { return Err(Error::Corrupt); }
            }
        }
        Ok(current)
    }
    fn pa_aliased(&self, pa: u64) -> bool {
        for t in 0..self.used {
            if self.depth[t] != 3 { continue; }
            for i in 0..ENTRIES {
                let entry = self.frames[t * ENTRIES + i];
                if entry & P != 0 && (entry & PHYS_ADDR_MASK) == pa {
                    return true;
                }
            }
        }
        false
    }
    pub fn map4k(&mut self, va: u64, pa: u64, rights: Rights) -> Result<(), Error> {
        if !canonical(va) || va & (PAGE-1) != 0 { return Err(Error::BadVirtual); }
        if guard(va) { return Err(Error::Guard); }
        if !addr_ok(pa) || pa == 0 { return Err(Error::BadPhysical); }
        let table_end = self.root_pa + TABLE_PAGES as u64 * PAGE;
        if pa >= self.root_pa && pa < table_end { return Err(Error::Alias); }
        if self.pa_aliased(pa) { return Err(Error::Alias); }
        if self.lookup(va)?.is_some() { return Err(Error::Overlap); }
        let pt = self.walk_table(va)?;
        let slot = pt * ENTRIES + index(va, 3);
        if self.frames[slot] & P != 0 { return Err(Error::Overlap); }
        self.frames[slot] = pa | rights.bits();
        Ok(())
    }
    pub fn map_kernel_segment(&mut self, seg: &Segment, physical: u64) -> Result<(), Error> {
        if seg.page_start < BASE || seg.page_end > LIMIT ||
            seg.page_start >= seg.page_end || seg.page_start & (PAGE-1) != 0 ||
            seg.page_end & (PAGE-1) != 0 {
            return Err(Error::Range);
        }
        let rights = match (seg.flags & 1 != 0, seg.flags & 2 != 0) {
            (true, false) => Rights::Rx,
            (false, true) => Rights::Rw,
            (false, false) => Rights::Ro,
            (true, true) => return Err(Error::Permission),
        };
        for i in 0..(seg.page_end - seg.page_start)/PAGE {
            let va = seg.page_start.checked_add(i * PAGE).ok_or(Error::Range)?;
            let pa = physical.checked_add(i * PAGE).ok_or(Error::Range)?;
            self.map4k(va,pa,rights)?;
        }
        Ok(())
    }
    pub fn map_stack(&mut self, va: u64, pa: u64, pages: usize) -> Result<(), Error> {
        if pages == 0 || pages > 4 || (va != MAIN_STACK_VA && va != IST_STACK_VA) {
            return Err(Error::Range);
        }
        for i in 0..pages {
            self.map4k(va + i as u64 * PAGE, pa + i as u64 * PAGE, Rights::Rw)?;
        }
        Ok(())
    }
    pub fn map_owned_low_bridge_stack(&mut self, pa: u64) -> Result<(), Error> {
        // Same-linear under old/new CR3, only after old-CR3 ownership proof.
        if pa >= BASE || pa < 0x10_0000 { return Err(Error::Range); }
        self.map4k(pa,pa,Rights::Rw)?;
        self.map4k(pa+PAGE,pa+PAGE,Rights::Rw)
    }
    pub fn lookup(&self, va: u64) -> Result<Option<Leaf>, Error> {
        if !canonical(va) { return Err(Error::BadVirtual); }
        let mut current=0usize;
        for level in 0..4 {
            let slot=current*ENTRIES + index(va,level);
            let val=self.frames[slot];
            if val & P == 0 { return Ok(None); }
            if val & U != 0 { return Err(Error::Permission); }
            if level == 3 {
                let rights=rights_of(val)?;
                return Ok(Some(Leaf {
                    pa:(val&PHYS_ADDR_MASK) | (va & (PAGE-1)),
                    rights,
                }));
            }
            if val & PS != 0 || val & NX != 0 { return Err(Error::Corrupt); }
            current=self.table_index(val&PHYS_ADDR_MASK)?;
            if self.depth[current] != (level+1) as u8 { return Err(Error::Corrupt); }
        }
        Err(Error::Corrupt)
    }
    pub fn audit_guard_pages(&self) -> Result<(), Error> {
        if self.lookup(MAIN_GUARD_VA)?.is_some() ||
           self.lookup(IST_GUARD_VA)?.is_some() { return Err(Error::Guard); }
        Ok(())
    }
    pub fn verify(&self, va: u64, pa: u64, expected: Rights) -> Result<(), Error> {
        let leaf=self.lookup(va)?.ok_or(Error::Corrupt)?;
        if leaf.pa != pa || leaf.rights != expected { return Err(Error::Permission); }
        Ok(())
    }
}

#[cfg(test)]
extern crate std;
#[cfg(test)]
mod tests {
    use super::*;
    use std::{vec, boxed::Box};
    fn builder() -> Builder<'static> {
        let frame=Box::leak(vec![0xdeadbeefu64; TABLE_WORDS].into_boxed_slice());
        Builder::new(frame,0x0200_0000).unwrap()
    }
    const TEXT: u64=BASE;
    const TEXT_PA: u64=0x0100_0000;

    #[test] fn kernel_text_rx_and_data_rw_nx() {
        let mut b=builder();
        b.map4k(TEXT,TEXT_PA,Rights::Rx).unwrap();
        b.map4k(TEXT+PAGE,TEXT_PA+PAGE,Rights::Ro).unwrap();
        b.map4k(TEXT+2*PAGE,TEXT_PA+2*PAGE,Rights::Rw).unwrap();
        b.verify(TEXT,TEXT_PA,Rights::Rx).unwrap();
        b.verify(TEXT+PAGE,TEXT_PA+PAGE,Rights::Ro).unwrap();
        b.verify(TEXT+2*PAGE,TEXT_PA+2*PAGE,Rights::Rw).unwrap();
        assert_eq!(b.audit_guard_pages(),Ok(()));
        assert!(b.used_tables() >= 4);
    }
    #[test] fn rejects_mapping_both_guards() {
        let mut b=builder();
        for g in [MAIN_GUARD_VA, IST_GUARD_VA] {
            assert_eq!(b.map4k(g,TEXT_PA,Rights::Rw),Err(Error::Guard));
        }
    }
    #[test] fn rejects_duplicate_physical_alias() {
        let mut b=builder();
        b.map4k(TEXT,TEXT_PA,Rights::Rx).unwrap();
        assert_eq!(b.map4k(TEXT+PAGE,TEXT_PA,Rights::Rw),Err(Error::Alias));
    }
    #[test] fn rejects_page_table_backing_alias() {
        let mut b=builder();
        assert_eq!(b.map4k(TEXT,0x0200_0000,Rights::Rw),Err(Error::Alias));
    }
    #[test] fn rejects_duplicate_virtual_page() {
        let mut b=builder();
        b.map4k(TEXT,TEXT_PA,Rights::Rx).unwrap();
        assert_eq!(b.map4k(TEXT,TEXT_PA+PAGE,Rights::Rx),Err(Error::Overlap));
    }
    #[test] fn rejects_noncanonical_va() {
        let mut b=builder();
        assert_eq!(b.map4k(0x0000_8000_0000_0000,TEXT_PA,Rights::Rx),Err(Error::BadVirtual));
    }
    #[test] fn rejects_unaligned_physical() {
        let mut b=builder();
        assert_eq!(b.map4k(TEXT,TEXT_PA+1,Rights::Rx),Err(Error::BadPhysical));
    }
    #[test] fn no_user_access_and_no_writable_rx() {
        let mut b=builder();b.map4k(TEXT,TEXT_PA,Rights::Rx).unwrap();
        let page=b.lookup(TEXT).unwrap().unwrap();
        assert_eq!(page.rights,Rights::Rx);
        let idx=3*ENTRIES+index(TEXT,3);
        // Physical table slots are never user accessible.
        assert!(b.frames.iter().all(|x| x & U == 0));
        assert_eq!(b.frames[idx]&U,0);
    }
    #[test] fn actual_kernel_segment_permissions() {
        let mut b=builder();
        for (flags,rights) in [(5,Rights::Rx),(4,Rights::Ro),(6,Rights::Rw)] {
            let i=match flags {5=>0,4=>1,_=>2};
            let seg=Segment {
                file_offset:0,file_size:4,mem_size:PAGE,vaddr:TEXT+i*PAGE,
                page_start:TEXT+i*PAGE,page_end:TEXT+(i+1)*PAGE,flags,
            };
            b.map_kernel_segment(&seg,TEXT_PA+i*PAGE).unwrap();
            b.verify(seg.page_start,TEXT_PA+i*PAGE,rights).unwrap();
        }
    }
    #[test] fn capacity_bound() {
        let mut b=builder();
        // 125 separate 2MiB regions force >128 tables.
        let mut rejected=false;
        for i in 0..125u64 {
            let va=0xffff_8000_0000_0000u64+i*(1<<21);
            if b.map4k(va,TEXT_PA+i*PAGE,Rights::Ro)==Err(Error::Capacity) {
                rejected=true;break;
            }
        }
        assert!(rejected);
    }
}

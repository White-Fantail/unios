//! E2n independent read-only verification of the *built but uninstalled*
//! private PML4. This decodes raw PTE words without Builder::lookup,
//! and never changes CR3 / ExitBootServices / hardware state.
use super::{Rights, PAGE, TABLE_PAGES, TABLE_WORDS, PHYS_ADDR_MASK, BASE, LIMIT,
            MAIN_STACK_VA, IST_STACK_VA, MAIN_GUARD_VA, IST_GUARD_VA,
            DESCRIPTORS_VA, CONTEXT_VA};

const P:u64=1;
const W:u64=2;
const U:u64=4;
const PS:u64=1<<7;
const NX:u64=1<<63;
const ENTRIES:usize=512;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Error { FrameCount, Root, Pointer, Superpage, User, Rights,
    NotMapped, ChangedPhysical, Guard, Entry, Code, Stack, Descriptor }
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Witness { pub va:u64,pub pa:u64 }
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct CutSpec {
    pub bridge_stack_pa:u64, pub main_stack_pa:u64, pub ist_stack_pa:u64,
    pub descriptor_pa:u64, pub context_pa:u64,
    pub kernel_entry:Witness,
    // Actual linker-defined EFI assembly begin / end-1 + two emergency
    // handlers. All four MUST separately have old-CR3/LOADER_CODE proofs.
    pub code:[Witness;4],
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Report { pub verified_pages:usize, pub code_anchors:usize,
    pub bridge_stack_top:u64, pub main_stack_top:u64, pub ist_stack_top:u64 }

fn canonical(va:u64)->bool {
    (va>>48)==if va&(1<<47)==0 {0} else {0xffff}
}
fn checked_end(base:u64,pages:u64)->Option<u64> {
    if base==0 || base&(PAGE-1)!=0 {return None;}
    base.checked_add(pages.checked_mul(PAGE)?)
}
/// Pure host-checkable walk of exactly four nonlarge, supervisor table
/// levels, starting at frame 0. No pointer deref or firmware dependency.
pub fn raw_leaf(frames:&[u64],root:u64,used:usize,va:u64)
    ->Result<Option<(u64,Rights)>,Error>
{
    if frames.len()!=TABLE_WORDS || used<4 || used>TABLE_PAGES {
        return Err(Error::FrameCount);
    }
    if root<PAGE || root&(PAGE-1)!=0 || root&!PHYS_ADDR_MASK!=0 ||
       !canonical(va) {return Err(Error::Root);}
    let mut table=0usize;
    for (level,shift) in [39u32,30,21,12].iter().enumerate() {
        let i=((va>>*shift)&511) as usize;
        let word=*frames.get(table*ENTRIES+i).ok_or(Error::Pointer)?;
        if word&P==0 {
            if word!=0 {return Err(Error::Pointer);}
            return Ok(None);
        }
        if word&U!=0 {return Err(Error::User);}
        if word&PS!=0 {return Err(Error::Superpage);}
        if level==3 {
            let rights=match (word&W!=0,word&NX!=0) {
                (false,false)=>Rights::Rx,
                (false,true)=>Rights::Ro,
                (true,true)=>Rights::Rw,
                (true,false)=>return Err(Error::Rights),
            };
            return Ok(Some(((word&PHYS_ADDR_MASK)|(va&(PAGE-1)),rights)));
        }
        if word&NX!=0 || word&W==0 {return Err(Error::Rights);}
        let child=word&PHYS_ADDR_MASK;
        let off=child.checked_sub(root).ok_or(Error::Pointer)?;
        if off%PAGE!=0 || off/PAGE>=used as u64 {return Err(Error::Pointer);}
        table=(off/PAGE) as usize;
    }
    Err(Error::Pointer)
}
fn require(frames:&[u64],root:u64,used:usize,va:u64,pa:u64,rights:Rights)
    ->Result<(),Error>
{
    match raw_leaf(frames,root,used,va)? {
        Some((got,actual)) if got==pa && actual==rights => Ok(()),
        Some((got,_)) if got!=pa=>Err(Error::ChangedPhysical),
        Some(_)=>Err(Error::Rights),
        None=>Err(Error::NotMapped),
    }
}
fn stack(frames:&[u64],root:u64,used:usize,va:u64,pa:u64,pages:u64)
    ->Result<u64,Error>
{
    let physical_end=checked_end(pa,pages).ok_or(Error::Stack)?;
    let virtual_end=checked_end(va,pages).ok_or(Error::Stack)?;
    if physical_end>=1<<32 {return Err(Error::Stack);}
    for n in 0..pages {
        require(frames,root,used,va+n*PAGE,pa+n*PAGE,Rights::Rw)?;
        require(frames,root,used,va+(n+1)*PAGE-1,
                pa+(n+1)*PAGE-1,Rights::Rw)?;
    }
    Ok(virtual_end)
}
pub fn audit(frames:&[u64],root:u64,used:usize,s:CutSpec)
    ->Result<Report,Error>
{
    if !canonical(s.kernel_entry.va) ||
       s.kernel_entry.va<BASE || s.kernel_entry.va>=LIMIT {
        return Err(Error::Entry);
    }
    if s.kernel_entry.pa==0 || s.kernel_entry.pa>=1<<32 {
        return Err(Error::Entry);
    }
    require(frames,root,used,s.kernel_entry.va,s.kernel_entry.pa,Rights::Rx)?;
    for item in s.code {
        if item.va<0x10_0000 || item.va>=1<<32 ||
           item.pa<0x10_0000 || item.pa>=1<<32 ||
           item.va&(PAGE-1)!=item.pa&(PAGE-1) {
            return Err(Error::Code);
        }
        require(frames,root,used,item.va,item.pa,Rights::Rx)?;
    }
    let bridge_top=stack(frames,root,used,s.bridge_stack_pa,s.bridge_stack_pa,2)?;
    let main_top=stack(frames,root,used,MAIN_STACK_VA,s.main_stack_pa,4)?;
    let ist_top=stack(frames,root,used,IST_STACK_VA,s.ist_stack_pa,4)?;
    for guard in [MAIN_GUARD_VA,IST_GUARD_VA] {
        if raw_leaf(frames,root,used,guard)?.is_some(){
            return Err(Error::Guard);
        }
    }
    if checked_end(s.descriptor_pa,1).ok_or(Error::Descriptor)?>1<<32 ||
       checked_end(s.context_pa,1).ok_or(Error::Descriptor)?>1<<32 {
        return Err(Error::Descriptor);
    }
    require(frames,root,used,DESCRIPTORS_VA,s.descriptor_pa,Rights::Rw)?;
    require(frames,root,used,CONTEXT_VA,s.context_pa,Rights::Ro)?;
    Ok(Report{verified_pages:2+4+4+1+1+1,
        code_anchors:4,bridge_stack_top:bridge_top,
        main_stack_top:main_top,ist_stack_top:ist_top})
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::Builder;
    use std::{vec,boxed::Box};
    fn fixture()->(Builder<'static>,CutSpec) {
        let f=Box::leak(vec![0u64;TABLE_WORDS].into_boxed_slice());
        let mut b=Builder::new(f,0x0200_0000).unwrap();
        let code=[
            Witness{va:0x1510_0010,pa:0x1510_0010},
            Witness{va:0x1510_0013,pa:0x1510_0013},
            Witness{va:0x1520_0000,pa:0x1520_0000},
            Witness{va:0x1530_0000,pa:0x1530_0000},
        ];
        b.map4k(BASE,0x0800_0000,Rights::Rx).unwrap();
        for (va,pa) in [(0x1510_0000,0x1510_0000),
                         (0x1520_0000,0x1520_0000),
                         (0x1530_0000,0x1530_0000)] {
            b.map_efi_executable_anchor(va,pa).unwrap();
        }
        b.map_owned_low_bridge_stack(0x0300_0000).unwrap();
        b.map_stack(MAIN_STACK_VA,0x0400_0000,4).unwrap();
        b.map_stack(IST_STACK_VA,0x0500_0000,4).unwrap();
        b.map4k(DESCRIPTORS_VA,0x0600_0000,Rights::Rw).unwrap();
        b.map4k(CONTEXT_VA,0x0700_0000,Rights::Ro).unwrap();
        let spec=CutSpec {
            bridge_stack_pa:0x0300_0000,main_stack_pa:0x0400_0000,
            ist_stack_pa:0x0500_0000,descriptor_pa:0x0600_0000,
            context_pa:0x0700_0000,
            kernel_entry:Witness{va:BASE,pa:0x0800_0000},code,
        };
        (b,spec)
    }
    #[test] fn complete_cut_is_verified(){
        let (b,s)=fixture();
        let r=b.audit_transition_cut(s).unwrap();
        assert_eq!(r.verified_pages,13);
        assert_eq!(r.code_anchors,4);
        assert_eq!(r.bridge_stack_top,0x0300_2000);
    }
    #[test] fn wrong_kernel_entry_physical_rejected(){
        let (b,mut s)=fixture();s.kernel_entry.pa+=PAGE;
        assert_eq!(b.audit_transition_cut(s),Err(Error::ChangedPhysical));
    }
    #[test] fn unwitnessed_code_rejected(){
        let (b,mut s)=fixture();s.code[3].pa+=PAGE;
        assert_eq!(b.audit_transition_cut(s),Err(Error::ChangedPhysical));
    }
    #[test] fn wrong_stack_physical_rejected(){
        let (b,mut s)=fixture();s.ist_stack_pa+=PAGE;
        assert_eq!(b.audit_transition_cut(s),Err(Error::ChangedPhysical));
    }
    #[test] fn context_must_be_read_only(){
        let (mut b,s)=fixture();
        let target=CONTEXT_VA;let _=b.map4k(target+PAGE,0x0710_0000,Rights::Rw);
        let ix=((target>>12)&511) as usize;
        // Search only present leaves in private backing for the exact
        // expected RO PTE and silently reject any changed permission.
        for word in b.frames.iter_mut() {
            if *word==0x0700_0000|Rights::Ro.bits() {
                *word=0x0700_0000|Rights::Rw.bits();break;
            }
        }
        let _=ix;
        assert_eq!(b.audit_transition_cut(s),Err(Error::Rights));
    }
    #[test] fn missing_middle_stack_page_rejected(){
        let (mut b,s)=fixture();
        for word in b.frames.iter_mut(){
            if *word==0x0400_1000|Rights::Rw.bits(){*word=0;break;}
        }
        assert_eq!(b.audit_transition_cut(s),Err(Error::NotMapped));
    }
    #[test] fn unauthorized_user_access_rejected(){
        let (mut b,s)=fixture();
        for word in b.frames.iter_mut(){
            if *word==0x0600_0000|Rights::Rw.bits(){*word|=U;break;}
        }
        assert_eq!(b.audit_transition_cut(s),Err(Error::User));
    }
    #[test] fn wrong_parent_pointer_rejected(){
        let (mut b,s)=fixture();
        let rootindex=((BASE>>39)&511) as usize;
        b.frames[rootindex]=0x9900_0003;
        assert_eq!(b.audit_transition_cut(s),Err(Error::Pointer));
    }
    #[test] fn huge_page_not_supported(){
        let (mut b,s)=fixture();
        let rootindex=((BASE>>39)&511) as usize;
        b.frames[rootindex]|=PS;
        assert_eq!(b.audit_transition_cut(s),Err(Error::Superpage));
    }
    #[test] fn descriptor_missing_rejected(){
        let (mut b,s)=fixture();
        for word in b.frames.iter_mut(){
            if *word==0x0600_0000|Rights::Rw.bits(){*word=0;break;}
        }
        assert_eq!(b.audit_transition_cut(s),Err(Error::NotMapped));
    }
    #[test] fn bad_entry_noncanonical_rejected(){
        let (b,mut s)=fixture();s.kernel_entry.va=0x8000_0000_0000;
        assert_eq!(b.audit_transition_cut(s),Err(Error::Entry));
    }
}

#![no_std]
// E2g: metadata-only gate before the first old-CR3 physical-as-linear
// dereference. A valid memory descriptor DOES NOT establish identity mapping.
pub const PAGE:u64=4096;
pub const MAX_MAP:usize=64*1024;
const PHYS_MASK:u64=0x000f_ffff_ffff_f000;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Error {
    CpuMode, Cr3LowBits, RootRange, BadMap, NoOwnership, InUse,
    ChangedCr3, Overlap, NonRamType,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Report {pub cr3:u64,pub root_pa:u64,pub descriptor_type:u32}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Owned {pub base:u64,pub pages:u64}
fn u32le(s:&[u8],n:usize)->u32 {
    u32::from_le_bytes(s[n..n+4].try_into().unwrap())
}
fn u64le(s:&[u8],n:usize)->u64 {
    u64::from_le_bytes(s[n..n+8].try_into().unwrap())
}
pub fn check(
    cr3:u64,cr4:u64,map:&[u8],used:usize,stride:usize,
    reserved:&[Owned],
)->Result<Report,Error> {
    if (cr4 & ((1<<12)|(1<<17)))!=0 || (cr4&(1<<5))==0 {
        return Err(Error::CpuMode);
    }
    if (cr3 & 0xfff & !0x18)!=0 {return Err(Error::Cr3LowBits);}
    let root=cr3&PHYS_MASK;
    // LAB-H1 pinned 256MiB QEMU profile. This is a conservative 32-bit
    // address-space bound, NOT evidence of linear access to root.
    if root<0x10_0000 || root>0xffff_f000 || root & (PAGE-1)!=0 {
        return Err(Error::RootRange);
    }
    if used==0 || used>MAX_MAP || used>map.len() ||
        stride<40 || stride>256 || stride%8!=0 || used%stride!=0 {
        return Err(Error::BadMap);
    }
    // The earlier bootmap crate audits global descriptor overlaps.
    // Here, independently inspect ownership of this ONE root frame.
    let end=root.checked_add(PAGE).ok_or(Error::RootRange)?;
    let mut owner=None;
    for i in 0..used/stride {
        let o=i*stride;
        let kind=u32le(map,o);
        let start=u64le(map,o+8);
        let pages=u64le(map,o+24);
        if pages==0 || start & (PAGE-1)!=0 {return Err(Error::BadMap);}
        let stop=start.checked_add(pages.checked_mul(PAGE).ok_or(Error::BadMap)?)
            .ok_or(Error::BadMap)?;
        if start<end && root<stop {
            if root<start || end>stop {return Err(Error::BadMap);}
            if owner.is_some(){return Err(Error::Overlap);}
            owner=Some(kind);
        }
    }
    // Firmware/loader allocated data; NOT conventional/free RAM, MMIO,
    // persistent/runtime memory, memory map buffer, or arbitrary bytes.
    let kind=owner.ok_or(Error::NoOwnership)?;
    if kind!=2 && kind!=4 {return Err(Error::NonRamType);}
    for r in reserved {
        let stop=r.base.checked_add(r.pages.checked_mul(PAGE).ok_or(Error::BadMap)?)
            .ok_or(Error::BadMap)?;
        if root<stop && r.base<end {return Err(Error::InUse);}
    }
    Ok(Report{cr3,root_pa:root,descriptor_type:kind})
}
pub fn verify_unchanged(root:Report,now_cr3:u64)->Result<(),Error>{
    if now_cr3!=root.cr3 {Err(Error::ChangedCr3)} else {Ok(())}
}
#[cfg(test)]
extern crate std;
#[cfg(test)]
mod tests{
    use super::*;
    use std::vec;
    const ROOT:u64=0x12_0000;
    fn fixture(kind:u32)->std::vec::Vec<u8>{
        let mut b=vec![0u8;96];
        for (i,(t,base,pages)) in [(kind,ROOT,1u64),(2,0x20_0000,10)].iter().enumerate(){
            let o=i*48;
            b[o..o+4].copy_from_slice(&t.to_le_bytes());
            b[o+8..o+16].copy_from_slice(&base.to_le_bytes());
            b[o+24..o+32].copy_from_slice(&pages.to_le_bytes());
        }
        b
    }
    fn gate(kind:u32)->Result<Report,Error> {
        let map=fixture(kind);
        check(ROOT,1<<5,&map,map.len(),48,&[Owned{base:0x20_0000,pages:3}])
    }
    #[test] fn boot_services_data_root(){assert_eq!(gate(4).unwrap().root_pa,ROOT);}
    #[test] fn loader_data_root(){assert_eq!(gate(2).unwrap().descriptor_type,2);}
    #[test] fn rejects_conventional(){assert_eq!(gate(7),Err(Error::NonRamType));}
    #[test] fn rejects_mmio(){assert_eq!(gate(11),Err(Error::NonRamType));}
    #[test] fn rejects_missing_descriptor(){
        let mut b=fixture(4);
        b[8..16].copy_from_slice(&0x30_0000u64.to_le_bytes());
        assert_eq!(check(ROOT,1<<5,&b,b.len(),48,&[]),Err(Error::NoOwnership));
    }
    #[test] fn rejects_reserved_root(){let b=fixture(4);
        assert_eq!(check(ROOT,1<<5,&b,b.len(),48,&[Owned{base:ROOT,pages:1}]),Err(Error::InUse));}
    #[test] fn rejects_la57(){let b=fixture(4);
        assert_eq!(check(ROOT,(1<<5)|(1<<12),&b,b.len(),48,&[]),Err(Error::CpuMode));}
    #[test] fn rejects_pcide(){let b=fixture(4);
        assert_eq!(check(ROOT,(1<<5)|(1<<17),&b,b.len(),48,&[]),Err(Error::CpuMode));}
    #[test] fn rejects_missing_pae(){let b=fixture(4);
        assert_eq!(check(ROOT,0,&b,b.len(),48,&[]),Err(Error::CpuMode));}
    #[test] fn rejects_cr3_bits(){let b=fixture(4);
        assert_eq!(check(ROOT|1,1<<5,&b,b.len(),48,&[]),Err(Error::Cr3LowBits));}
    #[test] fn accepts_pwt_pcd_cr3(){let b=fixture(4);
        assert!(check(ROOT|0x18,1<<5,&b,b.len(),48,&[]).is_ok());}
    #[test] fn rejects_bad_stride(){let b=fixture(4);
        assert_eq!(check(ROOT,1<<5,&b,b.len(),41,&[]),Err(Error::BadMap));}
    #[test] fn rejects_short_map(){let b=fixture(4);
        assert_eq!(check(ROOT,1<<5,&b,b.len()+1,48,&[]),Err(Error::BadMap));}
    #[test] fn detects_duplicate_root_descriptor(){let mut b=fixture(4);
        b[48+8..48+16].copy_from_slice(&ROOT.to_le_bytes());
        assert_eq!(check(ROOT,1<<5,&b,b.len(),48,&[]),Err(Error::Overlap));}
    #[test] fn rejects_root_at_zero(){let b=fixture(4);
        assert_eq!(check(0,1<<5,&b,b.len(),48,&[]),Err(Error::RootRange));}
    #[test] fn detects_cr3_change(){let r=gate(4).unwrap();
        assert_eq!(verify_unchanged(r,ROOT+PAGE),Err(Error::ChangedCr3));}
    #[test] fn same_cr3(){let r=gate(4).unwrap();
        assert_eq!(verify_unchanged(r,ROOT),Ok(()));}
}

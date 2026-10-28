#![no_std]
// Pure x86_64 LAB-H1 execution-profile gate. No MSR or CRx writes here.
// WP/NXE are reported; they MUST be enabled and rechecked at the future cut.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct State {
    pub max_extended_leaf: u32,
    pub cpuid_leaf1_edx: u32,
    pub cpuid_ext_edx: u32,
    pub cr0: u64,
    pub cr4: u64,
    pub efer: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Audit { pub wp_enabled: bool, pub nxe_enabled: bool }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error { NoMsr, NoPae, NoNx, NoLongMode, PagingMode, La57, Pcide, NotLongActive }
pub fn check(s: State) -> Result<Audit, Error> {
    if s.cpuid_leaf1_edx & (1 << 5) == 0 {return Err(Error::NoMsr);}
    if s.cpuid_leaf1_edx & (1 << 6) == 0 || s.cr4 & (1 << 5) == 0 {return Err(Error::NoPae);}
    if s.max_extended_leaf < 0x8000_0001 || s.cpuid_ext_edx & (1 << 20) == 0 {return Err(Error::NoNx);}
    if s.cpuid_ext_edx & (1 << 29) == 0 {return Err(Error::NoLongMode);}
    if s.cr0 & (1 << 31) == 0 {return Err(Error::PagingMode);}
    if s.cr4 & (1 << 12) != 0 {return Err(Error::La57);}
    if s.cr4 & (1 << 17) != 0 {return Err(Error::Pcide);}
    if s.efer & (1 << 10) == 0 {return Err(Error::NotLongActive);}
    Ok(Audit {
        wp_enabled:s.cr0 & (1 << 16) != 0,
        nxe_enabled:s.efer & (1 << 11) != 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn baseline()->State {
        State {
            max_extended_leaf:0x8000_0008,
            cpuid_leaf1_edx:(1<<5)|(1<<6),
            cpuid_ext_edx:(1<<20)|(1<<29),
            cr0:(1<<31)|(1<<16),
            cr4:1<<5,
            efer:(1<<10)|(1<<11),
        }
    }
    #[test] fn accepts_restricted_profile() {
        assert_eq!(check(baseline()),Ok(Audit{wp_enabled:true,nxe_enabled:true}));
    }
    #[test] fn records_but_does_not_enable_missing_wp_nxe() {
        let mut s=baseline();
        s.cr0 &= !(1<<16);
        s.efer &= !(1<<11);
        assert_eq!(check(s),Ok(Audit{wp_enabled:false,nxe_enabled:false}));
    }
    #[test] fn rejects_la57() {let mut s=baseline();s.cr4|=1<<12;assert_eq!(check(s),Err(Error::La57));}
    #[test] fn rejects_pcide() {let mut s=baseline();s.cr4|=1<<17;assert_eq!(check(s),Err(Error::Pcide));}
    #[test] fn rejects_no_nx() {let mut s=baseline();s.cpuid_ext_edx&=!(1<<20);assert_eq!(check(s),Err(Error::NoNx));}
    #[test] fn rejects_no_pae() {let mut s=baseline();s.cr4&=!(1<<5);assert_eq!(check(s),Err(Error::NoPae));}
    #[test] fn rejects_no_msr() {let mut s=baseline();s.cpuid_leaf1_edx&=!(1<<5);assert_eq!(check(s),Err(Error::NoMsr));}
    #[test] fn rejects_no_longmode() {let mut s=baseline();s.cpuid_ext_edx&=!(1<<29);assert_eq!(check(s),Err(Error::NoLongMode));}
    #[test] fn rejects_not_long_active() {let mut s=baseline();s.efer&=!(1<<10);assert_eq!(check(s),Err(Error::NotLongActive));}
}

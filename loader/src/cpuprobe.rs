// LAB-H1 x86_64 current CPU-state read-only probe, not a CR3 cut.
use core::arch::{asm, x86_64::__cpuid};
use unios_cpugate::{Audit, State};
#[derive(Debug)]
pub enum Error { UnsupportedCpu, InvalidCpu }
pub fn inspect() -> Result<Audit, Error> {
    let basic=unsafe { __cpuid(0) };
    if basic.eax < 1 { return Err(Error::UnsupportedCpu); }
    let leaf1=unsafe { __cpuid(1) };
    let ext=unsafe { __cpuid(0x8000_0000) };
    if ext.eax < 0x8000_0001 || leaf1.edx & (1<<5)==0 {
        return Err(Error::UnsupportedCpu);
    }
    let nx=unsafe { __cpuid(0x8000_0001) };
    let (cr0,cr4,efer_lo,efer_hi): (u64,u64,u32,u32);
    unsafe {
        asm!("mov {}, cr0",out(reg) cr0,options(nomem,nostack,preserves_flags));
        asm!("mov {}, cr4",out(reg) cr4,options(nomem,nostack,preserves_flags));
        asm!("rdmsr",in("ecx") 0xc0000080u32,out("eax") efer_lo,out("edx") efer_hi,
            options(nomem,nostack,preserves_flags));
    }
    let state=State {
        max_extended_leaf:ext.eax,cpuid_leaf1_edx:leaf1.edx,
        cpuid_ext_edx:nx.edx,cr0,cr4,
        efer:(efer_hi as u64)<<32 | (efer_lo as u64),
    };
    unios_cpugate::check(state).map_err(|_|Error::InvalidCpu)
}

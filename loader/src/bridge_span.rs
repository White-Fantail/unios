// E2e link-bounded EFI executable preflight stub. This code is NEVER
// called and contains NO CR3 write. Labels pin an actual linker-visible
// instruction range; future real transition requires a new audited stub.
// Section is explicitly allocated executable in the COFF EFI image.
use core::arch::global_asm;

global_asm!(r#"
    .section .text.unios_bridge,"xr"
    .p2align 4
    .globl unios_l1_bridge_begin
unios_l1_bridge_begin:
    cli
    cld
    nop
    ret
    .globl unios_l1_bridge_end
unios_l1_bridge_end:
"#);

extern "C" {
    fn unios_l1_bridge_begin();
    fn unios_l1_bridge_end();
}

pub fn bounds() -> (u64,u64) {
    (unios_l1_bridge_begin as usize as u64, unios_l1_bridge_end as usize as u64)
}

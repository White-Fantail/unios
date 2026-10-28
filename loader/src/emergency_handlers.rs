// LAB-H1 E2f dormant 64-bit interrupt gates, NEVER installed/invoked.
// If installed prematurely in a disposable VM, they fail-stop with
// distinct non-PASS debug-exit bytes; all error/no-error vectors are
// terminal and no stack frame is ever consumed or returned through.
use core::arch::global_asm;
global_asm!(r#"
    .section .text.unios_emergency,"xr"
    .p2align 4
    .globl unios_l1_emergency_generic
unios_l1_emergency_generic:
    cli
    mov dx, 0xf4
    mov al, 0x31
    out dx, al
1:
    hlt
    jmp 1b
    .p2align 4
    .globl unios_l1_emergency_df
unios_l1_emergency_df:
    cli
    mov dx, 0xf4
    mov al, 0x32
    out dx, al
2:
    hlt
    jmp 2b
"#);
extern "C" {
    fn unios_l1_emergency_generic();
    fn unios_l1_emergency_df();
}
pub fn addresses() -> (u64,u64) {
    (unios_l1_emergency_generic as usize as u64,
     unios_l1_emergency_df as usize as u64)
}

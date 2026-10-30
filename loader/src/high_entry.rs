// E2r-B: linked but unreachable SysV64 high-half final-entry stub.
//
// NOT AN EXECUTABLE HANDOFF: no call site or activation exists here.
// A later independently gated QEMU-only step must install and attest
// GDTR/IDTR/TR/IST and the private CR3 BEFORE invoking this assembly.
// The caller must prove high_stack_top - 8 remains in the mapped RW/NX
// owned kernel stack and entry is the exact linked ELF RX address.
use core::arch::global_asm;

global_asm!(r#"
    .section .text.unios_high_entry,"xr"
    .p2align 4
    .globl unios_l1_high_entry_jump
unios_l1_high_entry_jump:
    cli
    cld
    // SysV64 args: RDI=owned high-half stack top, RSI=boot context VA,
    // RDX=exact ELF high-half entry. No UEFI Microsoft x64 convention.
    mov rsp,rdi
    and rsp,-16
    // A tail JMP enters a SysV64 Rust function with RSP % 16 == 8,
    // exactly as a CALL would have, but never uses a firmware return.
    sub rsp,8
    mov qword ptr [rsp],0
    mov rdi,rsi
    jmp rdx
    .globl unios_l1_high_entry_jump_end
unios_l1_high_entry_jump_end:
"#);

extern "sysv64" {
    fn unios_l1_high_entry_jump(stack_top:u64, boot_context:u64, entry:u64)->!;
    fn unios_l1_high_entry_jump_end();
}

// Used strictly to retain and bound the linked PE RX code for auditing.
// Intentionally NO public function to execute the stub at E2r-B.
pub fn bounds()->(u64,u64) {
    (unios_l1_high_entry_jump as usize as u64,
     unios_l1_high_entry_jump_end as usize as u64)
}

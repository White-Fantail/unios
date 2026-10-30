// E2r-C3: OFFLINE ONLY descriptor activation and far-CS-reload transition.
//
// The linked code is NEVER invoked by this feature: there is no callable
// Rust wrapper. Before any later disposable-QEMU activation the host must
// prove that:
// - GDTR/IDTR operand pointers live in owned low bridge RW memory, and their
//   bases point at the independently attested high-half descriptor page;
// - CPU CR3 is already the audited private root and the 2-page bridge stack
//   is still identity mapped and writable;
// - the TSS/IST and high-half stack are mapped with exact access rights;
// - the final trampoline is supervisor RX, and RDX/R9 reference the exact
//   linked trampoline / ELF entry.
// This contract is NOT satisfied merely by seeing these code bytes in PE.
use core::arch::global_asm;

global_asm!(r#"
    .section .text.unios_descriptor_activate,"xr"
    .p2align 4
    .globl unios_l1_descriptor_activate
unios_l1_descriptor_activate:
    cli
    cld
    // SysV64 arguments:
    // RDI=GDTR operand, RSI=IDTR operand, RDX=high-entry trampoline,
    // RCX=main-stack top, R8=BootContext, R9=ELF high-half entry.
    // Requires an already installed private CR3 and owned low bridge stack.
    lgdt [rdi]
    lidt [rsi]
    mov ax,0x18
    ltr ax
    // A long-mode far return reloads the cached CS from *our* GDT.
    // Two bounded pushes use the already-verified low bridge stack.
    push 0x08
    lea rax,[rip + 1f]
    push rax
    retfq
1:
    mov ax,0x10
    mov ds,ax
    mov es,ax
    mov ss,ax
    xor eax,eax
    mov fs,ax
    mov gs,ax
    // Restore the SysV64 argument layout for the E2r-B final trampoline.
    mov r10,rdx
    mov rdi,rcx
    mov rsi,r8
    mov rdx,r9
    jmp r10
    .globl unios_l1_descriptor_activate_end
unios_l1_descriptor_activate_end:
"#);

extern "sysv64" {
    fn unios_l1_descriptor_activate(
        gdtr_operand:u64,idtr_operand:u64,trampoline:u64,
        main_stack_top:u64,boot_context:u64,kernel_entry:u64)->!;
    fn unios_l1_descriptor_activate_end();
}

// Linker keeps the real symbol for independent PE opcode inspection.
// This is NOT an API to activate descriptors.
pub fn bounds()->(u64,u64) {
    (unios_l1_descriptor_activate as usize as u64,
     unios_l1_descriptor_activate_end as usize as u64)
}

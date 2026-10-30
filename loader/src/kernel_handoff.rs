// E2r-C6: disposable-QEMU-only actual post-ExitBS/private-CR3,
// real descriptor activation then approved SysV64 high-half kernel entry.
//
// The complete linked span must be proved EFI_LOADER_CODE supervisor RX
// under the old CR3 and same-PA supervisor RX under the NEW private CR3.
// The descriptor PML4 mapping and its actual GDT/TSS/IST/IDT contents are
// independently proved by host QMP BEFORE ExitBS. Fail-stop on missing Q/R.
// The CPU does not dereference a firmware pointer after the CR3 switch.
use core::arch::global_asm;
global_asm!(r#"
    .section .text.unios_kernel_handoff,"xr"
    .p2align 4
    .globl unios_l1_kernel_handoff
unios_l1_kernel_handoff:
    cli
    cld
    // SysV64 RDI=owned new CR3; RSI=owned low bridge RSP;
    // RDX=exact audited high-half descriptor VA.
    mov r12,rdx
    // Preserve original SysV64 kernel-handoff arguments across UART/QMP stages.
    mov r13,rcx
    mov r14,r8
    mov r15,r9
    mov rsp,rsi
    mov cr3,rdi

    .macro com2_byte value
    mov dx,0x2fd
.Ltxwait\@:
    in al,dx
    test al,0x20
    jz .Ltxwait\@
    mov dx,0x2f8
    mov al,\value
    out dx,al
    .endm

    // Third host witness is the same Q barrier as E2q. Its QMP
    // verifies actual CPU CR3/RIP/RSP and private PTE on current RIP.
    com2_byte 0x7f
    com2_byte 0x55
    com2_byte 0x4e
    com2_byte 0x49
    com2_byte 0x32
    com2_byte 0x51
    com2_byte 0x7f
    mov ecx,0x10000000
1:
    mov dx,0x2fd
    in al,dx
    test al,1
    jnz 2f
    dec ecx
    jnz 1b
    jmp 8f
2:
    mov dx,0x2f8
    in al,dx
    cmp al,0x51
    jne 8f

    // Use only the identity-mapped, owned bridge stack for operands.
    // GDTR limit=39, base=DESCRIPTORS_VA; IDTR limit=511, base+0x200.
    // Assembly never touches the descriptor *contents* until the
    // hardware reads their already-mapped high-half virtual addresses.
    sub rsp,32
    mov word ptr [rsp],39
    mov qword ptr [rsp+2],r12
    mov word ptr [rsp+16],511
    lea rax,[r12+0x200]
    mov qword ptr [rsp+18],rax
    lgdt [rsp]
    lidt [rsp+16]
    mov ax,0x18
    ltr ax
    // Reload the *cached* CS from new GDT. No UEFI code calls.
    push 0x08
    lea rax,[rip+3f]
    push rax
    retfq
3:
    mov ax,0x10
    mov ds,ax
    mov es,ax
    mov ss,ax
    xor eax,eax
    mov fs,ax
    mov gs,ax

    // Fourth host witness now sees live GDTR/IDTR/TR/CS/SS and CR3.
    com2_byte 0x7f
    com2_byte 0x55
    com2_byte 0x4e
    com2_byte 0x49
    com2_byte 0x32
    com2_byte 0x52
    com2_byte 0x7f
    mov ecx,0x10000000
4:
    mov dx,0x2fd
    in al,dx
    test al,1
    jnz 5f
    dec ecx
    jnz 4b
    jmp 8f
5:
    mov dx,0x2f8
    in al,dx
    cmp al,0x52
    jne 8f
    // Host verified the live kernel-owned GDT/IDT/TSS/CS/CR3; now
    // commit the irreversible SysV64 high-half stack/entry transition.
    // No EFI service, low data pointer, firmware stack or RET afterward.
    mov rdi,r13
    mov rsi,r14
    mov rdx,r15
    jmp unios_l1_high_entry_jump
6:  hlt
    jmp 6b
8:
    mov dx,0xf4
    mov al,0x41
    out dx,al
9:  hlt
    jmp 9b
    .globl unios_l1_kernel_handoff_end
unios_l1_kernel_handoff_end:
"#);
extern "sysv64" {
    fn unios_l1_kernel_handoff(root:u64,bridge_rsp:u64,descriptor_va:u64,stack_top:u64,context_va:u64,entry:u64)->!;
    fn unios_l1_kernel_handoff_end();
}
pub fn bounds()->(u64,u64){
    (unios_l1_kernel_handoff as usize as u64,
     unios_l1_kernel_handoff_end as usize as u64)
}
pub unsafe fn activate(root:u64,bridge_rsp:u64,descriptor_va:u64,stack_top:u64,context_va:u64,entry:u64)->!{
    unios_l1_kernel_handoff(root,bridge_rsp,descriptor_va,stack_top,context_va,entry)
}

// E2q: true CR3 terminal switch in disposable QEMU only.
// Exact finite assembly span is independently mapped and host verified
// under OLD and NEW roots. No EFI calls after ExitBootServices.
// This does NOT branch into a freestanding high-half kernel.
use core::arch::global_asm;
global_asm!(r#"
    .section .text.unios_cr3_terminal,"xr"
    .p2align 4
    .globl unios_l1_cr3_terminal
unios_l1_cr3_terminal:
    cli
    cld
    // Explicit sysv64 ABI: RDI new private PML4 root, RSI bridge RSP.
    // Switch RSP under old identity-mapped firmware CR3 FIRST.
    mov rsp,rsi
    // All live following code bytes and low bridge stack are RX / RW
    // under BOTH roots, as validated independently before this point.
    mov cr3,rdi
    // UART COM2 was initialized before ExitBootServices. No I/O
    // to EFI boot services, no stack-relative memory or CALL.
    .macro uart_byte value
    mov dx,0x2fd
.Ltxwait\@:
    in al,dx
    test al,0x20
    jz .Ltxwait\@
    mov dx,0x2f8
    mov al,\value
    out dx,al
    .endm
    uart_byte 0x7f
    uart_byte 0x55
    uart_byte 0x4e
    uart_byte 0x49
    uart_byte 0x32
    uart_byte 0x51
    uart_byte 0x7f
    // The independent host MUST pause the vCPU, verify the actual
    // CPU CR3/RIP/RSP under NEW root, then authorize 'Q'.
    mov ecx,0x10000000
1:
    mov dx,0x2fd
    in al,dx
    test al,1
    jnz 2f
    dec ecx
    jnz 1b
    jmp 4f
2:
    mov dx,0x2f8
    in al,dx
    cmp al,0x51
    jne 4f
    mov dx,0xf4
    mov al,0x3c
    out dx,al
3:  hlt
    jmp 3b
4:
    mov dx,0xf4
    mov al,0x3d
    out dx,al
5:  hlt
    jmp 5b
    .globl unios_l1_cr3_terminal_end
unios_l1_cr3_terminal_end:
"#);
// EFI uses Microsoft x64 ABI for the default C calling convention.
// This handwritten stub explicitly uses SysV RDI/RSI arguments;
// pin the FFI to sysv64 so the compiler cannot pass RCX/RDX instead.
extern "sysv64" {
    fn unios_l1_cr3_terminal(root:u64,bridge_rsp:u64)->!;
    fn unios_l1_cr3_terminal_end();
}
pub fn bounds()->(u64,u64) {
    (unios_l1_cr3_terminal as usize as u64,
     unios_l1_cr3_terminal_end as usize as u64)
}
#[cfg(feature="lab-e2q-actual-cr3")]
pub unsafe fn activate(root:u64,bridge_rsp:u64)->! {
    unios_l1_cr3_terminal(root,bridge_rsp)
}

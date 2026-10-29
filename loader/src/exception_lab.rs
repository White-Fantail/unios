// LAB-H1 E2l: terminal disposable-QEMU CPU exception delivery smoke.
// Installed descriptors use the same verified EFI-owned PHYSICAL page,
// not the dormant high-half metadata VA. Never invoke on real hardware.
// No ExitBootServices or new CR3 is performed. CPU exception results
// are fail-stop, not a recoverable/production exception subsystem.
use core::arch::{asm,global_asm};
use unios_descriptors::Image;

global_asm!(r#"
    .section .text.unios_exception_lab,"xr"
    .p2align 4
    .globl unios_l1_lab_ud_gate
unios_l1_lab_ud_gate:
    cli
    mov dx,0xf4
    mov al,0x33
    out dx,al
1:  hlt
    jmp 1b
    .p2align 4
    .globl unios_l1_lab_real_df_gate
unios_l1_lab_real_df_gate:
    cli
    // E2m: actual CPU-delivered vector 8; #DF pushes error code 0.
    // Bound the hardware IST1 stack selected by TSS. This handler
    // contains no IRETQ or path back to firmware.
    cmp rsp,r10
    jb 5f
    cmp rsp,r11
    jae 5f
    cmp qword ptr [rsp],0
    jne 5f
    mov dx,0xf4
    mov al,0x36
    out dx,al
6:  hlt
    jmp 6b
5:  mov dx,0xf4
    mov al,0x37
    out dx,al
7:  hlt
    jmp 7b
    .p2align 4
    .globl unios_l1_lab_ist_gate
unios_l1_lab_ist_gate:
    cli
    // r10/r11 are injected by the terminal INT 8 test before dispatch.
    // Hardware IST1 should load RSP from the physical TSS.
    cmp rsp,r10
    jbe 3f
    cmp rsp,r11
    ja 3f
    mov dx,0xf4
    mov al,0x34
    out dx,al
2:  hlt
    jmp 2b
3:  mov dx,0xf4
    mov al,0x35
    out dx,al
4:  hlt
    jmp 4b
"#);

extern "C" {
    fn unios_l1_lab_ud_gate();
    fn unios_l1_lab_ist_gate();
    fn unios_l1_lab_real_df_gate();
}
pub fn addresses()->(u64,u64){
    (unios_l1_lab_ud_gate as usize as u64,
     unios_l1_lab_ist_gate as usize as u64)
}
pub fn real_df_address()->u64 {
    unios_l1_lab_real_df_gate as usize as u64
}
#[repr(C,packed)]
struct TableRegister { limit:u16, base:u64 }

/// Actual UD2 after LGDT/LIDT/LTR. Fault via vector 6, never a direct
/// call into the handler. Target mode deliberately does not return.
#[cfg(feature="lab-e2l-smoke-ud")]
pub unsafe fn trigger_ud(image:Image)->!{
    let gdt=TableRegister{limit:image.gdtr_limit,base:image.gdtr_base};
    let idt=TableRegister{limit:image.idtr_limit,base:image.idtr_base};
    asm!(
        "cli",
        "lgdt [rdi]",
        "lidt [rsi]",
        "mov ax, 0x18",
        "ltr ax",
        "ud2",
        in("rdi") &gdt,
        in("rsi") &idt,
        options(noreturn)
    );
}
/// Software INT vector 8 is NOT a genuine nested-fault #DF, but
/// x86 hardware still selects IDT gate 8 and its IST1 TSS stack.
/// r10/r11 bound the expected physical IST extent in the handler.
#[cfg(feature="lab-e2l-smoke-ist")]
pub unsafe fn trigger_ist(image:Image,ist_base:u64,ist_top:u64)->!{
    let gdt=TableRegister{limit:image.gdtr_limit,base:image.gdtr_base};
    let idt=TableRegister{limit:image.idtr_limit,base:image.idtr_base};
    asm!(
        "cli",
        "lgdt [rdi]",
        "lidt [rsi]",
        "mov ax, 0x18",
        "ltr ax",
        "int 8",
        "ud2",
        in("rdi") &gdt,
        in("rsi") &idt,
        in("r10") ist_base,
        in("r11") ist_top,
        options(noreturn)
    );
}

/// E2m true nested contributory fault generation (disposable QEMU ONLY):
/// 1. install reviewed physical GDT/IDT/TSS and active #DF IST1.
/// 2. MOV DS,0x28 triggers CPU #GP: selector lies past the GDTR limit.
/// 3. The #GP delivery gate deliberately uses invalid CS selector
///    0x28 -> another CPU #GP during first #GP delivery -> real #DF.
/// 4. Only vector 8's hardware delivery can reach the dedicated
///    real-DF handler; no INT 8 and no direct CALL is executed.
/// The dedicated handler demands error code 0 and RSP in IST1 bounds.
#[cfg(feature="lab-e2m-smoke-real-df")]
pub unsafe fn trigger_real_df(image:Image,ist_base:u64,ist_top:u64)->! {
    let gdt=TableRegister{limit:image.gdtr_limit,base:image.gdtr_base};
    let idt=TableRegister{limit:image.idtr_limit,base:image.idtr_base};
    asm!(
        "cli",
        "lgdt [rdi]",
        "lidt [rsi]",
        "mov ax, 0x18",
        "ltr ax",
        "mov ax, 0x28",
        "mov ds, ax",
        // No valid fallthrough. This must fault again if #DF not seen.
        "ud2",
        in("rdi") &gdt,
        in("rsi") &idt,
        in("r10") ist_base,
        in("r11") ist_top,
        options(noreturn)
    );
}

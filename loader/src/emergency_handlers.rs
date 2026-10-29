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
    .globl unios_l1_emergency_generic_end
unios_l1_emergency_generic_end:
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
    .globl unios_l1_emergency_df_end
unios_l1_emergency_df_end:
"#);
extern "C" {
    fn unios_l1_emergency_generic();
    fn unios_l1_emergency_df();
    fn unios_l1_emergency_generic_end();
    fn unios_l1_emergency_df_end();
}
pub fn addresses() -> (u64,u64) {
    (unios_l1_emergency_generic as usize as u64,
     unios_l1_emergency_df as usize as u64)
}

pub fn spans() -> [(u64,u64);2] {
    [
        (unios_l1_emergency_generic as usize as u64,
         unios_l1_emergency_generic_end as usize as u64),
        (unios_l1_emergency_df as usize as u64,
         unios_l1_emergency_df_end as usize as u64),
    ]
}

/// Called only after each span's old EFI code identity and new RX
/// mapping have been audited; NEVER invokes these failure handlers.
pub fn validate_linked_bytes() -> Result<(),unios_bridgecheck::Error> {
    for (idx,(first,last)) in spans().into_iter().enumerate() {
        let span=unios_bridgecheck::Span::new(first,last)?;
        if span.end-span.begin != unios_bridgecheck::EMERGENCY_BYTES as u64 {
            return Err(unios_bridgecheck::Error::StubBytes);
        }
        let bytes=unsafe {
            core::slice::from_raw_parts(first as *const u8,
                unios_bridgecheck::EMERGENCY_BYTES)
        };
        unios_bridgecheck::validate_emergency_handler(bytes,
            if idx==0 {0x31} else {0x32})?;
    }
    Ok(())
}

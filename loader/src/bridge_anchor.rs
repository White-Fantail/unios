// A link-address executable page canary, NOT the final CR3-switch stub.
// Only its code-page start translation is audited; no length or control-flow
// guarantee for a future real bridge has been established here.
#[no_mangle]
#[inline(never)]
pub extern "efiapi" fn unios_l1_executable_anchor() {
    unsafe { core::arch::asm!("nop", options(nomem, nostack, preserves_flags)); }
}
pub fn linear_address() -> u64 { unios_l1_executable_anchor as usize as u64 }

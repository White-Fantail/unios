#![no_std]
#![no_main]

// L1 freestanding SysV entry. Not yet reachable from EFI: the audited CR3
// bridge and emergency IDT/IST must be implemented before enabling handoff.
use core::arch::asm;
use core::panic::PanicInfo;

const COM1: u16 = 0x3f8;
const POLL_LIMIT: usize = 100_000;

// An actual zero-initialized PT_LOAD test fixture: ensures the first kernel
// ELF exercises a writable BSS payload rather than only RX/R-only segments.
#[no_mangle]
#[link_section = ".bss.l1_probe"]
static mut L1_BSS_PROBE: [u8; 4096] = [0; 4096];

#[inline(always)]
unsafe fn out(port: u16, byte: u8) {
    asm!("out dx, al", in("dx") port, in("al") byte, options(nomem, nostack, preserves_flags));
}
#[inline(always)]
unsafe fn input(port: u16) -> u8 {
    let v: u8;
    asm!("in al, dx", out("al") v, in("dx") port, options(nomem, nostack, preserves_flags));
    v
}
fn serial(message: &[u8]) {
    for byte in message {
        let mut ready = false;
        for _ in 0..POLL_LIMIT {
            if unsafe { input(COM1 + 5) } & 0x20 != 0 { ready = true; break; }
        }
        if !ready { return; } // finite fail-stop, no success marker
        unsafe { out(COM1, *byte); }
    }
}
#[no_mangle]
pub extern "C" fn _start(_boot_context: *const u8) -> ! {
    unsafe { asm!("cli", "cld", options(nomem, nostack)); }
    // A future full-entry test must reject failed BSS initialization.
    // Volatile reads keep both ends of the 4KiB BSS live in the linked ELF.
    let bss = core::ptr::addr_of!(L1_BSS_PROBE).cast::<u8>();
    let first = unsafe { core::ptr::read_volatile(bss) };
    let last = unsafe { core::ptr::read_volatile(bss.add(4095)) };
    if first != 0 || last != 0 { halt(); }
    serial(b"UNIOS:L1:KERNEL_ENTRY_UNVERIFIED\r\n");
    // Deliberately NOT HALTED_PASS. E2–E4 must validate the complete transition.
    halt()
}
fn halt() -> ! {
    loop { unsafe { asm!("cli; hlt", options(nomem, nostack)); } }
}
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    // No allocation, formatting, recursive UART lock, unwinding or fake PASS.
    halt()
}

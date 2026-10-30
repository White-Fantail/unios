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
    #[cfg(feature="lab-e2r-kernel-entry")]
    unsafe { lab_entry(_boot_context); }
    #[cfg(not(feature="lab-e2r-kernel-entry"))]
    serial(b"UNIOS:L1:KERNEL_ENTRY_UNVERIFIED\\r\\n");
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

#[cfg(feature="lab-e2r-kernel-entry")]
const L1_BOOT_MAGIC: u64=0x48314c534f494e55; // "UNIOSL1H" little-endian
#[cfg(feature="lab-e2r-kernel-entry")]
#[repr(C)]
struct L1Context {
    magic:u64, version:u64, entry:u64, stack_top:u64,
    private_cr3:u64, self_va:u64,
}
#[cfg(feature="lab-e2r-kernel-entry")]
#[inline(never)]
unsafe fn lab_entry(ptr:*const u8)->! {
    // A kernel pointer must be the exact fixed, canonical, RO context VA.
    if ptr as u64!=0xffffffff83fd0000 { lab_fail(); }
    let ctx=&*(ptr as *const L1Context);
    if ctx.magic!=L1_BOOT_MAGIC || ctx.version!=1 ||
       ctx.entry!=0xffffffff80000000 ||
       ctx.stack_top!=0xffffffff83fe5000 ||
       ctx.private_cr3<0x100000 || ctx.private_cr3&4095!=0 ||
       ctx.self_va!=ptr as u64 {
        lab_fail();
    }
    serial(b"UNIOS:L1:E2R_C6_HIGH_HALF_ENTRY\r\n");
    // COM2 was initialized by the pre-cut host gate. Never call firmware.
    for byte in b"\x7fUNI2S\x7f" {
        let mut ready=false;
        for _ in 0..1000000 {
            if input(0x2fd)&0x20!=0 {ready=true;break;}
        }
        if !ready {lab_fail();}
        out(0x2f8,*byte);
    }
    // The host stops CPU and verifies RIP+RSP+CR3+descriptors+context PA,
    // then grants a fifth distinct 'K' checkpoint. No premature success.
    for _ in 0..600000000u64 {
        if input(0x2fd)&1!=0 {
            if input(0x2f8)==b'K' {
                #[cfg(feature="lab-e2r-clean-pass")]
                {
                    // The CPU only reaches here after the host fifth
                    // QMP RIP/RSP/CR3/BootContext proof granted K.
                    // COM1 stage order and true QEMU debug-exit 65 are
                    // independently checked again after VM termination.
                    serial(b"UNIOS:L1:HALTED_PASS\r\n");
                    out(0xf4,0x20); // QEMU isa-debug-exit exit 65
                    halt();
                }
                #[cfg(not(feature="lab-e2r-clean-pass"))]
                {
                    out(0xf4,0x42); // C6 diagnostic exit 133 only
                    halt();
                }
            }
            lab_fail();
        }
        core::hint::spin_loop();
    }
    lab_fail()
}
#[cfg(feature="lab-e2r-kernel-entry")]
unsafe fn lab_fail()->! {
    out(0xf4,0x43); // rejected -> QEMU exit 135
    halt()
}

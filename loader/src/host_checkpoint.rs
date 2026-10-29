// QEMU-only E2h independent host/guest barrier. Called after the UEFI
// descriptor ownership gate but BEFORE the initial physical root deref.
// COM2 is an isolated lab socket; never use this path in production hardware.
use core::arch::asm;
const UART:u16=0x2f8;
unsafe fn input(port:u16)->u8 {
    let byte:u8;
    asm!("in al, dx",in("dx") port,out("al") byte,
        options(nomem,nostack,preserves_flags));
    byte
}
unsafe fn output(port:u16,byte:u8) {
    asm!("out dx, al",in("dx") port,in("al") byte,
        options(nomem,nostack,preserves_flags));
}
pub fn host_checkpoint()->bool {
    unsafe {
        // Configure COM2 UART 8N1, FIFO off, DLAB clear, IRQ off.
        // The external host monitor must perform a paused-CPU read of
        // CR3, gva2gpa(root), x(root), xp(root) before sending 'A'.
        output(UART+1,0); // IER
        output(UART+3,0x80); // DLAB
        output(UART,1); // baud divisor low
        output(UART+1,0); // baud divisor high
        output(UART+3,3); // 8N1
        output(UART+2,0); // FIFO off
        output(UART+4,3); // DTR|RTS
        for _ in 0..1_000_000u32 {
            if input(UART+5) & 0x20!=0 {break;}
        }
        // Unique multi-byte barrier marker. OVMF writes unrelated ANSI
        // traffic on COM2 before the UniOS UEFI app starts.
        for marker in b"\x7fUNI2H\x7f" {
            for _ in 0..1_000_000u32 {
                if input(UART+5)&0x20!=0 {break;}
            }
            output(UART,*marker);
        }
        // Fail closed: host cannot approve by simply seeing a serial
        // token; it must send exactly 'A' on the independent COM2 RX.
        for _ in 0..600_000_000u64 {
            if input(UART+5)&1!=0 {
                return input(UART)==b'A';
            }
            core::hint::spin_loop();
        }
        false
    }
}

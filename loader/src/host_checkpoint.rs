// QEMU-only E2h/E2i independent host/guest barrier. Called after the UEFI
// descriptor ownership gate but BEFORE the initial physical root deref.
// COM2 is an isolated lab socket; never use this path in production hardware.
use core::arch::asm;
use unios_bootmap::Reservation;
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
unsafe fn transmit(port:u16,byte:u8)->bool {
    for _ in 0..1_000_000u32 {
        if input(port+5)&0x20!=0 {output(port,byte);return true;}
    }
    false
}
pub fn host_checkpoint(reservations:&[Reservation])->bool {
    #[cfg(not(feature="lab-e2j-targeted-leaf-proof"))]
    let _=reservations;
    unsafe {
        // Configure COM2 UART 8N1, FIFO off, DLAB clear, IRQ off.
        // The external host monitor must perform a paused-CPU read of
        // CR3, gva2gpa(root), x(root), xp(root) before sending feature-specific authorization.
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
        #[cfg(feature="lab-e2j-targeted-leaf-proof")]
        let barrier:&[u8]=b"\x7fUNI2J\x7f";
        #[cfg(all(feature="lab-e2i-host-table-proof",not(feature="lab-e2j-targeted-leaf-proof")))]
        let barrier:&[u8]=b"\x7fUNI2I\x7f";
        #[cfg(not(feature="lab-e2i-host-table-proof"))]
        let barrier:&[u8]=b"\x7fUNI2H\x7f";
        for marker in barrier {
            if !transmit(UART,*marker) {return false;}
        }
        #[cfg(feature="lab-e2j-targeted-leaf-proof")]
        {
            if reservations.len()<8 || reservations.len()>16 {return false;}
            if !transmit(UART,reservations.len() as u8) {return false;}
            for r in reservations {
                let pages=match u32::try_from(r.pages) {Ok(v)=>v,Err(_)=>return false};
                if !transmit(UART,r.id) {return false;}
                for byte in pages.to_le_bytes() {
                    if !transmit(UART,byte) {return false;}
                }
                for byte in r.base.to_le_bytes() {
                    if !transmit(UART,byte) {return false;}
                }
            }
        }
        // Fail closed: host cannot approve by simply seeing a serial
        // token; it must send exactly 'A' on the independent COM2 RX.
        for _ in 0..600_000_000u64 {
            if input(UART+5)&1!=0 {
                #[cfg(feature="lab-e2j-targeted-leaf-proof")]
                {return input(UART)==b'J';}
                #[cfg(all(feature="lab-e2i-host-table-proof",not(feature="lab-e2j-targeted-leaf-proof")))]
                {return input(UART)==b'I';}
                #[cfg(not(feature="lab-e2i-host-table-proof"))]
                {return input(UART)==b'A';}
            }
            core::hint::spin_loop();
        }
        false
    }
}

#[cfg(feature="lab-e2o-host-private-witness")]
#[cfg(not(feature="lab-e2q-cr3-code"))]
pub const PRIVATE_TARGETS:usize=19;
#[cfg(all(feature="lab-e2q-cr3-code",not(feature="lab-e2r-high-entry-code")))]
pub const PRIVATE_TARGETS:usize=21;
#[cfg(all(feature="lab-e2r-high-entry-code",not(feature="lab-e2r-descriptor-code")))]
pub const PRIVATE_TARGETS:usize=23;
#[cfg(all(feature="lab-e2r-descriptor-code",not(feature="lab-e2r-active-descriptors")))]
pub const PRIVATE_TARGETS:usize=25;
#[cfg(feature="lab-e2r-active-descriptors")]
pub const PRIVATE_TARGETS:usize=27;
/// Compact COM2 evidence manifest. All fields are untrusted until the
/// independent paused-QMP host reads actual RAM and validates identity,
/// root reservation, exact PTE contents and target labels.
#[cfg(feature="lab-e2o-host-private-witness")]
#[derive(Clone,Copy)]
pub struct PrivateTarget {
    pub va:u64, pub pa:u64, pub rights:u8,
}
/// Stage 2: distinct NEW private root witness AFTER the guest finished
/// building and independently checking raw PTEs. CR3 is still the old
/// firmware root; this sends metadata only and NEVER switches CR3.
/// A second bounded, distinct O authorization is mandatory before
/// subsequent QEMU exception-lab tests or preflight success.
#[cfg(feature="lab-e2o-host-private-witness")]
pub fn private_checkpoint(root:u64,used:usize,targets:&[PrivateTarget;PRIVATE_TARGETS])->bool {
    if root<0x100000 || root >=1u64<<32 || root&4095!=0 ||
       used<4 || used>128 {return false;}
    unsafe {
        for byte in b"\x7fUNI2O\x7f" {
            if !transmit(UART,*byte) {return false;}
        }
        for b in root.to_le_bytes() {
            if !transmit(UART,b) {return false;}
        }
        if !transmit(UART,used as u8) || !transmit(UART,PRIVATE_TARGETS as u8) {
            return false;
        }
        for target in targets {
            if target.rights>3 {return false;}
            for b in target.va.to_le_bytes() {
                if !transmit(UART,b) {return false;}
            }
            for b in target.pa.to_le_bytes() {
                if !transmit(UART,b) {return false;}
            }
            if !transmit(UART,target.rights) {return false;}
        }
        for _ in 0..600_000_000u64 {
            if input(UART+5)&1!=0 {return input(UART)==b'O';}
            core::hint::spin_loop();
        }
        false
    }
}

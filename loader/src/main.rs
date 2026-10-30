#![no_std]
#![no_main]

// E1 UEFI entry / embedded ELF preflight. NO ExitBootServices/CR3 transition
// until E2/E3 mapping provenance and emergency exception path are reviewed.
// Returning EFI_UNSUPPORTED is intentional: preflight is not an L1 boot PASS.
use core::ffi::c_void;
#[cfg(feature = "lab-debug-exit")]
use core::arch::asm;
use core::panic::PanicInfo;
#[cfg(feature = "lab-e2-preflight")]
mod memory;
#[cfg(feature = "lab-e2b-staging")]
mod oldmap;
#[cfg(feature = "lab-e2d-bridge-audit")]
mod cpuprobe;
#[cfg(feature = "lab-e2d-bridge-audit")]
mod bridge_anchor;
#[cfg(feature = "lab-e2e-bounded-bridge")]
mod bridge_span;
#[cfg(feature = "lab-e2f-descriptors")]
mod emergency_handlers;
#[cfg(feature="lab-e2l-lab-descriptors")]
mod exception_lab;
#[cfg(feature="lab-e2q-cr3-code")]
mod cr3_terminal;
#[cfg(feature="lab-e2r-high-entry-code")]
mod high_entry;
#[cfg(feature="lab-e2r-descriptor-code")]
mod high_descriptor;
#[cfg(feature="lab-e2r-active-descriptors")]
mod live_descriptors;
#[cfg(feature="lab-e2h-host-root-proof")]
mod host_checkpoint;

const EFI_SYSTEM_TABLE_SIGNATURE: u64 = 0x5453_5953_2049_4249;
const EFI_LOAD_ERROR: usize = (1usize << (usize::BITS - 1)) | 1;
const EFI_UNSUPPORTED: usize = (1usize << (usize::BITS - 1)) | 3;
const EMBEDDED_ELF: &[u8] = include_bytes!(env!("UNIOS_KERNEL_ELF"));

#[repr(C)]
struct TableHeader {
    signature: u64,
    revision: u32,
    header_size: u32,
    crc32: u32,
    reserved: u32,
}

#[repr(C)]
struct SimpleTextOutput {
    reset: usize,
    output_string: unsafe extern "efiapi" fn(*mut SimpleTextOutput, *const u16) -> usize,
}

#[repr(C)]
pub struct SystemTable {
    header: TableHeader,
    firmware_vendor: *mut u16,
    firmware_revision: u32,
    console_in_handle: usize,
    console_in: *mut c_void,
    console_out_handle: usize,
    console_out: *mut SimpleTextOutput,
    standard_error_handle: usize,
    standard_error: *mut SimpleTextOutput,
    runtime_services: *mut c_void,
    boot_services: *mut c_void,
    number_of_table_entries: usize,
    configuration_table: *mut c_void,
}

// UEFI protocol only, while Boot Services are alive. NOT COM1 port I/O.
unsafe fn console(table: *mut SystemTable, message: &[u8]) {
    if table.is_null() { return; }
    let output = (*table).console_out;
    if output.is_null() { return; }
    for part in message.chunks(95) {
        let mut buffer = [0u16; 96];
        for (i, byte) in part.iter().enumerate() {
            buffer[i] = *byte as u16;
        }
        let _status = ((*output).output_string)(output, buffer.as_ptr());
    }
}

#[no_mangle]
pub extern "efiapi" fn efi_main(_image_handle: usize, system_table: *mut SystemTable) -> usize {
    if system_table.is_null() {
        return EFI_LOAD_ERROR;
    }
    if unsafe { (*system_table).header.signature } != EFI_SYSTEM_TABLE_SIGNATURE {
        return EFI_LOAD_ERROR;
    }
    match unios_elf::validate(EMBEDDED_ELF) {
        Err(_) => {
            unsafe { console(system_table, b"UNIOS:L1:E1_REJECTED\r\n"); }
            EFI_LOAD_ERROR
        }
        Ok(checked) => {
            if checked.count == 0 || checked.staged_bytes > unios_elf::MAX_STAGED {
                return EFI_LOAD_ERROR;
            }
            #[cfg(feature="lab-e2r-descriptor-code")]
            {
                let (begin,end)=high_descriptor::bounds();
                if begin==0 || end<=begin || end-begin>256 {return EFI_LOAD_ERROR;}
            }
            #[cfg(feature="lab-e2r-high-entry-code")]
            {
                // Linked candidate is inspected, NEVER invoked. No high-half jump.
                let (begin, end) = high_entry::bounds();
                if begin == 0 || end <= begin || end - begin > 64 {
                    return EFI_LOAD_ERROR;
                }
            }
            #[cfg(feature = "lab-e2-preflight")]
            {
                let service_ptr = unsafe { (*system_table).boot_services.cast::<memory::BootServices>() };
                match memory::preflight(service_ptr, _image_handle, &checked, EMBEDDED_ELF) {
                    Ok(_count) => {
                        // QEMU-only direct call after all code provenance checks.
                        // NOT an IDT-delivered CPU exception or live #DF.
                        #[cfg(feature="lab-e2k-smoke-generic")]
                        unsafe {emergency_handlers::smoke_generic();}
                        #[cfg(feature="lab-e2k-smoke-df")]
                        unsafe {emergency_handlers::smoke_df();}
                        // E2b still remains a pre-ExitBS diagnostic. A newer
                        // map snapshot is mandatory before any future ExitBS.
                        #[cfg(feature = "lab-e2b-staging")]
                        unsafe {
                            #[cfg(feature="lab-e2q-cr3-code")]
                            console(system_table, b"UNIOS:L1:E2Q_BOUNDED_REAL_CR3_BRIDGE_PREFLIGHT_ONLY\r\n");
                            #[cfg(all(feature="lab-e2p-exitbs-contract",not(feature="lab-e2q-cr3-code")))]
                            console(system_table, b"UNIOS:L1:E2P_FINAL_MAP_KEY_VERIFIED_ONLY_NO_EXITBS\r\n");
                            #[cfg(all(feature="lab-e2o-host-private-witness",not(feature="lab-e2p-exitbs-contract")))]
                            console(system_table, b"UNIOS:L1:E2O_INDEPENDENT_NEW_PML4_HOST_WITNESS_ONLY\r\n");
                            #[cfg(all(feature="lab-e2n-transition-closure",not(feature="lab-e2o-host-private-witness")))]
                            console(system_table, b"UNIOS:L1:E2N_RAW_PRIVATE_CR3_CUT_CLOSURE_PREFLIGHT_ONLY\r\n");
                            #[cfg(all(feature="lab-e2m-lab-descriptors",not(feature="lab-e2n-transition-closure")))]
                            console(system_table, b"UNIOS:L1:E2M_REAL_DOUBLE_FAULT_LAB_PREFLIGHT_ONLY\r\n");
                            #[cfg(all(feature="lab-e2l-lab-descriptors",not(feature="lab-e2m-lab-descriptors")))]
                            console(system_table, b"UNIOS:L1:E2L_DORMANT_PHYSICAL_IDT_LAB_PREFLIGHT_ONLY\r\n");
                            #[cfg(all(feature="lab-e2k-emergency-closure",not(feature="lab-e2l-lab-descriptors")))]
                            console(system_table, b"UNIOS:L1:E2K_LINK_BOUNDED_EXCEPTION_HANDLERS_PREFLIGHT_ONLY\r\n");
                            #[cfg(all(feature="lab-e2j-targeted-leaf-proof",not(feature="lab-e2k-emergency-closure")))]
                            console(system_table, b"UNIOS:L1:E2J_HOST_TARGETED_RESERVATION_LEAF_PREFLIGHT_ONLY\r\n");
                            #[cfg(all(feature="lab-e2i-host-table-proof",not(feature="lab-e2j-targeted-leaf-proof")))]
                            console(system_table, b"UNIOS:L1:E2I_HOST_CORROBORATED_UPPER_OLD_TABLES_ONLY\r\n");
                            #[cfg(all(feature="lab-e2h-host-root-proof",not(feature="lab-e2i-host-table-proof")))]
                            console(system_table, b"UNIOS:L1:E2H_HOST_CORROBORATED_ROOT_ONLY\r\n");
                            #[cfg(all(feature="lab-e2g-root-gate",not(feature="lab-e2h-host-root-proof")))]
                            console(system_table, b"UNIOS:L1:E2G_FIRMWARE_CR3_ROOT_DESCRIPTOR_ONLY\r\n");
                            #[cfg(all(feature = "lab-e2f-descriptors", not(feature = "lab-e2g-root-gate")))]
                            console(system_table, b"UNIOS:L1:E2F_DORMANT_GDT_IDT_TSS_PREFLIGHT_ONLY\r\n");
                            #[cfg(all(feature = "lab-e2e-bounded-bridge", not(feature = "lab-e2f-descriptors")))]
                            console(system_table, b"UNIOS:L1:E2E_BOUNDED_BRIDGE_SPAN_ONLY\r\n");
                            #[cfg(all(feature = "lab-e2d-bridge-audit", not(feature = "lab-e2e-bounded-bridge")))]
                            console(system_table, b"UNIOS:L1:E2D_EXEC_ANCHOR_AND_CPU_PREFLIGHT_ONLY\r\n");
                            #[cfg(all(feature = "lab-e2c-tables", not(feature = "lab-e2d-bridge-audit")))]
                            console(system_table, b"UNIOS:L1:E2C_PRIVATE_TABLES_AUDITED_ONLY\r\n");
                            #[cfg(not(feature = "lab-e2c-tables"))]
                            console(system_table, b"UNIOS:L1:E2B_STAGED_AND_VERIFIED_ONLY\r\n");
                        }
                        #[cfg(not(feature = "lab-e2b-staging"))]
                        unsafe { console(system_table, b"UNIOS:L1:E2A_MAP_PREFLIGHT_ONLY\r\n"); }
                        #[cfg(feature = "lab-debug-exit")]
                        unsafe {
                            #[cfg(feature="lab-e2q-cr3-code")]
                            const CODE: u8 = 0x3e;
                            #[cfg(all(feature="lab-e2p-exitbs-contract",not(feature="lab-e2q-cr3-code")))]
                            const CODE: u8 = 0x38;
                            #[cfg(all(feature="lab-e2o-host-private-witness",not(feature="lab-e2p-exitbs-contract")))]
                            const CODE: u8 = 0x30;
                            #[cfg(all(feature="lab-e2n-transition-closure",not(feature="lab-e2o-host-private-witness")))]
                            const CODE: u8 = 0x2f;
                            #[cfg(all(feature="lab-e2m-lab-descriptors",not(feature="lab-e2n-transition-closure")))]
                            const CODE: u8 = 0x2e;
                            #[cfg(all(feature="lab-e2l-lab-descriptors",not(feature="lab-e2m-lab-descriptors")))]
                            const CODE: u8 = 0x2d;
                            #[cfg(all(feature="lab-e2k-emergency-closure",not(feature="lab-e2l-lab-descriptors")))]
                            const CODE: u8 = 0x2c;
                            #[cfg(all(feature="lab-e2j-targeted-leaf-proof",not(feature="lab-e2k-emergency-closure")))]
                            const CODE: u8 = 0x2b;
                            #[cfg(all(feature="lab-e2i-host-table-proof",not(feature="lab-e2j-targeted-leaf-proof")))]
                            const CODE: u8 = 0x2a;
                            #[cfg(all(feature="lab-e2h-host-root-proof",not(feature="lab-e2i-host-table-proof")))]
                            const CODE: u8 = 0x29;
                            #[cfg(all(feature="lab-e2g-root-gate",not(feature="lab-e2h-host-root-proof")))]
                            const CODE: u8 = 0x28;
                            #[cfg(all(feature = "lab-e2f-descriptors", not(feature = "lab-e2g-root-gate")))]
                            const CODE: u8 = 0x27;
                            #[cfg(all(feature = "lab-e2e-bounded-bridge", not(feature = "lab-e2f-descriptors")))]
                            const CODE: u8 = 0x26;
                            #[cfg(all(feature = "lab-e2d-bridge-audit", not(feature = "lab-e2e-bounded-bridge")))]
                            const CODE: u8 = 0x25;
                            #[cfg(all(feature = "lab-e2c-tables", not(feature = "lab-e2d-bridge-audit")))]
                            const CODE: u8 = 0x24;
                            #[cfg(all(feature = "lab-e2b-staging", not(feature = "lab-e2c-tables")))]
                            const CODE: u8 = 0x23;
                            #[cfg(not(feature = "lab-e2b-staging"))]
                            const CODE: u8 = 0x22;
                            asm!("out dx, al", in("dx") 0xf4u16, in("al") CODE,
                                 options(nomem, nostack, preserves_flags));
                        }
                        return EFI_UNSUPPORTED;
                    }
                    Err(err) => {
                        #[cfg(feature="lab-e2p-exitbs-contract")]
                        unsafe {
                            // Explicit FAIL (never the expected E2p
                            // preflight/ExitBS codes) so QEMU cannot
                            // silently time out in OVMF Boot Manager.
                            let code:u8=if matches!(err,memory::Error::FinalMap) {
                                0x3a
                            } else if matches!(err,memory::Error::SwitchCode) {
                                0x3f
                            } else {0x3b};
                            asm!("out dx,al",in("dx") 0xf4u16,
                                 in("al") code,
                                 options(nomem,nostack,preserves_flags));
                        }
                        #[cfg(not(feature="lab-e2p-exitbs-contract"))]
                        let _=err;
                        unsafe { console(system_table, b"UNIOS:L1:E2_PRE_EXIT_REJECTED\r\n"); }
                        return EFI_LOAD_ERROR;
                    }
                }
            }
            #[cfg(not(feature = "lab-e2-preflight"))]
            unsafe { console(system_table, b"UNIOS:L1:E1_PREFLIGHT_ONLY\r\n"); }
            // QEMU-only E1 preflight signal: 0x21 => host exit status 67.
            // DIFFERENT from 0x20 (reserved for full post-handoff success).
            #[cfg(all(feature = "lab-debug-exit", not(feature = "lab-e2-preflight")))]
            unsafe { asm!("out dx, al", in("dx") 0xf4u16, in("al") 0x21u8,
                          options(nomem, nostack, preserves_flags)); }
            #[cfg(not(feature = "lab-e2-preflight"))]
            { EFI_UNSUPPORTED } // NOT a clean kernel boot, and NOT a handoff
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}

use std::{env, fs, process};
use unios_paging::{Builder, Rights, CONTEXT_VA, DESCRIPTORS_VA, MAIN_STACK_VA, IST_STACK_VA, TABLE_WORDS};
use unios_elf::validate;
// Host-only deterministic fixture for a Python inspector, NEVER live firmware table evidence.
fn fail(s: &str) -> ! { eprintln!("{s}"); process::exit(1) }
fn main() {
    let mut a=env::args().skip(1);
    let elf=a.next().unwrap_or_else(||fail("usage: audit <ELF> <PTE-output>"));
    let target=a.next().unwrap_or_else(||fail("missing PTE output path"));
    if a.next().is_some(){fail("unexpected argument")}
    let bytes=fs::read(elf).unwrap_or_else(|_|fail("cannot read ELF"));
    let image=validate(&bytes).unwrap_or_else(|_|fail("invalid kernel ELF"));
    let mut backing=vec![0u64;TABLE_WORDS];
    let mut pt=Builder::new(&mut backing,0x0100_0000).unwrap();
    for (i,segment) in image.segments.iter().take(image.count).enumerate() {
        let pa=0x0200_0000+(i as u64)*0x0100_0000;
        pt.map_kernel_segment(segment,pa).unwrap();
    }
    pt.map_owned_low_bridge_stack(0x2100_0000).unwrap();
    pt.map_stack(MAIN_STACK_VA,0x2200_0000,4).unwrap();
    pt.map_stack(IST_STACK_VA,0x2300_0000,4).unwrap();
    pt.map4k(DESCRIPTORS_VA,0x2400_0000,Rights::Rw).unwrap();
    pt.map4k(CONTEXT_VA,0x2500_0000,Rights::Ro).unwrap();
    pt.audit_guard_pages().unwrap();
    let used=pt.used_tables();
    let mut out=Vec::with_capacity(TABLE_WORDS*8);
    for v in backing { out.extend_from_slice(&v.to_le_bytes()); }
    fs::write(target,out).unwrap_or_else(|_|fail("failed writing PTE image"));
    println!("E2C_HOST_PTE_FIXTURE root=0x01000000 used_tables={used} segment_count={} physical_layout=synthetic",image.count);
    println!("E2C_ASSERT_NO_CR3_SWITCH and no firmware memory-map claim");
}

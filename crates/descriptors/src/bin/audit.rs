// Host-generated deterministic descriptor fixture; NOT a live guest image.
use std::{env,fs,process};
const BASE:u64=0xffffffff83fc0000;
const MAIN:u64=0xffffffff83fe5000;
const DF:u64=0xffffffff83ff5000;
const NORMAL_HANDLER:u64=0x1400_0200;
const DF_HANDLER:u64=0x1400_0400;
fn main(){
    let path=env::args().nth(1).unwrap_or_else(||{
        eprintln!("usage: audit <output file>");process::exit(2)
    });
    let mut page=[0u8;unios_descriptors::PAGE];
    let image=unios_descriptors::build(
        &mut page,BASE,MAIN,DF,NORMAL_HANDLER,DF_HANDLER
    ).expect("failed building descriptor fixture");
    unios_descriptors::verify(
        &page,image,MAIN,DF,NORMAL_HANDLER,DF_HANDLER
    ).expect("failed descriptor post-build audit");
    fs::write(path,page).expect("failed to emit fixture");
    println!("E2F_HOST_DESCRIPTOR_FIXTURE_OK bytes=4096 idt_vectors=32 df_ist=1 layout=synthetic");
}

use std::{env,fs,process};
use unios_rootgate::{check,Owned};
fn main(){
    let path=env::args().nth(1).unwrap_or_else(||{
        eprintln!("usage: audit <out-binary>");process::exit(2)
    });
    let mut map=[0u8;48*3];
    for (i,(kind,pa,pages)) in [
        (4u32,0x12_0000u64,1u64),
        (2,0x20_0000,8),
        (7,0x30_0000,16),
    ].iter().enumerate(){
        let o=i*48;
        map[o..o+4].copy_from_slice(&kind.to_le_bytes());
        map[o+8..o+16].copy_from_slice(&pa.to_le_bytes());
        map[o+24..o+32].copy_from_slice(&pages.to_le_bytes());
    }
    let r=check(0x12_0000,1<<5,&map,map.len(),48,
        &[Owned{base:0x20_0000,pages:8}]).expect("root gate fixture");
    assert_eq!((r.root_pa,r.descriptor_type),(0x12_0000,4));
    fs::write(path,map).expect("write root fixture");
    println!("E2G_HOST_ROOT_FIXTURE_OK root=0x120000 descriptor_type=4 owned_pages=8 synthetic=yes");
}

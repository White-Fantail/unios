use std::{env, fs, process};
fn main() {
    let path = match env::args().nth(1) {
        Some(p) => p, None => { eprintln!("usage: audit <kernel ELF>"); process::exit(2); }
    };
    let bytes = match fs::read(path) {
        Ok(b) => b, Err(e) => { eprintln!("error reading kernel: {e}"); process::exit(2); }
    };
    let image = match unios_elf::validate(&bytes) {
        Ok(i) => i, Err(e) => { eprintln!("ELF failed: {e:?}"); process::exit(1); }
    };
    for (i, seg) in image.segments.iter().take(image.count).enumerate() {
        let span = usize::try_from(seg.page_end - seg.page_start).unwrap();
        let mut stage = vec![0xa5;span];
        if let Err(e) = unios_staging::stage(seg, &bytes, &mut stage) {
            eprintln!("stage {i} failed: {e:?}"); process::exit(1);
        }
        if let Err(e) = unios_staging::verify(seg, &bytes, &stage) {
            eprintln!("verify {i} failed: {e:?}"); process::exit(1);
        }
        println!("E2B_HOST_STAGE_OK segment={} span={} bss={}",
            i, span, seg.mem_size - seg.file_size);
    }
}

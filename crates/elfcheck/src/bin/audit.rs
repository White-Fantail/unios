// Host invocation of the exact no_std loader ELF validator.
// Does not prove UEFI execution, page-table rights or QEMU boot.
use std::{env, fs, process};

fn main() {
    let path = match env::args().nth(1) {
        Some(p) => p,
        None => { eprintln!("usage: audit <kernel.elf>"); process::exit(2); }
    };
    let bytes = match fs::read(&path) {
        Ok(b) => b,
        Err(e) => { eprintln!("cannot read kernel: {e}"); process::exit(2); }
    };
    match unios_elf::validate(&bytes) {
        Ok(image) => {
            println!("LAB-ELF1 ACCEPT entry={:#x} segments={} staged={} bytes",
                image.entry, image.count, image.staged_bytes);
            for segment in image.segments.iter().take(image.count) {
                println!("  [{:#x},{:#x}) flags={} file={} mem={}",
                    segment.page_start, segment.page_end,
                    segment.flags, segment.file_size, segment.mem_size);
            }
        }
        Err(err) => {
            eprintln!("LAB-ELF1 REJECT {err:?}");
            process::exit(1);
        }
    }
}

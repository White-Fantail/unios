#!/usr/bin/env bash
# LAB-H1 E0/E1: build distinct targets; audit ELF and UEFI binaries.
# Not evidence of ExitBootServices, new CR3 or QEMU boot.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
mkdir -p out
command -v rustc
command -v cargo
command -v readelf
command -v file
rustc --version --verbose | tee out/rustc-version.txt
cargo --version | tee out/cargo-version.txt
cargo test --locked -p unios_elf

export CARGO_TARGET_DIR="$ROOT/target"
export RUSTFLAGS="-C relocation-model=static -C link-arg=-no-pie -C link-arg=-T$ROOT/kernel/link.ld -C link-arg=-zmax-page-size=4096"
cargo build --locked --release -p unios_kernel --target x86_64-unknown-none
KERNEL="$ROOT/target/x86_64-unknown-none/release/unios_kernel"
readelf -h "$KERNEL" | tee out/kernel-elf-header.txt
readelf -l "$KERNEL" | tee out/kernel-elf-programs.txt
readelf -r "$KERNEL" | tee out/kernel-elf-relocs.txt
grep -Eq 'Type:.*EXEC' out/kernel-elf-header.txt
grep -Eq 'Machine:.*X86-64' out/kernel-elf-header.txt
if grep -Eq '(INTERP|DYNAMIC|TLS)' out/kernel-elf-programs.txt; then
  echo "ERROR: unsupported ELF program header" >&2; exit 1
fi
if grep -Eq 'R_X86_64_|R_386_' out/kernel-elf-relocs.txt; then
  echo "ERROR: remaining ELF relocations" >&2; exit 1
fi
unset RUSTFLAGS
cargo run --locked --quiet -p unios_elf --bin audit -- "$KERNEL" | tee out/kernel-elf-audit.txt

export UNIOS_KERNEL_ELF="$KERNEL"
cargo build --locked --release -p unios_loader --target x86_64-unknown-uefi
EFI="$ROOT/target/x86_64-unknown-uefi/release/unios_loader.efi"
file "$EFI" | tee out/uefi-pe-header.txt
if ! grep -Eq 'PE32\+.*EFI application' out/uefi-pe-header.txt; then
  echo "ERROR: UEFI artifact is not PE32+ EFI application" >&2; exit 1
fi
mkdir -p out/esp/EFI/BOOT
cp "$EFI" out/esp/EFI/BOOT/BOOTX64.EFI
sha256sum "$KERNEL" "$EFI" | tee out/image-digests.txt
echo "E1_ARTIFACT_BUILT_ONLY (E2-E5 NOT RUN)" | tee out/status.txt

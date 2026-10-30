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
cargo test --locked -p unios_elf -p unios_bootmap -p unios_staging -p unios_paging -p unios_cpugate -p unios_bridgecheck -p unios_descriptors -p unios_rootgate -p unios_exitbs

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
cargo run --locked --quiet -p unios_staging --bin audit -- "$KERNEL" | tee out/kernel-staging-audit.txt
# An actual linked kernel segment must have nonzero BSS in this E2b test.
awk -F'bss=' '/E2B_HOST_STAGE_OK/ { if ($2+0 > 0) found=1 } END { exit !found }' out/kernel-staging-audit.txt

cargo run --locked --quiet -p unios_paging --bin audit -- "$KERNEL" out/e2c-page-tables.bin | tee out/host-pte-builder.txt
python3 scripts/audit_e2c_pte.py "$KERNEL" out/e2c-page-tables.bin | tee out/host-pte-independent-audit.txt
cargo run --locked --quiet -p unios_descriptors --bin audit -- out/e2f-descriptors.bin | tee out/host-descriptor-builder.txt
python3 scripts/audit_e2f_descriptors.py out/e2f-descriptors.bin | tee out/host-descriptor-independent-audit.txt
python3 scripts/audit_e2r_entry.py "$KERNEL" out/e2c-page-tables.bin out/e2f-descriptors.bin | tee out/e2r-entry-independent-audit.txt
cargo run --locked --quiet -p unios_rootgate --bin audit -- out/e2g-host-root-map.bin | tee out/host-root-builder.txt
python3 scripts/audit_e2g_root.py out/e2g-host-root-map.bin | tee out/host-root-independent-audit.txt
export UNIOS_KERNEL_ELF="$KERNEL"
cargo build --locked --release -p unios_loader --features lab-debug-exit,lab-e2q-cr3-code --target x86_64-unknown-uefi
EFI="$ROOT/target/x86_64-unknown-uefi/release/unios_loader.efi"
file "$EFI" | tee out/uefi-pe-header.txt
if ! grep -Eq 'PE32\+.*EFI application' out/uefi-pe-header.txt; then
  echo "ERROR: UEFI artifact is not PE32+ EFI application" >&2; exit 1
fi
python3 scripts/audit_e2k_pe.py "$EFI" | tee out/e2k-independent-pe-handler-audit.txt
python3 scripts/audit_e2q_cr3_pe.py "$EFI" | tee out/e2q-independent-machine-cr3-audit.txt
# E2r-B is deliberately LINKED AND AUDITED OFFLINE, never executed in QEMU.
# Build a separate opt-in PE, require exact jump ABI bytes, then restore and
# byte-compare the ordinary nonfatal E2q image before preparing the QEMU ESP.
cp "$EFI" out/e2r-safe-preflight.efi
cargo build --locked --release -p unios_loader --features lab-debug-exit,lab-e2r-high-entry-code --target x86_64-unknown-uefi
python3 scripts/audit_e2q_cr3_pe.py "$EFI" | tee out/e2r-candidate-cr3-machine-audit.txt
python3 scripts/audit_e2r_high_entry_pe.py "$EFI" | tee out/e2r-candidate-high-entry-machine-audit.txt
# C3: compile an entirely separate, unreachable CPU descriptor candidate.
# The linked precise LGDT/LIDT/LTR/far-CS sequence must be inspected.
cargo build --locked --release -p unios_loader --features lab-debug-exit,lab-e2r-descriptor-code --target x86_64-unknown-uefi
python3 scripts/audit_e2q_cr3_pe.py "$EFI" | tee out/e2r-c3-cr3-machine-audit.txt
python3 scripts/audit_e2r_high_entry_pe.py "$EFI" | tee out/e2r-c3-entry-machine-audit.txt
python3 scripts/audit_e2r_descriptor_pe.py "$EFI" | tee out/e2r-c3-descriptor-machine-audit.txt
# No operational EFI may contain even an unreachable activation candidate.
cargo build --locked --release -p unios_loader --features lab-debug-exit,lab-e2q-cr3-code --target x86_64-unknown-uefi
cmp "$EFI" out/e2r-safe-preflight.efi
python3 scripts/audit_e2r_high_entry_pe.py --absent "$EFI" | tee out/e2r-safe-preflight-absence-proof.txt
python3 scripts/audit_e2r_descriptor_pe.py --absent "$EFI" | tee out/e2r-c3-safe-activation-absence-proof.txt
rm -f out/e2r-safe-preflight.efi
mkdir -p out/esp/EFI/BOOT
cp "$EFI" out/esp/EFI/BOOT/BOOTX64.EFI
sha256sum "$KERNEL" "$EFI" | tee out/image-digests.txt
echo "E2Q_CR3_TERMINAL_BRIDGE_BYTE_AND_PTE_PREFLIGHT_ONLY (E3-E5 NOT RUN)" | tee out/status.txt

#!/usr/bin/env bash
# E2r-C6 opt-in disposable QEMU: genuine high-half ELF _start, diagnostic
# exit 133 only. A completed L1 boot/exit65 and physical hardware are NO-GO.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
BASE_ELF="$ROOT/target/x86_64-unknown-none/release/unios_kernel"
SAFE_EFI="out/esp/EFI/BOOT/BOOTX64.EFI"
test -s "$BASE_ELF"
test -s "$SAFE_EFI"
cp "$SAFE_EFI" out/e2r-c6-safe-efi.efi
restore() {
    cp out/e2r-c6-safe-efi.efi "$SAFE_EFI"
    rm -f out/preflight-disk.img out/preflight-fat.img out/preflight-vars.fd
}
trap restore EXIT
# Build the dedicated ELF with the exact reviewed high-half linker contract.
# Use a DISTINCT path so Rust include_bytes! recompiles the loader rather
# than accidentally embedding a stale normal-kernel ELF.
export RUSTFLAGS="-C relocation-model=static -C link-arg=-no-pie -C link-arg=-T$ROOT/kernel/link.ld -C link-arg=-zmax-page-size=4096"
cargo build --locked --release -p unios_kernel --features lab-e2r-kernel-entry --target x86_64-unknown-none
cp "$BASE_ELF" out/e2r-c6-kernel.elf
unset RUSTFLAGS
readelf -h out/e2r-c6-kernel.elf | tee out/e2r-c6-elf-header.txt
readelf -l out/e2r-c6-kernel.elf | tee out/e2r-c6-elf-loads.txt
cargo run --locked --quiet -p unios_elf --bin audit -- out/e2r-c6-kernel.elf | tee out/e2r-c6-elf-audit.txt
cargo run --locked --quiet -p unios_staging --bin audit -- out/e2r-c6-kernel.elf | tee out/e2r-c6-staging-audit.txt
export UNIOS_KERNEL_ELF="$ROOT/out/e2r-c6-kernel.elf"
cargo build --locked --release -p unios_loader --features lab-e2r-actual-entry --target x86_64-unknown-uefi
EFI=target/x86_64-unknown-uefi/release/unios_loader.efi
python3 scripts/audit_e2q_cr3_pe.py "$EFI" | tee out/e2r-c6-cr3-pe-audit.txt
python3 scripts/audit_e2r_high_entry_pe.py "$EFI" | tee out/e2r-c6-high-jump-pe-audit.txt
python3 scripts/audit_e2r_descriptor_pe.py "$EFI" | tee out/e2r-c6-descriptor-pe-audit.txt
cp "$EFI" "$SAFE_EFI"
UNIOS_E2Q_SMOKE=e2r_handoff bash scripts/qemu-preflight.sh | tee out/e2r-c6-qmp-transcript.txt
grep -q 'E2R_C6_FIFTH_QMP_HIGH_HALF_RIP_RSP_CR3_BSS_CONTEXT_PASS' out/e2r-c6-qmp-transcript.txt
grep -q 'mode=e2r_handoff ' out/e2r-c6-qmp-transcript.txt
grep -q 'new_targets=27 ' out/e2r-c6-qmp-transcript.txt
test "$(cat out/preflight-host-exit.txt)" = 133
python3 - <<'PY'
import json
from pathlib import Path
p=json.loads(Path("out/e2r-c6-kernel-proof.json").read_text())
assert p["result"]=="PASS" and p["kernel_entry_executed"] is True
assert p["kernel_cpu_active_descriptors"] is True
assert p["boot_context_pa_bytes_independently_verified"] is True
assert p["kernel_cpu_rx_physical_entry_provenance"] is True
assert p["kernel_cpu_stack_rw_nx"] is True
assert p["full_clean_boot"] is False
t=json.loads(Path("out/e2q-new-root-host-proof.json").read_text())
assert t["result"]=="PASS" and t["new_target_count"]==27
assert t["descriptor_pa_contents_independently_verified"] is True
assert t["live_descriptors_code_old_new_rx"] is True
print("E2R_C6_REAL_HIGH_HALF_ELF_ENTRY_QMP_PASS (FULL_L1_BOOT_NOT_YET_ACCEPTED)")
PY
cp out/e2q-new-root-host-proof.json out/e2r-c6-new-root-proof.json
cp out/preflight-serial.log out/e2r-c6-serial.log
# Restore the original non-C6 Rust kernel *in the normal target path*.
# Preserve the tested C6 ELF separately under out/ for audit provenance.
# The next E2q/C1/C4 regression must never silently embed the opt-in ELF.
unset UNIOS_KERNEL_ELF
export RUSTFLAGS="-C relocation-model=static -C link-arg=-no-pie -C link-arg=-T$ROOT/kernel/link.ld -C link-arg=-zmax-page-size=4096"
cargo build --locked --release -p unios_kernel --target x86_64-unknown-none
unset RUSTFLAGS
test -s "$BASE_ELF"
if cmp -s "$BASE_ELF" out/e2r-c6-kernel.elf; then
    echo "C6 default and opt-in ELF must be distinct" >&2
    exit 1
fi
echo "E2R_C6_DEFAULT_KERNEL_BINARY_RESTORED" | tee out/e2r-c6-kernel-restore-proof.txt
echo "E2R_C6_REAL_HIGH_HALF_ENTERED_EXIT133_ONLY_NO_CLEAN_BOOT_EXIT65" | tee out/e2r-c6-scope.txt

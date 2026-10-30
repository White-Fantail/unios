#!/usr/bin/env bash
# E2r-C7 disposable QEMU L1 clean-boot acceptance (exit 65) ONLY.
# Never execute on physical machines; never attach personal disks.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
BASE_ELF="$ROOT/target/x86_64-unknown-none/release/unios_kernel"
SAFE_EFI="out/esp/EFI/BOOT/BOOTX64.EFI"
test -s "$BASE_ELF" && test -s "$SAFE_EFI"
cp "$SAFE_EFI" out/e2r-c7-original-safe-efi.efi
restore() {
    cp out/e2r-c7-original-safe-efi.efi "$SAFE_EFI"
    rm -f out/preflight-disk.img out/preflight-fat.img out/preflight-vars.fd
}
trap restore EXIT
# Exact same static ET_EXEC high-half linker, independent C7 feature.
export RUSTFLAGS="-C relocation-model=static -C link-arg=-no-pie -C link-arg=-T$ROOT/kernel/link.ld -C link-arg=-zmax-page-size=4096"
cargo build --locked --release -p unios_kernel --features lab-e2r-clean-pass --target x86_64-unknown-none
cp "$BASE_ELF" out/e2r-c7-kernel.elf
unset RUSTFLAGS
readelf -h out/e2r-c7-kernel.elf | tee out/e2r-c7-elf-header.txt
readelf -l out/e2r-c7-kernel.elf | tee out/e2r-c7-elf-segments.txt
cargo run --locked --quiet -p unios_elf --bin audit -- out/e2r-c7-kernel.elf | tee out/e2r-c7-elf-audit.txt
cargo run --locked --quiet -p unios_staging --bin audit -- out/e2r-c7-kernel.elf | tee out/e2r-c7-staging-audit.txt
export UNIOS_KERNEL_ELF="$ROOT/out/e2r-c7-kernel.elf"
cargo build --locked --release -p unios_loader --features lab-e2r-actual-entry --target x86_64-unknown-uefi
EFI=target/x86_64-unknown-uefi/release/unios_loader.efi
python3 scripts/audit_e2q_cr3_pe.py "$EFI" | tee out/e2r-c7-cr3-pe-audit.txt
python3 scripts/audit_e2r_high_entry_pe.py "$EFI" | tee out/e2r-c7-high-jump-pe-audit.txt
python3 scripts/audit_e2r_descriptor_pe.py "$EFI" | tee out/e2r-c7-descriptor-pe-audit.txt
cp "$EFI" out/e2r-c7-loader.efi
sha256sum out/e2r-c7-kernel.elf out/e2r-c7-loader.efi | tee out/e2r-c7-image-digests.txt
cp "$EFI" "$SAFE_EFI"
# The pinned TCG/OVMF COM2 second pre-cut barrier can occasionally
# timeout. The first VM is rejected and preserved as failure evidence.
# Only this independently diagnosed host-barrier failure can run a
# SECOND fresh disposable VM, with every QMP/PTE/CPU/exit oracle repeated.
# No other error or clean-boot rejection may be silently retried.
C7_LOG=out/e2r-c7-qmp-transcript.txt
if ! UNIOS_E2Q_SMOKE=e2r_clean bash scripts/qemu-preflight.sh > "$C7_LOG" 2>&1; then
    if grep -Eq 'E2Q_(SECOND_HOST_BARRIER_TIMEOUT|POST_CR3_TIMEOUT_DEBUG)' "$C7_LOG"; then
        mv "$C7_LOG" out/e2r-c7-first-VM-COM2-rejected.log
        echo "E2R_C7_DISPOSABLE_VM_RETRY prior=FAILED reason=COM2_BARRIER attempt=2/2"
        rm -f out/e2r-c6-kernel-proof.json out/e2r-c7-clean-boot-proof.json \
              out/e2q-new-root-host-proof.json out/e2q-host-targeted-proof.json \
              out/preflight-outcome.txt out/preflight-host-exit.txt
        if ! UNIOS_E2Q_SMOKE=e2r_clean bash scripts/qemu-preflight.sh > "$C7_LOG" 2>&1; then
            cat "$C7_LOG"
            echo "E2R_C7_BOTH_VM_ATTEMPTS_REJECTED" >&2
            exit 1
        fi
    else
        cat "$C7_LOG"
        echo "E2R_C7_NONRETRYABLE_QEMU_REJECTED" >&2
        exit 1
    fi
fi
cat "$C7_LOG"
grep -q 'E2R_C6_FIFTH_QMP_HIGH_HALF_RIP_RSP_CR3_BSS_CONTEXT_PASS' out/e2r-c7-qmp-transcript.txt
grep -q 'E2R_C7_ORDERED_COM1_AND_EXACT_EXIT65_QMP_PASS' out/e2r-c7-qmp-transcript.txt
grep -q 'mode=e2r_clean ' out/e2r-c7-qmp-transcript.txt
grep -q 'new_targets=27 ' out/e2r-c7-qmp-transcript.txt
test "$(cat out/preflight-host-exit.txt)" = "65"
python3 - <<'PY'
import json
from pathlib import Path
p=json.loads(Path("out/e2r-c7-clean-boot-proof.json").read_text())
assert p["result"]=="PASS" and p["full_clean_boot"] is True
assert p["ordered_com1"] is True and p["exact_qemu_exit"]==65
q=json.loads(Path("out/e2r-c6-kernel-proof.json").read_text())
assert q["result"]=="PASS" and q["kernel_entry_executed"] is True
assert q["kernel_cpu_active_descriptors"] is True
assert q["boot_context_pa_bytes_independently_verified"] is True
n=json.loads(Path("out/e2q-new-root-host-proof.json").read_text())
assert n["result"]=="PASS" and n["new_target_count"]==27
assert n["descriptor_pa_contents_independently_verified"] is True
assert n["live_descriptors_code_old_new_rx"] is True
print("E2R_C7_QEMU_KERNEL_CLEAN_BOOT_EXIT65_PASS (PHYSICAL_HARDWARE_NOT_APPROVED)")
PY
cp out/e2r-c6-kernel-proof.json out/e2r-c7-high-cpu-proof.json
cp out/e2q-new-root-host-proof.json out/e2r-c7-new-root-proof.json
cp out/preflight-serial.log out/e2r-c7-com1-serial.log
cp out/preflight-host-exit.txt out/e2r-c7-host-exit.txt
# Original target/EFI must not silently retain the opt-in success image.
unset UNIOS_KERNEL_ELF
export RUSTFLAGS="-C relocation-model=static -C link-arg=-no-pie -C link-arg=-T$ROOT/kernel/link.ld -C link-arg=-zmax-page-size=4096"
cargo build --locked --release -p unios_kernel --target x86_64-unknown-none
unset RUSTFLAGS
if cmp -s "$BASE_ELF" out/e2r-c7-kernel.elf; then
    echo "C7 normal and opt-in kernel ELF must differ" >&2
    exit 1
fi
echo "E2R_C7_DEFAULT_KERNEL_RESTORED_SAFE_EFI_RESTORED_BY_TRAP" | tee out/e2r-c7-restore-proof.txt

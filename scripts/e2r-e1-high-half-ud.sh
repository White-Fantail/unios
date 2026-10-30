#!/usr/bin/env bash
# E2r-E1: real high-half kernel #UD, native QEMU vector6 trace, fail-stop.
# Not a clean boot: host exit 99, not 65. Disposable QEMU only; no disks.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
BASE_ELF="$ROOT/target/x86_64-unknown-none/release/unios_kernel"
SAFE_EFI=out/esp/EFI/BOOT/BOOTX64.EFI
test -s "$BASE_ELF" && test -s "$SAFE_EFI"
cp "$SAFE_EFI" out/e2r-e1-original-safe-efi.efi
restore() {
  cp out/e2r-e1-original-safe-efi.efi "$SAFE_EFI"
  rm -f out/preflight-disk.img out/preflight-fat.img out/preflight-vars.fd
}
trap restore EXIT
export RUSTFLAGS="-C relocation-model=static -C link-arg=-no-pie -C link-arg=-T$ROOT/kernel/link.ld -C link-arg=-zmax-page-size=4096"
cargo build --locked --release -p unios_kernel --features lab-e2r-high-half-ud --target x86_64-unknown-none
cp "$BASE_ELF" out/e2r-e1-ud-kernel.elf
unset RUSTFLAGS
readelf -h out/e2r-e1-ud-kernel.elf | tee out/e2r-e1-ud-elf-header.txt
cargo run --locked --quiet -p unios_elf --bin audit -- out/e2r-e1-ud-kernel.elf | tee out/e2r-e1-ud-elf-audit.txt
cargo run --locked --quiet -p unios_staging --bin audit -- out/e2r-e1-ud-kernel.elf | tee out/e2r-e1-ud-staging-audit.txt
export UNIOS_KERNEL_ELF="$ROOT/out/e2r-e1-ud-kernel.elf"
cargo build --locked --release -p unios_loader --features lab-e2r-actual-entry --target x86_64-unknown-uefi
EFI=target/x86_64-unknown-uefi/release/unios_loader.efi
python3 scripts/audit_e2q_cr3_pe.py "$EFI" | tee out/e2r-e1-ud-cr3-pe-audit.txt
python3 scripts/audit_e2r_high_entry_pe.py "$EFI" | tee out/e2r-e1-ud-final-jump-audit.txt
python3 scripts/audit_e2r_descriptor_pe.py "$EFI" | tee out/e2r-e1-ud-descriptor-audit.txt
cp "$EFI" out/e2r-e1-ud-loader.efi
sha256sum out/e2r-e1-ud-kernel.elf out/e2r-e1-ud-loader.efi > out/e2r-e1-ud-image-digests.txt
cp "$EFI" "$SAFE_EFI"
# Only known transient second COM2/pre-CR3 handoff timeouts may be retried
# in one FRESH disposable VM, quarantining first failure evidence.
LOG=out/e2r-e1-ud-qmp-transcript.txt
if ! UNIOS_E2Q_SMOKE=e2r_kernel_ud bash scripts/qemu-preflight.sh > "$LOG" 2>&1; then
  if grep -Eq 'E2Q_(SECOND_HOST_BARRIER_TIMEOUT|POST_CR3_TIMEOUT_DEBUG)' "$LOG"; then
    mv "$LOG" out/e2r-e1-first-VM-COM2-rejected.log
    echo "E2R_E1_DISPOSABLE_VM_RETRY attempt=2/2 prior=FAILED"
    rm -f out/e2r-c6-kernel-proof.json out/e2r-e1-ud-host-proof.json \
      out/e2q-new-root-host-proof.json out/e2q-host-targeted-proof.json \
      out/preflight-outcome.txt out/preflight-host-exit.txt
    UNIOS_E2Q_SMOKE=e2r_kernel_ud bash scripts/qemu-preflight.sh > "$LOG" 2>&1 || {
      cat "$LOG"; echo "E2R_E1_RETRY_REJECTED" >&2; exit 1;
    }
  else
    cat "$LOG";echo "E2R_E1_QEMU_REJECTED" >&2;exit 1
  fi
fi
cat "$LOG"
grep -q 'E2R_C6_FIFTH_QMP_HIGH_HALF_RIP_RSP_CR3_BSS_CONTEXT_PASS' "$LOG"
grep -q 'E2R_E1_NATIVE_HIGH_HALF_UD_QMP_PASS exit=99' "$LOG"
grep -q 'mode=e2r_kernel_ud ' "$LOG"
grep -q 'new_targets=27 ' "$LOG"
test "$(cat out/preflight-host-exit.txt)" = 99
python3 scripts/audit_e2r_high_ud_trace.py out/e2r-e1-ud-native-qemu.log | tee out/e2r-e1-native-trace-audit.txt
python3 - <<'PY'
import json
from pathlib import Path
p=json.loads(Path("out/e2r-e1-ud-host-proof.json").read_text())
assert p["result"]=="PASS" and p["kernel_entry_executed"] is True
assert p["live_idt_exception_dispatch"] is True and p["native_vector"]==6
assert p["native_ring"]==0 and p["native_selector"]==8
assert p["native_cpu_origin_i"]==0 and p["exact_fatal_exit"]==99
assert p["no_qemu_reset"] and p["clean_boot"] is False
cpu=json.loads(Path("out/e2r-c6-kernel-proof.json").read_text())
assert cpu["result"]=="PASS" and cpu["kernel_cpu_active_descriptors"] is True
assert cpu["boot_context_pa_bytes_independently_verified"] is True
assert cpu["kernel_entry_executed"] is True
serial=Path("out/preflight-serial.log").read_text(errors="replace")
assert serial.count("UNIOS:L1:E2R_C6_HIGH_HALF_ENTRY")==1
assert "HALTED_PASS" not in serial
print("E2R_E1_REAL_HIGH_HALF_UD_NATIVE_QEMU_99_PASS (NO CLEAN BOOT)")
PY
cp out/e2r-e1-ud-native-qemu.log out/e2r-e1-ud-native-trace-archive.log
cp out/preflight-serial.log out/e2r-e1-ud-serial.log
cp out/preflight-host-exit.txt out/e2r-e1-ud-exit.txt
unset UNIOS_KERNEL_ELF
export RUSTFLAGS="-C relocation-model=static -C link-arg=-no-pie -C link-arg=-T$ROOT/kernel/link.ld -C link-arg=-zmax-page-size=4096"
cargo build --locked --release -p unios_kernel --target x86_64-unknown-none
unset RUSTFLAGS
if cmp -s "$BASE_ELF" out/e2r-e1-ud-kernel.elf; then
  echo "E2r-E1 default kernel must differ from opt-in UD kernel" >&2;exit 1
fi
echo "E2R_E1_NORMAL_KERNEL_RESTORED_FATAL_FEATURE_ONLY" | tee out/e2r-e1-scope.txt

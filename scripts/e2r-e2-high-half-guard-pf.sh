#!/usr/bin/env bash
# E2r-E2: real high-half nonpresent stack guard CPU #PF under private CR3.
# QEMU-only fatal exit 99, never a clean boot, no attached personal media.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
BASE_ELF="$ROOT/target/x86_64-unknown-none/release/unios_kernel"
SAFE_EFI=out/esp/EFI/BOOT/BOOTX64.EFI
test -s "$BASE_ELF" && test -s "$SAFE_EFI"
cp "$SAFE_EFI" out/e2r-e2-safe.efi
restore() {
  cp out/e2r-e2-safe.efi "$SAFE_EFI"
  rm -f out/preflight-disk.img out/preflight-fat.img out/preflight-vars.fd
}
trap restore EXIT
export RUSTFLAGS="-C relocation-model=static -C link-arg=-no-pie -C link-arg=-T$ROOT/kernel/link.ld -C link-arg=-zmax-page-size=4096"
cargo build --locked --release -p unios_kernel --features lab-e2r-main-guard-pf --target x86_64-unknown-none
cp "$BASE_ELF" out/e2r-e2-pf-kernel.elf
unset RUSTFLAGS
readelf -h out/e2r-e2-pf-kernel.elf | tee out/e2r-e2-pf-elf-header.txt
cargo run --locked --quiet -p unios_elf --bin audit -- out/e2r-e2-pf-kernel.elf | tee out/e2r-e2-pf-elf-audit.txt
cargo run --locked --quiet -p unios_staging --bin audit -- out/e2r-e2-pf-kernel.elf | tee out/e2r-e2-pf-staging-audit.txt
export UNIOS_KERNEL_ELF="$ROOT/out/e2r-e2-pf-kernel.elf"
cargo build --locked --release -p unios_loader --features lab-e2r-actual-entry --target x86_64-unknown-uefi
EFI=target/x86_64-unknown-uefi/release/unios_loader.efi
python3 scripts/audit_e2q_cr3_pe.py "$EFI" | tee out/e2r-e2-pf-cr3-audit.txt
python3 scripts/audit_e2r_high_entry_pe.py "$EFI" | tee out/e2r-e2-pf-final-jump-audit.txt
python3 scripts/audit_e2r_descriptor_pe.py "$EFI" | tee out/e2r-e2-pf-descriptor-audit.txt
cp "$EFI" out/e2r-e2-pf-loader.efi
sha256sum out/e2r-e2-pf-kernel.elf out/e2r-e2-pf-loader.efi > out/e2r-e2-pf-image-digests.txt
cp "$EFI" "$SAFE_EFI"
LOG=out/e2r-e2-pf-qmp-transcript.txt
if ! UNIOS_E2Q_SMOKE=e2r_guard_pf bash scripts/qemu-preflight.sh > "$LOG" 2>&1; then
  if grep -Eq 'E2Q_(SECOND_HOST_BARRIER_TIMEOUT|POST_CR3_TIMEOUT_DEBUG)' "$LOG"; then
    mv "$LOG" out/e2r-e2-first-VM-COM2-rejected.log
    echo "E2R_E2_DISPOSABLE_VM_RETRY attempt=2/2 prior=FAILED"
    rm -f out/e2r-c6-kernel-proof.json out/e2r-e2-guard-pf-proof.json \
      out/e2q-new-root-host-proof.json out/e2q-host-targeted-proof.json \
      out/preflight-outcome.txt out/preflight-host-exit.txt
    UNIOS_E2Q_SMOKE=e2r_guard_pf bash scripts/qemu-preflight.sh > "$LOG" 2>&1 || {
      cat "$LOG"; echo "E2R_E2_RETRY_REJECTED" >&2; exit 1;
    }
  else
    cat "$LOG"; echo "E2R_E2_QEMU_REJECTED" >&2; exit 1
  fi
fi
cat "$LOG"
grep -q 'E2R_C6_FIFTH_QMP_HIGH_HALF_RIP_RSP_CR3_BSS_CONTEXT_PASS' "$LOG"
grep -q 'E2R_E2_QMP_LIVE_MAIN_STACK_GUARD_UNMAPPED PASS' "$LOG"
grep -q 'E2R_E2_HIGH_HALF_MAIN_GUARD_PF_QMP_NATIVE_PASS exit=99' "$LOG"
grep -q 'mode=e2r_guard_pf ' "$LOG"
test "$(cat out/preflight-host-exit.txt)" = 99
python3 scripts/audit_e2r_guard_pf_trace.py out/e2r-e2-pf-native-qemu.log |
  tee out/e2r-e2-pf-native-audit.txt
python3 - <<'PY'
import json
from pathlib import Path
from sys import path
path.insert(0,"scripts")
from e2o_new_tree import MAIN_GUARD
p=json.loads(Path("out/e2r-e2-guard-pf-proof.json").read_text())
assert p["result"]=="PASS" and p["kernel_entry_executed"] is True
assert p["live_idt_guard_fault_dispatch"] is True
assert p["private_guard_unmapped_qmp"] is True
assert p["native_vector"]==14 and p["native_fault_error"]==0
assert p["native_cr2"]==hex(MAIN_GUARD)
assert p["native_selector"]==8 and p["native_cpu_origin_i"]==0
assert p["exact_fatal_exit"]==99 and p["clean_boot"] is False
c=json.loads(Path("out/e2r-c6-kernel-proof.json").read_text())
assert c["result"]=="PASS" and c["kernel_entry_executed"] is True
assert c["kernel_cpu_active_descriptors"] is True
serial=Path("out/preflight-serial.log").read_text(errors="replace")
assert serial.count("UNIOS:L1:E2R_C6_HIGH_HALF_ENTRY")==1
assert "HALTED_PASS" not in serial
print("E2R_E2_REAL_HIGH_HALF_MAIN_GUARD_CPU_PF99_PASS (NO CLEAN BOOT)")
PY
cp out/preflight-serial.log out/e2r-e2-pf-serial.log
cp out/preflight-host-exit.txt out/e2r-e2-pf-exit.txt
unset UNIOS_KERNEL_ELF
export RUSTFLAGS="-C relocation-model=static -C link-arg=-no-pie -C link-arg=-T$ROOT/kernel/link.ld -C link-arg=-zmax-page-size=4096"
cargo build --locked --release -p unios_kernel --target x86_64-unknown-none
unset RUSTFLAGS
if cmp -s "$BASE_ELF" out/e2r-e2-pf-kernel.elf; then
  echo "E2r-E2 default kernel must differ from fatal guard test ELF" >&2;exit 1
fi
echo "E2R_E2_NORMAL_KERNEL_RESTORED_GUARD_PF_FATAL_ONLY" | tee out/e2r-e2-scope.txt

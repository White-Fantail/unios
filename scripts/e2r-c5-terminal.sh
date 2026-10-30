#!/usr/bin/env bash
# QEMU-only E2r-C5: real ExitBS/private CR3 and CPU descriptor registers.
# Never upload a default bootable EFI image with actual descriptor activation.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export UNIOS_KERNEL_ELF="$ROOT/target/x86_64-unknown-none/release/unios_kernel"
test -s "$UNIOS_KERNEL_ELF"
cp out/esp/EFI/BOOT/BOOTX64.EFI out/e2r-c5-safe-efi.efi
restore() {
  cp out/e2r-c5-safe-efi.efi out/esp/EFI/BOOT/BOOTX64.EFI
  rm -f out/preflight-disk.img out/preflight-fat.img out/preflight-vars.fd
}
trap restore EXIT
cargo build --locked --release -p unios_loader \
  --features lab-e2r-active-descriptors --target x86_64-unknown-uefi
EFI=target/x86_64-unknown-uefi/release/unios_loader.efi
python3 scripts/audit_e2q_cr3_pe.py "$EFI" | tee out/e2r-c5-existing-cr3-audit.txt
python3 scripts/audit_e2r_descriptor_pe.py "$EFI" | tee out/e2r-c5-existing-descriptor-audit.txt
cp "$EFI" out/esp/EFI/BOOT/BOOTX64.EFI
UNIOS_E2Q_SMOKE=e2r_live bash scripts/qemu-preflight.sh | tee out/e2r-c5-qmp-transcript.txt
grep -q 'E2R_C5_FOURTH_QMP_CPU_GDTR_IDTR_TR_CS_CR3_PASS' out/e2r-c5-qmp-transcript.txt
grep -q 'mode=e2r_live ' out/e2r-c5-qmp-transcript.txt
grep -q 'new_targets=27 ' out/e2r-c5-qmp-transcript.txt
test "$(cat out/preflight-host-exit.txt)" = "129"
python3 - <<'PY'
import json
from pathlib import Path
p=json.loads(Path("out/e2r-c5-live-descriptor-proof.json").read_text())
assert p["result"]=="PASS"
assert p["cpu_descriptor_activation_verified"] is True
assert p["kernel_entry_executed"] is False
t=json.loads(Path("out/e2q-new-root-host-proof.json").read_text())
assert t["result"]=="PASS" and t["new_target_count"]==27
assert t["live_descriptors_code_old_new_rx"] is True
assert t["descriptor_pa_contents_independently_verified"] is True
print("E2R_C5_LIVE_CPU_DESCRIPTORS_PASS (KERNEL_ENTRY=NOT_RUN)")
PY
cp out/e2q-new-root-host-proof.json out/e2r-c5-private-PML4.json
cp out/preflight-serial.log out/e2r-c5-serial.log
cmp out/e2r-c5-safe-efi.efi out/e2q-preflight-loader.efi
echo "E2R_C5_QEMU_CPU_DESCRIPTORS_ONLY_NO_KERNEL_BOOT" | tee out/e2r-c5-scope.txt

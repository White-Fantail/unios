#!/usr/bin/env bash
# E2r-C4: independently prove exact linked (UNREACHABLE) descriptor stub
# EFI old-root + dormant new-root executable PA/rights in disposable QEMU.
# No real LGDT/LIDT/LTR, no high-half kernel entry or clean exit 65.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export UNIOS_KERNEL_ELF="$ROOT/target/x86_64-unknown-none/release/unios_kernel"
test -s "$UNIOS_KERNEL_ELF"
test -s out/esp/EFI/BOOT/BOOTX64.EFI
cp out/esp/EFI/BOOT/BOOTX64.EFI out/e2r-c4-safe-preflight.efi
restore() {
    if test -s out/e2r-c4-safe-preflight.efi; then
        cp out/e2r-c4-safe-preflight.efi out/esp/EFI/BOOT/BOOTX64.EFI
    fi
    rm -f out/preflight-disk.img out/preflight-fat.img out/preflight-vars.fd
}
trap restore EXIT
cargo build --locked --release -p unios_loader \
    --features lab-debug-exit,lab-e2r-descriptor-code \
    --target x86_64-unknown-uefi
EFI=target/x86_64-unknown-uefi/release/unios_loader.efi
python3 scripts/audit_e2q_cr3_pe.py "$EFI" | tee out/e2r-c4-cr3-pe-audit.txt
python3 scripts/audit_e2r_high_entry_pe.py "$EFI" | tee out/e2r-c4-entry-pe-audit.txt
python3 scripts/audit_e2r_descriptor_pe.py "$EFI" | tee out/e2r-c4-descriptor-pe-audit.txt
cp "$EFI" out/esp/EFI/BOOT/BOOTX64.EFI
UNIOS_E2Q_SMOKE=e2r_desc bash scripts/qemu-preflight.sh | tee out/e2r-c4-qmp-preflight.txt
grep -q 'mode=e2r_desc ' out/e2r-c4-qmp-preflight.txt
grep -q 'new_targets=25 ' out/e2r-c4-qmp-preflight.txt
test "$(cat out/preflight-host-exit.txt)" = "125"
python3 - <<'PY'
import json
from pathlib import Path
p=json.loads(Path("out/e2q-new-root-host-proof.json").read_text())
assert p["result"]=="PASS"
assert p["new_target_count"]==25
assert p["high_entry_stub_old_new_rx"] is True
assert p["descriptor_code_old_new_rx"] is True
assert p["descriptor_pa_contents_independently_verified"] is True
assert p["descriptor_activated"] is False
assert p["kernel_entry_executed"] is False
print("E2R_C4_QMP_25_TARGET_DESCRIPTOR_CODE_AND_CONTENTS PASS INERT")
PY
cp out/e2q-new-root-host-proof.json out/e2r-c4-host-proof.json
cp out/preflight-serial.log out/e2r-c4-serial.log
cmp out/e2r-c4-safe-preflight.efi out/e2q-preflight-loader.efi
echo "E2R_C4_DORMANT_ONLY_NO_DESCRIPTOR_ACTIVATION_NO_KERNEL_BOOT" |
    tee out/e2r-c4-status.txt

#!/usr/bin/env bash
# E2r-C1 QEMU (nonfatal): third executable stub code-span proven in dormant
# private PML4 by independent paused-host physical table reads.
# NO ExitBootServices, no new CR3 activation and no high-half entry.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export UNIOS_KERNEL_ELF="$ROOT/target/x86_64-unknown-none/release/unios_kernel"
test -s "$UNIOS_KERNEL_ELF"
test -s out/esp/EFI/BOOT/BOOTX64.EFI
cp out/esp/EFI/BOOT/BOOTX64.EFI out/e2r-c1-safe-preflight.efi
restore() {
    if test -s out/e2r-c1-safe-preflight.efi; then
        cp out/e2r-c1-safe-preflight.efi out/esp/EFI/BOOT/BOOTX64.EFI
    fi
    rm -f out/preflight-disk.img out/preflight-fat.img out/preflight-vars.fd
}
trap restore EXIT
cargo build --locked --release -p unios_loader \
    --features lab-debug-exit,lab-e2r-high-entry-code \
    --target x86_64-unknown-uefi
EFI=target/x86_64-unknown-uefi/release/unios_loader.efi
python3 scripts/audit_e2q_cr3_pe.py "$EFI" | tee out/e2r-c1-cr3-code-audit.txt
python3 scripts/audit_e2r_high_entry_pe.py "$EFI" | tee out/e2r-c1-entry-linked-audit.txt
cp "$EFI" out/esp/EFI/BOOT/BOOTX64.EFI
UNIOS_E2Q_SMOKE=e2r bash scripts/qemu-preflight.sh | tee out/e2r-c1-qemu-host-proof.txt
grep -q 'mode=e2r ' out/e2r-c1-qemu-host-proof.txt
grep -q 'new_targets=23 ' out/e2r-c1-qemu-host-proof.txt
test "$(cat out/preflight-host-exit.txt)" = "125"
python3 - <<'PY'
import json
from pathlib import Path
p=json.loads(Path("out/e2q-new-root-host-proof.json").read_text())
assert p["result"]=="PASS"
assert p["new_target_count"]==23
assert p["high_entry_stub_old_new_rx"] is True
assert p["high_entry_stub_executed"] is False
assert p["kernel_entry_executed"] is False
print("E2R_C1_QMP_23_TARGET_HIGH_ENTRY_RX_PASS (NO HIGH-HALF JUMP)")
PY
cp out/e2q-new-root-host-proof.json out/e2r-c1-private-host-proof.json
cp out/preflight-serial.log out/e2r-c1-serial.log
cp out/preflight-outcome.txt out/e2r-c1-outcome.txt
# The currently installed ESP image is intentionally restored by trap.
cmp out/e2r-c1-safe-preflight.efi out/e2q-preflight-loader.efi
echo "E2R_C1_ONLY_NO_EXITBS_NO_NEW_CR3_NO_KERNEL_BOOT" | tee out/e2r-c1-status.txt

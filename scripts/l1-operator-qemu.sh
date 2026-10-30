#!/usr/bin/env bash
# UniOS L1 operator QEMU-only acceptance, isolated LAB-H1 profile.
# No guest network, real disks, physical UEFI boot or USB writes.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
if [[ "$(uname -s)" != Linux ]] || [[ "$(id -u)" == 0 ]]; then
  echo "Only a non-root Linux operator can run the validated QEMU test" >&2
  exit 2
fi
if ! grep -q '^ID=ubuntu$' /etc/os-release ||
   ! grep -q '^VERSION_ID="24.04"$' /etc/os-release; then
  echo "Only the verified Ubuntu 24.04 environment is admitted" >&2
  exit 2
fi
for tool in rustc cargo readelf file python3 qemu-system-x86_64 sgdisk mkfs.vfat mcopy sha256sum git; do
  command -v "$tool" >/dev/null || { echo "Missing tool: $tool" >&2; exit 2; }
done
mkdir -p out
if [[ "$(git branch --show-current)" != main ]] ||
   [[ -n "$(git status --porcelain --untracked-files=no)" ]]; then
  echo "Checkout a clean main branch before running L1 operator QEMU" >&2
  exit 2
fi
git rev-parse HEAD | tee out/e2r-d-tested-source-sha.txt
bash scripts/build-l1.sh
bash scripts/qemu-preflight.sh
bash scripts/e2r-c7-clean-boot.sh
sha256sum --check out/e2r-c7-image-digests.txt
python3 - <<'PY'
import json
from pathlib import Path
proof=json.loads(Path("out/e2r-c7-clean-boot-proof.json").read_text())
assert proof["result"]=="PASS"
assert proof["full_clean_boot"] is True
assert proof["fifth_host_cpu_proof_required"] is True
assert proof["exact_qemu_exit"]==65
assert Path("out/e2r-c7-host-exit.txt").read_text().strip()=="65"
serial=Path("out/e2r-c7-com1-serial.log").read_text(errors="replace")
from sys import path
path.insert(0,"scripts")
from e2r_clean_oracle import verify_trace
assert verify_trace(serial,65,True)["full_clean_boot"]
assert Path("out/e2r-c7-restore-proof.txt").is_file()
print("UNIOS_L1_OPERATOR_QEMU_ACCEPTANCE_PASS exit=65")
PY
test ! -e out/preflight-disk.img
test ! -e out/preflight-fat.img
test ! -e out/preflight-vars.fd
echo "UNIOS_L1_OPERATOR_QEMU_ONLY_COMPLETE (NO PHYSICAL MACHINE SUPPORT)"

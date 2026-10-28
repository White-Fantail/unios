#!/usr/bin/env bash
# E2e QEMU: link-bounded EFI code under old/new map; NEVER CR3 switch.
# Distinct debug-exit byte 0x26 returns 77; 65 is reserved for E4 clean PASS.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
mkdir -p out
for tool in qemu-system-x86_64 sgdisk mkfs.vfat mmd mcopy timeout sha256sum; do
  command -v "$tool" >/dev/null
done
CODE=/usr/share/OVMF/OVMF_CODE_4M.fd
VARS_TEMPLATE=/usr/share/OVMF/OVMF_VARS_4M.fd
test -s "$CODE"
test -s "$VARS_TEMPLATE"
cp "$VARS_TEMPLATE" out/preflight-vars.fd
# Disposable GPT ESP only; there is NO attached writable personal disk.
truncate -s 64M out/preflight-disk.img
sgdisk -o -n 1:2048:+48M -t 1:ef00 out/preflight-disk.img >/dev/null
truncate -s 48M out/preflight-fat.img
mkfs.vfat -F 32 -n UNIOS_LAB out/preflight-fat.img >/dev/null
mmd -i out/preflight-fat.img ::/EFI
mmd -i out/preflight-fat.img ::/EFI/BOOT
mcopy -i out/preflight-fat.img out/esp/EFI/BOOT/BOOTX64.EFI ::/EFI/BOOT/BOOTX64.EFI
dd if=out/preflight-fat.img of=out/preflight-disk.img bs=1M seek=1 conv=notrunc status=none
qemu-system-x86_64 --version | tee out/qemu-version.txt
sha256sum "$CODE" "$VARS_TEMPLATE" "$(command -v qemu-system-x86_64)" |
  tee out/preflight-host-digests.txt
cat > out/qemu-profile.txt <<'PROFILE'
LAB-H1 E2e-only, Q35 TCG, qemu64 NX, LA57=0, PCIDE=0,
1 vCPU, 256 MiB, OVMF DevUnsigned, immutable CODE,
disposable pflash VARS, disposable GPT ESP with ephemeral snapshot overlay,
no network and no real user media, COM1 to finite log.
E2e host exit=77, full L1 CLEAN_PASS exit=65 (NOT ENABLED).
PROFILE
set +e
timeout --signal=TERM --kill-after=5s 35s qemu-system-x86_64 \
  -machine q35,accel=tcg -cpu qemu64,+nx,-la57,-pcid \
  -smp 1 -m 256 -nic none \
  -drive if=pflash,unit=0,format=raw,readonly=on,file="$CODE" \
  -drive if=pflash,unit=1,format=raw,file="$ROOT/out/preflight-vars.fd" \
  -drive if=none,id=esp,format=raw,file="$ROOT/out/preflight-disk.img",snapshot=on \
  -device ide-hd,drive=esp \
  -device isa-debug-exit,iobase=0xf4,iosize=0x04 \
  -serial file:"$ROOT/out/preflight-serial.log" \
  -display none -monitor none -no-reboot -no-shutdown \
  >out/preflight-qemu-stdout.log 2>out/preflight-qemu-stderr.log
rc=$?
set -e
echo "$rc" | tee out/preflight-host-exit.txt
if [[ "$rc" -ne 77 ]]; then
  echo "ERROR: E2e bounded executable span preflight was not independently observed (code $rc)" >&2
  cat out/preflight-qemu-stderr.log >&2
  exit 1
fi
if grep -Eq 'HALTED_PASS' out/preflight-serial.log; then
  echo "ERROR: E1 smoke printed forbidden full-kernel success" >&2
  exit 1
fi
echo "E2E_QEMU_BOUNDED_BRIDGE_PREFLIGHT_ONLY (NO EXITBS / NO CR3 / NO CLEAN_PASS)" |
  tee out/preflight-outcome.txt

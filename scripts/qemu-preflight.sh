#!/usr/bin/env bash
# E2q QEMU: old/new tables and post-ExitBS actual new-CR3 CPU state terminal experiment.
# Distinct debug-exit byte 0x3e returns 125; opt-in actual CR3 0x3c returns 121; 65 is reserved for E4 clean PASS.
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
LAB-H1 E2q-only, Q35 TCG, qemu64 NX, LA57=0, PCIDE=0,
1 vCPU, 256 MiB, OVMF DevUnsigned, immutable CODE,
disposable pflash VARS, disposable GPT ESP with ephemeral snapshot overlay,
no network and no real user media; COM1 log, COM2 host barrier and QMP.
E2q host preflight exit=125, optional post-ExitBS CR3=121, full L1 CLEAN_PASS exit=65 (NOT ENABLED).
PROFILE
# Host controls a separate COM2 Unix socket and pauses vCPU0 at the root
# dereference checkpoint. Guest cannot advance without successful QMP proof.
# CPU-origin evidence: QEMU's native exception trace is collected only
# for the opt-in nested-fault VM (not the default preflight).
LOG_ARGS=()
if [[ "${UNIOS_E2Q_SMOKE:-preflight}" == "cr3" ]]; then
  LOG_ARGS=(-d int,cpu_reset -D "$ROOT/out/e2q-cr3-qemu-exception-trace.log")
elif [[ "${UNIOS_E2Q_SMOKE:-preflight}" == "df" ]]; then
  LOG_ARGS=(-d int -D "$ROOT/out/e2q-real-df-qemu-trace.log")
elif [[ "${UNIOS_E2Q_SMOKE:-preflight}" == "e2r_kernel_ud" ]]; then
  # QEMU's independent CPU-generated exception trace is mandatory.
  # The guest cannot forge this host-side source using COM1 or debug-exit.
  LOG_ARGS=(-d int -D "$ROOT/out/e2r-e1-ud-native-qemu.log")
fi
python3 scripts/e2q_host.py -- qemu-system-x86_64 \
  "${LOG_ARGS[@]}" \
  -machine q35,accel=tcg -cpu qemu64,+nx,-la57,-pcid \
  -smp 1 -m 256 -nic none \
  -drive if=pflash,unit=0,format=raw,readonly=on,file="$CODE" \
  -drive if=pflash,unit=1,format=raw,file="$ROOT/out/preflight-vars.fd" \
  -drive if=none,id=esp,format=raw,file="$ROOT/out/preflight-disk.img",snapshot=on \
  -device ide-hd,drive=esp \
  -device isa-debug-exit,iobase=0xf4,iosize=0x04 \
  -serial file:"$ROOT/out/preflight-serial.log" \
  -display none -monitor none -no-reboot -no-shutdown

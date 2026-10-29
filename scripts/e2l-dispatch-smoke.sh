#!/usr/bin/env bash
# E2l QEMU-only CPU-delivered UD2 #UD, and INT8 software-interrupt IST1; NOT a real #DF.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export UNIOS_KERNEL_ELF="$ROOT/target/x86_64-unknown-none/release/unios_kernel"
test -s "$UNIOS_KERNEL_ELF"
# Preserve the audited safe E2l preflight image. Smoke variants are
# intentionally fatal and must NEVER become the default CI artifact.
cp out/esp/EFI/BOOT/BOOTX64.EFI out/e2l-preflight-loader.efi
cp out/preflight-serial.log out/e2l-preflight-serial.log
cp out/preflight-outcome.txt out/e2l-preflight-outcome.txt
cp out/preflight-host-exit.txt out/e2l-preflight-host-exit.txt
cp out/e2l-host-targeted-proof.json out/e2l-preflight-host-proof.json
restore_default_image() {
  if test -s out/e2l-preflight-loader.efi; then
    cp out/e2l-preflight-loader.efi out/esp/EFI/BOOT/BOOTX64.EFI
  fi
}
trap restore_default_image EXIT
for mode in ud ist; do
  cargo build --locked --release -p unios_loader \
    --features "lab-debug-exit,lab-e2l-lab-descriptors,lab-e2l-smoke-$mode" \
    --target x86_64-unknown-uefi
  cp target/x86_64-unknown-uefi/release/unios_loader.efi out/esp/EFI/BOOT/BOOTX64.EFI
  python3 scripts/audit_e2k_pe.py out/esp/EFI/BOOT/BOOTX64.EFI |
    tee "out/e2l-$mode-independent-pe.txt"
  UNIOS_E2L_SMOKE="$mode" bash scripts/qemu-preflight.sh |
    tee "out/e2l-$mode-smoke-host.txt"
  cp out/preflight-outcome.txt "out/e2l-$mode-smoke-outcome.txt"
  cp out/preflight-serial.log "out/e2l-$mode-smoke-serial.log"
  cp out/e2l-host-targeted-proof.json "out/e2l-$mode-smoke-host-proof.json"
  printf "E2L_%s_ACTIVE_IDT_SMOKE_ONLY (UD2=REAL_UD; INT8=SOFTWARE_INTERRUPT_NOT_REAL_DF)\n" "$mode" |
    tee "out/e2l-$mode-smoke-scope.txt"
done

# Restore the non-fatal preflight image and final release target artifact.
cargo build --locked --release -p unios_loader \
  --features "lab-debug-exit,lab-e2l-lab-descriptors" \
  --target x86_64-unknown-uefi
cmp target/x86_64-unknown-uefi/release/unios_loader.efi out/e2l-preflight-loader.efi
cp out/e2l-preflight-loader.efi out/esp/EFI/BOOT/BOOTX64.EFI
cp out/e2l-preflight-serial.log out/preflight-serial.log
cp out/e2l-preflight-outcome.txt out/preflight-outcome.txt
cp out/e2l-preflight-host-exit.txt out/preflight-host-exit.txt
cp out/e2l-preflight-host-proof.json out/e2l-host-targeted-proof.json
# Never upload an implicitly bootable disposable GPT holding a fatal
# exception dispatch smoke EFI; retain separate text and digest evidence only.
rm -f out/preflight-disk.img out/preflight-fat.img out/preflight-vars.fd
echo "E2L_PRE_FLIGHT_DEFAULT_RESTORED (NO FATAL SMOKE BY DEFAULT)" |
  tee out/e2l-artifact-restore-proof.txt

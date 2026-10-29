#!/usr/bin/env bash
# E2k QEMU-only direct-call fatal handler path; NOT live CPU exceptions.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export UNIOS_KERNEL_ELF="$ROOT/target/x86_64-unknown-none/release/unios_kernel"
test -s "$UNIOS_KERNEL_ELF"
# Preserve the audited safe E2k preflight image. Smoke variants are
# intentionally fatal and must NEVER become the default CI artifact.
cp out/esp/EFI/BOOT/BOOTX64.EFI out/e2k-preflight-loader.efi
cp out/preflight-serial.log out/e2k-preflight-serial.log
cp out/preflight-outcome.txt out/e2k-preflight-outcome.txt
cp out/preflight-host-exit.txt out/e2k-preflight-host-exit.txt
cp out/e2k-host-targeted-proof.json out/e2k-preflight-host-proof.json
restore_default_image() {
  if test -s out/e2k-preflight-loader.efi; then
    cp out/e2k-preflight-loader.efi out/esp/EFI/BOOT/BOOTX64.EFI
  fi
}
trap restore_default_image EXIT
for mode in generic df; do
  cargo build --locked --release -p unios_loader \
    --features "lab-debug-exit,lab-e2k-emergency-closure,lab-e2k-smoke-$mode" \
    --target x86_64-unknown-uefi
  cp target/x86_64-unknown-uefi/release/unios_loader.efi out/esp/EFI/BOOT/BOOTX64.EFI
  python3 scripts/audit_e2k_pe.py out/esp/EFI/BOOT/BOOTX64.EFI |
    tee "out/e2k-$mode-independent-pe.txt"
  UNIOS_E2K_SMOKE="$mode" bash scripts/qemu-preflight.sh |
    tee "out/e2k-$mode-smoke-host.txt"
  cp out/preflight-outcome.txt "out/e2k-$mode-smoke-outcome.txt"
  cp out/preflight-serial.log "out/e2k-$mode-smoke-serial.log"
  cp out/e2k-host-targeted-proof.json "out/e2k-$mode-smoke-host-proof.json"
  printf "E2K_%s_DIRECT_CALL_SMOKE_ONLY (NOT CPU EXCEPTION)\n" "$mode" |
    tee "out/e2k-$mode-smoke-scope.txt"
done

# Restore the non-fatal preflight image and final release target artifact.
cargo build --locked --release -p unios_loader \
  --features "lab-debug-exit,lab-e2k-emergency-closure" \
  --target x86_64-unknown-uefi
cmp target/x86_64-unknown-uefi/release/unios_loader.efi out/e2k-preflight-loader.efi
cp out/e2k-preflight-loader.efi out/esp/EFI/BOOT/BOOTX64.EFI
cp out/e2k-preflight-serial.log out/preflight-serial.log
cp out/e2k-preflight-outcome.txt out/preflight-outcome.txt
cp out/e2k-preflight-host-exit.txt out/preflight-host-exit.txt
cp out/e2k-preflight-host-proof.json out/e2k-host-targeted-proof.json
# Never upload an implicitly bootable disposable GPT holding a fatal
# direct-call smoke EFI; retain separate text and digest evidence only.
rm -f out/preflight-disk.img out/preflight-fat.img out/preflight-vars.fd
echo "E2K_PRE_FLIGHT_DEFAULT_RESTORED (NO FATAL SMOKE BY DEFAULT)" |
  tee out/e2k-artifact-restore-proof.txt

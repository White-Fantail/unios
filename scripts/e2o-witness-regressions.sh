#!/usr/bin/env bash
# E2o: independent second QMP new-PML4 witness precedes terminal CPU #UD/IST/real #DF regressions.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export UNIOS_KERNEL_ELF="$ROOT/target/x86_64-unknown-none/release/unios_kernel"
test -s "$UNIOS_KERNEL_ELF"
# Preserve the audited safe E2o preflight image. Smoke variants are
# intentionally fatal and must NEVER become the default CI artifact.
cp out/esp/EFI/BOOT/BOOTX64.EFI out/e2o-preflight-loader.efi
cp out/preflight-serial.log out/e2o-preflight-serial.log
cp out/preflight-outcome.txt out/e2o-preflight-outcome.txt
cp out/preflight-host-exit.txt out/e2o-preflight-host-exit.txt
cp out/e2o-host-targeted-proof.json out/e2o-preflight-host-proof.json
cp out/e2o-new-root-host-proof.json out/e2o-preflight-new-root-proof.json
restore_default_image() {
  if test -s out/e2o-preflight-loader.efi; then
    cp out/e2o-preflight-loader.efi out/esp/EFI/BOOT/BOOTX64.EFI
  fi
}
trap restore_default_image EXIT
for mode in ud ist df; do
  if [[ "$mode" == "df" ]]; then
    FEATURES="lab-debug-exit,lab-e2o-host-private-witness,lab-e2m-smoke-real-df"
  else
    FEATURES="lab-debug-exit,lab-e2o-host-private-witness,lab-e2l-smoke-$mode"
  fi
  cargo build --locked --release -p unios_loader \
    --features "$FEATURES" \
    --target x86_64-unknown-uefi
  cp target/x86_64-unknown-uefi/release/unios_loader.efi out/esp/EFI/BOOT/BOOTX64.EFI
  python3 scripts/audit_e2k_pe.py out/esp/EFI/BOOT/BOOTX64.EFI |
    tee "out/e2o-$mode-independent-pe.txt"
  UNIOS_E2O_SMOKE="$mode" bash scripts/qemu-preflight.sh |
    tee "out/e2o-$mode-smoke-host.txt"
  cp out/preflight-outcome.txt "out/e2o-$mode-smoke-outcome.txt"
  cp out/preflight-serial.log "out/e2o-$mode-smoke-serial.log"
  cp out/e2o-host-targeted-proof.json "out/e2o-$mode-smoke-host-proof.json"
  cp out/e2o-new-root-host-proof.json "out/e2o-$mode-new-root-proof.json"
  if [[ "$mode" == "df" ]]; then
    python3 scripts/audit_e2m_df_trace.py out/e2o-real-df-qemu-trace.log |
      tee out/e2o-df-native-trace-proof.txt
    cp out/e2o-real-df-qemu-trace.log out/e2o-df-native-trace-artifact.log
  fi
  printf "E2O_%s_TERMINAL_VM_EXCEPTION_ONLY (DF=CPU_GENERATED; INT8=SOFTWARE_VECTOR)\n" "$mode" |
    tee "out/e2o-$mode-smoke-scope.txt"
done

# Restore the non-fatal preflight image and final release target artifact.
cargo build --locked --release -p unios_loader \
  --features "lab-debug-exit,lab-e2o-host-private-witness" \
  --target x86_64-unknown-uefi
cmp target/x86_64-unknown-uefi/release/unios_loader.efi out/e2o-preflight-loader.efi
cp out/e2o-preflight-loader.efi out/esp/EFI/BOOT/BOOTX64.EFI
cp out/e2o-preflight-serial.log out/preflight-serial.log
cp out/e2o-preflight-outcome.txt out/preflight-outcome.txt
cp out/e2o-preflight-host-exit.txt out/preflight-host-exit.txt
cp out/e2o-preflight-host-proof.json out/e2o-host-targeted-proof.json
cp out/e2o-preflight-new-root-proof.json out/e2o-new-root-host-proof.json
# Never upload an implicitly bootable disposable GPT holding a fatal
# exception dispatch smoke EFI; retain separate text and digest evidence only.
rm -f out/preflight-disk.img out/preflight-fat.img out/preflight-vars.fd
echo "E2O_PRE_FLIGHT_DEFAULT_RESTORED (NO FATAL SMOKE BY DEFAULT)" |
  tee out/e2o-artifact-restore-proof.txt

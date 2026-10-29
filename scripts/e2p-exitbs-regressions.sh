#!/usr/bin/env bash
# E2p: independent second QMP new-PML4 witness precedes terminal CPU #UD/IST/real #DF regressions.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export UNIOS_KERNEL_ELF="$ROOT/target/x86_64-unknown-none/release/unios_kernel"
test -s "$UNIOS_KERNEL_ELF"
# Preserve the audited safe E2p preflight image. Smoke variants are
# intentionally fatal and must NEVER become the default CI artifact.
cp out/esp/EFI/BOOT/BOOTX64.EFI out/e2p-preflight-loader.efi
cp out/preflight-serial.log out/e2p-preflight-serial.log
cp out/preflight-outcome.txt out/e2p-preflight-outcome.txt
cp out/preflight-host-exit.txt out/e2p-preflight-host-exit.txt
cp out/e2p-host-targeted-proof.json out/e2p-preflight-host-proof.json
cp out/e2p-new-root-host-proof.json out/e2p-preflight-new-root-proof.json
restore_default_image() {
  if test -s out/e2p-preflight-loader.efi; then
    cp out/e2p-preflight-loader.efi out/esp/EFI/BOOT/BOOTX64.EFI
  fi
}
trap restore_default_image EXIT
for mode in ud ist df exitbs; do
  if [[ "$mode" == "df" ]]; then
    FEATURES="lab-debug-exit,lab-e2p-exitbs-contract,lab-e2m-smoke-real-df"
  elif [[ "$mode" == "exitbs" ]]; then
    FEATURES="lab-debug-exit,lab-e2p-actual-exitbs"
  else
    FEATURES="lab-debug-exit,lab-e2p-exitbs-contract,lab-e2l-smoke-$mode"
  fi
  cargo build --locked --release -p unios_loader \
    --features "$FEATURES" \
    --target x86_64-unknown-uefi
  cp target/x86_64-unknown-uefi/release/unios_loader.efi out/esp/EFI/BOOT/BOOTX64.EFI
  python3 scripts/audit_e2k_pe.py out/esp/EFI/BOOT/BOOTX64.EFI |
    tee "out/e2p-$mode-independent-pe.txt"
  UNIOS_E2P_SMOKE="$mode" bash scripts/qemu-preflight.sh |
    tee "out/e2p-$mode-smoke-host.txt"
  cp out/preflight-outcome.txt "out/e2p-$mode-smoke-outcome.txt"
  cp out/preflight-serial.log "out/e2p-$mode-smoke-serial.log"
  cp out/e2p-host-targeted-proof.json "out/e2p-$mode-smoke-host-proof.json"
  cp out/e2p-new-root-host-proof.json "out/e2p-$mode-new-root-proof.json"
  if [[ "$mode" == "df" ]]; then
    python3 scripts/audit_e2m_df_trace.py out/e2p-real-df-qemu-trace.log |
      tee out/e2p-df-native-trace-proof.txt
    cp out/e2p-real-df-qemu-trace.log out/e2p-df-native-trace-artifact.log
  fi
  printf "E2P_%s_TERMINAL_QEMU_ONLY (EXITBS_TERMINATES; NO_NEW_CR3; NO_KERNEL_BOOT)\n" "$mode" |
    tee "out/e2p-$mode-smoke-scope.txt"
done

# Restore the non-fatal preflight image and final release target artifact.
cargo build --locked --release -p unios_loader \
  --features "lab-debug-exit,lab-e2p-exitbs-contract" \
  --target x86_64-unknown-uefi
cmp target/x86_64-unknown-uefi/release/unios_loader.efi out/e2p-preflight-loader.efi
cp out/e2p-preflight-loader.efi out/esp/EFI/BOOT/BOOTX64.EFI
cp out/e2p-preflight-serial.log out/preflight-serial.log
cp out/e2p-preflight-outcome.txt out/preflight-outcome.txt
cp out/e2p-preflight-host-exit.txt out/preflight-host-exit.txt
cp out/e2p-preflight-host-proof.json out/e2p-host-targeted-proof.json
cp out/e2p-preflight-new-root-proof.json out/e2p-new-root-host-proof.json
# Never upload an implicitly bootable disposable GPT holding a fatal
# exception dispatch smoke EFI; retain separate text and digest evidence only.
rm -f out/preflight-disk.img out/preflight-fat.img out/preflight-vars.fd
echo "E2P_PRE_FLIGHT_DEFAULT_RESTORED (NO FATAL SMOKE BY DEFAULT)" |
  tee out/e2p-artifact-restore-proof.txt

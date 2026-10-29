#!/usr/bin/env bash
# E2q: independent second QMP new-PML4 witness precedes terminal CPU #UD/IST/real #DF regressions.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export UNIOS_KERNEL_ELF="$ROOT/target/x86_64-unknown-none/release/unios_kernel"
test -s "$UNIOS_KERNEL_ELF"
# Preserve the audited safe E2q preflight image. Smoke variants are
# intentionally fatal and must NEVER become the default CI artifact.
cp out/esp/EFI/BOOT/BOOTX64.EFI out/e2q-preflight-loader.efi
cp out/preflight-serial.log out/e2q-preflight-serial.log
cp out/preflight-outcome.txt out/e2q-preflight-outcome.txt
cp out/preflight-host-exit.txt out/e2q-preflight-host-exit.txt
cp out/e2q-host-targeted-proof.json out/e2q-preflight-host-proof.json
cp out/e2q-new-root-host-proof.json out/e2q-preflight-new-root-proof.json
restore_default_image() {
  if test -s out/e2q-preflight-loader.efi; then
    cp out/e2q-preflight-loader.efi out/esp/EFI/BOOT/BOOTX64.EFI
  fi
}
trap restore_default_image EXIT
for mode in ud ist df exitbs cr3; do
  if [[ "$mode" == "df" ]]; then
    FEATURES="lab-debug-exit,lab-e2q-cr3-code,lab-e2m-smoke-real-df"
  elif [[ "$mode" == "cr3" ]]; then
    FEATURES="lab-debug-exit,lab-e2q-actual-cr3"
  elif [[ "$mode" == "exitbs" ]]; then
    FEATURES="lab-debug-exit,lab-e2q-cr3-code,lab-e2p-actual-exitbs"
  else
    FEATURES="lab-debug-exit,lab-e2q-cr3-code,lab-e2l-smoke-$mode"
  fi
  cargo build --locked --release -p unios_loader \
    --features "$FEATURES" \
    --target x86_64-unknown-uefi
  cp target/x86_64-unknown-uefi/release/unios_loader.efi out/esp/EFI/BOOT/BOOTX64.EFI
  python3 scripts/audit_e2k_pe.py out/esp/EFI/BOOT/BOOTX64.EFI |
    tee "out/e2q-$mode-independent-pe.txt"
  UNIOS_E2Q_SMOKE="$mode" bash scripts/qemu-preflight.sh |
    tee "out/e2q-$mode-smoke-host.txt"
  cp out/preflight-outcome.txt "out/e2q-$mode-smoke-outcome.txt"
  cp out/preflight-serial.log "out/e2q-$mode-smoke-serial.log"
  cp out/e2q-host-targeted-proof.json "out/e2q-$mode-smoke-host-proof.json"
  cp out/e2q-new-root-host-proof.json "out/e2q-$mode-new-root-proof.json"
  if [[ "$mode" == "cr3" ]]; then
    cp out/e2q-live-cr3-proof.json out/e2q-cr3-live-host-proof.json
    cp out/e2q-post-cr3-cpu-registers.txt out/e2q-cr3-live-cpu-registers.txt
  fi
  if [[ "$mode" == "df" ]]; then
    python3 scripts/audit_e2m_df_trace.py out/e2q-real-df-qemu-trace.log |
      tee out/e2q-df-native-trace-proof.txt
    cp out/e2q-real-df-qemu-trace.log out/e2q-df-native-trace-artifact.log
  fi
  printf "E2Q_%s_TERMINAL_QEMU_ONLY (EXITBS_AND_NEW_CR3_ONLY_IN_CR3_VARIANT; NO_HIGH_HALF_KERNEL_BOOT)\n" "$mode" |
    tee "out/e2q-$mode-smoke-scope.txt"
done

# Restore the non-fatal preflight image and final release target artifact.
cargo build --locked --release -p unios_loader \
  --features "lab-debug-exit,lab-e2q-cr3-code" \
  --target x86_64-unknown-uefi
cmp target/x86_64-unknown-uefi/release/unios_loader.efi out/e2q-preflight-loader.efi
cp out/e2q-preflight-loader.efi out/esp/EFI/BOOT/BOOTX64.EFI
cp out/e2q-preflight-serial.log out/preflight-serial.log
cp out/e2q-preflight-outcome.txt out/preflight-outcome.txt
cp out/e2q-preflight-host-exit.txt out/preflight-host-exit.txt
cp out/e2q-preflight-host-proof.json out/e2q-host-targeted-proof.json
cp out/e2q-preflight-new-root-proof.json out/e2q-new-root-host-proof.json
# Never upload an implicitly bootable disposable GPT holding a fatal
# exception dispatch smoke EFI; retain separate text and digest evidence only.
rm -f out/preflight-disk.img out/preflight-fat.img out/preflight-vars.fd
echo "E2Q_PRE_FLIGHT_DEFAULT_RESTORED (NO FATAL SMOKE BY DEFAULT)" |
  tee out/e2q-artifact-restore-proof.txt

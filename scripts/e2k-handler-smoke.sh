#!/usr/bin/env bash
# E2k QEMU-only direct-call fatal handler path; NOT live CPU exceptions.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export UNIOS_KERNEL_ELF="$ROOT/target/x86_64-unknown-none/release/unios_kernel"
test -s "$UNIOS_KERNEL_ELF"
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

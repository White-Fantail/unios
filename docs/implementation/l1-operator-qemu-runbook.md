# UniOS L1 — isolated QEMU operator test (E2r-D)

**Scope:** Ubuntu **24.04** in a disposable Linux VM, QEMU **8.2.2**, one x86_64 virtual CPU, OVMF 4M, Rust **1.90.0**. This is **not** a physical-machine boot guide, a USB writer, a production OS, or authorization for L2.

## Evidence and strict acceptance

The L1 code has already passed the E2r-C7 CI run [#40990906949](https://github.com/White-Fantail/unios/actions/runs/40990906949): an actual, isolated high-half ELF entry on a private CR3 with active GDT/IDT/TSS and QEMU-native exit **65**. It followed an independent fifth QMP CPU/physical BootContext check and ordered COM1 `E2R_C6_HIGH_HALF_ENTRY` then `HALTED_PASS`. The exact test image's SHA-256 and source SHA must be checked again when replayed. The normal E2q EFI and ELF must be restored afterward.

## How to reproduce in a disposable Ubuntu 24.04 VM

On the **VM**, as a normal (non-root) user with internet access to clone/install development dependencies:

```bash
sudo apt-get update
sudo apt-get install -y git python3 binutils file qemu-system-x86 ovmf gdisk dosfstools mtools coreutils build-essential
# Install rustup from https://rustup.rs if not already installed.
rustup toolchain install 1.90.0 --profile minimal \
  --component llvm-tools-preview \
  --target x86_64-unknown-none --target x86_64-unknown-uefi
git clone https://github.com/White-Fantail/unios.git
cd unios
git switch main
mkdir -p out
set -o pipefail
bash scripts/l1-operator-qemu.sh 2>&1 | tee out/l1-operator.log
```

Do **not** run on macOS directly without separate QEMU/OVMF validation. The host's other personal disks/USB devices must never be attached to the VM. The scripts create a local disposable GPT ESP and separate disposable OVMF variable image under `out/`, use `-nic none`, never write a real block device, and remove the temporary images on success.

## Expected evidence

- Console: `UNIOS_L1_OPERATOR_QEMU_ACCEPTANCE_PASS exit=65` and `UNIOS_L1_OPERATOR_QEMU_ONLY_COMPLETE`.
- `out/e2r-d-tested-source-sha.txt`: exact checked-out main commit.
- `out/e2r-c7-kernel.elf`, `out/e2r-c7-loader.efi`: the **C7-only** tested images, retained distinctly from normal release artifacts.
- `out/e2r-c7-image-digests.txt`: SHA-256 for both images. Independently check with `sha256sum -c out/e2r-c7-image-digests.txt`.
- `out/e2r-c7-clean-boot-proof.json`: ordered COM1 + exact exit65 oracle.
- `out/e2r-c7-high-cpu-proof.json`, `out/e2r-c7-new-root-proof.json`: independently paused QMP CPU/register/physical RAM and 27-target PML4 witnesses.
- `out/e2r-c7-com1-serial.log`: kernel entry then `HALTED_PASS` in that order.
- `out/e2r-c7-host-exit.txt`: precisely `65`.

Any different exit, missing marker, unexpected reset, failed SHA, absent QMP witness, or nonzero script status is a **FAIL**. Do not interpret a printed marker alone as success. Keep `out/l1-operator.log` and these evidence files for diagnosis. Do not silently retry after a FAIL as if the first run succeeded.

## Boundaries

This tests the approved **L1 Experimental Kernel** in pinned QEMU/OVMF only. No disk filesystem, application compatibility, drivers, L2 services or general hardware support is claimed. Running UniOS on physical PCs or writing boot USB media remains **NO-GO**, subject to a future separate authorization.

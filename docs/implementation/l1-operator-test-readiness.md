# UniOS L1 — operator test-stage admission policy

**Status as of 2026-10-29:** **NOT YET A USER KERNEL BOOT TEST**.

The existing `scripts/build-l1.sh` and `scripts/qemu-preflight.sh` on the
**experiment** branch are reproducible *firmware preflight diagnostics*,
not an OS installation or user-operable kernel. Exit codes 67, 69, 71, 73
and 75 intentionally mark only lab preflight phases, not success.

## Explicit threshold to announce "TEST STAGE — QEMU"

The assistant must first demonstrate and cite **an actual CI run**
satisfying all:
- Known SHA-256, signed-off review scope, pinned compiler/OVMF/QEMU.
- ELF64 ET_EXEC / UEFI PE+ checks; bounded image copy, zeroed BSS and map
  ownership, verified bridge address provenance.
- Actual successful ExitBootServices with no post-cut firmware calls,
  new CR3 enforced (CR0.WP + EFER.NXE), kernel SysV entry on protected
  high-half stack, GDT/IDT/TSS/#DF IST and absence of guards.
- Independent host-confirmed QEMU success exit **65** with correct
  ordered COM1 stage trace and no reset/shutdown/timeout/panic.
- At least the essential negative/fault controls and regression suite
  for the exact test image; no premature blanket E5 signoff.
- A bootable, separately versioned QEMU artifact with checksums and a
  documented disposable test harness that cannot touch personal disks.

Only THEN explicitly tell the user **"가상환경 테스트 단계 진입"** and supply
tailored copy/paste commands, expected console/QEMU exit, SHA256 verification,
failure capture and safe cleanup. Provide host instructions for the user's
platform: primarily Linux/QEMU or a disposable Linux VM; adapt for macOS
only after its tooling/image compatibility has been verified.

**Physical-machine testing is a separate, later approval:** unvalidated
ACPI, hardware memory maps, firmware differences, DMA/IOMMU and serial
diagnostics make real bare-metal boot outside the approved current LAB-H1
profile. Do not instruct loading on personal PCs or writing bootable
USB/media until explicitly expanded and tested.

## Operator-facing expected future flow (not yet executable)

1. Obtain exact release/test candidate and SHA-256, plus disposable QEMU
   firmware and disk artifacts.
2. Verify all hashes, isolate network and all real user disks, use a
   separate OVMF VARS copy, no user media.
3. Run finite QEMU with explicit machine/CPU/memory/serial and debug-exit;
   capture COM1, QMP reset events and host exit.
4. Require the **entire** stage-trace oracle, not just a string or exit
   status, and inspect negative-path expected failures separately.
5. Save a structured report and replay any failure against identical
   fixture hashes. Never regard resets, hangs, printed PASS alone or
   ordinary UEFI preflight as a successful kernel boot.

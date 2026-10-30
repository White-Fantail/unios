# UniOS L1 — operator QEMU test admission and limits

**2026-10-30 status:** The approved L1 Experimental Kernel's **E2r-C7
isolated QEMU clean-boot oracle is verified**, including real high-half
kernel execution and exact QEMU host exit **65**. The separate E2r-D
operator wrapper and exact-main CI remain the final reproduction gate.

## Verified on disposable Ubuntu 24.04 GitHub Actions

- [C6 high-half CPU entry CI #40990455349](https://github.com/White-Fantail/unios/actions/runs/40990455349): ExitBootServices, private CR3, high-half RIP/RSP, kernel ELF/BSS/BootContext, independent fifth QMP witness, diagnostic exit 133.
- [C7 clean boot CI #40990906949](https://github.com/White-Fantail/unios/actions/runs/40990906949): all of the above plus fifth host authorization, ordered COM1 kernel-entry and HALTED_PASS markers, exact exit **65**, 27 private mappings, descriptor/GDT/IDT/TSS/IST witnesses, fault and previous-stage regressions.
- Separate feature-built EFI and ELF have SHA-256 digests in `out/e2r-c7-image-digests.txt`, preserved inside the CI artifact with the structured host proofs. The **default safe EFI and normal kernel** are restored after each test.

## Explicit QEMU operator test gate

An operator may only declare **가상환경 테스트 단계 진입** when:

1. The entire approved L1 CI suite succeeds for the **exact main source SHA**.
2. The tested EFI/ELF build is reproducible with pinned Rust 1.90.0, QEMU 8.2.2 and OVMF 4M in Ubuntu 24.04.
3. Independently verified ExitBootServices, CR3, WP/NXE, high-half RIP/RSP, GDT/IDT/TSS/IST, stack guards, BootContext and actual loaded RX/BSS provenance remain intact.
4. The guest emits ordered COM1 stage markers, an independently paused QMP fifth proof grants K, and native host QEMU exit is **65** without panic/reset/timeout.
5. Negative controls and previous E2q/E2r QEMU regressions pass; no C7 opt-in EFI replaces the safe default.
6. The separate tested EFI/ELF, digest file, structured QMP/COM1 proof, and the **documented disposable Ubuntu-only harness** exist.

The local operator must replay from a clean `main`, retaining the exact
source SHA and confirming both binary SHA-256 digests and the clean exit.
See [L1 operator QEMU runbook](l1-operator-qemu-runbook.md).

## In scope versus not admitted

- **In scope:** a throwaway, **non-root Ubuntu 24.04 Linux VM** running
  pinned QEMU 8.2.2/OVMF 4M with a disposable GPT ESP, disconnected
  guest network, no personal disks/USB passthrough, and saved evidence.
- **Not admitted:** native macOS QEMU without separate compatibility
  verification, any physical PC/USB boot, production use, other firmware,
  multi-vCPU/hardware, or L2 functionality.
- **No general OS claim:** the L1 result is a kernel entry and minimal
  fail-stop boot oracle, not a desktop system, filesystem or driver stack.

Never take a guest-written success marker or debug-exit alone as a boot.
A missing fifth QMP witness, altered binary, firmware difference, or
any unexpected CPU fault is a rejection, even if COM1 printed PASS.

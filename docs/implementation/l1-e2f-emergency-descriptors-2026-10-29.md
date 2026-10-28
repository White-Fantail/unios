# UniOS LAB-H1 E2f — dormant GDT/IDT/TSS and double-fault IST

**Date:** 2026-10-29 NZDT.
**Branch:** `experiment/l1-e2f-emergency-descriptors`.
**Last tested code:** `16f18a926924261df19ec9c94043334489da21ea`.
**Actual Actions / QEMU:** https://github.com/White-Fantail/unios/actions/runs/40777489144

## Implemented, but not installed in the CPU

- Exactly one old-CR3-checked, UEFI-allocated 4KiB page is reused as
  dormant GDT/TSS/IDT storage. The compact layout is 40-byte GDT at
  offset 0, 104-byte x86_64 TSS at offset 0x100, and a 32-entry IDT
  (512 bytes) at offset 0x200; all unused bytes are zeroed.
- GDT contains ring0 64-bit code, ring0 data, and available 16-byte
  x86_64 TSS system descriptor. The kernel CS selector is 0x08,
  data selector 0x10, and TSS selector 0x18.
- TSS RSP0 points at the upper end of the four-page main kernel stack.
  IST1 points at the upper end of the independent four-page double-fault
  stack. Each stack's **separate 4KiB unmapped virtual guard** remains
  absent in the new PML4; stack tips were verified against exact physical
  page translations and supervisor RW/NX rights.
- For vectors 0..31, the dormant IDT has present ring0 interrupt gates.
  Vector 8 is the unique IST1 gate; all other gates use the normal
  stack. The emergency handler symbols are ELF/UEFI-resident code
  that is audited under the *current firmware* CR3 and mapped RX
  in the new, not-yet-loaded CR3. All vectors terminate rather than
  `iretq` if the stubs are ever invoked in a disposable VM.
- The two EFI handlers write **non-PASS** debug-exit bytes 0x31
  and 0x32, respectively; no attempt to invoke them exists in E2f.
  The handler byte ranges, full exception entry ABI and live descriptor
  activation are **not yet independently verified**.
- No `lgdt`, `lidt`, `ltr`, `mov cr3`, ExitBootServices, kernel entry
  or hardware/VM negative-fault injection was performed.
- The distinct E2f success *preflight-only* byte 0x27 corresponds to
  observed QEMU host exit **79**. Full success exit **65** is never emitted.

## Verified evidence

| Check | Observed |
|---|---|
| Host outcome oracle negative controls | **15/15** |
| ELF parser | **9/9** |
| Boot memory map/reservation | **11/11** |
| ELF staging/BSS | **8/8** |
| PML4 private page builder | **13/13** |
| CPU profile gate | **9/9** |
| EFI bounded code span verifier | **16/16** |
| New GDT/IDT/TSS serialization unit tests | **14/14** |
| Total host/unit tests | **95/95** |
| Independent Python host descriptor layout audit | **PASS** |
| Independent corrupted #DF IST negative control | **PASS** |
| Independent prior host PTE fixture audit | **PASS** |
| Actual QEMU/OVMF EFI preflight | **PASS, host exit 79** |
| Kernel CR3 switch / ExitBS / installed IDT / #DF runtime | **NOT RUN** |

Independent descriptor audit examines a **synthetic host-generated image**
only; it does not claim to read back QEMU guest descriptors. The actual
QEMU E2f pass separately confirms the preflight path built and checked
the structures, without activating them.

## Priority blockers before E3 / user QEMU boot test

1. P0: The *first* firmware CR3 root physical-as-linear table access
   remains uncorroborated by an independent host-side check.
2. E2e assembly canary is not a complete executable CR3 transition
   trampoline; its exact bridge RIP/stack/control flow remains unproven.
3. Emergency exception handler machine bytes, error-code conventions,
   IST stack faults and fail-stop terminal signals need protected-mode
   dynamic tests **after** reviewed descriptor installation.
4. BootContext content, final GetMemoryMap and ExitBootServices bounded
   retry, post-cut firmware-call ban and complete high-half SysV kernel
   entry have not been exercised.
5. Genuine full boot needs ordered COM1 stages and independently
   confirmed host success exit **65**, with reset/timeout/fault negative
   controls and the accepted LAB-H1 L1 traceability gates.

**Status:** E2f pre-ExitBootServices preflight only; E2 overall partial,
E3–E5 not independently validated. User QEMU kernel TEST STAGE = NO-GO.
Physical PC test = NO-GO. No L2 or higher functions.

# LAB-H1 E2r — high-half entry handoff implementation contract

**Status:** IMPLEMENTATION GATE / NOT YET EXECUTED  
**Baseline:** E2q verified QEMU private CR3 terminal path (host exit 121); L1-only.  
**Scope:** single-vCPU disposable x86_64 QEMU/OVMF; no physical host or L2 work.

## Goal and anti-goal

Move from the proven low EFI-code post-CR3 terminal bridge toward the approved freestanding high-half kernel entry **without conflating** a CR3 marker or debug-exit with a boot. Until all independently observed conditions below pass, keep the operator/kernel boot gate **NO-GO** and do not claim exit 65.

## Required pre-cut evidence

1. Pin the exact linked ELF64 ET_EXEC entry VA, loaded RX text, RO rodata, RW data/BSS, high-half kernel stack, two unmapped stack guards, kernel-owned GDT/IDT/TSS and IST1 pages to unique physical allocations and explicit PTE permissions. Require canonical addresses, bounded arithmetic, no overlaps, 4KiB alignment as applicable and correct zero-initialized BSS.
2. Verify each target against **both** the staged ELF segment provenance and the final QMP physical PTE bytes. Neither symbolic ELF addresses nor a self-reported manifest is sufficient proof.
3. Audit the actual linked transition machine code, including all page-crossing bytes: the exact post-CR3 execution range must remain mapped supervisor RX; writable aliases of executable pages and user-accessible table entries must fail.
4. Pin the concrete calling convention at both ends: UEFI x64 vs Rust/assembly SysV64, required 16-byte stack alignment, direction flag clear, interrupts disabled, no red-zone dependency and no firmware call after ExitBootServices.
5. Enumerate descriptor activation preconditions. A dormant, staged GDT/IDT/TSS/IST is **not** an installed GDT/IDT/TSS/IST. Assert selectors, limits, TSS descriptor presence, IST1 stack bounds and no unmapped descriptor fetch.

## Irreversible sequence

1. Start from the existing E2q fail-closed loader and J/O QMP host authorization barriers. Preserve the default safe preflight artifact.
2. Perform bounded final GetMemoryMap/key handling and real ExitBootServices, with no post-success boot services use.
3. Switch to the independently audited bridge RSP, disable interrupts, clear DF, and install the proven private CR3.
4. In a separately feature-gated **disposable QEMU-only** path, establish the approved kernel-owned descriptors/IST and high-half main RSP **only after** their physical and page-table provenance has been independently checked. Reject any incomplete transition rather than continuing with firmware descriptors.
5. Jump/call to the exact linked high-half ELF entry under its documented SysV64 ABI. The kernel must produce ordered COM1 stage markers with distinct terminal success/fail-stop codes.
6. Verify using independently paused QMP CPU registers and physical-table walks: new CR3, high-half RIP and high-half RSP each within the exact expected bounds; verify selected GDT/IDT/TSS/IST active CPU state, not merely allocated metadata.
7. Declare QEMU kernel entry success only on complete ordering/provenance, no reset/timeout/unexpected exception, and exact host exit **65**.

## Negative controls (must reject before any GO)

- Wrong EFI-vs-SysV argument registers or misaligned RSP.
- An unmapped entry byte, split-code page, wrong NX/W/U bit, unexpected huge page or physical alias.
- Missing high-half stack guard, corrupted IDTR/GDTR or TSS/IST target.
- Premature COM1 success marker, correct marker with wrong host exit, correct exit without kernel RIP witness.
- Host QMP observation absent/ambiguous, stale authorization, timeout/reset or unexpected interrupt/fault.
- Default preflight byte image changed by an opt-in fatal experiment.

## Exit acceptance matrix

| Observation | Meaning |
|---|---|
| E2q 121 with third QMP witness | **Already verified** low-bridge real CR3 only |
| E2r descriptor/high-stack witness, high RIP but no terminal kernel oracle | Partial handoff; **NO-GO** |
| Exit 65 plus complete ordered COM1 trace, independent live QMP witness, negative/regression suite | Candidate **QEMU-only** L1 kernel-boot acceptance |
| Physical PC boot | Separate future authorization; currently **NO-GO** |

## Development sequence

- **E2r-A:** add independently testable ELF entry / descriptor / stack / table provenance audit and mutation tests. Do not execute a new irreversible handoff yet.
- **E2r-B:** add bounded, explicitly feature-gated post-CR3 transition assembly and offline linked-binary opcode validation.
- **E2r-C:** add real disposable QEMU checkpoint(s), COM1 ordered stage oracle, success/fault exit controls, and regression runs.
- **E2r-D:** verify exact source SHA, CI artifacts and checksums; only then revise operator readiness, if the complete exit-65 contract passed.

**This document is a control specification, not execution evidence.** E2q is the last verified runtime boundary. No high-half kernel execution, QEMU boot acceptance or physical hardware admission is claimed here.

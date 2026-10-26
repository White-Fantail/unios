# UniOS — L1 Experimental Kernel Scoped User-Go Record v0.1

> **Decision date:** 2026-10-26 NZDT.
> **Decision:** **L1 SCOPED USER GO RECORDED — IMPLEMENTATION NOT STARTED.**
> **User authorization wording:** `L1 최소 커널 구현 승인`.
> **Applies to:** only the experimental L1 scope in [L1 Go/No-Go Assessment v0.1](unios-l1-implementation-go-no-go-assessment-v0.1.md). **Does not approve L2/L3, a product, hardware support, or stable ABI.**
> **Execution mode as of this record:** repository remains **design/research-only**; recording this decision is documentation work. Do not represent this record as permission to override the current project-level no-implementation working constraint. No source, build script, test harness, CI, executable, branch or kernel artifact was created by this decision record.

## 1. Decision and boundaries

The user explicitly accepted the **narrow** L1 experimental kernel proposal on 2026-10-26. The earlier review's `READY FOR EXPLICIT USER GO` has now advanced to `USER GO RECORDED` **for L1 only**. Earlier readiness, contract and assessment files remain dated snapshots, not current authorization registers; this file is the subsequent decision record.

The approved *future experimental implementation envelope*, not a claim of present implementation:

1. **Disposable VM:** QEMU Q35 / OVMF / x86_64 **one vCPU**, constrained firmware + CPU feature profile, COM1 ISA serial; no physical PC or writable real-user media.
2. **Split boot image:** Rust-first UEFI PE32+ loader (`x86_64-unknown-uefi`) embedding a **bounded ELF64 ET_EXEC** freestanding kernel (`x86_64-unknown-none`); inspect exact linker headers, dependency licenses and artifact provenance before running.
3. **Handoff:** bounded staging, checked UEFI memory descriptors and MapKey, restricted ExitBootServices retry, immutable resident BootContext and **zero firmware Boot Services calls after successful exit**.
4. **Supervisor mappings:** four-level paging, verified old/new CR3 shared-linear transition bridge + dedicated bridge/main/IST stacks, W^X with no writable text aliases, guarded stack, CR0.WP / EFER.NXE and privileged kernel entry.
5. **CPU/fault/diagnostics:** resident IDT/GDT/TSS/IST, IF=0 and no SMP, bounded polled COM1 records, deterministic final `HALTED_PASS` only after all defined success conditions; faulty/reset/timeout paths are failure outcomes.
6. **Lab verification:** QEMU-only positive boot and bounded negative tests, evidence log recording source/revision, compiler/linker/QEMU/OVMF provenance, PE/ELF inspection, CPU profile, mapping audits, exception results and unresolved failures.

The numerical caps and machine-state obligations remain those of the [L1 fixed-link/CR3 contract](../contracts/unios-l1-fixed-link-elf-cr3-bridge-entry-state-closure-v0.1.md), subject to fail-and-re-review if actual toolchains or firmware contradict them. This approval **does not waive any contract stop condition**.

## 2. Explicit exclusions

No native user Task, capability/Handle, IPC SEND/RECEIVE, Pager, PMM, heap, scheduler, SMP, real driver, DMA, network stack, filesystem write, verified boot/TPM, physical hardware, GUI, SDK, stable syscall ABI or Linux/Windows/Darwin binary compatibility. No untrusted app execution, security assurance, production claim or general-OS release. L2's C01–C09 and higher-level RFC open blockers remain open.

## 3. Controlled future implementation gates (not performed)

| Gate | Required evidence after implementation is separately enabled in the project working mode | Failure handling |
|---|---|---|
| **E0 Source isolation** | approved experiment branch, README experimental marking, pinned toolchain/QEMU/OVMF sources and hashes, license review, no secrets/disks | stop without changing protected main |
| **E1 Artifact audit** | genuine UEFI PE32+ and bounded ET_EXEC `PT_LOAD`/entry/segment permission audit, no relocations or W+X | reject binary, revise design before scope change |
| **E2 Handoff** | descriptor stride/MapKey and exit-retry trace; no late EFI use | fail-stop; inspect ownership/state |
| **E3 CR3 and protections** | old/new bridge linear→physical identity, high-half mapped entry, guard and W^X faults | fail-stop; do not assume identity map |
| **E4 Exception and serial** | correct IDT/TSS/IST handling; bounded negative serial and externally detected reset/timeout | classify as FAIL; never false PASS |
| **E5 Evidence review** | positive boot and negative-case report with actual outputs; explicit Go/No-Go before L2 | if any critical invariant fails, return to design |

**Do not predeclare any E0–E5 gate as passed.** No measurable performance or proof is possible from this authorization record.

## 4. Stop-and-return-to-design conditions

- Real compiler output requires unsupported relocations, linker model or ABI changes.
- OVMF old-linear/physical page provenance cannot be authenticated and carried through the CR3 transition.
- Allocations exceed laboratory memory bounds or need a general allocator/unbounded firmware operations.
- Entry/exception path requires forbidden firmware interaction, user mode, writable executable aliases or unsafe implicit mappings.
- A negative test yields false PASS, persistent hang without host failure recognition, or any unwanted media write.
- Scope drifts into L2, drivers, physical hardware or compatibility functionality.

## 5. Status register

| Decision / artifact | Status on 2026-10-26 |
|---|---|
| High-level D1–D8 | **Accepted — Direction** (unchanged) |
| L1 paper feasibility / first experimental slice | **READY** per dated L1 assessment |
| **L1 user scoped implementation approval** | **RECEIVED** |
| **Actual source/branch/build/CI creation** | **NOT STARTED** |
| **QEMU boot / faults / executable verification** | **NOT PERFORMED** |
| L2 native capability microkernel | **NO-GO / NOT AUTHORIZED** |
| L3 developer preview / general OS | **NO-GO / NOT AUTHORIZED** |

**Next design-stage action:** freeze an L1 execution checklist and traceability matrix linking E0–E5 to the contract tests; keep all source, build and runtime activity out of the repository until the project moves out of its currently design-only operating mode. This is **not another request for user L1 approval**.

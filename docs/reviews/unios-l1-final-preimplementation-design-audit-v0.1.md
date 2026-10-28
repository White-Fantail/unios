# UniOS — L1 Final Pre-Implementation Design Audit v0.1

> **Audit date:** 2026-10-29 NZDT.
> **Scope:** [User-approved L1 laboratory boundary](unios-l1-scoped-user-go-2026-10-26.md), not L2 / general-purpose OS.
> **Disposition:** **CONDITIONAL GO FOR A BOUNDED L1 FIRST-CODE EXPERIMENT, NOT A RUNTIME/SECURITY GO.**
> **Execution in this audit:** design review + repository documents ONLY. **0 compiled kernel images, 0 QEMU runs, 0 executed negative tests, 0 measured performance figures.** The current UniOS project operating mode remains research/design-only; a user's earlier scoped approval is recorded, but this audit **does not initiate implementation**.
> **Historical precedence:** this dated audit adds clarifying gates to, but does not rewrite, [LAB-H1 fixed-link/CR3 contract](../contracts/unios-l1-fixed-link-elf-cr3-bridge-entry-state-closure-v0.1.md), [E0–E5 runbook](../contracts/unios-l1-execution-readiness-runbook-v0.1.md) or [60-case matrix](unios-l1-verification-traceability-matrix-v0.1.md). No L2 RFC or ABI is accepted by this review.

## 1. Executive verdict and limits

**The L1 experiment is sufficiently scoped to attempt implementation in a future implementation-enabled phase.** The two-language-target EFI→freestanding ELF separation, privileged supervisor-only scope, finite ExitBootServices retry, guarded four-level mappings and deliberately test-only QEMU environment are coherent as **design hypotheses**. The audit found **no reason to expand L1 into Tasks, IPC, Pager or drivers** before initial boot.

**However, there is no unconditional engineering GO:** three critical pre-first-code/first-artifact assumptions require explicit evidence and stop-on-failure handling, and several additional integration controls must be recorded. A paper test plan cannot be represented as a successful run.

**P0 must-not-bypass boundary:**
1. Exact **ET_EXEC/non-PIE build output** is not guaranteed by selecting the Rust freestanding target; the actual toolchain and linker result must be inspected.
2. The **firmware-old-CR3 → new-CR3 bridge** must survive the final firmware/ExitBootServices path and preserve known linear→physical mapping and exception/stack invariants.
3. The **host success oracle** must distinguish a deliberately completed kernel from a CPU merely hung in \`hlt\`, an unexpected shutdown, a reset/triple fault and a serial marker printed prematurely.

These are **bounded experimental validation gates**, not reasons to require the full OS to be designed before the first boot attempt.

## 2. Adversarial finding register

| ID | Severity | Finding, evidence basis, and failure risk | Required disposition |
|---|---|---|---|
| **FA-01** | **P0** | The current Rust x86_64-unknown-none target uses the kernel code model and a freestanding SysV/no-red-zone ABI, **but the target definition permits/defaults to position-independent executables**. The contract expects **ELF ET_EXEC with no dynamic relocations**, not an automatic result of the target triple. | E0 must pin the precise codegen/linker flags and review the linker script (candidate static relocation / non-PIE, not pre-validated). E1 must inspect real e_type, program headers, relocation records, linked VAs and entry disassembly. If ET_DYN or runtime-required relocations appear, **STOP**, do not silently teach the loader a new format. |
| **FA-02** | **P0** | A **pre-ExitBS** audit of firmware's old-linear→physical bridge is insufficient by itself to establish stability **after** firmware callbacks and a stale-MapKey retry. The source does not yet exist, and it is not proven that an ordinary UEFI pointer directly yields old-current-page-table backing. | E3 must identify the exact old-page-table walk/provenance method and safe frame access, prove old/new mappings for bridge RIP, stack, descriptors and post-switch instructions, and recheck current CR3 / bridge assumptions on the actual final post-ExitBS cut **without banned firmware calls**. On unprovable map or mutation, **STOP and redesign**. |
| **FA-03** | **P0** | \`HALTED_PASS\` + an x86 \`hlt\` with IF=0 is **not by itself an independent host proof of terminal success**. QMP \`query-status=running\` cannot establish a guest CPU executed the intended final HLT, nor can absence of reset prove progress. | Before first runtime E0 must freeze a QEMU-only terminal evidence path. **Recommended lab candidate:** a separately configured ISA \`isa-debug-exit\` synthetic exit port used only by the final verified path, with a distinct success code, correlated prior serial marker and QMP RESET/SHUTDOWN/STOP events; fatal/negative paths never write success. QEMU's debug-exit write encodes a requested status rather than generic normal POSIX zero—verify decoded status against the pinned QEMU version. Alternative: independently inspect guest RIP/CPU halted state through a pinned debugger harness and corroborate external state. Do not treat serial-only success as clean. This is a **test fixture**, not a native device/production ABI. |
| **FA-04** | P1 / cut-dependent | Before the first ExitBS attempt, UEFI owns device access; direct polled COM1 may collide with OVMF console use. After **first** ExitBS attempt the selected conservative L1 rule forbids console/protocol/allocator calls, even if firmware only partially shut down. | Freeze a clear phase-specific diagnostic policy: optional UEFI console only before first attempt and only while legal; direct COM1 only with documented ownership/quiescence; after first attempt do not assume any firmware console. The host must classify silent failures without requiring COM1. No infinite TX wait and no false PASS. |
| **FA-05** | P1 / safety | \`GetMemoryMap\` descriptor size differs from a Rust struct's natural array stride. \`ExitBootServices\` can invalidate the current key; initial failed exit can partially stop services; Rust drop/deallocation or logging may inadvertently use boot services on failure. | Audit raw descriptor stride/version, held capacity, all last-map allocations, all allocations/drop/console/error paths and retry path at the **source and disassembly** level. After first attempt only existing-buffer GetMemoryMap and ExitBootServices; at most 2 attempts; after success no Boot Services, protocols or return. Rejected attempt terminates via host without illegal EFI fallback. |
| **FA-06** | P1 / reproducibility | The planned "read-only boot media" language does not alone fix an OVMF variable-store strategy: OVMF variants may use writable pflash \`VARS\`, and Secure Boot state influences whether DevUnsigned EFI will load. | E0 manifest explicitly chooses OVMF CODE, disposable VM-only copy of VARS (if applicable), Secure Boot **off for DevUnsigned**, CPUID, RAM, vCPU=1, machine and COM1, no writable real-user disk/network/secrets. Record SHA/provenance of CODE and initial VARS template; disposable VARS are not user-data disks and must not be silently mounted persistently. |
| **FA-07** | P1 / CPU safety | \`IF=0\` masks **maskable** interrupts but not NMI or architecturally unavoidable machine exceptions. The early IDT, TSS, #DF IST and critical transition may fault before normal logging. | E3/E4 must document NMI/#MC policy for the **restricted emulator profile** (either safely provisioned emergency paths or explicitly classified unsupported/host FAIL). Never claim that IF=0 makes the CR3 transition universally interrupt-safe. Audit #PF/#GP/#UD/#BP/#DF error-code frames and stack failure; do not share one IST for nested fatal handlers without review. |
| **FA-08** | P1 / evidence honesty | There are **60 referenced paper test IDs**, but H-X/L1-X/LC-X cases often exercise the same underlying boundary; \`L1-X14\` is DevSigned-only and out-of-profile. Pure mocks are not identical to real OVMF integration evidence. | Preserve 60 unique case IDs and separate **59 in-scope rows + 1 DEFERRED/out-of-profile row** before any execution. Each future row requires a declared method (real artifact, source audit, controlled simulation, host injection, QEMU VM) and honest evidentiary strength. Never claim 60 independent vulnerabilities or 60 executed tests. Include adversarial negative controls for the host terminal oracle and the old CR3 recheck. |

**No finding above is recorded as empirically reproduced on UniOS software**; these are design, toolchain specification and host-oracle risks. P0 means **blocking clean-boot validation or requiring pre-run artifact closure**, not a blanket claim that writing the first experimental source is impossible.

## 3. Final E0–E5 release/stop policy

| Gate | May implementation proceed to the next stage in a future authorized mode when… | Immediate STOP / review |
|---|---|---|
| **E0** provenance | Exact Rust/LLVM linker, OVMF CODE/VARS template, QEMU, VM machine/CPU/COM1, DevUnsigned policy, no real writable media and **terminal host oracle chosen/pinned** | unreviewed toolchain, persistent user-data media, unknown secureboot/device state, host oracle not defined |
| **E1** artifact | **Actual** PE32+ UEFI/EFIAPI and ELF64 ET_EXEC non-PIE fixed-image constraints, segment/relocation/ownership/stack bounds pass artifact audit | ET_DYN, linker gap, unbounded ELF load, W+X alias, missing build provenance |
| **E2** firmware exit | MapKey/DescriptorSize correct, last-map ownership stable, first-attempt and retry paths legal; no firmware call after success | banned EFI operations after attempt, stale map misclassified, no safe fail-stop |
| **E3** CR3 | real old/new VA→PA bridge and **post-ExitBS** mapping/CR3 state are inspected; high-half entry ABI, descriptors, guards and WP/NXE are valid | unprovable mapping provenance, invalid NMI/fatal path assumption, relocated/writable text |
| **E4** observability | positive guest trace + independent host terminal signal agree; faults/logging/reset/timeout all rejected as clean | success on mere marker, unexpected VM reset, silent logger counted success, infinite wait |
| **E5** review | 59 in-scope cases all have required per-row evidence and approved expected result, deferred L1-X14 explicitly documented, no critical blocker remains | any missing/misclassified required row, wrong scope, no reproducible manifest |

**Before source:** at minimum E0's terminal oracle design and VM profile must be frozen and FA-01's link strategy must be explicitly configured on paper. **After first source:** E1–E4 are tested as source and machines become available. The P0 FA-02 requirement intentionally cannot be evidenced by documentation alone and therefore must not be mislabeled paper-satisfied. **Full L1 evidence review occurs only at E5**, after 59 applicable case results or explicit disposition of any legitimately non-executable method.

## 4. Updated compact verification acceptance conditions

1. **Positive:** verified restricted machine + correct UEFI PE32+ loader + ET_EXEC payload + bounded map + ExitBS (one attempt or legal retry) + exact bridge/CR3/RSP/GDT/IDT/TSS/IST + correct SysV entry + fixed map protection + fully correlated guest stages and **independent host terminal completion**, with no reset/timeout.
2. **Malformed inputs:** reject **before** unowned memory write, branch or protected cut; no retries that need forbidden firmware services.
3. **Fault probes:** expected #PF/#GP/#UD/#BP/#DF, missing COM1 and corrupted bridge cases may terminate with missing serial, host reset or timeout **as specified for that case**, but must never emit a successful clean outcome.
4. **Host negative controls:** (a) print \`HALTED_PASS\` prematurely then deliberately reset, (b) emit \`HALTED_PASS\` then hang, (c) exit without serial marker, (d) print success-looking text from a negative fixture. All must fail clean-boot classification.
5. **Scope:** a Rust kernel entry does not prove a capability microkernel, native ABI, driver DMA isolation, verified boot, a desktop, real hardware or Windows/Linux/Darwin compatibility.

## 5. Findings requiring precise first-code evidence (LC-O crosswalk)

| Older open point | Linked finding | Needed first future evidence |
|---|---|---|
| LC-O01 ELF/compiler | FA-01 | A01 tool flags + A02 real ELF e_type, PT_LOAD, relocations, disassembly |
| LC-O02 old-CR3 bridge | FA-02 | A04 old/new mapping and post-ExitBS pre-switch audit |
| LC-O03 memory caps | FA-01/05 | A03 exact allocated frames and table/stack totals |
| LC-O04 IST/CPU exception | FA-07 | A04 selectors/IST + A05 injected fault result |
| LC-O05 pinned build/firmware | FA-06 | A01 CODE/VARS, CPU, machine, compiler and QEMU manifest |
| LC-O06 host oracles | FA-03 | A05 QMP/guest correlation and negative-host classifier tests |
| LC-O07 mapping/ExitBS negative controls | FA-02/05 | A03–A05 audited actual negative evidence |
| LC-O08 no L2 expansion | FA-08 | A06 bounded L1 disposition; independent L2 decision |

## 6. Sources and evidence quality

**Repository evidence:** [Architecture v0.2](../architecture/unios-architecture-v0.2.md), [L1 user-Go](unios-l1-scoped-user-go-2026-10-26.md), [L1 implementation gate review](unios-l1-implementation-go-no-go-assessment-v0.1.md), [fixed-link closure](../contracts/unios-l1-fixed-link-elf-cr3-bridge-entry-state-closure-v0.1.md), [execution runbook](../contracts/unios-l1-execution-readiness-runbook-v0.1.md), [60-case traceability](unios-l1-verification-traceability-matrix-v0.1.md).

**External authoritative design sources to revalidate against exact pinned versions:**
- [UEFI 2.11 Boot Services, GetMemoryMap and ExitBootServices](https://uefi.org/specs/UEFI/2.11/07_Services_Boot_Services.html) (variable descriptor stride, latest MapKey, partial first exit, no Boot Services after successful exit);
- [Rust x86_64-unknown-uefi target](https://doc.rust-lang.org/rustc/platform-support/unknown-uefi.html) (PE32+ and \`efiapi\` conventions);
- [Rust x86_64-unknown-none target](https://doc.rust-lang.org/rustc/platform-support/x86_64-unknown-none.html) (freestanding ELF, kernel code model, SysV entry, no red zone);
- [Rust target definition x86_64_unknown_none.rs](https://github.com/rust-lang/rust/blob/main/compiler/rustc_target/src/spec/targets/x86_64_unknown_none.rs) and [rustc relocation-model options](https://doc.rust-lang.org/rustc/codegen-options/) (default PIE capability and explicit code generation/link behavior must be checked in actual artifact);
- [QEMU QMP reference](https://www.qemu.org/docs/master/interop/qemu-qmp-ref.html) (RESET, SHUTDOWN, STOP and query-status; neither lack of reset nor \`running\` proves HLT);
- [QEMU isa-debug-exit source](https://github.com/qemu/qemu/blob/master/hw/misc/debugexit.c) (VM-only terminal signal design candidate; verify actual behavior after pinning QEMU version).

**No L1 source was created, compiled or run.** No physical mapping or correctness guarantee is inferred from these external sources. Any third-party source / toolchain release can change.

## 7. Final decision register

| Question | Answer as of 2026-10-29 |
|---|---|
| Was specific L1 user approval recorded? | **YES**, 2026-10-26 |
| Is the whole UniOS ready to implement or ship? | **NO** |
| Is L1 sufficiently narrowed to attempt a controlled first-code experiment in an implementation-enabled future mode? | **YES, CONDITIONAL GO** with E0–E5 and FA-01–FA-08 stop conditions |
| Are FA-01/FA-02/FA-03 demonstrated satisfied? | **NO**, require future pinned artifact, VM and host evidence |
| Is any Rust/UEFI loader or kernel actually implemented? | **NO** |
| Are 60 planned cases executed? | **NO; 0/60**, one out of current scope |
| Is the project operating mode now implementation-enabled? | **NO, design/research-only** |
| Is L2 user Go or stable native ABI granted? | **NO** |

**Recommendation:** close the pre-implementation paper review with this **conditional** result. When implementation mode is enabled, begin E0 with toolchain/provenance and a verifiable host completion protocol; run E1 artifact audit **before** VM boot, then E2/E3/E4 on a disposable QEMU profile. Any violation returns to design; **never** silently broaden the Linux/Windows/Darwin, Pager, DMA or product scope.

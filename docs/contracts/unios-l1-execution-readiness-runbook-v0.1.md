# UniOS — L1 Experimental Kernel Execution Readiness Runbook v0.1

> **Prepared:** 2026-10-29 NZDT. **Status: DESIGN BASELINE / IMPLEMENTATION NOT STARTED.**
> **Authority:** [Scoped L1 user-Go, 2026-10-26](../reviews/unios-l1-scoped-user-go-2026-10-26.md). This records an authorization decision, **not** a tested implementation. The project's current operating constraint remains design/research only; this runbook is documentation and creates no source, toolchain, CI, emulator, test or executable artifact.
> **Primary contract:** [Fixed-Link ELF / CR3 Bridge / Entry State v0.1](unios-l1-fixed-link-elf-cr3-bridge-entry-state-closure-v0.1.md). **Source test-index:** [L1 verification traceability matrix](../reviews/unios-l1-verification-traceability-matrix-v0.1.md).
> **Scope:** disposable QEMU Q35 / OVMF / x86_64 **one-vCPU** boot and privileged diagnostics only. **No L2 Tasks, Capabilities, IPC, Pager, drivers, filesystem writes, compatibility, physical PC or security-product claims.**

## 0. Decision hierarchy and exact meaning of "ready"

- Architecture v0.2's D1–D8 are **accepted directions**; detailed kernel RFCs remain proposals. No product API or ABI becomes stable by completing L1.
- 2026-10-26 L1 Go approves the **bounded experiment in principle**. It does not mean software exists or boots. This runbook converts the paper approval into **ordered future engineering gates**.
- Current mode: **documentation/design only**. Future implementation activity requires the project to actually move into an implementation-enabled working mode. No additional *L1 scope approval* is requested here; an expanded L2 scope would require a fresh user decision.
- Evidence level vocabulary: **PAPER** (specified), **BUILT** (real binary inspected), **OBSERVED** (VM trace), **VALIDATED** (recorded pass of all mandatory gates). Today: **PAPER only**.
- Every required gate defaults to **NOT RUN / UNSATISFIED**. "GO" is permission for a bounded attempt, not an assurance verdict.

## 1. Strict LAB-H1 parameters — fixed for first evaluation

| Domain | Initial reviewable value / contract | Reject if |
|---|---|---|
| Platform | x86_64 QEMU Q35, OVMF, one vCPU, 4-level paging, LA57=0, PCIDE=0, NX available, known ISA COM1 device, disposable read-only boot media | mode/device/CPU/firmware differ without design review |
| Loader | single PE32+ UEFI image built for \`x86_64-unknown-uefi\`; Rust UEFI interfaces use the **\`efiapi\`** ABI where required; UEFI entry call must not be confused with kernel entry | UEFI call convention or firmware-lifecycle assumption wrong |
| Kernel | embedded ELF64 little-endian \`EM_X86_64/ET_EXEC\`, built separately for \`x86_64-unknown-none\`, non-PIE, fixed high-half mapping, **no required runtime relocation** | PT_DYNAMIC, dynamic dependency, unsupported relocation, bad ELF subset |
| Link VA | base \`0xffffffff80000000\`, maximum envelope \`[0xffffffff80000000, 0xffffffff84000000)\`, aligned | actual artifact outside declared envelope |
| ELF load | <=8 PT_LOAD, <=16 MiB staged load span, bounds/overlaps/permissions fully checked; entry within RX mapping | bad filesz/memsz, overflow, W+X, duplicate/conflicting page, noncanonical entry |
| Pages | 4 KiB mappings, PML4, CR0.WP=1, EFER.NXE=1, high-half text RX, rodata R/NX, data/BSS RW/NX | writable text alias, unbounded direct map, guard mapped |
| Precharged tables and memory | tentative <=512 KiB page tables; <=64 KiB map buffer; all image, tables, stacks, context, descriptors allocated **before final map** | limits exceeded: stop and **review**, not silently enlarge |
| Stacks | 8 KiB dedicated bridge stack; 16 KiB main stack + unmapped 4 KiB guard; 16 KiB #DF IST stack + unmapped 4 KiB guard | overlapping/missing guard, unmapped bridge, stale firmware stack |
| Transition | authenticated old-linear->physical mapping of bridge code/stack/critical descriptors; **same linear addresses** mapped in new CR3 | assumption based only on identity mapping; unauditable old CR3 |
| CPU | single bootstrap processor, interrupts masked IF=0, resident GDT/IDT/TSS/IST, explicit exception stubs, high-half Rust \`extern "C"\` entry with **SysV, no red zone** | EFI ABI leaks into kernel call, invalid selectors, missing IST or wrong error-code frame |
| Handoff | latest GetMemoryMap stride/version/MapKey, <=2 ExitBootServices attempts; after *first attempt* only existing-buffer GetMemoryMap + ExitBootServices retry; **after success no firmware calls** | late allocator/console/file/service use; map unexpectedly grows beyond buffer |
| Diagnostics | bounded polled serial records, static/fixed panic output, no raw memory dump or heap; HALTED_PASS only after all success predicates, otherwise halt/externally detected failure | infinite polling, false PASS, panic recursive call, unsafe persistence |

**Limits are experimental hypotheses**, not measured resource requirements, device guarantees or stable interfaces. Never resolve a failed hypothesis by silently broadening scope.

## 2. Proposed ownership and execution order (no implementation yet)

| Cut | Before -> after | Ownership/CPU invariant | Evidence expected in future |
|---|---|---|---|
| T0 | OVMF loads EFI application | UEFI Boot Services active; only loader-allocated pages may be touched | firmware build/hash and entry signature |
| T1 | validate embedded kernel and preallocate | segments owned; ELF validated; BSS zeroed; stacks/tables/descriptors/BootContext reserved | artifact manifest, allocation ledger |
| T2 | construct mappings and audit old bridge | exact old VA->PA provenance for bridge RIP/RSP, tables, descriptors; new CR3 maps required same-linear pages + high-half kernel, no RX writable alias | old/new PTE dump or machine-readable audit |
| T3 | final map/ExitBootServices | no allocations or protocol activity after final GetMemoryMap; retry constrained after first ExitBS; successful ExitBS irrevocably transfers control | ordered events, map stride/version/key, ExitBS attempts |
| T4 | bridge/CR3/stack/descriptor cut | IF=0; bridge stack valid both roots; RIP executable across CR3; emergency GDT/IDT/TSS/IST valid; transition to high-half stack then SysV kernel | entry-state audit, disassembly and captured trace |
| T5 | guarded diagnostic kernel | kernel RX/R/NX/RW/NX; guard stays absent; no scheduler, Task, Pager, DMA or UEFI call; bounded fault/COM1 behavior | positive/fault probe classifications and serial output |
| T6 | terminal observation | exactly one terminal marker for clean success; stop with IF=0; host must independently reject resets, triple faults, timeouts and wrong-phase PASS | host/guest correlated result record |

**Key safety cuts:** T3 must precede T4 without post-exit firmware dependence; T2 must prove a usable old->new CR3 bridge **before** ExitBS; T4 must preserve exception-stack safety even if the first Rust call faults.

## 3. E0–E5 ordered future engineering gate checklist

Each item is a **not-yet-executed acceptance condition**. A gate may be entered only when previous gate's required evidence exists. All STOP conditions return to design review before any broader change.

### E0 — Reproducible, isolated lab governance

- [ ] Confirm current project operating policy permits actual code; limit source to isolated experimental branch, keep main documentation/protected until review.
- [ ] Record exact Rust \`rustc\`, Cargo, linker/LLVM tools, architecture target definitions and flags, QEMU and OVMF revisions, firmware hashes and provenance, host OS, 1-vCPU CPU model, RAM, machine options, COM1 configuration.
- [ ] Document licenses of compiler/firmware/dependencies and source; use only disposable read-only media or in-memory volatile artifacts, no personal disks/network/secrets.
- [ ] Define immutable run manifest: commit, compiler/linker/QEMU/OVMF hashes, embedded payload SHA, VM profile, fault-injection ID, start/finish time, expected oracle. SHA is *provenance*, **not authenticated boot**.
- [ ] Pre-register a finite host timeout and reset/triple-fault detection rule, without treating firmware watchdog as host oracle.
- **Exit:** reviewable isolated profile + provenance + test manifest. **STOP:** unknown firmware provenance, writable real media, hidden host-dependence, any L2 drift.

### E1 — UEFI/ELF artifact and memory-construction proof

- [ ] Inspect **actual** EFI PE32+ subsystem and EFI ABI disassembly; separately inspect ELF64 ET_EXEC e_entry/PHDRs and absence of dynamic relocations, PT_INTERP/PT_DYNAMIC/TLS.
- [ ] Validate <=8 PT_LOAD, bounds/arithmetic, page-aligned rights, W^X/no writable aliases, high-half linked VA, 16 MiB staged cap, BSS zeroing; reject malformed embedded payload before touching unowned memory.
- [ ] Record the full ownership ledger: EFI image, segment frame PAs, map buffer, page-table root+pages, emergency descriptors, bridge/main/IST stacks, BootContext and COM1 scratch.
- [ ] Verify <=512 KiB preallocated table cap, 64 KiB map buffer and guard holes under actual section layout; reject if any is insufficient.
- **Exit:** real PE/ELF header audit + ELF negative-case proof + ownership/memory allocation evidence. **STOP:** compiler/code-model mismatch, unsupported PHDR, overrun, unsafe alias or needed allocation above approved caps.

### E2 — EFI map and one-way firmware handoff

- [ ] DescriptorSize-aware map traversal, descriptor version validation, overflow and physical-range checks; preallocated map buffer sized with headroom; never treat variable stride as Rust packed array.
- [ ] Confirm *all* required persistent pages allocated and validated before last GetMemoryMap; no changed key from accidental later allocations.
- [ ] Distinguish initial ExitBS success, first EFI_INVALID_PARAMETER followed by **one** legal map/exit retry, second failure or too-small retry buffer. **At most 2 ExitBS calls total**.
- [ ] After first ExitBS attempt (even if failed) allow only GetMemoryMap+ExitBootServices on already allocated buffers per stricter L1 subset. After success call **no Boot Services or handle-based protocols**, never return to UEFI.
- **Exit:** trace showing valid fresh key, committed ExitBS, no banned calls and conservative retry fault outcomes. **STOP:** reliance on EFI console/file/alloc after first attempt, map truncation or impossible retry semantics.

### E3 — CR3 and privileged mapping boundary

- [ ] Measure old CR3 bridge RIP/RSP/descriptors' actual VA->PA backing before exit; audit matching new CR3 PTEs and coherent RX/RW/NX rights at identical linear addresses.
- [ ] Verify LA57/PCIDE=0, 4-level CR3, WP/NXE, no W+X, high-half entry canonical+RX, guard pages unmapped and no alternate writable text alias or broad identity-map leakage.
- [ ] Maintain separate bridge and main stacks; after transition use final System V kernel entry **without** EFI ABI. Audit RIP/RSP and all descriptor/TSS/IST selectors, stack alignment and error-code conventions.
- [ ] Execute planned negative mapping tests; classify inaccessible bridge or triple fault as **FAIL**, never as an expected successful kernel boot.
- **Exit:** machine-state/PTE + bridge audit and exception-safe entry in VM. **STOP:** cannot authenticate old-linear mapping, unreviewed map/ABI assumptions, missing descriptor/stacks, runtime page-table overrun.

### E4 — Fault handling, diagnostics and correct oracle

- [ ] Confirm #PF (#PF error code/CR2), #GP, #UD, #BP, and #DF paths with correct architectural stub stack frames and separate #DF IST where appropriate.
- [ ] Verify guard read/write and RX-write fault probes; accept **controlled expected fault** only in negative-test mode with independently verifiable classification. No fault path may manufacture a clean PASS.
- [ ] COM1 finite per-byte polling; unavailable UART must terminate/fail via host timeout, not deadlock indefinitely. Panic no unwind/heap, no recursive formatter or raw memory dump.
- [ ] Clean test emits ordered stage markers and exactly one \`HALTED_PASS\` only after declared checks. Host independently verifies final halt/state/no reset within prescribed observation interval; a premature PASS followed by crash is **FAIL**.
- **Exit:** valid positive boot and all mandatory negative cases with host-correlated results. **STOP:** false PASS, uncaught reset, unmapped #DF dependency, infinite logger or unauthorized media writes.

### E5 — Evidence review and next bounded decision

- [ ] Publish review record with reproducible environment, actual PE/ELF/CPUID, ownership/maps, attempt count, event trace, all case IDs, expected vs observed, proof limitations and reproducibility instructions (not a release/security claim).
- [ ] Show PASS/FAIL/EXPECTED_FAULT/REJECTED/UNRUN **per case**; fail any test with missing evidence, case-ID mismatch or error incorrectly labeled PASS.
- [ ] If a mandatory invariant breaks, halt effort and open a design issue; never quietly amend LAB-H1 caps or bypass tests.
- [ ] Decide independently whether to iterate **L1 only**. **L2 Task/Capability/IPC/Pager requires separate design C01–C09 assessment and fresh user Go**.
- **Exit:** reviewed lab result for exactly the tested VM profile; no inference of physical PC, product security or application compatibility.

## 4. Mandatory host and guest outcome rules

| Classification | Conditions | May count as clean positive L1 PASS? |
|---|---|---|
| \`CLEAN_PASS\` | authorized positive case; required ordered stages; one final HALTED_PASS; host corroborates terminal non-return without subsequent reset/triple fault; exact profile and manifest match | **YES**, for this VM profile only |
| \`EXPECTED_REJECT\` | malformed input or unsupported VM profile rejected before unsafe transfer, identifiable per test expectation | **NO** (counts as passing an individual negative case only) |
| \`EXPECTED_FAULT\` | intentional test fault accurately identified and safely halted or externally detected, as specified for that case | **NO** (negative case can pass) |
| \`FAIL_STOP\` | controlled stop, bounded logger failed, or incomplete staged boot | **NO** |
| \`CRASH_RESET\` | VM reset/triple fault or other non-clean termination, including after HALTED_PASS | **NO** |
| \`TIMEOUT\` | external deadline, no reliable terminal state | **NO** |
| \`INVALID_RUN\` | wrong vCPU/machine/firmware, missing provenance/host trace, invalid test fixture or artifact | **NO** |

**Two independent notions:** a *negative test* can **pass its expected rejection/fault criterion**, but its guest may **never report the positive boot success**. No serial text alone establishes successful machine termination. No guarantee can be made that #DF or triple-fault paths print output; host reset/timeout detection remains mandatory.

## 5. Required evidence package template (future, unpopulated)

| Artifact ID | Minimum contents | Owner / status |
|---|---|---|
| \`A00\` | scoped decision + exact tested repository commit | L1 reviewer / **PAPER** |
| \`A01\` | toolchain + OVMF/QEMU versions, license and hash/provenance manifest | experiment owner / **NOT RUN** |
| \`A02\` | EFI PE / ELF PHDR and relocation audit; linker map | compiler/reviewer / **NOT RUN** |
| \`A03\` | firmware memory map stride/version/key, allocations and BootContext bounds | loader/reviewer / **NOT RUN** |
| \`A04\` | old/new CR3 mapping proof, alias/guard scan, GDT/IDT/TSS/IST/entry ABI inspection | kernel/reviewer / **NOT RUN** |
| \`A05\` | time-ordered guest serial stages + independent host VM reset/timeout/final-state trace | harness/reviewer / **NOT RUN** |
| \`A06\` | test results indexed by all case IDs; expected vs observed; blockers and review decision | reviewer / **NOT RUN** |

Every logged outcome must identify **(source commit, image digest, VM manifest, test ID, expected outcome, observed guest trace, observed host trace, result, reviewer)**. A missing required field fails audit; do not infer it from a neighboring test.

## 6. Blocking uncertainties and exact escalation

| Existing ID | Risk | Evidence needed before proceeding |
|---|---|---|
| LC-O01 | Rust emits unsupported fixed ELF/relocations | real header+relocation/disassembly audit |
| LC-O02 | no trustworthy old CR3 mapping evidence under chosen OVMF | measured old/new linear->physical translation; stop if unprovable |
| LC-O03 | 512 KiB tables / 64 MiB VA / other caps inadequate | real allocation+mapping ledger; re-review cap before change |
| LC-O04 | TSS/IST exception bridge fails on real CPU-state cut | fault traces and host classification, stop on recursive faults |
| LC-O05 | toolchain/firmware/CPU profile not reproducible | pinned manifest and hashes |
| LC-O06 | serial/reset/timeout falsely awards PASS | explicit host/guest independent oracle evidence |
| LC-O07 | negative controls do not actually challenge W^X or EFI boundary | exercised maps/aliases/ExitBS path |
| LC-O08 | L1 work implicitly expands into L2 | immediate scope halt and separate approval process |

## 7. Document precedence, source validity and exclusions

- **Primary design:** [fixed ELF/CR3/entry closure](unios-l1-fixed-link-elf-cr3-bridge-entry-state-closure-v0.1.md).
- **Earlier constraints and paper tests:** [UEFI handoff microcontract](unios-l1-uefi-handoff-kernel-entry-microcontract-v0.1.md), [minimal boot MVP](unios-minimal-boot-exception-memory-mvp-contract-v0.1.md).
- **Decision and historical status:** [scoped user-Go](../reviews/unios-l1-scoped-user-go-2026-10-26.md), [pre-Go assessment](../reviews/unios-l1-implementation-go-no-go-assessment-v0.1.md), [official architecture v0.2](../architecture/unios-architecture-v0.2.md).
- **External standards to re-check when pinning a toolchain:** [UEFI 2.11 source](https://github.com/UEFI/UEFI-Specification-Release/blob/main/source/07_Services_Boot_Services.rst), [Rust UEFI target](https://doc.rust-lang.org/rustc/platform-support/unknown-uefi.html), [Rust freestanding kernel target](https://doc.rust-lang.org/rustc/platform-support/x86_64-unknown-none.html), Intel SDM (paging, interrupts and double fault). UEFI 2.11 allows a wider Memory Allocation Services set after first ExitBS than this **intentionally stricter lab**. Never describe the narrower lab policy as a UEFI specification mandate.
- **No code, VM runs, executed tests, measured numbers, formal proofs, published ABI, physical hardware or user OS capability** are evidenced by this document. All checkboxes are deliberately unchecked.

**Next design artifact:** [verification traceability matrix](../reviews/unios-l1-verification-traceability-matrix-v0.1.md), enumerating the full L1-X/H-X/LC-X inventory with stage ownership, expectation and mandatory status.

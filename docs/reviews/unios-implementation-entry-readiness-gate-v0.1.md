# UniOS — Implementation Entry & Design-Completion Readiness Gate v0.1

> **Status:** Proposed, **NOT READY FOR IMPLEMENTATION APPROVAL**, 2026-10-26 NZDT.
> **Scope:** Define when UniOS research is sufficient to **ask the user for permission to start an explicitly bounded implementation**, instead of indefinitely extending design or misrepresenting documentation as proof. This document creates **no implementation approval**, code, executable model, test, project schedule, hardware claim, native ABI or accepted kernel ADR.
> **Architecture baseline:** [UniOS architecture v0.2](../architecture/unios-architecture-v0.2.md), whose **eight upper-level directions are Accepted**, while technical RFC-001…018 and CAPIPC models remain Proposed with open safety/feasibility blockers.
> **Immediate technical evidence:** [Paired Mapping/Publication and Invalidation Study](../models/capipc-paired-mapping-publication-invalidation-ordering-v0.1.md), [Destination Output Cross-RFC Table](capipc-destination-memory-publication-cross-rfc-decision-table-v0.1.md), [CAPIPC P0 Conflict & ADR Matrix](capipc-p0-conflict-resolution-adr-readiness-v0.1.md), [RFC-001 TCB](../rfcs/rfc-001-kernel-boundary-and-tcb.md), [RFC-002 capabilities](../rfcs/rfc-002-object-capability-lifetime-delegation-revocation.md), [RFC-003 IPC](../rfcs/rfc-003-ipc-wire-protocol-and-synchronization.md), [RFC-004 VM/Pager](../rfcs/rfc-004-virtual-memory-pager-cache-and-oom.md), [RFC-005 scheduler](../rfcs/rfc-005-scheduling-preemption-and-cpu-budget.md), [RFC-006 boot/M0](../rfcs/rfc-006-boot-trust-chain-service-startup-and-recovery.md), [RFC-009 KABI](../rfcs/rfc-009-native-abi-and-system-call-contract.md).
> **Governance:** The developer/user project constraint is **design and research only** until separately, explicitly authorized. A **READY** recommendation means "the user may reasonably authorize that scoped work", **not** an automatic tool instruction to implement.

## 0. Current verdict — research can continue, no implementation-start notice yet

**As of 2026-10-26: NO-GO for implementation entry under the currently unscoped UniOS build request.** Documentation is extensive and architecture direction is clear, but the following lack sufficiently narrowed reviewable first-slice contracts: boot-to-exception startup ownership, minimal memory/CPU safety invariants, first runnable syscall/Task surface and its negative test strategy, and any general CAPIPC native Handle MOVE+receive's disputed output/publication contract.

**Critical distinction:** A small experimental boot/diagnostic kernel does **NOT** need every GUI, Linux/Windows/Darwin compatibility, physical IOMMU, filesystem durability, advanced Pager or general pageable RECEIVE algorithm designed before its **first code**. Demanding implementation-level test results as a prerequisite to beginning an *experiment* is circular. Conversely shipping a user-facing secure OS **does** need later adversarial source/hardware evidence.

**Recommended policy:** ask for explicit **R6-scoped experimental entry approval** once its finite paper checklist below is ready. Then implement **only** the approved kernel slice, with gates to expand scope. This is an optional refinement to architecture v0.2's R6 approval process, **not** an amendment or implicit waiver of v0.2.

## 1. Three levels of readiness — do not conflate

| Level | What a truthful recommendation means | Needed before it | NOT implied |
|---|---|---|---|
| **L0 Design Study** | alternatives and risks recorded, review ongoing | documented hypotheses/proposed RFCs | safe implementation feasible |
| **L1 Scopable Boot/Diagnostic Prototype Entry** | user may approve limited UEFI/QEMU experimental code | B01–B10 paper checklist complete, explicit R6-scoped Go, negative plan | usable secure microkernel, native app ABI, real PC driver support |
| **L2 Native Microkernel MVP Entry** | user may approve scoped Task/Cap/IPC/VM features | L1 design approved, plus C01–C09 scoped semantic contract, clear fail-closed paths and test design | formal proof already run, GUI/compatibility support, product guarantee |
| **L3 Developer Preview/Reference PC Product Readiness** | experimental system has actually been implemented and survives required tests | source/refinement tests, SMP/hardware/Pager/resource/error recovery, security audits and stated hardware profiles | arbitrary PC/macOS/Windows/Linux interoperability |

**No universal "UniOS design complete" is required before L1.** The product has incremental boundaries and later research. Announce **exact level and limits** to the user; never say "ready to implement the whole OS" when only L1 is ready.

## 2. L1 initial code-entry checklist (paper evidence only) B01…B10

| ID | Required design/approval artifact | Current evidence / missing item | Today |
|---|---|---|---|
| **B01** | x86_64 QEMU Q35/OVMF Tier 0A, Rust-first, independent kernel and supported boot environment selected for *this slice* | high-level direction D2/D4 accepted, explicit emulator profile in architecture | **PARTIAL** |
| **B02** | precise UEFI loader→kernel trust/ownership, ExitBootServices memory map and failure/status disposition scoped to prototype | RFC-006 Proposed boot chain; exact stage entry/error contract not frozen for prototype | **OPEN** |
| **B03** | early virtual/physical memory map, guarded kernel stack, image placement, page-table ownership, zeroing and forbidden writable aliases for this slice | RFC-001/004 Proposed; exact minimal no-Pager memory contract and invalidation exclusions not yet selected | **OPEN** |
| **B04** | x86_64 exceptions/IDT, timer/IRQ mode for first boot, panic path, what runs before/sans scheduler | RFC-001/005 Proposed; no approved minimal exception/interrupt profile | **OPEN** |
| **B05** | minimal trusted object universe and **explicitly excluded** Task/IPC/Grant features until L2 | RFC-001/002 conceptual; no frozen bounded L1 object/resource scope | **OPEN** |
| **B06** | approved diagnostic output/debug channel and logs/panic dump redaction/fail-stop without persisting secrets | README/RFC-006 general guidance; exact Tier0A console policy TBD | **OPEN** |
| **B07** | toolchain/build/boot source-layout, target/triple/loader boundary and reproducibility/versioning *plan* (not implementation) | Rust-first accepted, exact experimental build layout and dependency audit TBD | **OPEN** |
| **B08** | negative verification **plan**: malformed boot info, memory-map churn, stack overflow, invalid pointer/exception, panic and watchdog/serial observation; explicit test environment | test intent across RFCs, not a signed-off slice-specific pass/fail matrix | **OPEN** |
| **B09** | repo/license/sandbox policy: code only on user-approved branch, no destructive real disks or secrets, clear experimental status and no public security claim | architecture permissive core direction accepted, user implementation authorization absent | **PARTIAL** |
| **B10** | one-page **L1 MVP scope + exclusions + residual risks + milestone + explicit user Go** under v0.2 R6 (or separately approved scoped R6) | not authored/approved; no Go permission | **OPEN** |

**Entry threshold:** Each B item must become documented **READY FOR REVIEW** with named document link and owner; any intentional exception is explicit and narrow. Actual testing of first code belongs **after** user approval, not before. Before implementation, user receives an unmistakable **"L1 design ready; implementation may be authorized"** summary and chooses whether to approve.

## 3. L2 native microkernel entry checklist C01…C09 (follows L1 gate)

| ID | Required semantics before implementing that native subsystem | Today |
|---|---|---|
| **C01** | TaskGeneration/AddressSpace/Handle Grant/lineage and scoped root/sponsor quotas | **OPEN — RFC-002 Proposed** |
| **C02** | actual whole native SEND K-S + relevant B/revoke and E1 owner E-M linearization, fail-before-commit semantics | **OPEN — CO-S/CO-E-C candidates** |
| **C03** | qS/qR preknown protected receipts with bounded no-replay/GC and lost-result truth | **OPEN — KR-M1/2 fork** |
| **C04** | **first *restricted* control IPC** envelope and register ReturnRelease under issuer/Endpoint/Task death; explicit all-N cap result rules or zero-cap first profile | **OPEN — U-R paper candidate** |
| **C05** | if VA-based receives are in L2, paired (M,P) PublishStart, M-DENY/M-DRAIN and P-DENY/P-DRAIN, safe COW/backing/fault/alias rules | **OPEN — cannot infer from pins or postcopy retry** |
| **C06** | whole RECEIVE K-I, queued revoked mandatory Grant and exposure-aware terminal/cap/TokenID cleanup | **OPEN — HPA-B+ Proposed** |
| **C07** | Pager/VM/IPC/runtime wait-for graph and independent M0 CPU/IRQ/frame/IPC/cleanup credits; bounded fail-safe terminal | **OPEN — RFC-004/005/006 Proposed** |
| **C08** | explicit negative-race oracle/proof strategy; small reachable success cases + intentionally unsafe altered guards; time/fault/Task death | **OPEN — paper tests only** |
| **C09** | syscall/ABI experimental versioning, supported restricted scope and separately explicit user scope authorization | **OPEN — RFC-009 Proposed** |

**Scope reductions are allowed only if truthful:** A first zero-cap short control IPC profile could defer **general cap-vector RECEIVE copyout and advanced pageable usercopy** while maintaining explicit no-cap/size limits; it cannot claim unrestricted RFC-003 CAPIPC E1 behavior. Advanced service integrity and general-purpose capability sharing need C02/C03/C06 before exposure to untrusted workloads.

## 4. What need not block an L1 experiment (but blocks later claims)

- Linux ELF guest binary, Wine Windows PE and Darwin Mach-O compatibility can remain research objectives, not L1 gates.
- GUI, sound, NVMe, arbitrary USB, Wi-Fi and hardware driver isolation aren't needed for a console-only Tier 0A prototype. When physical DeviceCaps/DMA become real, RFC-007 actual IOMMU group and fence evidence is mandatory.
- Secure Boot production rollout, TUF updater and storage power-loss durability do not need to be *implemented* before experimental UEFI boot; the initial image provenance and no-user-data assumption must remain explicit.
- Product performance p99 and formal refinement results require a working system; they cannot be valid prerequisites to first code. **A high-assurance release** may require them, and the design must leave testable seams from the start.
- Full public Native ABI v1 cannot be declared merely because an experimental internal syscall is selected.

## 5. Decision status and honest next milestone

| Area | Current gate | Immediate document needed | Why |
|---|---|---|---|
| **Architecture v0.2 direction** | accepted upper-level only | preserve baseline | independent Rust-first x86_64 capability OS and initial VM profile fixed as direction |
| **L1 initial boot entry** | **NO-GO, B01/B09 partial; rest OPEN** | **Minimal Boot/Exception/Memory MVP Contract v0.1** covering B02–B08/B10 | explicit bounded first code target and test-plan required |
| **L2 small control IPC entry** | **NO-GO** | **CAPIPC Restricted Register ReturnRelease and Minimal Control IPC v0.1**, then receipt/Task cut resolution | avoid requiring broad pageable output just to bring up initial services |
| **L2 general pageable CAPIPC** | **NO-GO** | paired VM (M,P) implementation-feasibility evidence, full K-I/TokenID/M0 semantics | owner/VM invalidation and issuer cutoff still race |
| **L3 product/security readiness** | **NO-GO** | future actual running kernel tests, adversarial proof/refinement and selected Reference PC | cannot prove OS deployed before implementation |

**Practical recommendation:** stop treating every unresolved advanced CAPIPC race as an automatic blocker to writing a *strictly limited* boot kernel. Finish the **L1 minimal boot design** and a small **U-R IPC** research track, then make a specific **R6-scoped Go/No-Go proposal**. Never begin implementing before the user explicitly approves this proposal.

## 6. Notification contract for future UniOS design reviews

At the end of each significant review, record the following in the conversation and, when material, in this file:

- **L1 boot prototype:** NOT READY / READY FOR USER GO / USER GO GIVEN (only after explicit approval).
- **L2 microkernel:** NOT READY / READY FOR USER GO / USER GO GIVEN (with exact included IPC/VM).
- **L3 developer OS:** NOT READY / TESTED FOR NAMED PROFILE / PRODUCT REVIEW (only with actual evidence).
- **Blocking criteria:** cite B/C IDs, don't repeat long generic lists.
- **Next minimal resolving paper:** one focused scope, not endless broad RFC expansion.

**Notify explicitly at the first point any defined implementation level becomes READY FOR USER GO**; name the proposed scope, exclusions, known security caveats and what authorization is needed. This is a **response-time review policy**, not a promise of autonomous work or a scheduled background monitoring agent. Do not auto-start code because a document flips to READY.

## 7. External evidence and known limitations

- [seL4 manual, message registers versus IPC buffer](https://sel4.com/Info/Docs/seL4-manual-latest.pdf) and [seL4 FAQ, short messages/shared buffers](https://sel4.systems/About/FAQ.html) illustrate a credible restricted control-message profile; not UniOS runtime evidence.
- [UEFI Specifications](https://uefi.org/specifications), [Rust bare-metal x86_64 target](https://doc.rust-lang.org/rustc/platform-support/x86_64-unknown-none.html) inform future boot/toolchain design, not a usable UniOS boot loader.
- [Linux page pins](https://docs.kernel.org/core-api/pin_user_pages.html) and [MMU notifier/COW](https://docs.kernel.org/next/mm/mmu_notifier.html) warn against assuming arbitrary pageable buffer copyout is already solved.
- All current GitHub UniOS RFCs remain **Proposed** at detailed level. The readiness status is the evaluator's paper judgment based on those documents, not a tested claim or estimate of completion date.

---

**History:** 2026-10-26 v0.1 Design-stage entry gate. Distinguishes L1 bounded boot code from L2 native Task/Capability/IPC and L3 Developer Preview; 10 proposed boot-entry criteria and 9 native-MVP criteria, no Go approvals, no source implementation.

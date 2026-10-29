# UniOS

> **L1 Experimental Kernel — 2026-10-29:** E2j's EFI reservation
> boundary old-CR3 path witness is merged in `main`. E2k adds
> **11-byte exact linked code closure for both emergency error
> handlers**, independently audits real PE32+ executable bytes,
> and confirms three real disposable QEMU/OVMF outcomes:
> preflight **89**, generic-handler direct call **99**, and
> DF-handler direct call **101**. **180/180 host/Rust tests pass**.
> Direct calls are NOT CPU exception delivery or #DF IST proof.
> **No GDT/IDT/TSS installation, ExitBootServices, CR3 switch,
> or real high-half kernel boot has occurred**. True kernel
> clean success exit **65** is unverified. See
> [E2k evidence](docs/implementation/l1-e2k-emergency-handler-closure-2026-10-29.md)
> and [operator testing gate](docs/implementation/l1-operator-test-readiness.md).

**UniOS** implements its independently approved **L1 Experimental Kernel**
as a staged preflight and boot-handoff research project. It is **not
yet a successfully booted or released OS**. L2 and higher work
requires a separate approval.

## Vision

Design and evaluate an operating system with its **own kernel, native ABI, system services, and desktop experience**—not a Linux distribution or a fork of an existing operating system.

Interoperability with applications originally built for **Linux, Windows, and macOS/Darwin** is a staged research goal, **not an implemented or guaranteed feature**.

## Architecture direction (accepted at design level)

Eight high-level project decisions were accepted on 2026-10-24:

| Area | Direction |
|---|---|
| Product | General-purpose desktop ambition; developer-first delivery |
| Initial hardware | x86_64 UEFI; QEMU/OVMF/VirtIO first, with early validation on a selected physical PC |
| Application compatibility | Native SDK first, POSIX source ports and selected Linux CLI binaries next; investigate Wine/Windows in parallel |
| Language policy | Rust-first hybrid; carefully bounded Assembly, unsafe Rust, and C/C++ interoperability |
| Kernel | Practical capability microkernel, with narrowly reviewed hybrid exceptions if supported by evidence |
| Licensing | Permissive core; conditional, compliant copyleft use in user space |
| Security | Design for eventual general-user threats from the start; verify and deliver protections in stages |
| Darwin/macOS | Prioritize limited Mach-O/Darwin CLI research; GUI/AppKit/Metal are long-term, high-risk research topics |

These are **accepted design directions**, **not validated implementations**. Kernel interfaces, driver support, compatibility behavior, delivery milestones, and performance/security claims require further specifications and evidence.

## Research priorities

- **Kernel and TCB:** objects and capabilities, threads/scheduling, virtual memory, IPC, privilege boundaries, recovery.
- **Native developer platform:** language-neutral C ABI, SDK, libc/POSIX source ports, Rust runtime/toolchain feasibility.
- **Hardware:** reference virtual machines and early reference-PC boot/driver/DMA/IOMMU validation.
- **Compatibility:** Linux syscall-entry semantics and user-space personality, Wine port dependencies, Darwin/Mach-O feasibility.
- **Desktop:** basic terminal, files, input, software 2D compositor, and measured interactive responsiveness.
- **Trust and release:** sandboxing, signed boot/updates, user data protection, licensing, and supply-chain review.

## Current status

**Architecture design baseline v0.2 plus explicitly experimental L1 E0/E1 source.**
The user approved a *scoped* L1 experiment on 2026-10-26 and directed implementation
on 2026-10-29. The approved architecture, RFCs and L1 technical contracts
remain the design baseline; their dated paper-only reviews are **historical
snapshots**, not evidence of a later successful run.

The experimental code includes a freestanding high-half ELF and UEFI
preflight loader, but the firmware handoff deliberately returns
`EFI_UNSUPPORTED`; **there is no completed UniOS kernel boot**.
The 2026-10-29 E1 QEMU/OVMF preflight observed host exit **67**.
The full L1 terminal oracle exit **65** has not been exercised.
E0/E1 are **partial**, E2/E3/E4/E5 are **not implemented/validated**.
No L2 features, product security claim, stable ABI or desktop application
compatibility are authorized or delivered.

The source can be integrated for traceability and incremental review
without declaring any L1 acceptance gate passed. Later work must
still use the experimental branch, controlled builds and actual QEMU
proof before claiming complete L1 validation.

For reproducibility and exact evidence limits, see the
[L1 implementation log](docs/implementation/l1-bootstrap-e0-e1.md),
[2026-10-29 E1 evidence](docs/implementation/l1-e1-evidence-2026-10-29.md),
[L1 execution runbook](docs/contracts/unios-l1-execution-readiness-runbook-v0.1.md)
and [60-case traceability matrix](docs/reviews/unios-l1-verification-traceability-matrix-v0.1.md)
(59 DevUnsigned cases pending, one DevSigned-only case deferred).

## L1 experimental verification boundary

- **Observed:** Rust 1.90.0 cross-target binaries and ELF/PE audits;
  9 ELF parser and 9 simulated host-oracle unit tests; QEMU/OVMF
  loader-only E1 preflight ending with exit 67.
- **Not yet observed:** bounded EFI frame allocation and segment
  staging; real ExitBootServices; old/new CR3 bridge; exception/guard
  probes; post-handoff kernel execution; independent full-success
  host evidence; per-case E5 negative coverage.
- **Acceptance policy:** do not infer full L1 boot, memory safety,
  security assurance or a complete E0–E5 gate from E1-only evidence.
  Keep the approved L1 stop conditions in force. No L2 implementation.

## Final L1 paper audit (historical design snapshot, 2026-10-29)

The [L1 final pre-implementation design audit](docs/reviews/unios-l1-final-preimplementation-design-audit-v0.1.md)
recorded **conditional GO for the first-code experiment**, not an
engineering assurance decision. Its three P0 concerns were fixed ELF
ET_EXEC output, post-ExitBootServices CR3 bridge proof, and independent
host completion. The new E1 evidence addresses the artifact and preflight
**partly**, but does not close the CR3/ExitBS or full host-completion gates.

## Architecture documents

- **[L1 Scoped User Go — 2026-10-26 (Recorded, Not Implemented)](docs/reviews/unios-l1-scoped-user-go-2026-10-26.md)** — explicit user approval of the limited QEMU/OVMF single-CPU experimental boot/diagnostic kernel; execution still postponed under design-only working mode; source/build/test evidence absent and L2/L3 not approved.
- **[Architecture & Research Design v0.2 — Official Draft](docs/architecture/unios-architecture-v0.2.md)** — current design baseline; eight accepted directions, system boundaries, unresolved risks, normalized ADR index, RFC research registry, security and validation gates, and research roadmap.
- **[RFC-001 Kernel Boundary & Trusted Computing Base v0.2 — Proposed Specification](docs/rfcs/rfc-001-kernel-boundary-and-tcb.md)** — independently reviewed kernel/user-space responsibility split, asset-specific TCB, Linux guest execution alternatives, DMA/IOMMU group boundaries, and verification gates; not yet accepted or implemented.
- **[RFC-002 Object, Capability, Lifetime, Delegation & Revocation v0.2 — Proposed Specification](docs/rfcs/rfc-002-object-capability-lifetime-delegation-revocation.md)** — critically reviewed capability model covering grant lineage anchors, IPC commit/result ambiguity, scope attenuation, resource accounting, and separate family-quiescence vs object-reclamation guarantees; not accepted or implemented.
- **[RFC-003 IPC Wire Protocol & Synchronization v0.2 — Proposed Specification](docs/rfcs/rfc-003-ipc-wire-protocol-and-synchronization.md)** — reviewed E1 exclusive Endpoint and CAPIPC v0.2 receive addendum, receive disclosure/Claim rules, scoped ReplyToken, one-shot vs persistent WaitSet, transactional MOVE, bounded flow control, recovery and security review; not accepted or implemented.
- **[RFC-004 Virtual Memory, Pager, Cache & OOM Contract v0.2 — Proposed Specification](docs/rfcs/rfc-004-virtual-memory-pager-cache-and-oom.md)** — re-reviewed kernel fault-waiter vs Pager page-supply authority, staged M0–M3 feasibility, mapping/COW/revoke lifetimes, immutable backing, dirty data-loss limits, wired boot/OOM recovery and CPU/DMA fences; not accepted or implemented.
- **[RFC-005 Scheduling, Preemption & CPU Budget v0.2 — Proposed Specification](docs/rfcs/rfc-005-scheduling-preemption-and-cpu-budget.md)** — adversarially re-reviewed fair weight vs overcommittable CPU caps vs admitted minimum reserve, SMP CPU-time conservation, kernel/IRQ charging, M0 recovery dependency and timer/lock proof gates, async IPC server-budget inversion, deferred donation/RT and workload verification; not accepted or implemented.
- **[RFC-006 Boot Trust Chain, Service Startup & Recovery v0.2 — Proposed Specification](docs/rfcs/rfc-006-boot-trust-chain-service-startup-and-recovery.md)** — deeply reviewed EFI ExitBootServices/watchdog boundaries, trusted bootstrap policy parser, scoped M0 recovery fail-stop, bounded service restart and DMA quarantine, persistent reboot budgets, floor-safe A/B rescue and cross-RFC proof gates; not accepted or implemented.
- **[RFC-007 User-space Drivers, DMA & IOMMU v0.2 — Proposed Specification](docs/rfcs/rfc-007-user-space-drivers-dma-and-iommu.md)** — adversarially re-reviewed IOMMU ownership vs reset-coupling and firmware RMRR/IVRS exceptions, protected MMIO, physically evidenced DMA drain/invalidation R2-F, IRQ/P2P scope, uncontained no-IOMMU fail-stop, driver-restart/quarantine and physical hardware validation gates; not accepted or implemented.
- **[RFC-008 Storage, VFS, Filesystems & Crash Durability v0.2 — Proposed Specification](docs/rfcs/rfc-008-storage-filesystems-and-crash-durability.md)** — adversarially re-reviewed independent volume/FS algorithm choices, WAL checkpoint/reader pin and safe reset, file/parent directory durability, FS/SQLite sync-mode differences, delayed writeback error lifetime, torn writes vs PSOW, conditional Flush/FUA receipts, A/B schema rollback and power-cut proof gates; not accepted or implemented.
- **[RFC-009 Native ABI, System Call & User-Service Contract v0.2 — Proposed Specification](docs/rfcs/rfc-009-native-abi-and-system-call-contract.md)** — adversarially re-reviewed minimal KABI, versioned SABI and C-compatible SDK; G1 guest-versus-native broker separate AddressSpaces, Fuchsia/UniOS MOVE-on-failure distinction, protected ReceiveEscrow, public handle ABA, postcommit copyout UNKNOWN, Linux futex/rseq/clone feasibility, strict SABI resource evolution and explicit no-stable-ABI proof gates; not accepted or implemented.
- **[RFC-010 Linux Personality & Binary Compatibility v0.2 — Proposed Specification](docs/rfcs/rfc-010-linux-personality-and-binary-compatibility.md)** — adversarially re-reviewed independent Linux x86_64 GuestDomain and protected broker, ELF/libc and futex/clone/rseq gates, execve irreversible failures and FD-table unshare, close/dup2/O_CLOEXEC atomicity, epoll (FD,OFD) registration and lingering events, SA_RESTART/partial transfers, openat2 race-safe path flags, getrandom entropy and cross-RFC proof gaps; not accepted or implemented.
- **[RFC-017 Verified Boot, Updates & Rollback Protection v0.2 — Proposed Specification](docs/rfcs/rfc-017-verified-boot-updates-and-rollback-protection.md)** — adversarially reviewed UEFI variable persistence vs access/monotonicity, increment-only TPM NV counter versus multi-domain security floor, sequential dual-threshold TUF Root rotation and trusted-client-state rollback, owner/vendor/unlocked boot provenance, A/B Good and physical rescue/key/schema compatibility, offline freshness and multi-store crash proof gates; not accepted or implemented.
- **[RFC-018 Versioned IDL, ABI Evolution & Compatibility Contracts v0.2 — Proposed Specification](docs/rfcs/rfc-018-versioned-idl-abi-evolution-and-compatibility-contracts.md)** — adversarially re-reviewed typed out-of-band Capability transfer, committed MOVE versus decode rejection, delegated Endpoint identity/confused-deputy protection, endpoint/service/policy-epoch-bound negotiation, strict/flexible unknown value vs resource semantics, Protobuf binary-to-JSON field loss, exact signed BootPolicy parser equivalence, scoped effect/durability/UNKNOWN handling and cross-RFC P0 research gates; not accepted, implemented or stable ABI.
- **[Cross-RFC Architecture Consistency Audit v0.1 — Open Findings](docs/reviews/cross-rfc-architecture-consistency-audit-v0.1.md)** — joint design-only review of RFC-002/003/007/008/009/010/017/018 and boundary RFC-001/004/005/006: one historically confirmed, subsequently editorially corrected E1 Endpoint DERIVE rights-table inconsistency, three ambiguous boundaries, thirteen joint proof gaps, custody/effect/fence/boot-status map, twelve unexecuted concurrency scenarios and proposed ADR/validation gates. All underlying RFCs remain Proposed and unimplemented.
- **[CAPIPC Publication Permit & Destination Memory Contract v0.1 — Proposed Design Research](docs/models/capipc-publication-permit-destination-memory-contract-v0.1.md)** — separates physical frame lifetime, actual Task VA/MappingGeneration correctness and issuer publication authority, comparing bounded P1-MB mapping leases, P1-MA typed backing delivery, P0-R register ReturnRelease and P2 mailbox. 20 design transitions, 28 paper scenarios, 26 unproved invariants and 16 open blockers; no implementation, checker or ABI freeze.
- **[UniOS L1 Implementation Go/No-Go Assessment v0.1 — Historical Pre-Approval Assessment](docs/reviews/unios-l1-implementation-go-no-go-assessment-v0.1.md)** — latest scoped implementation-entry ruling: experimental QEMU Q35/OVMF, one vCPU UEFI→freestanding ELF kernel, fixed protected CR3 + exception/COM1 diagnostics **was ready for an explicit user Go** at review time; the later 2026-10-26 approval is separately recorded; no boot, test or security proof yet. L2 native Task/IPC and L3 product remain NOT READY.
- **[UniOS L1 Fixed-Link ELF, CR3 Bridge & Entry-State Closure v0.1 — Proposed Lab Design](docs/contracts/unios-l1-fixed-link-elf-cr3-bridge-entry-state-closure-v0.1.md)** — fixes disposable-lab high-half ELF ET_EXEC scope, Rust UEFI→System V ABI, dual-linear mapped trampoline/stack and x86_64 IDT/TSS/IST, 4-level NX/WP kernel map, bounded ExitBootServices retry and fail-closed run assumptions. 22 unexecuted paper faults, 20 unproved invariants, 8 post-Go validation issues; no code, ABI or boot evidence.
- **[UniOS L1 UEFI Handoff & Kernel Entry Microcontract v0.1 — Proposed](docs/contracts/unios-l1-uefi-handoff-kernel-entry-microcontract-v0.1.md)** — narrows first QEMU/OVMF boot to one EFI PE32+ application embedding a restricted freestanding ELF64 kernel payload, validated current GetMemoryMap/ExitBootServices ownership, higher-half protected CR3 map with dual-access trampoline and SysV kernel entry, bounded COM1/exception fail-stop. 20 unexecuted fault schedules, 20 unproved invariants and 8 outstanding H-O decisions. **historical superseded readiness caveat** (prior to fixed-link closure and the later scoped user Go); no source, tests or accepted ABI.
- **[UniOS Minimal Boot, Exception & Memory MVP Contract v0.1 — Proposed L1 Scope](docs/contracts/unios-minimal-boot-exception-memory-mvp-contract-v0.1.md)** — isolates the *first possible* implementation slice to QEMU Q35/OVMF single-BSP UEFI loader → freestanding Rust-first diagnostic kernel, fixed supervisor-only memory, IDT/GDT/TSS exception fail-stop and bounded COM1 serial. Excludes Tasks/Cap IPC, Pager, real drivers/DMA, disk writes, GUI and compatibility; reviews ExitBootServices, memory-map/BootContext handoff, 18 future fault tests and 20 unproved invariants. **historical pre-closure status**, superseded for the narrow L1 experiment by later contracts, assessment and Go; no source code or tests.
- **[UniOS Implementation Entry & Design-Completion Readiness Gate v0.1 — Proposed](docs/reviews/unios-implementation-entry-readiness-gate-v0.1.md)** — defines a scoped R6 approval assessment: L1 experimental UEFI/QEMU boot/diagnostic kernel, L2 minimal Task/Capability/IPC, L3 later product assurance. L1 requires B01–B10, L2 C01–C09; its **historical initial NO-GO** for L1 was later superseded by the scoped assessment and user-Go record; L2/L3 remain NO-GO. Advanced pageable RECEIVE must not block narrowly scoped boot research; explicit user authorization remains required before any code.
- **[CAPIPC Paired Mapping–Publication Cut & Invalidation Ordering v0.1 — Proposed](docs/models/capipc-paired-mapping-publication-invalidation-ordering-v0.1.md)** — compares one joint (M,P) publisher Start U-J, two-ticket rollback U-T and restricted register ReturnRelease U-R. Separates VM M-DENY/M-DRAIN and issuer P-DENY/P-DRAIN, requires nofault bounded publications, accurate Grant K-I/receipt, M0/Pager safety and honest stale-binding failure. 22 paper attack schedules, 18 unproved invariants, 12 OPEN questions; no executable model, ABI or code.
- **[CAPIPC Destination Memory & Publication Barrier Cross-RFC Decision Table v0.1 — Proposed](docs/reviews/capipc-destination-memory-publication-cross-rfc-decision-table-v0.1.md)** — compares VA-targeted, typed MemoryObject and register-only receive outputs, paired VM M-DENY/M-DRAIN and issuer P-DENY/P-DRAIN, audience/backing classification, K-I/receipt and Pager/M0 proof conditions; 18 decision rows, 26 unexecuted races, 24 unproved properties, 16 open blockers. Design only; no approved ADR or ABI.
- **[CAPIPC Publication Barrier & Pager-Safe Copyout Feasibility v0.1 — Proposed Design Research](docs/models/capipc-publication-barrier-pager-safe-copyout-feasibility-v0.1.md)** — distinguishes issuer-scoped **P-DENY** (no new publication permits) from **P-DRAIN** (old publishers safely quiesced); compares pre-resolved bounded no-fault usercopy slices P1, register-only IPC P0, controlled mailbox P2 and unsafe unbounded faultable lease P3. Pinning/epoch recheck alone cannot undo data disclosure or prove shared-buffer confidentiality. 18 paper transitions, 24 unexecuted races, 24 unproved properties, 16 OPEN questions; no executable model, ABI or code.
- **[CAPIPC Joint A/C/E Semantic-Cut Resolution v0.1 — Proposed Design Only](docs/models/capipc-joint-semantic-cut-resolution-a-c-e-v0.1.md)** — narrows sender K-S/revoke/Endpoint order (A0), receiver HPA-B+T1 Vp→Vt→K-I paper candidate (C0), and precharged Pager/M0/queue/Claim cleanup (E0). Newly isolates publication-during-Pager-fault versus E1 owner/issuer PolicyEpoch handoff as a **P0 safety/liveness barrier**, not solved by K-I revalidation. 17 candidate transitions, 24 unexecuted attack traces and 24 unproved invariants; no executable model, implementation or Accepted ADR.
- **[CAPIPC P0 Conflict Resolution & ADR Readiness Matrix v0.1 — Proposed Open Findings](docs/reviews/capipc-p0-conflict-resolution-adr-readiness-v0.1.md)** — prioritizes 18 unified IPC safety/feasibility blockers into 9 workstreams, 8 *draft-only* ADR decision packages, 12 cross-RFC owner edges, 24 unexecuted races and 20 unproved predicates. Critical joint chain: K-S/B/E-M, receiver Vm/Vp/Vt/K-I/revoked vector, and M0/Pager/cleanup. T1 baseline distinct from conditional trusted T2. All ADRs D0/open: no formal checker, code, stable ABI, benchmark or Accepted decision.
- **[CAPIPC Unified SEND–RECEIVE Semantic Contract v0.1 — Proposed Design-Only Joint Contract](docs/contracts/capipc-unified-send-receive-semantic-contract-v0.1.md)** — integrates CO-S / CO-E-C sender K-S/revoke/Endpoint handoff, exclusive audience-bound K-C, HPA-B+ Vm/Vp/Vt/K-I, distinct Task qS/qR receipts, full-vector atomic transfer, truthful NO_KI versus possible disclosure and post-K-S revocation of queued Grant vector, public token anti-ABA, Task death and independent M0/effect/hardware debt. 13 ownership/friction categories, 32 unexecuted race schedules, 30 unproved invariants, 18 OPEN blockers and 12 Proposed candidate decisions. No implementation, formal model execution or Accepted ABI.
- **[Capability · IPC · SABI Joint Integration Contract v0.2 — Proposed Adversarial Review](docs/contracts/capability-ipc-sabi-integration-contract-v0.2.md)** — new independent deep review of E1 atomic MOVE, concurrent endpoint handoff and committed-result recovery, irreversible first user-byte visibility and no foreign requeue after ReceiveClaim copyout failure, caps usable before IDL authorization versus inert escrow, unknown resource/revocation, Channel cycles and sponsor/physical fence cleanup; 28 paper counterexamples, 32 unproved invariants, 22 open blockers, 16 Proposed decisions. V0.1 retained below as historical research. No implementation, formal proof or approval.
- **[CAPIPC Receive Exposure, K-I & Handle Publication Ordering v0.1 — Proposed Semantic-Cut Research](docs/models/capipc-receive-exposure-ki-handle-publication-v0.1.md)** — independently compares HPA-B+ T1 receive against install-first, staged HPA-C and independently trusted T2. Separates K-C, metadata/payload/token visibility Vm/Vp/Vt, full-vector K-I, historical Receipt and current Task-wide capability use; distinguishes terminal NO_KI from possible disclosure, anti-ABA ID retirement, Claim/Endpoint MOVE, revoke, Pager faults and M0. 30 unexecuted cases, 30 unproved invariants, 18 open blockers and 12 Proposed decisions. No executable model, implementation or accepted ABI.
- **[CAPIPC Commit, Revocation Barrier & E1 Endpoint Ownership Ordering v0.1 — Proposed Semantic-Cut Research](docs/models/capipc-commit-revocation-ordering-v0.1.md)** — compares A(q) identity admission with K-S actual transactional source MOVE, scoped revocation B and E1 owner MOVE; recommends CO-S commit-vs-barrier strict ordering and CO-E-C cancel/settle-before-handoff **as research candidates**, clarifies that precommit no-MOVE cannot undo independent close/revoke, and separates InTransit ownerless Endpoint, B/R1/R2-F/R2-O and lost-result receipts. 28 paper races, 26 unproved invariants, 12 Proposed decisions and 16 open blockers; **no implementation, model checker, accepted ABI or verified guarantee**.
- **[CAPIPC Formal Model Preparation v0.1 — Proposed, Design Only](docs/models/capipc-formal-model-preparation-v0.1.md)** — non-executable plan for four finite formalization stages (KR-M2 receipt identity, E1 SEND/revoke, ReceiveClaim first byte visibility and K-I, Pager/M0/death), TLA+/TLC tool comparison with Apalache/Alloy 6, 21 candidate transitions, 28 unproved invariants, 8 conditional liveness claims, 20 unexecuted adversarial schedules and 14 open blockers. Contains **no formal code, model-checker run, kernel implementation, numerical ABI or Accepted ADR**.
- **[CAPIPC Cross-RFC Safety Case v0.1 — Proposed Independent Review / Open Findings](docs/reviews/capipc-cross-rfc-safety-case-v0.1.md)** — joint claim–argument–evidence review of KR-M2 receipt admission against RFC-001/002/003/004/005/006/009 plus hardware, durability, guest and SABI boundaries; 18 OPEN assurance claims, 24 unexecuted adversarial schedules, 24 unproved invariants, 18 open blockers and 10 Proposed decisions. Reiterates no newly confirmed live text contradiction, but six integration proof/authority/TCB holes; no code, model checks, proof, numeric ABI or acceptance.
- **[CAPIPC Ordered vs Sliding-Window Receipt Admission v0.1 — Proposed Independent Feasibility Review](docs/models/capipc-ordered-vs-sliding-window-receipt-admission-v0.1.md)** — compares KR-M1 strict Task-sequence admission with KR-M2 bounded out-of-order window and KR-A nonce history; proposes KR-M2 as multi-Thread *research priority* while retaining KR-M1 baseline; specifies protected contiguous consumed frontier C, bitmap B, separate R[q], explicit no-operation SKIP cut, lost-return races, GC and bounded-resource/gap-liveness trade-offs. 30 paper-only adversarial cases, 28 unproved obligations, 18 blockers and 12 Proposed decisions. No implementation, executable model, performance claim or Accepted ABI.
- **[CAPIPC Finite Handle Namespace, Receipt Ledger & GC v0.1 — Proposed Feasibility Research](docs/models/capipc-finite-handle-namespace-receipt-ledger-gc-v0.1.md)** — arithmetic exhaustion scenarios for HN-A versus HN-B hot-slot retirement, and a new KR-M1 kernel Task sequence high-water-mark candidate that prevents repeated admission of historical operation IDs after bounded receipt GC without perpetual per-operation tombstones. Separates no-replay from result availability, covers ordered multi-Thread admission and gap liveness, quota/memory budgeting, 20 unexecuted race cases, 18 unproved obligations, 8 Proposed decisions. No code, proof, simulator, selected width or approved ABI.
- **[CAPIPC Task Handle Namespace & Kernel Receive Receipts v0.1 — Proposed Research](docs/models/capipc-task-handle-namespace-and-receipt-v0.1.md)** — compares monotonic nonreused public Handles (HN-A), public slot+nonwrapping generation (HN-B) and alternatives against Fuchsia's documented integer Handle reuse; designs preidentified Task-scoped protected kernel receipts and nonces for K-S/K-I/K-U lost-result reconciliation, strict PENDING vs final NOT_COMMITTED vs EXPIRED evidence, bounded ACK/GC and native-vs-Linux FD identity. 26 unexecuted counterexamples, 32 unproved safety invariants, 20 open issues, 16 Proposed decisions. RFC-002 §21, RFC-003 §26 and RFC-009 §26 linked; no OS code, executable model, verified security or stable ABI.
- **[CAPIPC Handle Publication & Activation v0.2 — Independent Adversarial Review (Proposed)](docs/models/capipc-handle-publication-activation-v0.2.md)** — reverses previous universal HPA-C research priority after comparing real bearer trust, lost initial ClaimID, Task-wide concurrent AcceptCommit, public token exhaustion, malicious stalled Stage, TOCTOU of staged user data and effects before Receive Commit. Recommends **HPA-B+ first for general T1 proof research**, with HPA-C optional for meaningful deferred activation or independently trusted T2; 24 unexecuted counterexamples, 28 unproved safety predicates, 18 open issues, 14 Proposed decisions. RFC-003 §25 records the changed advice; no code, executable model or ABI freeze.
- **[CAPIPC Handle Publication, Receive Commit & Capability Activation v0.1 — Historical Proposed Design Study](docs/models/capipc-handle-publication-activation-v0.1.md)** — dedicated paper analysis of user-visible handle token publication versus actual native cap activation; historical earlier HPA-C proof-leader proposal, now independently re-ranked in HPA v0.2, compared with lower-cost HPA-A/B/R alternatives. Includes partial copyout/ABA, policy-revoke, lost activation result, T1/T2 trust, bounded sponsor/M0, 20 unexecuted counterexamples and 24 unproved invariants; RFC-003 §24 linked; no implementation, executable model or Accepted ABI.
- **[CAPIPC SEND–RECEIVE–REVOKE Unified State Model v0.2 — Proposed Adversarial Review](docs/models/capipc-send-receive-revoke-state-model-v0.2.md)** — independently re-reviewed T1/T2 receive trust and authorization, payload vs sensitive metadata exposure, handle number publication vs cap-table installation/activation, receiver Endpoint MOVE, revoked or replayed T2 release, token ABA, bounded orphan/terminal/DMA debt, 20 paper counterexamples, 24 unproved safety obligations and 16 P0/P1 research issues. RFC-003 §6/§23 updated as Proposed-only cross-reference. No OS or formal model implemented.
- **[CAPIPC SEND–RECEIVE–REVOKE Unified State Model v0.1 — Proposed Research](docs/models/capipc-send-receive-revoke-state-model-v0.1.md)** — design-only comparison of RCV-T1 (exposure-aware claim/terminalization) vs optional RCV-T2 (inert-cap escrow with independently trusted release); separate send/receiver Endpoint ownership, 21 guarded transitions, 22 unproved safety properties, 18 unexecuted adversarial schedules, 12 Proposed decisions, sponsor/quarantine limits and OPEN model acceptance gates. No executable model, tests, kernel/SDK implementation or ABI approval.
- **[Capability · IPC · SABI Joint Integration Contract v0.1 — Proposed](docs/contracts/capability-ipc-sabi-integration-contract-v0.1.md)** — cross-RFC 002/003/009/018 joint design research: exclusive E1 endpoints with separate per-client connections; MOVE/ReceiveClaim and copyout/UNKNOWN custody; public-handle ABA, revocation and sponsor accounting; issuer-attested caller vs bearer rights and policy-epoch session revalidation; 24 paper race cases, 36 unproved invariants, 24 blockers and 16 decision proposals. RFC-003 E1 rights-table typo corrected and RFC-018 endpoint topology clarified, but no implementation or acceptance.
- [Five-Kernel Comparison & Feasibility v0.1 — Historical Research Draft](docs/architecture/kernel-architecture-and-feasibility-v0.1.md) — Linux, Windows NT, Apple XNU, FreeBSD, and Fuchsia Zircon comparison and earlier proposed designs. Preserved as research history; its “all decisions pending” status has been superseded by v0.2.

## Guiding principles

1. Preserve a genuinely independent kernel and native architecture.
2. Use public specifications and legally compatible third-party components without assuming they provide automatic binary compatibility.
3. Make security boundaries, failure semantics, and user-data integrity explicit before promising production use.
4. Report compatibility **per tested application, version, CPU architecture, runtime, hardware, and feature set**.
5. Separate **accepted direction**, **proposed specification**, **validated evidence**, and **implemented software**.
6. Keep architecture decisions traceable and revisit them only with documented technical or product evidence.

## Repository

This repository is currently for **research and design documentation**. Implementation must be authorized separately.

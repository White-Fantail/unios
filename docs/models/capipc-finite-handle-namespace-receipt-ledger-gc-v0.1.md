# UniOS CAPIPC — Finite Handle Namespace, Receipt Ledger & Garbage Collection v0.1

> **Status:** Proposed targeted feasibility research — 2026-10-25 NZDT. **NOT Accepted, implemented, model-checked, simulated, benchmarked, or ABI-frozen.**
> **Prior work:** [Task Handle Namespace & Kernel Receive Receipts v0.1](capipc-task-handle-namespace-and-receipt-v0.1.md), [Handle Publication & Activation v0.2](capipc-handle-publication-activation-v0.2.md), and RFC-[002](../rfcs/rfc-002-object-capability-lifetime-delegation-revocation.md)/[003](../rfcs/rfc-003-ipc-wire-protocol-and-synchronization.md)/[009](../rfcs/rfc-009-native-abi-and-system-call-contract.md).
> **Scope:** Arithmetic, adversarial schedules, quota/GC policy alternatives and design-stage go/no-go criteria only. No OS source, syscalls, bit layout, executable test or approved security claim.

## 0. Executive review: do not confuse identity reuse, deduplication and evidence retention

Three independent promises require separate accounting:

1. **Native Handle no-stale-rebind:** an old public value in one live TaskGeneration never resolves to a *different* CapGrant, even if the kernel recycles private slot storage. This is a proposed stronger-than-Fuchsia contract and creates finite-namespace exhaustion.
2. **Operation no-replay within a TaskGeneration:** an old accepted operation identity cannot start a *second* independent irreversible K-S/K-I/K-U after its receipt record is ACKed or garbage-collected.
3. **Outcome recoverability:** the caller can retrieve proof of COMMITTED versus FINAL_NOT_COMMITTED for a *bounded* period while protected final evidence exists. Permanent no-replay does **not** imply permanent receipt evidence.

**New feasibility candidate, KR-M1:** a **strictly sequential per-Task admission sequence / high-water mark (HWM)** plus a **bounded protected pending/final receipt table** can reject every previously admitted sequence forever *for that live TaskGeneration* without retaining a tombstone for each historic operation. When an old detailed record is gone, repeat submissions return **EXPIRED_EVIDENCE / UNKNOWN**, never execute again and never claim NOT_COMMITTED. This reduces *historical dedup state* to O(1) per Task plus bounded active/result entries, at the cost of serialized **admission order**, out-of-order retry friction and finite sequence-space exhaustion. It is **not** an implementation selection, nor cross-Task/reboot exactly-once semantics.

**Proposed research ranking:** compare simple monotonic Handle IDs (HN-A) **at equal priority** with slot+nonwrapping-generation (HN-B); HN-B should not be called the only leading engineering choice until targeted per-slot churn and generation bit budgets are calculated. Compare KR-M1 with KR-A1/A2 slots-and-nonce with retained issued-history; favor whichever preserves proof simplicity and bounded resources under realistic SMP workloads.

## 1. Reference facts and scope of claims

- Official Fuchsia Zircon Handle documentation explicitly permits integer reuse after close; its valid handle integers are local to a process. UniOS's proposed lifetime no-rebind is a conscious additional policy [S1].
- Linux close(2) may release an FD number for reuse before late close errors; retry can close another Thread's new FD. Linux guest FD and OpenFileDescription identities must remain separate [S2].
- Linux io_uring user_data correlates SQE with CQE but does not inherently prevent repeated submissions, grant native authority, or prove remote durable effects [S3].
- UniOS K-S message+MOVE and K-I receive-install are distinct irreversible kernel transitions. Retained kernel-lifetime status is not a storage/fsync, network delivery, or SABI exactly-once receipt.

## 2. Finite native public Handle namespace budget

Let U be the number of usable **distinct public values** in one TaskGeneration after invalid/sentinel values and reserved encodings; let I(T) be *all* values issued, previously active, or conservatively maybe published by time T. Strong no-rebind requires I(T) ≤ U, and every allocation after exhaustion must fail closed rather than wrap.

**HN-A monotonic native token**:
- Conceptual public value issuance is strictly increasing and never repeated in the Task lifetime. A kernel-internal slot can be recycled independently.
- Necessary bound: integral from 0 to T of issue_rate(t) dt ≤ U. This includes IDs consumed by failed or partially published copyout if exposure cannot be ruled out.
- Lookup needs a secure map from sparse public ID to live current CapGrant plus TaskGeneration; monotonic token alone does not authorize any operation.
- Near-exhaustion reporting, safe process termination and *explicit* app policy are design questions; resetting counters without changing TaskGeneration is prohibited.

**HN-B slot+nonwrapping generation**:
- With s external slot-index bits and g external generation bits, conceptual maximum addressable slots ≤ 2^s and per-slot *distinct* generations ≤ 2^g, subtracting reserved encodings. **This is symbolic and NOT a selected ABI layout.**
- For slot j, let C_j(T) be the number of public versions consumed at that slot. It must satisfy C_j(T) ≤ G_j, the actual usable generation count. When exhausted, permanently retire that slot for the Task lifetime, or fail closed.
- Total theoretical U ≤ sum_j G_j is **not** an availability guarantee: one repeatedly recycled slot can exhaust far ahead of other slots. Allocation fairness and churn concentration are separate constraints.
- A generation check rejects stale public tokens only if the **complete externally represented** generation is checked against the current slot and cannot silently wrap. Private counters wider than the public field do not repair a wrapped public identity.
- The time to exhaust one hot slot is G_j / churn_j; retirement decreases remaining concurrent-handle capacity even when aggregate generations elsewhere are unused.

**HN-C opaque random** fails deterministic never-reuse if previously issued IDs are forgotten and only active-value collisions are checked. A collision-checked lifetime issued-history or a never-repeating construction is needed; random guessing resistance is not a same-Task security boundary.

### 2.1 Illustrative arithmetic, NOT workload assumptions or ABI decisions

Assume no reserved values for illustration only; numbers are ideal upper bounds and ignore admission/allocator overhead.

| Hypothetical public namespace | Rate assumed | Exhaustion approximation | Interpretation |
|---|---:|---:|---|
| 32-bit HN-A, 2^32 IDs | 1,000 issued IDs/s | 49.7 days | problematic for a long-lived, high-churn Task |
| 32-bit HN-A, 2^32 IDs | 1,000,000 issued IDs/s | 71.6 minutes | adversarial churn demonstrates finite risk |
| 64-bit HN-A, 2^64 IDs | 1,000,000 issued IDs/s | ~584,542 years | enormous *arithmetic* headroom; no guarantee a product can sustain that rate or all 64 bits are available |
| HN-B, 16-bit generation **on one slot** | 1,000 reissues/s on that slot | 65.5 seconds | concentrated churn defeats a naïve small generation field |
| HN-B, 32-bit generation **on one slot** | 1,000 reissues/s on that slot | 49.7 days | unrelated free slots do not save the exhausted hot slot |
| HN-B, 40-bit generation **on one slot** | 1,000 reissues/s on that slot | 34.8 years | still finite for always-on tasks |
| HN-B, 40-bit generation **on one slot** | 1,000,000 reissues/s on that slot | 12.7 days | hypothetical hostile churn; caps/budget may prevent reaching this rate |

A packed hypothetical 64-bit (s=24,g=40) representation is **merely arithmetic**, not a recommendation: it gives at most 2^24 directly encodable slots and 2^40 versions per slot before reserved values. Dense memory for every possible slot may be prohibitive; a sparse table changes lookup/accounting. **Do not promote those example widths into RFC-009 KABI.**

### 2.2 Competing policy choices — none accepted

| Choice | Safety | Availability/TCB concern |
|---|---|---|
| H-A: HN-A large monotonic finite values | straightforward no-rebind proof | sparse lookup; inevitable theoretical lifetime exhaustion |
| H-B: HN-B generations, retirement on wrap | straightforward current-generation lookup | hot-slot burn, shrinking slot capacity, harder allocator fairness/retirement proof |
| H-C: relax native no-rebind to Linux/Fuchsia-like reuse | no lifetime identity exhaustion for fixed slot | explicit stale-number ABA risk; must amend security claim and use handle hygiene; not silently compatible with current proposal |
| H-D: opt-in strict namespace / different Task policy | may allocate strict cost to targeted tasks | two security promises and conditional SDK behaviors increase verification surface |

**Current recommendation:** preserve the strong policy as Proposed; prove and measure H-A vs H-B first. Do not select H-C or H-D without an explicit separate threat-model/ADR revision.

## 3. Why naïve Receipt GC permits dangerous replay

If a Task chooses nonce N and kernel records a committed SEND under N, then ACK/GC removes its **only** historical evidence, a later duplicate N must not be interpreted as a new operation merely because no record is found. Retaining a tombstone for *every* historical N creates lifetime-linear memory; expiry without a separate reuse fence defeats the promise.

A **deduplication fence** and a **result record** are logically different:
- A fence only says “this old sequence **cannot** be admitted as a new operation”; it need not preserve original payload, final status or cap tokens.
- A result record says what happened at K-S/K-I/K-U and may require private tokens/rights/Endpoint binding to reconstruct.
- After final record GC, it is coherent to return **EXPIRED_EVIDENCE/UNKNOWN + definitely not readmitted**, but **not** safe to report definitely NOT_COMMITTED or automatically resend.

## 4. KR-M1: strictly ordered Task admission sequence (new candidate, Proposed)

**Scope:** one exact kernel TaskGeneration, one protected Task-level receipt/query authority, and an independent per-Task admission counter H. A caller supplies a known positive request sequence number before any irreversible cut; sequence number and operation fingerprint are correlation, not bearer rights. Internal H and records are kernel-protected.

**Candidate admission contract:**
1. Validate actual Task/receipt-query authority, operation class, kernel CapGrants and Endpoint generation, and bound user-controlled input before commit.
2. For a *new* sequence q, accept only if q == H+1, sufficient receipt capacity was reserved, and all operation admission checks pass. In one protected admission cut, **advance H to q and create immutable PENDING(q,fingerprint)**. The pending record exists before any K-S/K-I/K-U commit.
3. If q ≤ H, **never** start another irreversible operation. Query matching retained q record if authorized; if evidence has been retired, return **EXPIRED_EVIDENCE/UNKNOWN**, never “fresh.”
4. If q > H+1, reject as **NOT_ADMITTED_NO_CUT** without advancing H. The caller may retry this *same* q after missing prior sequences are admitted. Do not treat this rejection as an applied effect.
5. A successful admission may later become COMMITTED_KS, COMMITTED_KI, COMMITTED_KU or FINAL_NOT_COMMITTED using a kernel-coherent commit/result transition. Query PENDING does not imply replay permission.
6. Final records can be ACKed/GCed under bounded retention. The unchanged H **still forbids** replay of every q ≤ H; H must never wrap or decrease while that TaskGeneration is live.
7. Pending records cannot be blindly GCed while an operation can still commit; they require an authoritative finalization/fence first. Task death transfers/settles outstanding queue/claim/DMA liabilities under independent custodians.

**New invariant:** for any q in one TaskGeneration, count of successfully admitted independent kernel operations using q ≤ 1, even after final-result GC. This is **not** a claim that the operation committed, that status remains queryable, or that the same external SABI request was never applied after a Task restart.

**SMP cost / failure mode:** q=12 may race ahead of q=11 from a different Thread. Strict sequencing rejects 12 until 11 is admitted; parallel *execution/completion* after admission can still proceed. A caller that permanently skips 11 can block progress. Cancellation/skip or a bounded reorder window must be researched and cannot be casually treated as a commit-free no-op; concurrent Task runtimes need a defined sequencer.

**Security scope:** a malicious Thread inside the same Task may intentionally burn H or starve its own Task, just as it may use Task-wide caps. If Threads require different security principals, isolate them in different Tasks; receipt sequence values are not secret authorization.

### 4.1 KR-M2 alternative: bounded out-of-order admission window (not selected)

Let a protected low watermark L and a bounded bitmap/window of W sequence identities represent admissible unretired range, each atomically marked used. Any q ≤ L is forever refused or answered from retained status; within the window each q is admitted at most once; any q beyond the window is rejected until the window can advance. This might support concurrent out-of-order callers but adds hole management, edge cases around window advancement, malicious gaps and GC. **A sliding window without persistent L and correct used-bit roll is NOT sufficient**. No size W or exact scheme has been selected or proved.

### 4.2 KR-A1/A2 with unordered random nonce and lifetime ledger (reference)

Allows simple out-of-order submission but requires an exact uniqueness fence through the Task lifetime even after final receipts vanish: a growing issued-key set, nonwrapping slot generation, or a distinct bounded probabilistic contract. It may be preferred if admission sequencing is too expensive, but cannot claim O(1) lifetime memory merely by time-based TTL.

## 5. Receipt capacity, quotas and costs

Define Q as *total protected pending + unacknowledged final records* admitted per Task, c as bytes per protected record including indexes/metadata, a as admitted operations per second and τ as worst-case time a completed record remains unacknowledged. Under sustained a and no ACK, occupancy grows roughly as a·τ until Q; memory upper bound **at least** Q·c plus fixed Task ledger, sponsor/claim and independently owned queue/physical-debt metadata. Actual sizing requires object overhead and allocator fragmentation.

**Illustration only:** Q=4,096, c=96 bytes implies 384 KiB raw record storage per Task, before indexing, alignment and sponsor metadata. At 1,000 operations/s with *all* results unacknowledged, these Q slots fill in about 4.1 seconds. Without fixed admission/backpressure, receipt storage can starve unrelated Tasks or M0 recovery.

Proposed invariant: **reserve all mandatory receipt space before the irreversible operation cut**; if unavailable, reject new admission as definitely no-cut. Avoid solving the problem with unbounded tombstones or compulsory pageable allocation in protected Ring-0 commit paths.

A high-water mark stays O(1) per Task and is **not** a replacement for quota Q: outstanding PENDING operations, committed message/cap custody, unresolved DMA fences, terminal claim cleanup and actual final records still consume separately charged bounded resources. Receipt-expiry policy can delete evidence, but **must not** release physical or external-effect obligations.

### 5.1 Expiry policy and truth contract

| Condition | Allowed assertion | Forbidden assertion |
|---|---|---|
| New q admission rejected before any cut | NOT_ADMITTED_NO_CUT | application/service effect definitely absent globally |
| Admitted q retained as PENDING | might later commit; cannot replay | definitive FINAL_NOT_COMMITTED |
| Admitted q retained final NOT_COMMITTED | exactly this kernel mutation cannot later commit | another independent service side effect absent |
| Admitted q retained COMMITTED_KS/KI | exact kernel cut happened | receiver application saw full bytes; storage/network effect durable |
| Admitted q final result GCed but H ≥ q | no readmission; old outcome UNKNOWN | absent record means safe resend |
| q sequence exceeds representable range | fail closed / new Task lifecycle if separately authorized | silent counter wrap and reused identity |
| Kernel reboot/old Task death | normal Task-local evidence unavailable | Task H alone proves cross-reboot service exactly-once |

**No guarantee of eventual queryable outcome** is possible with bounded evidence and unbounded client absence; this availability-versus-memory trade-off must appear in the public-facing design contract.

## 6. Twenty adversarial schedules (FQ-C01…FQ-C20) — NOT executed

| ID | Schedule to examine | Required outcome / risk |
|---|---|---|
| FQ-C01 | HN-A high churn approaches U | allocation rejects before public identity wrap |
| FQ-C02 | HN-B attacker repeatedly frees/reuses one hot slot | generation retirement or allocator preserves no-rebind |
| FQ-C03 | HN-B generation wrapped privately but not in public bits | stale token must not bind newly unrelated grant |
| FQ-C04 | token copyout writes only a prefix then faults | maybe-published value treated as spent |
| FQ-C05 | internal slot recycled with independent object grant lineage | public compare rejects stale value |
| FQ-C06 | prior N committed, final record ACKed, exact N resubmitted | old operation **not** executed again |
| FQ-C07 | Task Thread B submits q=12 before Thread A submits q=11 | 12 no-cut retry allowed; no implicit gap consumption |
| FQ-C08 | q=11 never submitted, all later q rejected | documented liveness limitation, not false success |
| FQ-C09 | H advanced but pending record not installed due allocation fault | **forbidden** admission cut; atomicity proof needed |
| FQ-C10 | first call committed but return lost; duplicate q races with GC | query final or EXPIRED/UNKNOWN; never second K-S |
| FQ-C11 | query sees PENDING immediately before commit | no retry or final-negative inference |
| FQ-C12 | final receipt ACK races unrelated Thread's query | coherent final or EXPIRED, not different operation |
| FQ-C13 | malicious Thread submits q near representable maximum | strict q==H+1 prevents a remote jump; Task can still self-starve |
| FQ-C14 | Q is full before K-S then another receiver frees record | no optimistic commit without reserved receipt |
| FQ-C15 | pending record TTL fires during in-flight K-I | may not disappear while future commit remains possible |
| FQ-C16 | COMMITTED_KI token later closed/moved before status requery | no cap resurrection or second grant issuance |
| FQ-C17 | Task restarts with same display ID and q=1 | fresh TaskGeneration; old kernel history not conflated |
| FQ-C18 | reply transmitted externally before kernel/task crash | no claim of external exactly-once without SABI ledger |
| FQ-C19 | guest Linux FD reused under same native Task | guest fd/OFD generation independent of HN-A/B and KR-M1 |
| FQ-C20 | receipt GC frees sponsor but DMA pin remains | pinned frames stay charged/fenced until independent proof |

## 7. Proposed proof obligations (FQ-I01…FQ-I18) — NONE verified

1. All user-visible Handle values (including maybe-published) remain non-rebind within one live TaskGeneration.
2. HN-A token issuance never wraps into an old valid value.
3. HN-B public generation does not wrap into an old externally visible slot/generation value.
4. Native internal slot reuse cannot supply authority without current Task and grant validation.
5. Quota or identifier exhaustion fails closed before any new grant/operation exposure.
6. Receipt request sequence is not a bearer right; Task+scoped grant are independently authenticated.
7. For each q under KR-M1, at most one successful admission exists in a live TaskGeneration.
8. Protected H monotonicity survives all final GC/ACK within that TaskGeneration.
9. H and PENDING record admission have one coherent protected cut; no H advance without its durable kernel-lifetime record until settled.
10. K-S/K-I/K-U transition and final receipt phase have a single coherent irreversible cut.
11. A final receipt may GC only after its protected operation is conclusively settled; PENDING cannot disappear into an unsafe future commit.
12. Missing old evidence returns UNKNOWN/EXPIRED and never fresh/new for q ≤ H.
13. Authorization, privacy and no-silent-rights-expansion survive source Handle MOVE and revocation.
14. Historical COMMITTED_KI doesn't create/revive a new CapGrant at query.
15. New TaskGeneration cannot query prior Task evidence by stale numeric q alone.
16. Queue/claim/reply/DMA/accounting obligations outlive unrelated receipt GC appropriately.
17. Under bounded Q, saturated admission fails without sacrificing independently protected M0 recovery.
18. All the above are design obligations, not proofs, and say nothing about cross-reboot/external exactly-once.

## 8. Decision candidates and design-stage exit gates

**FQ-D01 — PROPOSED:** keep strong Task-lifetime native no-rebind until a separately reviewed ADR relaxes it.

**FQ-D02 — PROPOSED:** put HN-A and HN-B on equal footing for proof/performance feasibility; do not promote an HN-B bit layout based on generic table efficiency.

**FQ-D03 — PROPOSED:** research KR-M1 strictly sequential monotonic admission fence as a memory-bounded dedup alternative to lifetime tombstones; retain KR-A1/A2 as reference.

**FQ-D04 — PROPOSED:** explicitly separate the no-reexecution horizon (Task lifetime), final-result availability horizon (bounded quota/TTL/ACK), and external-effect idempotency horizon (SABI service-specific).

**FQ-D05 — PROPOSED:** keep pending protected, bounded, coherent and sponsor-charged; when Q is exhausted, fail new admission before K-S/K-I/K-U.

**FQ-D06 — PROPOSED:** reject stale/expired q from ever becoming a fresh operation, but do not assert its original final outcome after evidence GC.

**FQ-D07 — PROPOSED:** Task admission sequencer must define multi-Thread ordering and skip/gap liveness before recommending KR-M1 for general use.

**FQ-D08 — PROPOSED:** kernel reboot, Task restart, guest Linux FD reuse and storage/network idempotence are separate contracts, not features of a receipt HWM.

**P0 still OPEN:** choose native Handle width/cost and applicable issue-rate assumptions; compare HN-A map vs HN-B retirement under hostile churn; specify Task admission sequence ownership, concurrent callers, gap-cancellation and receipt query rights; prove coherent HWM+PENDING allocation and K-S/K-I mutation; bound Q/c/TTL/sponsor; demonstrate safe GC/close/Task death; define truthful SDK/error/effect mapping. **No numerical parameter or ABI is approved by the worked examples.**

**Next paper recommendation:** analyze **KR-M1 strict sequencing versus KR-M2 bounded sliding window** with lost-result and multi-Thread traces, then a cross-RFC 002/003/009 proof-obligation reconciliation. Do not build simulator or implement kernel until separately authorized.

## 9. Sources and methodological limits

- [S1] Official Fuchsia Zircon Handles: https://fuchsia.dev/fuchsia-src/concepts/kernel/handles
- [S2] Linux close(2): https://man7.org/linux/man-pages/man2/close.2.html
- [S3] Linux io_uring(7): https://man7.org/linux/man-pages/man7/io_uring.7.html
- [S4] Linux epoll(7): https://man7.org/linux/man-pages/man7/epoll.7.html
- [S5] UniOS prior design-only RFCs and models linked in header.

**Limitations:** numerical examples are arithmetic thought experiments, not empirical rates; counterexamples and invariants are unexecuted paper cases; no proof, feature test, security assurance, accepted ABI or OS implementation is claimed.

---

**History:** 2026-10-25 initial Proposed finite-namespace and GC follow-up. Novel candidate KR-M1 separates perpetual Task-lifetime duplicate-admission rejection via kernel monotonic HWM from bounded outcome evidence, with explicit admission-order liveness cost. **20 paper-only races, 18 unproved obligations, 8 candidate decisions.**

## 10. 2026-10-25 Follow-up: Ordered vs sliding-window admission — Proposed research pointer

This v0.1 remains historical primary **feasibility arithmetic and KR-M1 proposal**. The independent [Ordered vs Sliding-Window Receipt Admission v0.1](capipc-ordered-vs-sliding-window-receipt-admission-v0.1.md) now prioritizes **KR-M2 as a multi-Thread *research candidate***, while retaining KR-M1 as simplicity/reference. It specifies protected contiguous-consumed frontier C, bounded used bitmap B, separate pending/final result R[q], and an explicit SKIP(q) cut for never-admitted gaps. Advancing C across an unseen q without SKIP is not accepted; bounded W ultimately stalls on permanent holes. No acceptance, ABI freeze or implementation results from this cross-reference.

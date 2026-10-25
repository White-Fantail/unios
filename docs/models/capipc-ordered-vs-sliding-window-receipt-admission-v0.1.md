# UniOS CAPIPC — Ordered vs Sliding-Window Receipt Admission v0.1

> **Status:** Proposed Independent Design Feasibility Review — 2026-10-25 NZDT. **NOT Accepted, implemented, verified, benchmarked, simulated, security-certified or ABI-frozen.**
> **Scope:** Compare KR-M1 strict Task-sequence admission with KR-M2 bounded out-of-order sliding-window admission, and KR-A unordered nonce/slot history as a control. No kernel/runtime source, executable state model, syscall signature, size/bit selection or adoption of a wire format.
> **Preceding research:** [Finite Handle Namespace, Receipt Ledger & GC v0.1](capipc-finite-handle-namespace-receipt-ledger-gc-v0.1.md), [Task Handle Namespace & Kernel Receive Receipts v0.1](capipc-task-handle-namespace-and-receipt-v0.1.md), [Handle Publication & Activation v0.2](capipc-handle-publication-activation-v0.2.md).
> **Owning RFCs:** [RFC-002](../rfcs/rfc-002-object-capability-lifetime-delegation-revocation.md) CapGrant/Task/ABA, [RFC-003](../rfcs/rfc-003-ipc-wire-protocol-and-synchronization.md) SEND/RECEIVE, [RFC-009](../rfcs/rfc-009-native-abi-and-system-call-contract.md) native operation boundary; dependent [RFC-004](../rfcs/rfc-004-virtual-memory-pager-cache-and-oom.md), [RFC-005](../rfcs/rfc-005-scheduling-preemption-and-cpu-budget.md), [RFC-006](../rfcs/rfc-006-boot-trust-chain-service-startup-and-recovery.md), [RFC-007](../rfcs/rfc-007-user-space-drivers-dma-and-iommu.md), [RFC-010](../rfcs/rfc-010-linux-personality-and-binary-compatibility.md), [RFC-018](../rfcs/rfc-018-versioned-idl-abi-evolution-and-compatibility-contracts.md).

## 0. Executive verdict

**Advance KR-M2 as the leading *research candidate* for ordinary multi-Thread native IPC, but keep KR-M1 as the primary simplicity/reference model; do not approve either yet.** KR-M1 serializes *admission*, **not necessarily execution or completion**. KR-M2 allows q=13 to be admitted while q=11 is not yet admitted, within a fixed bounded window. It can avoid unnecessary head-of-line blocking under Thread preemption. It requires strictly more kernel state and careful proof of window movement, concurrency, gaps and GC.

**New central finding:** A bounded replay window cannot be advanced by *dropping unresolved holes* or resetting old bitmap bits merely to make progress. A missing early sequence creates **head-of-line blocking at the window frontier**, just later than under KR-M1. Safe recovery requires a scoped, explicit **SKIP/ABANDON admission cut** for each never-admitted identity, or a complete redesign of identity allocation. SKIP is **not** rollback or cancellation of an already admitted SEND/RECEIVE, and may intentionally prevent a tardy legitimate request from ever running.

**Most important separation:**
- **Identity admission fence:** at most one distinct kernel operation is admitted per (actual TaskGeneration, protected receipt namespace, q) while that TaskGeneration lives.
- **Historical outcome receipt:** tells what actual K-S, K-I or K-U cut occurred, only while bounded protected evidence survives.
- **Service/backend effect ledger:** independently handles a user-level operation or durable/network effect; neither Task sequence approach grants cross-reboot exactly-once.

A high-water mark or sliding bitmap solves **replay of an identical operation identity**. It does **not** stop the application submitting the same business request with a **new q**. Thus never advertise application exactly-once solely from KR-M1/KR-M2.

### 0.1 Comparison

| Aspect | KR-M1 (strict consecutive admission) | KR-M2 (bounded out-of-order admission) | KR-A (unordered nonce/slot history) |
|---|---|---|---|
| New admissible q | exactly H+1 | C < q ≤ C+W, unconsumed bit | unused validated key/slot epoch |
| Older q | q≤H never readmitted | q≤C or bit=used never readmitted | depends on persistent nonreuse fence |
| Inter-Thread out-of-order calls | early higher q rejected | supported within W | supported |
| Execution/completion concurrency | allowed after admission | allowed after admission | allowed |
| Hole response | next q blocked | window advances until earliest hole stalls it | independent keys, but GC/ABA hard |
| Lifetime historical fence memory | O(1) Task H | O(W) Task bitmap + O(1) C | may require growing history or nonwrapping slot generations |
| Detailed evidence space | bounded Q records | bounded Q records | bounded Q records |
| Kernel TCB risk | smaller admission state | bitmap, advancement, epoch/tag, skip races | anti-reuse history/nonce collision |
| Liveness hazard | runtime never sends q=H+1 | window fills behind oldest gap | exhaustion of key/slot/history budget |
| Strong guarantee | Task-local no-readmission for old q | same, **only with protected C/bitmap and no unsafe slide** | only after separate uniqueness/GC proof |
| Status | Proposed comparator | Proposed research leader | Proposed reference |

**Why the preference changed:** KR-M2 is a better *candidate for the common case* of simultaneous native Threads because one preempted pre-entry caller need not immediately reject other callers. This is a research priority ranking, **not** verified performance or an Accepted ADR. If a small bounded concurrency design proves too complicated, prefer KR-M1 or a different authorized reservation model over false exactly-once claims.

## 1. Primary technical precedents and limits

1. **IETF RFC 6479** describes a bitmap-based IPsec anti-replay window which tolerates bounded out-of-order authenticated packet sequences [S1]. It is a useful **data-structure analogy**, not proof that a CAPIPC transfer is safe.
2. **IETF RFC 9000 §12.3** forbids packet-number reuse in a QUIC packet-number space, permits minimum-number rejection to limit duplicate tracking and warns that keeping every packet identity indefinitely consumes state [S2]. Packet duplicate rejection does not provide UniOS's coherent capability ownership and postcommit receipts.
3. **Linux io_uring(7)** states submission order does not imply execution or completion order and uses caller-provided `user_data` to correlate completions [S3]. This supports separating sequence admission from actual K-S/K-I ordering. `user_data` is not permission or native idempotency proof.
4. **Fuchsia Zircon Handles** may reuse closed handle *integer values* [S4]; that design does not automatically apply to UniOS's independently proposed stronger Task Handle anti-ABA promise.
5. Source UniOS CAPIPC documents above remain paper proposals and have not proven these candidate transitions.

## 2. Identity and threat model: avoid category errors

**Authenticated request identity:** (TaskGeneration T, ReceiptNamespace R, sequence q). The kernel must authenticate the current Task, operation/query right and protected namespace at invocation. Caller-controlled q alone is correlation, **not a capability**. Protected operation fingerprint F binds the actual kernel-validated invocation class K-S/K-I/K-U, Endpoint object/generation, cap GrantIDs/lineage/rights, ReceiverClaim where applicable, relevant policy epoch and frozen request/cap vector. Never trust merely a hash of user-provided raw Handle integers or unvalidated SABI payload.

**Distinct principals:** Two Threads in one Task share the Task-level native cap authority unless explicitly isolated in different Tasks/compartments. A malicious in-Task Thread can consume future q or request SKIP and cause **same-Task denial of service**; nonce secrecy and bitmap ordering do not create a new intra-Task privilege boundary. Cross-Task receipt reads and post-Task death Supervisor exceptions must require separate narrow grants.

**Non-goals:** guaranteed receipt evidence after unlimited caller absence, automatic recovery of user semantic intent after lost memory, native FD/PID mapping for Linux GABI, service-external exactly-once effects, rollback of first user byte visibility, or hardware DMA fence inference.

**Existing E1 and HPA boundaries unchanged:** E1 Endpoint exclusivity, single source cap MOVE at K-S, ReceiveClaim and possible payload visibility V, coherent full cap-vector K-I, separate optional T2 K-U and R2-F/R2-O are not solved by q admission.

## 3. Core state and cut vocabulary — logical only, not ABI

| State | Kernel-owned meaning | Caller-visible safe interpretation |
|---|---|---|
| **UNSEEN(q)** | q was never admitted or explicitly consumed in T/R | a future admission *may* be possible subject to window/fence |
| **CONSUMED_SKIP(q)** | protected SKIP cut fences q before any operation admission | that q cannot later perform a native mutation; **not** cancellation of another q/effect |
| **ADMITTED_PENDING(q,F)** | charged protected receipt and immutable F admitted; irreversible operation may still happen | PENDING; **do not retry as new mutation** |
| **FINAL_NOT_COMMITTED(q,F)** | this admitted operation has settled before K-S/K-I/K-U and cannot later commit | definitive noncommit for this exact native operation |
| **COMMITTED_KS/KI/KU(q,F)** | matching atomic kernel transition happened | kernel-level only; not proof of app visibility, live current cap, service effect or storage durability |
| **TERMINAL(q,F)** | claim/message settlement with possible exposed payload or cleanup debt | not necessarily zero external effect |
| **EXPIRED_EVIDENCE(q)** | detailed protected outcome no longer retained, but replay fence persists | UNKNOWN historical result, **q cannot be readmitted** |
| **REJECTED_FUTURE/REJECTED_RESOURCE** | admission not performed and q not consumed | no K-S/K-I/K-U for that invocation; may retry the *same* q after conditions change |
| **REJECTED_STALE/CONFLICT** | q already consumed or bound to another operation | must not create fresh operation; do not infer outcome from rejection |
| **TASK_DEAD/BOOT_LOST** | no ordinary live Task query authority or protected kernel record | cannot claim a remote effect never happened |

**Atomic cuts:**
- **A(q):** protected admission/consumption fence and protected record reservation.
- **S(q):** actual K-S MOVE + queue append and protected commit phase agree.
- **I(q):** actual K-I cap install + dequeue and protected commit phase agree.
- **U(q):** optional K-U activation + protected commit phase agree.
- **X(q):** protected SKIP of as-yet-unadmitted q.
- **G(q):** safe final-evidence GC; admission fence must remain.
These cuts may be different events; no user-visible copyout counts as authority for A/S/I/U/X/G.

## 4. KR-M1 precise admission model (reference baseline)

Protected Task state: H = greatest consecutively **consumed** q in this TaskGeneration; bounded map of retained per-q records; fixed quotas/sponsor; narrower query authority. Initially H=0 for a fresh TaskGeneration namespace. q is positive and nonwrapping.

- **New operation:** after validation and mandatory record reservation, only q=H+1 may atomically set H=q and install immutable ADMITTED_PENDING(q,F). All later S/I/U cuts must consistently update this record; result-copyout may be lost.
- **Older or equal q:** if q≤H, no second operation admission; disclose retained record only after correct receipt-query authorization or return EXPIRED/UNKNOWN after G.
- **Future q:** if q>H+1, reject definitely without a cut (do **not** advance H). Caller may retry q later, not invent a new key merely to bypass earlier uncertainty.
- **SKIP q=H+1:** if q has not been admitted, a separately authorized, explicitly acknowledged abandonment may consume q as CONSUMED_SKIP, advancing H without running K-S/I/U. A delayed native invocation with this q is then permanently rejected. Same-Task DoS availability is explicit; authorizing automatic SKIP simply to keep progress is unsafe.
- **No wrap:** if H+1 cannot be represented as a distinct external q, reject further admission or require a separately authorized new Task-generation lifecycle. No silent reset inside a live Task.
- **Admission != execution ordering:** A(11) before A(12) does **not** itself impose S(11) before S(12), same-Endpoint FIFO, completion ordering or external effects.

**Crash hole:** if Thread A reserves q=11 in *user space*, is preempted before A(11), and Thread B submits q=12, B is denied until q=11 arrives or is deliberately SKIPped. Runtime token allocation, termination notification and recovery orchestration are distinct from the protected kernel ledger.

## 5. KR-M2 precise bounded sliding-window model (leading paper candidate)

### 5.1 Protected state and invariants

Maintain within actual T/R:
- **C**, a monotonic *contiguous consumed frontier*: every q≤C has either been admitted **once** or explicitly SKIPped, and is forbidden from new admission for the rest of T/R lifetime.
- **W**, finite window capacity, with admissible range **(C, C+W]**, subject to checked integer overflow; value W is **not selected**.
- **B**, a protected bitmap/tagged set of exactly W logical positions representing consumed q in that range. Position bit 1 means ADMITTED_PENDING/final or CONSUMED_SKIP, not “operation committed”; position bit 0 means never admitted or skipped.
- **R[q]**, a separate sponsor-charged bounded record map for active/pending and optionally retained finalized operations. R survives C frontier advancement while its outcome or pending ownership must be retained.
- Explicit TaskGeneration, receipt-query authority, operation fingerprint, resource quotas, policy/revoke/Endpoint preconditions.

**C is not the largest q observed.** Unlike a typical “largest seen” packet anti-replay window, C advances **only across a contiguous series of consumed positions**. Jumping C past a hole would wrongly let the kernel forget a legitimate not-yet-admitted request *without an explicit cancellation decision*. If a product intentionally chooses “drop missing old requests,” it must model this as explicit X(q) cuts and publish the resulting loss policy; it is not a harmless bitmap optimization.

### 5.2 Candidate transition rules

| Input | Preconditions | Protected transition | Return truth |
|---|---|---|---|
| Submit q≤C | actual Task and scoped right checked | **no new A**; query R[q] only if authorized | prior final/PENDING if retained; else EXPIRED/UNKNOWN |
| Submit C<q≤C+W, B[q]=1 | actual Task/receipt scope checked | no new A; compare immutable F when R[q] exists | same operation's status or CONFLICT/EXPIRED, not new |
| Submit C<q≤C+W, B[q]=0 | validate native grants/Endpoint/revoke; reserve R[q] + sponsor first | in one protected cut set B[q]=1 and create PENDING(q,F), then advance C across *contiguous used* positions | admitted, may commit later or finish not committed |
| Submit q>C+W | q verified, no wrap arithmetic | no A, no bit set, no HWM skip | FUTURE_WINDOW, no cut, retry **same q** later |
| SKIP C<q≤C+W, B[q]=0 | scoped abandonment authority; distinguish real A vs X race | atomically consume q as SKIP, set B[q]=1; advance contiguous C | no mutation for exact q, late Submit q refused |
| SKIP already consumed q | same Task authority, compare existing state | cannot change pending/final to skipped | existing PENDING/final or conflict |
| Slide C | the **next** position C+1 has B=1 from protected A or X | increment C and retire/rebase covered bitmap position, without destroying R[q] or cap custody | replay of all q≤C remains barred |
| GC final R[q] | final irreversible cut and settlement proven; retention/ACK policy satisfied | erase detailed evidence only, do **not** revert C or a still-needed B bit | future query may be EXPIRED/UNKNOWN, no readmission |
| GC pending R[q] | operation still may S/I/U | **forbidden** | no false final noncommit |
| Task death | authority cut, bounded outstanding native queue/claim/DMA liability disposition | Task namespace inaccessible to ordinary new Task; separate sponsor/fence rules | no reboot/external exactly-once claim |

**Important:** bitmap advancement **may occur while R[q] is PENDING**, because C is about **admission**, not completion. That pending record cannot be freed by advancing C. If R[q] is finalized and later GCed, C/B still blocks duplicate admission.

**Identity invariance under circular indexing:** if implementation later uses a ring bitmap, logical position is tied to exact q and C epoch/rotation, never just modulo W. Before any bit reset/reuse, prove the old q has already entered immutable q≤C rejected range. A raw `q mod W` bit without protected interval/epoch permits ABA.

### 5.3 Paper trace: accepts out-of-order, then slides safely

Assume conceptual W=4, C=10; admissible q∈{11,12,13,14} (numbers are examples, **not ABI parameters**).

| Event | C | Used bitmap positions after event | Explanation |
|---|---:|---|---|
| Initial | 10 | none | four q inside window |
| Submit 13, A(13) | 10 | {13} | accepted although 11 and 12 missing |
| Submit 12, A(12) | 10 | {12,13} | independently admitted and may execute/finish out of order |
| Submit 15 | 10 | {12,13} | 15 outside (10,14], **no-cut rejection** |
| Explicit SKIP 11 | 13 | none | consumed {11,12,13} contiguous; C moves to 13; R[12],R[13] remain as necessary |
| Submit 15, A(15) | 13 | {15} | now inside (13,17], admitted |
| Late Submit 11 | 13 | {15} | q≤C, **cannot run**, even if 11's original caller wakes |
| GC final R[13] | 13 | {15} | result may be UNKNOWN, but q=13 remains replay-fenced |

The example SKIP(11) intentionally loses an operation that **was never admitted**. Any claimed business-level success for 11 is false. No pending or already committed operation can be retroactively SKIPped.

### 5.4 Why textbook network replay windows are not a drop-in

A packet anti-replay design may discard a packet that arrives too far behind the highest authenticated sequence; the external protocol intentionally tolerates that loss. UniOS has an additional contract: if a Task has designated q for an unadmitted *non-idempotent operation*, silently treating its hole as irrevocably skipped changes native call admission guarantees. Therefore **do not** lift RFC 6479 “advance to new highest number and drop old packets” semantics without an explicit abandon/rejection policy. Native K-S/K-I evidence, cap custody and leaked output bytes still require protected commit accounting after a window slide.

## 6. Gaps, abandoned calls, concurrency and the recovery protocol

### 6.1 Preknown ID bootstrap — what is and isn't guaranteed

A caller must know q *before* it crosses an irreversible A/S/I/U cut. A Task-local runtime atomic counter can propose q, but the allocation may be interrupted, Thread may die before Submit, shared memory may be corrupted, or two runtimes may choose the same q. The kernel's protected C/B ledger remains authoritative, not userspace counter state.

A scoped, read-only **frontier query** could disclose C/window occupancy and retained result identities to that Task to assist recovery, but:
- it does not tell which user-level request (if any) was intended for a missing q;
- a snapshot can race another Thread's admission/SKIP, so no client may trust it as an atomic reservation;
- revealing privileged endpoint/cap metadata demands separate per-receipt field rights;
- it cannot revive a closed source Handle, recover bytes from a dead Task, or prove service effect absent.

A per-Task *kernel-side reservation allocator that returns a fresh q only via a lost output* reintroduces the bootstrap paradox. Any advanced reservation scheme must use a **preknown slot/ordinal or discoverable protected reservation**, and must budget reservations and their lifetime separately; otherwise stay with caller-proposed q and explicit admission.

### 6.2 SKIP semantics — no secret cancellation and no automatic rollback

**SKIP is a protected never-admit barrier**, not a termination status for an already committed operation. Required serializable race outcomes for same q:
- A(q) wins first: X(q) must fail/read existing pending/final result; cannot undo K-S/I/U or free receipt.
- X(q) wins first: later A(q) must refuse; no source MOVE or unexpected cap activation from that late caller.
- Neither wins due quota/revoke/invalid authority: neither may claim admission or SKIP succeeded.
- A(q) already accepted but return lost: q cannot be SKIPped to make caller's UNKNOWN look like a final no-cut.

**Authorization and policy:** an ordinary app-wide Task right to SKIP lets one malicious Thread suppress another Thread's operation. That is an availability attack inside a shared Task, not bypass of an actual independently isolated Task. An optional delegated **sequencer/recovery authority** and per-lane policy could reduce accidental skips, but increases TCB/user-service dependency. No general automatic SKIP after a timeout without a written data-loss contract.

### 6.3 Missing low q indefinitely — impossible triangle

With finite W, strict per-q no-readmission, no unbounded historical memory and refusal to silently abandon unseen q, **permanent gaps eventually block progress**. The design cannot promise all four simultaneously:
1. arbitrary future out-of-order q admitted;
2. finite bitmap/table;
3. no lost unadmitted q and no forced abandonment;
4. unbounded forward progress under permanently missing early q.

Practical alternatives are: explicit authenticated SKIP with loss semantics, coordinator guarantees gap-free issuance, bounded parked-gaps with forced Task termination/restart and external effect reconciliation, or changing the API to use protected preidentified reservation slots with a different proof burden. Infinite W is not an acceptable M0 solution.

### 6.4 Endpoint order is a separate cut

Admitting A(13) before A(12) can permit S(13) before S(12). An E1 Channel must define its own FIFO commit order, revocation barriers and exclusive owner handoff. Neither q ordering nor completion-order sorting can replace Endpoint commit serialization. Application RPC causal dependencies must be expressed explicitly by the runtime/SABI, not inferred from q.

## 7. Capacity, worst-case resource pressure and runtime implications

Let W = window positions, Q = charged active + retained-final receipt records per Task, c = worst-case charged bytes per receipt including allocator/index overhead, a = successful receipt admissions per unit time, τ = maximum record retention time under no ACK, and B_task = Task sponsor budget. All are unknown and **not configured**.

- **KR-M1 replay fence:** O(1) H; **KR-M2 replay fence:** O(W) bitmap plus protected C and possible tags/index; both require independent O(Q·c) receipt storage and queue/claim/DMAs.
- **Window ≠ Q:** W bounds how far out-of-order admission can outrun the earliest gap. It does **not** cap the total operations simultaneously PENDING after C moves; Q still controls pending/final receipt memory and separate IPC/backpressure budgets.
- **Conceptual bitmap arithmetic:** W=256 consumes **32 raw bytes of bits**, W=4,096 consumes **512 raw bytes of bits**, excluding synchronization, ring tags, per-Task metadata and records. Small bitmap bytes do not prove low CPU/TCB cost.
- **Record illustration:** Q=4,096 with hypothetical c=96 charged bytes is 384 KiB raw receipt records per Task, before indexing. With 1,000 admitted requests/s and zero ACK/GC, capacity fills after ~4.1s. These are *stress arithmetic examples*, not measurements.
- **Worst-case scheduling:** a malicious stalled Thread can leave q=C+1 vacant while consuming all later q inside W, making further arrivals reject. A sponsor quota should bound this Task's damage; it must not starve M0, Pager, recovery IRQ/CPU time or other Tasks.
- **Adversarial valid records:** attacker submits q inside window with bogus F, races loss/revocation/endpoint handoff; validation must occur against real protected grants and checked admission cut. Rejection from invalid parameters **must not** consume q unless explicit X(q) semantics are separately invoked.
- **Overflow:** perform checked C+W and q calculations; when the namespace nears representational end, safely reduce admissible window or fail closed. No modulo-wrapped q accepted as fresh.
- **GC:** expiry can remove FINAL detailed evidence, but must not remove PENDING, ReplayFence C/B, queue-cap custodian, leaked secret/terminal disclosure history where required, or DMA fence/pins.

**Benchmark criteria if separately approved in the future:** admission-throughput vs active concurrency, lock contention at W=1 and bounded W, p99 submission rejection/latency under preempted lowest-q Thread, recovery after SKIP, memory per Task under Q exhaustion, and cross-Task isolation/M0 latency. **No measurements have been made.**

## 8. Adversarial schedule register (SW-C01…SW-C30) — NONE EXECUTED

| ID | Schedule/attack | Required outcome |
|---|---|---|
| SW-C01 | M1 q=12 reaches kernel before assigned q=11 | no-cut FUTURE, not silently reissued under q=13 |
| SW-C02 | M1 q=11 preempted before admission, 12 blocked indefinitely | documented liveness bound, not false commit |
| SW-C03 | M1 SKIP(11) races A(11) | exactly one wins; no retroactive rollback |
| SW-C04 | M2 q=13 admitted before q=11,12 | B[13] consumed; others still eligible |
| SW-C05 | M2 bitmap slides past unseen q=11 on q=13 arrival | forbidden silent hole deletion |
| SW-C06 | q=C+W+1 arrives with valid caps | rejected no-cut; source cap not MOVEd |
| SW-C07 | q=C+W accepted after C increment | must use freshly validated actual grant/Endpoint |
| SW-C08 | duplicate q races original A(q) on different CPUs | at most one protected operation admission |
| SW-C09 | same q with different fingerprint F | conflict, never operate on second F |
| SW-C10 | q bit set but R[q] allocation failed | forbidden torn admission, must fail before cut |
| SW-C11 | R[q] exists but bit/C admission fence absent | forbidden state; otherwise GC/retry duplicates |
| SW-C12 | M2 C advances across q with R[q] still PENDING | retain R[q], may commit later; no fresh q |
| SW-C13 | GC final R[q] while q still in bitmap range | B remains used or C already covers q |
| SW-C14 | ring bit reused by q+W while old q not yet behind C | forbidden modulo-alias ABA |
| SW-C15 | ACK vs another Thread's result Query | linearizable final or EXPIRED, no different F |
| SW-C16 | Submit q races SKIP q during copyin/page fault | cut remains protected; safe pre/post ordering |
| SW-C17 | SKIP wins while Thread has delayed preflight but no A cut | late preflight cannot commit K-S/I/U |
| SW-C18 | SKIP loses after A(q) while SEND result copyout lost | cannot falsify old COMMITTED as no-cut |
| SW-C19 | Task runtime crashes after allocating q but before first kernel entry | missing identity requires explicit recovery/abandonment |
| SW-C20 | caller reuses q after evidence GC under same TaskGeneration | stale refused as EXPIRED/UNKNOWN, no second operation |
| SW-C21 | caller submits identical business request with new q | native fence cannot prevent second service effect |
| SW-C22 | Task dies, new Task with same visible display ID and q | TaskGeneration/authority separation |
| SW-C23 | Kernel restarts after external service received request | no cross-boot effect guarantee |
| SW-C24 | moved source Handle queried for receipt but no longer owned | separate preissued scoped result authority works or explicitly unavailable |
| SW-C25 | malicious neighbor Task guesses q and ReceiptNamespace | must not learn another Task's pending/cap/Endpoint metadata |
| SW-C26 | revoked grant after admission but before K-S | actual revocation barrier respected at commit |
| SW-C27 | K-I succeeded; other receiver Thread closes token; result queried | historic COMMITTED_KI, no cap resurrection |
| SW-C28 | receive bytes partly exposed, claim terminal, q final receipt GC | no foreign requeue or false no-disclosure claim |
| SW-C29 | W or Q saturated while M0 needs resources | new admissions fail before cut, M0 unaffected |
| SW-C30 | close/Task death while DMA pins remain after receipt GC | independent R2-F/quarantine charge preserved |

## 9. Proposed safety and liveness proof obligations (SW-I01…SW-I28) — NONE PROVED

| ID | Obligation |
|---|---|
| SW-I01 | q namespace is scoped to authenticated live TaskGeneration and receipt right; nonce/sequence alone never authorizes caps. |
| SW-I02 | M1 H monotonically increases only with one protected A(q) or X(q). |
| SW-I03 | M2 C monotonically increases only past contiguous positions already consumed by A or X. |
| SW-I04 | Every M2 B used position refers to exact q, not only q modulo W. |
| SW-I05 | An accepted q cannot be admitted again after record GC, window slide, ACK or Thread restart in the same T/R. |
| SW-I06 | At most one distinct immutable F binds any admitted q. |
| SW-I07 | A(q) atomically creates protected pending record and consumes admission identity, after reserved quota, before irreversible S/I/U. |
| SW-I08 | Rejected new q due FUTURE/QUOTA/INVALID does not consume q or source cap, except explicit X. |
| SW-I09 | X(q) and A(q) are linearizable; X cannot cancel an existing admitted q. |
| SW-I10 | X(q) emits no K-S/I/U and never claims it undid independent earlier service effects. |
| SW-I11 | PENDING cannot be interpreted as final noncommit or trigger blind replay. |
| SW-I12 | K-S + protected result phase agree on exact message/cap vector custody. |
| SW-I13 | K-I + protected result phase agree on all-or-none grant install and no duplicate cap issuance. |
| SW-I14 | K-U + protected phase does not activate already revoked or untrusted escrow. |
| SW-I15 | Receipt queries are linearizable against A/S/I/U/X/G or conservatively UNKNOWN. |
| SW-I16 | Final record GC never destroys required pending or independent queue/claim/DMA liabilities. |
| SW-I17 | After G, q result UNKNOWN does not imply fresh admission or original NOT_COMMITTED. |
| SW-I18 | Strictly checked sequence arithmetic never wraps q/C/H into prior public identity. |
| SW-I19 | Queries after source MOVE require independently scoped Task/result rights, not old source cap ownership. |
| SW-I20 | Query/skip/frontier exposure never leaks another Task/ServiceIncarnation sensitive metadata. |
| SW-I21 | Two same-Task Threads can race submit/skip/query without violating replay fence; liveness may need coordinator. |
| SW-I22 | Admission order never falsely implies E1 Endpoint send commit FIFO or service effect order. |
| SW-I23 | Task destruction/Restart cannot transfer old receipt namespace authority to same numerical TaskID. |
| SW-I24 | Sponsor quotas and bounded Q/W prevent hostile workload from drawing down protected M0 recovery reserve. |
| SW-I25 | Revoke barrier, Endpoint ownership MOVE and possible V exposure remain independent enforced cuts. |
| SW-I26 | Valid Task operations cannot be indefinitely denied by scheduling alone under declared fairness assumptions **unless** a known gap/quota/resource starvation condition holds. |
| SW-I27 | If a gap permanently remains without authorized SKIP, eventual W saturation is reported honestly; no unconditional forward-progress promise. |
| SW-I28 | Kernel receipt never by itself proves remote idempotency, storage durability or cross-reboot exactly-once. |

## 10. Open blockers, priority and owning RFCs (SW-O01…SW-O18)

| ID | Level | Decision/proof still required | Owner |
|---|---|---|---|
| SW-O01 | P0 | Decide strict operation identity uniqueness scope (Task vs scoped receipt lane) and TaskGeneration bootstrap | 002/009 |
| SW-O02 | P0 | Define preknown q issuance under concurrent Threads without relying on lost-return allocation | 003/009 |
| SW-O03 | P0 | Prove KR-M2 contiguous frontier C/bitmap invariants under SMP and bitmap/ring slot turnover | 003/009 |
| SW-O04 | P0 | Define SKIP authorization, consent/data-loss semantics and racing late caller visibility | 002/003/009 |
| SW-O05 | P0 | Bound, detect and recover permanent holes without involuntary silent commit/no-cut claims | 003/005/006 |
| SW-O06 | P0 | Atomic admission A with quota reservation, F capture, cap grant/revoke/Endpoint validation | 002/003/004 |
| SW-O07 | P0 | Coherent S/I/U vs protected receipt state and usercopy/result-loss | 003/004/009 |
| SW-O08 | P0 | GC/ACK: finalized evidence vs pending record vs C/B fence independent retention | 002/003/009 |
| SW-O09 | P0 | Determine truthful status taxonomy for stale q, skipped q, expelled final receipt and mismatched F | 003/009 |
| SW-O10 | P0 | Bound total Task Q, W, sponsor and M0/OOM/Task death paths | 004/005/006 |
| SW-O11 | P0 | Separate current CapHandle lookup, historic receipt, revoked grant and guest FD/OFD identities | 002/009/010 |
| SW-O12 | P0 | No incorrect external SABI exactly-once/effect/durability claims | 008/009/018 |
| SW-O13 | P0 | Endpoint owner handoff, revocation B, claim V and terminal cleanup ordering | 002/003/007 |
| SW-O14 | P1 | Choose sliding bitmap/ring data structure only after abstract invariants (no premature source layout) | 003/009 |
| SW-O15 | P1 | Define runtime sequencer recovery after Thread death/lost q mapping | 006/009 |
| SW-O16 | P1 | Evaluate Task-global vs per-lane sequence contention and fine-grained rights | 002/003/005 |
| SW-O17 | P1 | Specify availability goals and measurement workloads to compare KR-M1/KR-M2/nonce slots | 003/005/009 |
| SW-O18 | P1 | Decide narrow Supervisor post-Task-death observation separately from ordinary Task authority | 006/009 |

## 11. Candidate decisions (SW-D01…SW-D12) — ALL PROPOSED, NONE ACCEPTED

- **SW-D01:** Promote KR-M2 as a **paper research priority** for multi-Thread workloads, subject to failure of no-replay/Gc/gap proof or unacceptable TCB cost.
- **SW-D02:** Retain KR-M1 as the minimal reference and potentially preferable choice for single-coordinator Tasks.
- **SW-D03:** Preserve a protected **contiguous consumed** C, not “maximum q ever seen,” for M2. Every skip is an explicit X cut.
- **SW-D04:** Replay-fence storage C/B lives for TaskGeneration even when bounded final R[q] is GCed.
- **SW-D05:** Separate admission sequence/identity from actual K-S/K-I/K-U execution, E1 FIFO and external SABI effects.
- **SW-D06:** Admit A only with protected quota reservation, F binding and native rights validation; allocate no permanent record *after* a commit it claims to reconcile.
- **SW-D07:** Permit SKIP only for never-admitted q and via appropriate explicit scoped authority; never synthesize success/no-effect when an already-admitted call is lost.
- **SW-D08:** Do not automatically silently SKIP holes to advance a bounded window; require a declared recovery/loss policy.
- **SW-D09:** Reject counter overflow, ring ABA, stale TaskGeneration and old q fail closed, without selecting bit widths.
- **SW-D10:** Preserve limited result evidence semantics: old no-replay after GC is guaranteed only inside a live TaskGeneration, old committed outcome may be UNKNOWN.
- **SW-D11:** Keep KR-A unordered nonce/slot history as alternative until a bounded uniqueness fence and concurrency proof exists.
- **SW-D12:** Require an independent cross-RFC + model feasibility gate before any Accepted ADR/ABI. No implementation authorization.

## 12. Future bounded *paper* validation and exit gate (no runnable model)

**Actors:** kernel K; Task A with Threads A1/A2/A3; receiver Task B with distinct generation; E1 Endpoint half and peer; one external revoker R; two distinct source CapGrants; sponsor M0 and ordinary M1 budget; optional backend effect ledger.

**Small finite state domain for a later separately authorized model:** q∈{1,2,3,4,5}; W=2 or 3; Q=2; one TaskGeneration death/restart; one same-q conflicting F; two simultaneous submitters; one explicit SKIPper; one lost-return event; one GC; one K-S and K-I. Use artificially small finite counters for a rollover case. *These bounds are for future model scoping only*, not ABI or realistic performance.

**Safety gate:** find no trace violating SW-I01…SW-I25 and SW-I28 under all bounded interleavings. **Liveness gate:** under explicit scheduling/fairness, admitted work reaches a final state unless legitimate blocker (resource, revocation, gap) is declared; show concrete gap exhaustion traces. **Refusal conditions:** any second A for same q, a silent skip across unseen q, missing record after cut, a q modulo alias, a false NOT_COMMITTED after result loss, M0 debt leak, or incorrect claim of backend effect exactly-once.

**Suggested next research milestone:** write an independent cross-RFC safety-case matrix reconciling KR-M2 A/X/C/B with ReceiveClaim V/K-I, HPA-B+ publication, Endpoint E1 and M0/revocation; clarify whether a per-Task receipt lane is feasible without granting extra native rights. Do **not** implement or run a model without separate authorization.

## 13. Sources, limitations and history

- **[S1] IETF RFC 6479 — IPsec Anti-Replay Algorithm without Bit Shifting:** https://www.rfc-editor.org/rfc/rfc6479.html
- **[S2] IETF RFC 9000 — QUIC, §12.3 Packet Numbers:** https://www.rfc-editor.org/rfc/rfc9000.html#section-12.3
- **[S3] Linux io_uring(7) — correlation and completion ordering:** https://man7.org/linux/man-pages/man7/io_uring.7.html
- **[S4] Fuchsia Zircon Handle reuse semantics:** https://fuchsia.dev/fuchsia-src/concepts/kernel/handles
- **[S5] UniOS source design contracts:** linked in the header, all Proposed.

**Research limits:** Only official written primary sources and internal documents were compared. No executable interleaving schedules, formal proof, kernel code, performance benchmark or selected ABI exists; language such as “must” is a **candidate proof obligation**, not an already enforced OS invariant.

---

**History:** 2026-10-25 Proposed targeted paper — KR-M1 vs KR-M2 and KR-A; explicit contiguous-consumed frontier C, bitmap B, independent receipt R[q], SKIP/abandon cut X, source result-loss, gap-stall impossibility triangle, GC and M0 budget. **30 unexecuted paper race cases, 28 unproved predicates, 18 open blockers and 12 proposed decisions.**

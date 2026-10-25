# UniOS CAPIPC — Formal Model Preparation v0.1

> **Status:** Proposed research preparation — 2026-10-25 NZDT. **NOT Accepted, implemented, executable, formally proved, model-checked, measured, or ABI-frozen.**
> **Constraint:** Documentation and feasibility study **only**. No .tla/.als source, no runnable model, no tool installation, no simulator, no kernel/SDK code, no CI/tests and no deployed behavior.
> **Direct predecessors:** [Cross-RFC Safety Case v0.1](../reviews/capipc-cross-rfc-safety-case-v0.1.md), [Ordered vs Sliding-Window Receipt Admission v0.1](capipc-ordered-vs-sliding-window-receipt-admission-v0.1.md), [Finite Handle Namespace, Receipt Ledger & GC v0.1](capipc-finite-handle-namespace-receipt-ledger-gc-v0.1.md), [Task Handle Namespace & Kernel Receipts v0.1](capipc-task-handle-namespace-and-receipt-v0.1.md), [HPA v0.2](capipc-handle-publication-activation-v0.2.md), [Capability–IPC–SABI Integration v0.2](../contracts/capability-ipc-sabi-integration-contract-v0.2.md).
> **RFC owners:** [RFC-001](../rfcs/rfc-001-kernel-boundary-and-tcb.md) kernel TCB, [RFC-002](../rfcs/rfc-002-object-capability-lifetime-delegation-revocation.md) Capability, [RFC-003](../rfcs/rfc-003-ipc-wire-protocol-and-synchronization.md) IPC, [RFC-004](../rfcs/rfc-004-virtual-memory-pager-cache-and-oom.md) VM/Pager, [RFC-005](../rfcs/rfc-005-scheduling-preemption-and-cpu-budget.md) scheduler, [RFC-006](../rfcs/rfc-006-boot-trust-chain-service-startup-and-recovery.md) M0/recovery, [RFC-009](../rfcs/rfc-009-native-abi-and-system-call-contract.md) ABI; boundary RFC-007/008/010/018.

## 0. Executive recommendation: a small, falsifiable model before any ABI commitment

**Preparation preference: TLA+ + TLC**, first for small finite-state concurrency exploration. **Apalache** is a secondary candidate for symbolic bounded checking/inductiveness, and **Alloy 6** for independent relational checks of Endpoint/Grant/lineage. This is **tool research, not tool installation, executable formalization or checker results**.

The research target is four compositional stages:
1. **FM-1 — Receipt identity:** KR-M2 TaskGeneration, window consumed frontier C, in-window Used, q fingerprint binding, protected receipt R[q], A(q) admission, X(q) explicit SKIP, ACK/GC and returned-result loss.
2. **FM-2 — Capability SEND:** add actual native CapGrant/Handle generation, E1 Endpoint owner, revoke B, immutable input snapshot, source-handle MOVE, queue custody and K-S with protected result.
3. **FM-3 — Receive/exposure:** add receiver's **separate** Task receipt q, exclusive ReceiveClaim, earliest possible byte visibility V, K-I all-or-none cap-vector installation, possible partial copyout and terminal/no-foreign-requeue behavior.
4. **FM-4 — Recovery/resources:** abstract Pager faults, M0 reserve, per-Task sponsor/receipt/claim quotas, Task death/reincarnation, independent hardware/storage/backend-effect evidence.

**Core insight:** “consumed operation q,” “moved sender cap,” “receiver installed grants,” “user byte possibly exposed,” and “remote effect applied” are **five different facts**. Treating them as one transaction would let a future checker report false reassurance.

**Current verdict: OPEN / NO-GO for Accepted KR-M2 or frozen KABI.** There is no proof that the composed protocol preserves safety on SMP or remains useful under finite capacity.

## 1. Official formal-method precedents, assessed narrowly

| Candidate | Documented capability | Why useful | Limitation |
|---|---|---|---|
| **TLA+ / TLC** | Explicit reachable-state exploration of finite model instances; safety and specified liveness/fairness | Natural for competing A/X, K-S/B, V/K-I interleavings | State explosion; a finite instance pass does not prove unmodeled hardware or implementation |
| **Apalache** | SMT-backed symbolic bounded safety checks; separate inductive-invariant capability under finite structures | Independent exploration of guarded transitions and invariants | Bounded depth k misses longer bugs; liveness support and supported TLA+ subset require review |
| **Alloy 6** | Relational finite-scope assertions and temporal/lasso traces | Task/Endpoint/Grant graph constraints, lineage and authority | Bounded object scope and temporal horizon; does not replace full concurrent kernel refinement |

**Technical references:** TLC [S1], Apalache manual [S2, S3], Alloy 6 [S4]. Formal tools examine the **model that was written**, not an actual UniOS kernel. Agreement between two differently abstracted models is not independent proof unless their semantic correspondence is reviewed.

**Tool-selection gate (OPEN):** document exact semantics, license and pinned versions before any future run; choose which checker is authoritative for each property; reject results with unexplained unreachable actions or mismatched assumptions.

## 2. Assurance boundary and explicit assumptions

### 2.1 Model actors

- K: kernel protected state, but **no assumed correctness** for transitions except explicit atomic cuts subject to verification.
- A1: live sender TaskGeneration, two concurrent Threads A.t1/A.t2; A2: a fresh TaskGeneration that could share the same human-readable PID/display name.
- B1: receiving TaskGeneration, two possible receiver Threads; B2: fresh ServiceIncarnation/TaskGeneration.
- E and peer E2: separate halves of **one E1 Channel**; one effective Task owner per Endpoint, subject to bounded admitted invocation refs and explicit MOVE.
- g1, g2: actual kernel CapGrants with type/rights, lineage, sponsor, current Task holder/queue holder and revoke epoch. Raw Handle integers are **not bearer authority in another Task**.
- Revoker, Pager, Supervisor, external backend: separate actors with narrowly assumed authorities; none gets ambient grants from ServiceID/UID/PID/q strings.

**Threat model:** User pointers may fault/alias/change, two Threads in the same Task may be adversarial, syscall completion may be lost, the queue and receiver may die, service generation may restart, Q/W capacity may fill, and revocation or Endpoint MOVE can race a previously admitted operation. Two Threads in one Task share Task-level native rights unless separately isolated; q secrecy is not a new principal boundary.

### 2.2 Deliberately excluded from first proof scope

Exact x86_64 registers, Rust/C memory model, physical TLB/IOMMU timing, filesystem commit protocol, Linux errno/futex/FD translation, SABI parser internals, verified boot cryptography, real device persistence and network idempotency. Represent them only as **conservative environment events with stated assumptions**, never as mechanically proven concrete mechanisms. Example: a nondeterministic hardware-fence-complete event can test receipt independence, but cannot establish that any real NIC/SSD has fenced.

**No actual kernel code exists to refine yet.** Later mapping of implementation linearization points, compiler/CPU ordering, OS policy and hardware behavior is a separate body of evidence.

## 3. Candidate finite model universes — examples, not ABI limits

| Profile | Actors | q | Window W | Receipt quota Q | Target |
|---|---|---|---:|---:|---|
| **P1** | one Task, two Threads | 1–4 | 2 | 2 | A/X/GC/replay |
| **P2** | old+new TaskGeneration, three Threads | 1–5 per Task | 2–3 | 2 | q reuse/generation/status loss |
| **P3** | sender/receiver, one revoker | 1–4 per Task | 2 | 2 | E1 K-S/B/MOVE |
| **P4** | sender, receiver, new B2 | 1–4 per Task | 2 | 2 | V/usercopy failure/K-I |
| **P5** | A/B/M0 + one Pager | small finite | 1–2 | 1–2 | bounded credit, death/cleanup |

Future exploratory sweeps must include **W=1**, W=2 and W=3; Q<W, Q=W and, if tractable, Q>W; minimum and artificially maximal q; duplicate q from different CPUs; expired result with replay fence still live; two different TaskGenerations with same display ID; receiver q numerically equal to sender q without being the same receipt.

**Bound semantics:** q is a whole, **nonwrapping** identity for its authenticated Task receipt namespace; do not quietly use mathematical unbounded integers to avoid finite-encoding exhaustion, and do not reuse ring bits by q mod W without checking actual C/identity. These finite examples do **not** select public Handle bit fields or production quotas.

## 4. Abstract protected state dictionary (NOT code)

| Variable | Owner/lifetime | Meaning |
|---|---|---|
| **TGen[t] / Life[t]** | K, Task admission/death | real Task incarnation; not display ID |
| **C[t]** | K, live TaskGeneration | greatest *contiguously consumed identity*, NOT greatest completed request |
| **Used[t]** | K, live TaskGeneration | consumed q in (C,C+W]; bits denote A or X, not K-S/K-I |
| **Binding[t,q]** | K | None / immutable (operation kind, F, actual grant/Endpoint identity) / Skipped |
| **R[t,q]** | K, protected bounded records | Absent / Pending / FinalNotCommitted / CommittedKS / CommittedKI / CommittedKU / Terminal / ExpiredEvidence |
| **Observed[thread,q]** | caller epistemic state | Unknown vs protected result seen; loss cannot undo actual transition |
| **ReceiptRight[t] / SkipRight[t]** | K grant scope | narrow query/abandonment rights independent of the moved source Handle |
| **Eowner[e], Egen[e]** | K | exclusive actual E1 ownership and owner MOVE epoch |
| **Grant[g]** | K | exact CapGrant, lineage, rights, revoke barrier B/possible bounded invocation ref |
| **Custody[g]** | K | sender / queued-message / receiver / closed / quarantine |
| **Message[m]** | K | unqueued / queued / exclusively claimed / installed / terminal |
| **Claimant[m]** | K | None or authenticated receiver TaskGeneration+Egen |
| **Exposed[m]** | K | monotone conservative possible user-byte disclosure flag |
| **Token[t,h]** | K native table | current grant mapping and maybe-exposed/nonrebind public identity |
| **FaultWaiter[w]** | K | Task/Mapping/PagerBinding generations, pending or settled |
| **Charge[k]** | K sponsor ledger | separate finite receipt, queue, claim, grant, pin and M0 budgets |
| **ServiceInstance[s]** | trusted issuer/service | no authority from raw ServiceID equality |
| **RemoteEffect[x]** | independent backend | not applied / possible / applied, not inferred from R[q] |

### 4.1 Semantics that must not be conflated

- C can move past q **while R[t,q] stays Pending**; pending record and associated queue/cap debt cannot be GCed merely because q≤C.
- Sender A(qA)/K-S and receiver B(qB)/K-I are separate operations in distinct Task/receipt namespaces even when qA=qB numerically. Message ID links the two states, **not** raw receipt-key equality.
- Optional HPA-C/K-U would have a separately admitted operation identity if it independently changes invokable rights.
- Querying historic CommittedKI must **not** reissue a CapGrant already closed/MOVEd by another Thread.
- Kernel “receipt says CommittedKS” cannot promise decoded SABI method success, storage flush, guest FD state, physical DMA R2-F or BootGood.

## 5. Guarded action contracts — natural language only

| ID | Action/cut | Precondition | Atomic state change / safe failure |
|---|---|---|---|
| **FM-T01** | TaskAdmission | authorized image/budget/new generation | fresh TGen; own C=0, Used empty; no old authority by display ID |
| **FM-T02** | PrepareInput | bounded copyin and optional fault | no q consumed; read-only preflight is not future grant |
| **FM-T03** | A(q): Admit | live T/receipt right, q in window and unused, current validated native grant/Endpoint, reserved R/quota, immutable snapshot F | set Binding and Pending R and Used as one protected cut; advance C over *contiguous consumed* positions |
| **FM-T04** | X(q): SKIP | scoped explicit abandonment, q still unadmitted in window | consume q as Skipped without K-S/I/U; atomic against A; late A rejected |
| **FM-T05** | QueryOld(q) | authenticated scoped read grant | return retained R or Expired/Unknown; no cap/queue change |
| **FM-T06** | RejectFuture | q>C+W / representational overflow | no admission, no source MOVE and no implicit SKIP |
| **FM-T07** | K-S SendCommit | matched Pending SEND, actual cap lineage/E1/revoke ordering valid *at cut*, charged queue, immutable bytes | whole vector to one message, sender handles invalid, R=CommittedKS coherently |
| **FM-T08** | AbortBeforeK-S | future commit of *that* operation conclusively prevented | R=FinalNotCommitted; source grants preserved if none moved |
| **FM-T09** | ClaimReceive | authenticated receiver with own qB and matching EndpointGeneration, quota | exclusive message claim and Pending receiver R; bytes still private |
| **FM-T10** | V: MarkExposed | authorized claimant about to attempt user-byte copy | Exposed true **before first possible user write**; irrevocable |
| **FM-T11** | K-I ReceiveCommit | correct claim, whole destination slot budget, lineage/revoke checks | full vector installed once and message retired; R_B=CommittedKI |
| **FM-T12** | UsercopyFault | arbitrary allowed page/remap fault before/after V | no false “nothing seen”; after V no foreign requeue |
| **FM-T13** | TerminalSettlement | receive can't finish, claim bounded | no second receiver after V; terminal custody/quarantine and truthful exposure status |
| **FM-T14** | RevokeB / EndpointMove | authorized controller/owner; concurrency active | explicit barrier cuts; stale preflight is never blanket later K-S permission |
| **FM-T15** | CloseToken / QueryHistorical | exact current grant/token status | close current authority; historic committed result never reactivates |
| **FM-T16** | ReturnLost / Observe | result written, fails or Thread dies | observer knowledge changes only; K-S/K-I truth unchanged |
| **FM-T17** | FinalACK/GC | definite final cut, no future native commit for identity, scoped retention | evict evidence but **preserve q replay fence**, independent queue/pins |
| **FM-T18** | TaskDeath/Reincarnate | authentic termination and fresh admission | remove ordinary query right; sponsor, queue, DMA and external effects separately settled |
| **FM-T19** | PagerFault/Resume | usercopy may block; mapping/Task/Pager epochs may change | revalidate waiters, no forbidden global-lock wait cycle |
| **FM-T20** | QuotaReject/M0Cleanup | ordinary Q/W/full; wired reserve independent | reject A before cut or bounded cleanup; no unbounded M0 charge |
| **FM-T21** | BackendEffect | separate service request/admission/durability policy | remote state may change even if kernel completion observer remains Unknown |

**Open A→K-S authority fork:** (a) revalidate current grant/owner/B at K-S and final-abort if stale; or (b) maintain a **specific bounded pre-barrier invocation reference** with explicitly sequenced rights. Either must be modeled transparently; “A accepted, so SEND anytime after revoke or owner MOVE” is an unacceptable implicit assumption.

**Open V→K-I ordering fork:** performing V first creates unretractable disclosure after fault; performing K-I first may make native caps usable before user-space SABI method validation. Both need truthful separate receipt status, with HPA-B+ bearer as the current general research comparator and HPA-C/T2 as an optional trust-policy alternative, not a universally chosen design.

## 6. Proposed safety invariants FM-I01…FM-I28 — NONE CHECKED OR PROVED

| ID | Candidate invariant |
|---|---|
| FM-I01 | Receipt access is authorized by actual live TGen/scoped grant, not q/ServiceID/guest UID. |
| FM-I02 | At most one immutable operation fingerprint F admitted per (T, receipt namespace, q). |
| FM-I03 | A(q) and X(q) are linearizable; X never cancels already-admitted q. |
| FM-I04 | C monotone; advances across contiguous q consumed by A or X only. |
| FM-I05 | q≤C or Used-bit=1 never permits new admission after GC/ACK/Thread restart. |
| FM-I06 | A consumes q and reserves protected Pending R in the *same* admission cut. |
| FM-I07 | Pending R cannot expire while operation might still commit, or before authoritative settlement. |
| FM-I08 | q, C+W and ring indexing never silently overflow/rebind earlier consumed public q. |
| FM-I09 | Query, ACK, ResultLost cannot MOVE, mint or reinstate any cap. |
| FM-I10 | K-S moves all or none of the exact authenticated source grant vector, with coherent receipt. |
| FM-I11 | K-S respects ordered grant revoke B and exclusive E1 Endpoint owner MOVE. |
| FM-I12 | FinalNotCommitted prevents later K-S/I/U of same admitted operation; sender retains sources if no K-S. |
| FM-I13 | Each E1 Endpoint has at most one effective public owning Task across generations. |
| FM-I14 | A stale Task-public native Handle value cannot authorize an unrelated grant within live TaskGeneration. |
| FM-I15 | Each queued message has at most one exclusive active ReceiveClaim. |
| FM-I16 | Exposed monotone, and no user-visible copy attempt before conservative V publication. |
| FM-I17 | A message Exposed to B may never be normally requeued to a different Task/incarnation. |
| FM-I18 | K-I installs all caps once or none; historic CommittedKI query cannot reissue a grant. |
| FM-I19 | Sender qA, receiver qB and optional separate K-U aren't conflated by equal integers. |
| FM-I20 | Terminal/FinalNotCommitted receive doesn't falsely certify absence of earlier V. |
| FM-I21 | Status query can survive authorized source MOVE via narrow right without other-Task metadata leak. |
| FM-I22 | Revoke B/R1/R2-F/O and already-admitted effects remain separate; post-B invocations not implicitly reauthorized. |
| FM-I23 | PENDING/EXPIRED/observer UNKNOWN aren't mapped to definitive no-cut or safe blind retry. |
| FM-I24 | Receipt/bitmap GC cannot release queued grant, Claim, FaultWaiter, pin/DMA or external-effect debt. |
| FM-I25 | Sponsor/Q/W/claim accounting cannot consume reserved independent M0 capacity. |
| FM-I26 | Pager/usercopy waits cannot depend cyclically on global IPC/MM/cleanup locks. |
| FM-I27 | New TaskGeneration/ServiceIncarnation cannot inherit old receipt/ReplyToken by same display name. |
| FM-I28 | Kernel COMMITTED never, by itself, proves service exactly-once/media durability/BootGood. |

Some require **history/ghost state or auxiliary abstract environment variables** to become checkable. No checklist label is itself a proof.

## 7. Liveness contracts FM-L01…FM-L08 — conditional, NOT demonstrated

| ID | Candidate conditional progress | Required assumptions or legitimate exception |
|---|---|---|
| FM-L01 | valid enabled q eventually admitted | caller retries, Q available, stable rights, fair scheduling |
| FM-L02 | admitted operation eventually final/terminal | bounded service progress or explicit failure path; no indefinite hardware/Pager stall |
| FM-L03 | missing q eventually SKIPped if owner abandons it | scoped SKIP authority and intentional actor action, not timeout alone |
| FM-L04 | C advances when next consumed q exists | bounded fair internal advancement and no overflow |
| FM-L05 | retained final receipt eventually queryable | caller live/authorized, fair query capacity, evidence not expired first |
| FM-L06 | M0 recovery continues under ordinary saturation | wired CPU/queue/frame/cleanup budgets and no wait-for cycle |
| FM-L07 | blocked Pager/usercopy path settles or fails safely | live Pager or bounded failure/kill path |
| FM-L08 | Task shutdown can finish or honestly quarantine | bounded iterative cleanup/actual hardware fence conditions |

**Finite-window impossibility:** with permanent missing q=C+1, no authorized SKIP, finite W and no new TaskGeneration, eventually high-q admission blocks. This is a **declared availability limitation**, not a reason to silently jump C past unadmitted q. Fairness does not make a disabled operation enabled; any future liveness property must explicitly handle permanent gaps.

## 8. Directed adversarial traces FM-X01…FM-X20 — paper cases, NOT executed

| ID | Future targeted schedule | Detect |
|---|---|---|
| FM-X01 | two Threads concurrently A(same q,different F) | I02/I06 |
| FM-X02 | X wins while A's copied arguments are Pager-fault blocked | I03 |
| FM-X03 | A wins, result lost, X tries retroactive cancellation | I03/I09 |
| FM-X04 | q=3 admitted before 1,2; incorrect C jumps to 3 | I04/I05 |
| FM-X05 | old final R GCed, ring bit reset, stale q resubmitted | I05/I08 |
| FM-X06 | C passes Pending q and GC wrongly removes its R | I07/I23 |
| FM-X07 | record quota allocation fails after q consumed, then SEND commits | I06/I10 |
| FM-X08 | toy q maximum overflows into old identity | I08 |
| FM-X09 | K-S commits then copyout faults, source handles falsely restored | I09/I10/I23 |
| FM-X10 | revoke B between A and K-S | I11/I22 |
| FM-X11 | E1 Endpoint owner MOVE races old authorized invoker | I11/I13 |
| FM-X12 | stale public h now resolves to different g | I10/I14 |
| FM-X13 | V happens, usercopy faults, message requeued to B2 | I16/I17 |
| FM-X14 | K-I commits then B2 Thread closes token; status query remints it | I18 |
| FM-X15 | sender qA equals receiver qB numerically, records merged | I19 |
| FM-X16 | receive Terminal with V but “no bytes disclosed” emitted | I20/I23 |
| FM-X17 | source cap MOVEd, protected receipt requires old invalid source h | I21 |
| FM-X18 | dead Task restarts under same displayed ID and reads old receipt | I27 |
| FM-X19 | receipt final GC frees still-pinned DMA page | I24/I25 |
| FM-X20 | Q full, Pager blocked holding/awaiting kernel global IPC lock, M0 starved | I25/I26/L06 |

**Negative controls required in later separately approved checking:** deliberately unsafe alternate rules (skip across unconsumed hole, replay after GC, commit without R update, foreign requeue after V, stale owner authorization) should produce witnesses violating the corresponding invariant. If no error is found when the modeled rule is intentionally broken, the assertion set might be vacuous. **No altered rule or executable scenario has been created/run now.**

## 9. Non-vacuity, refinement and coverage discipline

- **Reachability witnesses:** future checker must demonstrate legal successful A, X, K-S, V, K-I, GC, true FinalNotCommitted, rejected stale q, lost result, revocation and restart — not just “all invariants passed” because no actions occur.
- **SMP interleavings:** actions are atomic only at explicitly justified cuts. Don't hide usercopy/Pager calls, external revoke or Endpoint MOVE inside a large monolithic SEND step.
- **Deadlocks:** distinguish valid intentionally terminal/quarantined states from ordinary starvation and cyclic waits; disabling a checker deadlock warning alone proves nothing.
- **Fairness:** record exact weak/strong fairness per actor/action; don't use scheduler fairness to manufacture a missing q or an authorized SKIP.
- **Symmetry:** cannot arbitrarily interchange sender, receiver, revoker, M0, old/new incarnation or source/destination grants; role-sensitive symmetry can hide bugs.
- **Bounded scope:** “no counterexample within P1–P5” is **not** an all-sizes theorem. Apalache bounded depth k especially cannot exclude a shortest witness with length >k [S3].
- **Implementation refinement:** future code must identify real linearization cuts, CPU memory order and fault/race behavior, and show concrete executions refine this model. A Rust function with matching action name does not establish equivalence.
- **External effects:** store/hardware/network status require separate tests and assumptions. An abstract true fence event can't certify a real IOMMU reset or SSD flush.

### 9.1 Traceability / owner map

| Abstract portion | Cross-RFC case claims | Owners |
|---|---|---|
| FM-1 A/X/C/Used/R | SC-C01–03, SC-C15–16 / SC-I01–07, 23–24 | 002/003/009 |
| FM-2 grant/E1/K-S/revoke | SC-C04, 06, 13 / SC-I08–10 | 002/003/009 |
| FM-3 claim/V/K-I/token/terminal | SC-C05–07 / SC-I11–14 | 002/003/004/018 |
| FM-4 usercopy/M0/death/effects | SC-C08–12,14,17–18 / SC-I15–22 | 004/005/006/007/008/010 |

## 10. Future exit-gate deliverables and refusal conditions

**Gate FM-R1 = OPEN / NOT PASSED.** Before approving an *actual* model/checker (separate user authorization required), demand exact assumption/version matrix, model-state ownership map, approved abstract-cut contract, finite bounds, declared fairness, failure injection, and non-vacuous trace plan.

**If separately approved later**, require model artifacts; configuration sweep P1–P5; checked invariant-by-invariant evidence and reproducible traces; negative controls; explicit partial/exhausted result classifications; documented tool limitations; independent design-to-model review; and a distinct implementation/hardware refinement plan. No such artifact is claimed here.

**STOP criteria for future work:** double A/q, X after admission rewriting history, silent q alias/overflow, K-S with false FinalNotCommitted or restored source grants, post-B unauthorized transfer, foreign redelivery after V, K-I remint, cross-Task receipt leakage, prematurely refunded device debt or M0 reserve overdraw.

**PAUSE for semantic clarification before any executable model:** (1) K-S after A but racing revoke B/Endpoint MOVE, (2) V/K-I/public Handle publication order and terminal status, (3) Task-global vs per-lane receipt/SkipRight semantics, (4) bounded Q/W budget and pending records on Task death. Model authors should not silently decide disputed kernel contracts by encoding convenient guards.

## 11. Proposed decisions FM-D01…FM-D10 — NONE Accepted

- **FM-D01:** Design one canonical finite KR-M2 identity transition model first; use KR-M1 as simpler reference.
- **FM-D02:** Prioritize TLC exploration for concurrent cuts; assess Apalache and Alloy 6 only with comparable assumptions.
- **FM-D03:** Explicitly keep A, X, K-S, V, K-I, K-U, external effect and hardware fence independent.
- **FM-D04:** q is Task/receipt namespace scoped, not authorization; send and receive use distinct identities.
- **FM-D05:** Preserve protected R-before-commit and post-GC no-replay without preserving outcome forever.
- **FM-D06:** Include revocation and E1 owner races at commit, not just at preflight.
- **FM-D07:** Include Pager/copyout and result-loss nondeterminism; no false user-byte rollback.
- **FM-D08:** Require negative-control witnesses and reachable good states before treating checker output as meaningful.
- **FM-D09:** Explicitly separate safety from conditional liveness and external/hardware effects.
- **FM-D10:** Do not create runnable model/kernel code/install tool/change ABI before separate explicit permission.

## 12. Open research questions FM-O01…FM-O14

| ID | Priority | Unresolved exact contract | Owner |
|---|---|---|---|
| FM-O01 | P0 | A→K-S admitted-reference rights vs revoke barrier B and Endpoint owner MOVE | 002/003 |
| FM-O02 | P0 | Exact Task-global receipt namespace versus issuer-scoped per-lane identity and generation | 002/009 |
| FM-O03 | P0 | SKIP authorization, racing X, and lost SKIP response reconciliation | 002/003/009 |
| FM-O04 | P0 | Immutable operation fingerprint including authenticated kernel grants without SABI parser in TCB | 001/002/018 |
| FM-O05 | P0 | C+W overflow and exact bitmap/slot q binding under churn | 003/009 |
| FM-O06 | P0 | Pending R ownership when C advances, Task dies or GC/timeout fires | 002/003/006 |
| FM-O07 | P0 | Receive V, K-I and public Handle/token publication ordering | 002/003/004 |
| FM-O08 | P0 | Receive terminal/FinalNotCommitted status accurately retains possible exposure | 003/009 |
| FM-O09 | P0 | Pager fault wait-for graph and protected M0 no-recursion/no-allocation path | 004/005/006 |
| FM-O10 | P0 | Independent K-U escrow q and activation/revoke receipt effects | 002/003/018 |
| FM-O11 | P0 | Actual Task/Service incarnation identity and post-death receipt-query rights | 006/009/018 |
| FM-O12 | P0 | Symbolic finite sponsor/Q/W/claim/queue budget and admission bound | 002/003/005/006 |
| FM-O13 | P1 | Formal-tool subset/versions and reproducible checking coverage plan | research |
| FM-O14 | P1 | Proof-to-implementation refinement and external hardware/media assumptions | 001/007/008/009 |

**Best next design-only research:** resolve **FM-O01** (the accepted-invocation/revoke/Endpoint MOVE race) and **FM-O07/08** (V/K-I/publication order and honest receive terminal status) as explicit alternative transition contracts **before** implementing any checker. Those assumptions determine the model's validity.

## 13. Official sources, evidence limits, history

- [S1] TLC model checker official TLA+ docs: https://docs.tlapl.us/using:tlc:start
- [S2] Apalache official manual (finite checking, TLC comparison): https://apalache-mc.org/docs/apalache/index.html
- [S3] Apalache bounded execution and inductive checks: https://apalache-mc.org/docs/apalache/running.html
- [S4] Alloy 6 temporal/relational language: https://alloytools.org/alloy6.html
- [S5] Leslie Lamport TLA+ tools: https://lamport.org/tla/tools.html
- [S6] Kernel usercopy/fault contexts: https://docs.kernel.org/kernel-hacking/hacking.html
- [S7] Fuchsia Channel read: https://fuchsia.dev/reference/syscalls/channel_read

**Limitations:** Official first-party tool documentation and UniOS main-branch Proposed design documents were reviewed. Every state variable/action/invariant/attack trace above is a **planned modeling requirement**, not a proven property or test result.

---

**History:** 2026-10-25 Proposed v0.1. Four decomposition stages; 21 candidate actions; 28 unproved invariants; 8 conditional liveness properties; 20 paper-only race schedules; 10 Proposed decisions; 14 open contract questions. No executable formal model, OS code, ABI or Accepted ADR.

## 14. Follow-up: COMMIT versus B/E-M semantic-cut research — Proposed

**FM-O01 remains unproved, but its candidate contract is now sharper:** [CAPIPC Commit, Revocation Barrier & E1 Endpoint Ownership Ordering v0.1](capipc-commit-revocation-ordering-v0.1.md) compares strict K-S-before/B-before outcomes and E1 cancel/settle-before-ownership-transfer (CO-S and CO-E-C), distinguishes A(q) identity admission from final K-S authority, and clarifies that `FINAL_NOT_COMMITTED` means *that attempted SEND did not consume the source*, not that independently closed/revoked Handles remain usable. The Endpoint itself may be Task-ownerless while InTransit. These are **Proposed paper transitions only**, not a change to the original 21-action formal preparation count, an executable spec, proof, ABI decision or kernel implementation. FM-O07/08 Receive V/K-I/publication is still separate and OPEN.

## 15. Follow-up: RECEIVE exposure, K-I and public Handle token cuts — Proposed

[CAPIPC Receive Exposure, K-I & Handle Publication Ordering v0.1](capipc-receive-exposure-ki-handle-publication-v0.1.md) extends the previously OPEN FM-O07/08 analysis with separate K-C, confidential metadata K-Vm, possible payload K-Vp, token output K-Vt and whole-vector K-I, plus optional independent T2 K-U. For HPA-B+ T1, reserved public Handle IDs stay inert until K-I, but after K-I rights are Task-wide even if initiating Thread loses return. A terminal NO_KI cannot imply no information was seen; V-dependent no-foreign-requeue, ID nonrebind and M0 cleanup are distinct. This is a proposed **paper-only specification**, not an executable formal model, proof, accepted ABI or closure of FM-O07/08.

## 16. 2026-10-25 Unified SEND–RECEIVE formal trace inputs — Proposed handoff

**No formal model created:** [CAPIPC Unified SEND–RECEIVE Semantic Contract v0.1](../contracts/capipc-unified-send-receive-semantic-contract-v0.1.md) gives a single kernel queue/message Grant-custody transition relation from sender A(qS)/K-S, through receiver A(qR)/K-C and possible metadata/payload/token V, to all-or-none K-I or exposure-aware terminal. It exposes 13 cross-RFC friction rows, 32 paper-only attack interleavings and 30 unproved invariants; especially mandatory grant B after K-S and before K-I must not silently install a partial vector. This sharpens **future** model FM-2/3 semantic inputs but does not execute TLA+/TLC/Alloy or close FM-R1.

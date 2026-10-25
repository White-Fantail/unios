# UniOS CAPIPC — Commit, Revocation Barrier & E1 Endpoint Ownership Ordering v0.1

> **Status:** Proposed independent semantic-cut and feasibility research — 2026-10-25 NZDT. **NOT Accepted, implemented, formally verified, simulated, benchmarked, or ABI-frozen.**
> **Design stage only:** This paper records candidate contracts, paper counterexamples and future proof gates; **no kernel/SDK code, executable transition model, checker, tests, numeric syscall layout, or launch date**.
> **Primary owners:** [RFC-002 Capabilities and Revocation](../rfcs/rfc-002-object-capability-lifetime-delegation-revocation.md) §§7–11, [RFC-003 E1 IPC](../rfcs/rfc-003-ipc-wire-protocol-and-synchronization.md) §§4–6, [RFC-009 native ABI](../rfcs/rfc-009-native-abi-and-system-call-contract.md) §§5–7/16. Supporting [RFC-004 VM/Pager](../rfcs/rfc-004-virtual-memory-pager-cache-and-oom.md), [RFC-005 Scheduling](../rfcs/rfc-005-scheduling-preemption-and-cpu-budget.md), [RFC-006 Recovery/M0](../rfcs/rfc-006-boot-trust-chain-service-startup-and-recovery.md), [RFC-007 DMA](../rfcs/rfc-007-user-space-drivers-dma-and-iommu.md), [RFC-018 SABI](../rfcs/rfc-018-versioned-idl-abi-evolution-and-compatibility-contracts.md).
> **Upstream:** [CAPIPC Formal Model Preparation v0.1](capipc-formal-model-preparation-v0.1.md), [Cross-RFC Safety Case v0.1](../reviews/capipc-cross-rfc-safety-case-v0.1.md), [CAPIPC Joint Integration Contract v0.2](../contracts/capability-ipc-sabi-integration-contract-v0.2.md), [KR-M2 receipt admission](capipc-ordered-vs-sliding-window-receipt-admission-v0.1.md). This paper refines **FM-O01 / SC-O02** without silently resolving **FM-O07/08** receive publication.

## 0. Executive verdict — distinguish acceptance, authorization, irreversible effect and cleanup

**Recommended research-first default: CO-S ("commit-or-barrier strict ordering") for E1 SEND.** A(q), the Task-scoped KR-M2 *receipt identity admission*, is **not a perpetual capability-to-SEND authorization**. The final **K-S** irrevocable enqueue+complete source MOVE is the native SEND effect cut. If a relevant lineage revoke deny-barrier **B** or effective Endpoint-owner handoff **E-M** precedes K-S, the previously admitted but not yet committed operation should **settle as FINAL_NOT_COMMITTED**, rather than commit a new transfer under stale authorization. If K-S wins first, the already-committed queue/cap custody remains historical fact; B/E-M does not undo it, while subsequent new uses of revoked grants remain restricted.

**Strong new precision:** a final `NOT_COMMITTED` guarantees **no MOVE-induced invalidation from that attempted SEND**. It **does not guarantee every original Handle is still usable** if another Thread independently closed/MOVEd it or its lineage was revoked during the race. The parent RFC wording “all source handles retained” must be understood as the SEND's *own transactional effect* absent, **not a magic undo of independent concurrent mutations**.

**Endpoint owner MOVE has a separate condition from descendant Grant revoke.** A proposed first-profile E1 transition is **cancel-or-settle all precommit invocations before publishing effective ownership transfer** (CO-E-C). A bounded kernel commit section may finish and order strictly before E-M; faultable preflight cannot pin owner indefinitely. Once E-M has won, no old-owner K-S is newly performed. If the Endpoint itself is transferred **as payload** at another K-S, it can be in kernel **InTransit** with *no effective public Task owner* until authorized K-I installs it; one-owner-at-most is not one-owner-at-every-instant.

**Do not confuse four cuts:**
- **A(q):** consume unique caller operation identity and reserve PENDING protected receipt, after bounded immutable input snapshot.
- **K-S:** atomically enqueue exact bytes + *all* source GrantIDs and remove source Task Handle entries as one SEND effect; mark the same receipt COMMITTED_KS coherently.
- **B(g):** publish a deny-new-use barrier for the **selected grant lineage** only, not all independent roots and not proven hardware drain.
- **E-M(e):** transfer **effective owner** of the specific E1 Endpoint, potentially via an intervening InTransit state, with all previously admitted old operations canceled or K-S committed before handoff.

This is a **Proposed ranking**, not a verified algorithm or universal choice for all object types. Prior admitted CPU/mapping/DMA effects have distinct B→R1→R2-F→R2-O obligations and cannot be rolled back by canceling a pending SEND.

## 1. Primary comparison — precedent is not proof

| Official source | Verified documented reference behavior | UniOS implication and limit |
|---|---|---|
| [S1] Fuchsia Channel object | Two Endpoints, no duplicated Channel Endpoint; SEND is whole message+Handle move, Endpoint move has documented old/new message order | E1 and explicit K-S/E-M order is plausible; **does not prove** UniOS's commit-vs-revoke race or lost-return receipt |
| [S2] Fuchsia `zx_channel_write_etc` | MOVE closes source Handle, DUPLICATE retains it, whole message only written when all per-handle operations succeed | Fuchsia **MOVE failure consumption differs** from UniOS proposed precommit retain-the-transaction semantics; cannot transplant error handling |
| [S3] seL4 CNode Revoke API | Revoke explicitly deletes child capabilities of the addressed capability, distinct from per-object action | Lineage scope matters; does **not** establish UniOS's synchronous IPC E1 or B/R1/R2-F guarantee |
| [S4] Linux RCU requirements | Readers can overlap updater; grace period separates removal from reclamation, and does not block *new* readers | Lifetime/grace period alone does not reject **new post-B** SENDs. Revocation needs active authorization cut, not just a reference pin |
| [S5] UniOS RFC-002/003/009 | Atomic MOVE/no restoration after K-S, protected receipt, B/R1/R2-F/R2-O and E1 one owning Endpoint | These are **Proposed target contracts, not running implementation proofs** |

Fuchsia specifically documents owner-transfer relative ordering of in-progress Channel read/write. UniOS's first-profile CO-E-C chooses a potentially **stricter cancel-before-owner-transfer default** to reduce cross-owner admitted-work proof burden, **with higher liveness/latency cost**. It must not be represented as Fuchsia's implementation.

## 2. Identity and effect boundaries

| Identity / reference | Who validates? | What it does **not** mean |
|---|---|---|
| TaskGeneration / actual caller | kernel current execution domain | human-readable TaskID/PID or ServiceID |
| q, KR-M2 A(q), C/Used and receipt R[q] | protected Task/receipt namespace and independently scoped right | source Handle ownership, SEND commit, privilege, delivery or application idempotency |
| CapHandle token / HandleSlot | kernel current Task table + GrantID generation | immutable right if independently closed/MOVEd/revoked |
| GrantID + lineage + revocation epoch | kernel grant graph / authorized controller | whole-object exclusive root; broad cross-lineage revoke |
| E1 EndpointObject / Egen / owner | kernel one effective owning Task or kernel InTransit | logical service name or every client using one identical Endpoint |
| InvocationReference (optional) | protected kernel, bounded credit, exact Task/Endpoint/Grant binding | permission to launch a **new SEND** after B/E-M, or a second effective owner |
| QueueRecord and K-S | kernel atomic source-to-queue cap custody | receiver processed SABI, observed bytes or file persisted |
| B → R1 → R2-F → R2-O | kernel + required CPU/hardware fences | B alone proves physical DMA finished or object memory reusable |
| ResultReceipt R[q] | kernel protected status while retained | general bearer permission, cap reissue or infinite evidence after GC |

**Specific authorization composition at K-S (Proposed):** current TaskGeneration with SEND-bearing actual Endpoint Grant, effective current E1 owner, exact **all** source CapGrants/Handle token entries holding TRANSFER, admissible attenuation rights/types/lineage, no applicable B for a new transfer, queue and sponsor capacity, immutable committed byte+grant vector, and receipt q with immutable matching operation fingerprint. All checks must correspond to **one coherent commit decision**. An unlocked earlier preflight is insufficient.

## 3. Candidate semantic cuts and ordering

### 3.1 Logical phases (not executable pseudocode)

1. **P: Snapshot/prepare** user bytes/descriptor vector into bounded protected storage, then validate Task/Endpoint/current Handle lookups; any fault or buffer alias is handled outside non-faultable commit section.
2. **A: Admit q**: under KR-M2, atomically consume q, bind immutable F and install charged PENDING protected result. A result may be lost. This **reserves an identity, not current authority forever**.
3. **C: Commit contest** among K-S, applicable B, Endpoint E-M, close or another MOVE of a source Handle, Task death. One kernel-consistent order must emerge for every shared authority record.
4. **K-S winner:** protected queue append of entire payload/grant vector + invalidation of exact source entries + COMMITTED_KS phase, all or none for this operation.
5. **B/E-M/close winner:** after a proven no-future-K-S settlement, R[q]=FINAL_NOT_COMMITTED (or remain PENDING while kernel is still safely deciding/canceling). No source entry is removed **by this attempted SEND**; unrelated revoke/close/other MOVE effects still stand.
6. **R: Report** return or a separately scoped protected receipt query; if caller's result is lost, its knowledge is UNKNOWN until authoritative status. Query never remints GrantIDs.
7. **Later** receiver Claim/V/K-I/Service effect, R1/R2-F/R2-O cleanup are independent and cannot be inferred from A, K-S, B or R alone.

**Indispensable atomicity rule:** never enter a state where K-S has invalidated a source or enqueued a Grant while a racing B/E-M has been recorded as *winning before K-S* for that same lineage/Endpoint, yet the receipt is falsely FINAL_NOT_COMMITTED. The chosen serialization may be coarse or fine-grained, but the **observable outcome must refine one coherent total order for conflicting cuts**.

### 3.2 Strict candidate CO-S: B/E-M before K-S prohibits new K-S

| Established cut order | SEND result (historical) | Source rights and consequences |
|---|---|---|
| **K-S < B(g)** | COMMITTED_KS | source grant MOVEd into queue; B later restricts queued/receiver descendant uses; no rollback |
| **B(g) < K-S attempt** | FINAL_NOT_COMMITTED after safe cancellation/settlement | attempted SEND moved none; source token may still exist **but is revoked/unusable** |
| **K-S < E-M** | COMMITTED_KS | queue record belongs to original SEND ordering; new owner gets later privileges only |
| **E-M < K-S attempt** | FINAL_NOT_COMMITTED after safe cancellation | old owner must not emit new message; no transfer-induced source loss |
| **Close(g) < K-S attempt** | FINAL_NOT_COMMITTED | source token independently closed, not “restored”; no SEND-initiated consumption |
| **K-S < Close(g) attempt** | COMMITTED_KS | later close observes source token absent/invalid; no double close or resurrection |
| **Other MOVE(g) < K-S attempt** | FINAL_NOT_COMMITTED | grant moved by other operation; attempted vector send not partially committed |
| **K-S < Other MOVE(g) attempt** | COMMITTED_KS | other operation must fail/settle without MOVing g a second time |
| **TaskDeath < K-S attempt** | cancel pending operation (if not committed), FINAL_NOT_COMMITTED in protected evidence when possible | Task's own receipt query dies; mandatory custody/cleanup still separately accounted |
| **K-S < TaskDeath** | COMMITTED_KS | queued resources survive source Task death; original handle never restored |
| **A(q) < B(g) < K-S attempt** | *still* FINAL_NOT_COMMITTED | A is no authorization lease for fresh post-B SEND |
| **K-S < B(g) < lost return** | COMMITTED_KS actual, caller UNKNOWN until valid query | no blind resubmit with same/new q |

Here “<” means a **defined protected linearization order**, not mere wall-clock entry/exit timestamps or syscall response order. B is scoped: if the controller revoked an independent lineage unrelated to every authorization required by this SEND, that B does **not** block K-S.

### 3.3 Snapshot and lock lifetime: P and A do not pin a future right by accident

A frozen message byte vector + source Handle numbers cannot authenticate future rights by raw integers. Protect the actual GrantID/lineage/slot generation checked at K-S. If one Handle entry has changed between P and K-S, fail/settle the entire vector; never accidentally MOVE a new unrelated grant at the same raw token.

Any prospective **InvocationReference** may pin object memory and protect an in-progress kernel frame from use-after-free until its bounded cleanup. **It does not automatically authorize K-S after B**. A narrow exception for already *irrevocably initiated* CPU/device work is tracked separately by RFC-002 B/R1/R2-F, not by turning A(q) into K-S.

**Candidate non-faultable commit scope:** after all bounded copyin/queue/receipt preparation, compare Task/Grant/E1 owner/revoke epochs and publish K-S vs B/E-M/Close order as a finite protected operation. No faultable usercopy, Pager call, unbounded wait, recursive destructor or global lock held across such work. This statement is a correctness demand, **not a selected mutex, RCU, memory-barrier scheme or known bounded critical-section measurement**.

## 4. Endpoint handoff is not equivalent to revoke

### 4.1 Three candidates, none approved

| Candidate | Effective E1 owner change | Safety proof burden | Availability trade-off |
|---|---|---|---|
| **CO-E-C:** cancel/settle precommit old invocations, then MOVE | all old K-S irrevocable commits finish before owner cut; remaining uncommitted work canceled | relatively smaller: no old new commits after E-M | owner MOVE may stall on old noncancellable commit, bounded quiesce/progress proof required |
| **CO-E-A:** strict quiescence before MOVE without forced cancel | no outstanding admitted invocations at E-M | simple ownership argument | hostile stalled Pager/caller may deny MOVE forever |
| **CO-E-R:** bounded old admitted-reference operations may finish after handoff | owner transition allows carefully scoped old work on formerly owned E1 | harder: two effective send paths/timing FIFO despite one public owner, exact queue order and sponsor | better nonblocking transition in some cases, wider TCB/proof surface |

**Provisional priority: CO-E-C**, *conditional* on finite cancellation and bounded non-faulting K-S. If CO-E-C cannot deliver acceptable owner-transfer latency safely, compare CO-E-R with a separately acknowledged admitted-inflight policy, but never choose it without proving both one effective public owner and queue ordering. CO-E-R is **not** silently authorized by current Proposed E1 contract.

### 4.2 Endpoint moving *as a transferred capability*

A source owner might MOVE E itself as a payload on a **different** Channel, following usual cap transfer restrictions and direct self-send prohibition. At its K-S, the public owner handle for E ceases and E is **InTransit** (no new Task owner) until a later matching K-I installs it in B. While InTransit, no Task gets E1 SEND authority by merely guessing E's ID, and the old owner's pending noncommitted calls cannot use E. When K-I later installs E, B is the new owner with an authenticated new generation/ownership scope; queued old messages have their historical K-S ordering. Receiver usercopy failure and K-I publication remain subject to existing HPA and ReceiveClaim rules, not proved by this paper.

**Owner uniqueness invariant:** at most one **effective public Task owner** at all times; zero while in transit is permitted. A bounded kernel invocation ref or temporary queue custodian does not become a second independent Task owner. The exact point E-M occurs relative to transferring E at K-S, channel queue publication and recipient K-I **must be fixed before future model execution**.

### 4.3 Queue order and E-M race

The E1 direction's FIFO order is determined by **successful protected K-S cuts**, not caller A(q), Thread scheduling order, receipt sequence q or ServiceID. Under CO-E-C, old owner's earlier K-S cuts precede effective handoff, new owner's K-S cuts follow later authorized acquisition. If E remains InTransit, no public owner is allowed to insert messages on E in the gap. Other channels or the peer direction have their own independent ordering.

## 5. Revocation scope: B is not R1/R2-F/R2-O

**B barrier winning a pending SEND only forbids the new SEND authority within the revoked Grant lineage.** A previously committed message may carry a descendant Grant in a protected kernel queue; it retains lineage and sponsor debt. When the new owner later attempts to install/use that Grant, revocation scope and current rights are rechecked; it cannot become newly invokable merely because K-S preceded B.

**Separate revocation promises:**
- **B:** deny *new* selected-lineage native operations/derivations/transfers. Race with K-S is ordered.
- **R1:** enforce grant-dependent CPU map rights cutoff with TLB/fault revalidation. Cannot be inferred from B or receipt.
- **R2-F:** all specified lineage-admitted CPU/device/IRQ/DMA obligations fenced/drained or honestly pending/quarantined; an expired K-S receipt is not evidence.
- **R2-O:** object-wide reclamation only after every independent root/ref and unsafe hardware access has retired; B and R2-F of one family alone are insufficient.

**Independently rooted grants** referencing the same physical object may remain live. Global emergency kill/DeviceGroup reset is a different explicitly granted authority; don't smuggle it through `revoke_descendants`.

**Cascading queued-cap ambiguity:** if B hits a queued transit Grant after K-S, preserve the historical COMMITTED_KS result and settle the queued revoked Grant according to a separately defined close/quarantine/claim policy. This paper does not invent an automatic sender refund, nor a guarantee that a later receiver successfully receives usable rights.

## 6. Receipt and result truth during races

| Actual operation state | What protected query may say | What it cannot say |
|---|---|---|
| A(q) succeeded; race not finalized | **PENDING** | source still owned/unrevoked; safe to retry MOVE; FINAL_NOT_COMMITTED |
| Precommit work conclusively canceled, K-S impossible | **FINAL_NOT_COMMITTED** for this q | all source CapHandles still usable despite independent close/revoke |
| K-S irrevocable cut succeeded | **COMMITTED_KS** | receiver processed; cap remains currently invokable; remote effect durable |
| K-S outcome known to kernel but register copyout lost | actual protected result if retained; caller may know only **UNKNOWN** | undo ownership just because status unreadable |
| Final receipt acknowledged/expired; replay fence C/Used intact | **EXPIRED_EVIDENCE/UNKNOWN**, no re-admission under old q | absence means no commit or new q retry has no external effect |
| Task died/new incarnation | ordinary old Task query unavailable; narrow Supervisor recovery only if authorized | same displayed TaskID, ServiceID or new q transfers old rights |
| B published but R1/R2-F incomplete | **B established**, other phases pending | DMA complete; every copied byte erased; object reclaimable |

**Cancel isn't `SKIP`.** KR-M2 X(q) may consume only **never-admitted** q. After A(q), an abort caused by B/E-M is a **PENDING→FINAL_NOT_COMMITTED** settlement for that already-admitted identity, not reclassifying it as X. The Task's C frontier may already have passed q, so result/queue/sponsor records have independent lifetimes.

## 7. Race catalogue CO-X01…CO-X28 — ALL PAPER CASES, NONE EXECUTED

| ID | Competing events | Unsafe behavior to rule out |
|---|---|---|
| CO-X01 | A(q) succeeds; B published; K-S attempted | A acts as future unrevokable permission |
| CO-X02 | K-S wins; B later publishes; result copyout lost | committed sender grant resurrected |
| CO-X03 | B applies to only one of two source GrantIDs | partial vector MOVE, other grant silently invalidated |
| CO-X04 | B applies only to independent grant family | unrelated legitimate SEND wrongly blocked globally |
| CO-X05 | sender CLOSE(g) wins before K-S | K-S moves stale/no-longer-owned handle |
| CO-X06 | K-S wins then caller CLOSE(g) | close deletes unrelated newly reused token or undoes commit |
| CO-X07 | two concurrent SENDs transfer same g | same exact GrantID has two queue custodians |
| CO-X08 | Endpoint E-M after A(q) but before K-S | former owner sends into E after owner change |
| CO-X09 | old K-S wins before E-M, new owner SEND follows | FIFO violated by q/scheduler-order sorting |
| CO-X10 | Endpoint being MOVEd is also SEND Endpoint in same payload | self-send/cycle invalidity bypass |
| CO-X11 | Endpoint moved as payload to different Channel, sits InTransit | old/new Task both send before recipient K-I |
| CO-X12 | Endpoint recipient dies before K-I | owner magically restored to old Task after original K-S |
| CO-X13 | old owner trapped in Pager faultable preflight during E-M | E-M waits forever under global lock |
| CO-X14 | global lock held during commit and B needs lock while Pager waits | IPC/VM/Recovery wait-for deadlock |
| CO-X15 | B's grant lineage epoch modified after preflight, before K-S | race commits without latest policy |
| CO-X16 | source descriptor bytes changed by another Thread after snapshot | unintended new GrantID or payload transferred |
| CO-X17 | stale public Handle token reuses old internal slot | ABA authorizes unrelated object |
| CO-X18 | K-S receipt marked COMMITTED but one source still active | two effective source/queue custodians |
| CO-X19 | K-S sources invalidated but queue append/receipt allocation fails | lost grant and false FINAL_NOT_COMMITTED |
| CO-X20 | B wins, R[q] remains PENDING, GC removes only record | late K-S possible with no truth evidence |
| CO-X21 | B before K-S, Task claims sender Handle “still usable” | confused use of revoked though un-MOVEd Handle |
| CO-X22 | after K-S, receiver sees revoked queued descendant | B bypassed merely because transit completed |
| CO-X23 | Task dies while commit decision races B | orphan queue/lineage sponsor prematurely refunded |
| CO-X24 | old ServiceIncarnation restarted under same ServiceID | stale owner/receipt metadata retargeted to new grant |
| CO-X25 | B observed, R1 CPU mapping still live | B falsely reported CPU access cutoff |
| CO-X26 | R2-F DMA pending after Task death | recycled pinned frame corrupted by device |
| CO-X27 | q C-frontier advanced but SEND still PENDING | frontier mistaken for final commit/cleanup |
| CO-X28 | new q used for same SABI business operation after ambiguous return | external non-idempotent effect duplicated |

## 8. Proposed cross-RFC safety invariants CO-I01…CO-I26 — NONE PROVED

| ID | Unproved candidate obligation |
|---|---|
| CO-I01 | A(q) authorizes *identity admission* but cannot by itself bypass later B/E-M at K-S. |
| CO-I02 | K-S/B conflicts have one serializable order for every relevant required Grant lineage. |
| CO-I03 | K-S/E-M conflicts have one serializable owner order per exact E1 Endpoint object/generation. |
| CO-I04 | For every q, R[q] final K-S result matches actual whole-vector source/queue custody cut. |
| CO-I05 | Before K-S, failed SEND causes no source invalidation **by that SEND**; independent close/revoke remains effective. |
| CO-I06 | After K-S, no source token/grant is reminted by copyout fault, receipt query, peer death or B. |
| CO-I07 | A matching exact source GrantID cannot be simultaneously moved by two competing SENDs. |
| CO-I08 | No partial MOVE: complete vector+bytes+queue custody change or none. |
| CO-I09 | A direct self-send Endpoint included in own payload remains forbidden in initial E1. |
| CO-I10 | Public Endpoint ownership is unique or in-transit ownerless, with no unauthorized old/new overlap. |
| CO-I11 | Under CO-E-C, no old Task's new K-S occurs after E-M protected owner-change cut. |
| CO-I12 | Queue FIFO per direction follows K-S commit ordering, not q/Thread order. |
| CO-I13 | Source Handle lookup validates actual current Task, entry generation, GrantID, lineage and TRANSFER rights at protected commit. |
| CO-I14 | B applies only to selected revoked lineage, not independent roots or unrelated grant families. |
| CO-I15 | A queued revoked Grant cannot become newly usable after B merely by later K-I delivery. |
| CO-I16 | A pending receipt can't be GCed while K-S can still occur or final settlement is unknown. |
| CO-I17 | FinalNotCommitted means exact K-S can never later commit; it doesn't assert source Handle is live. |
| CO-I18 | Task q and C/Used replay-fence life is independent from COMMITTED_KS receipt/detail retention. |
| CO-I19 | Kernel query/status never grants/revives a CapGrant nor exposes other Task's sensitive metadata. |
| CO-I20 | Lock ordering never blocks Pager/usercopy under incompatible global IPC/VM/Revocation lock. |
| CO-I21 | Every cancellation/drain/commit lock/charge stays bounded, and cannot consume M0 reserve. |
| CO-I22 | B, R1, R2-F and R2-O are distinct checks; never use q result as physical fence proof. |
| CO-I23 | Controller scope and GrantLineage anchor survive source close/queue transit/Task death while effects exist. |
| CO-I24 | Restart/incarnation cannot reuse old Endpoint effective ownership, receipt authority or grant by name alone. |
| CO-I25 | q used for SEND and q used for separate RECEIVE/K-I remain scoped to respective Task/operation. |
| CO-I26 | K-S receipt never alone certifies SABI idempotent effect or storage/boot durability. |

## 9. Liveness and performance trade-offs (conditional, unmeasured)

- **CO-S no-post-B SEND** simplifies authority but means an A(q) admitted before a revoke can be safely canceled rather than given a chance to complete. Service retries require new authorization and independent SABI effect reconciliation.
- **CO-E-C owner transfer** may need bounded cancellation of preflight and bounded waiting for already-entered non-faulting K-S. If any old kernel commit can block indefinitely, the design must fail closed/quarantine or reject an availability guarantee, not silently allow two owner paths.
- **B response** should be separable into “deny barrier established” and “R1/R2-F/R2-O actually achieved”; waiting for DMA while holding cap-table/queue locks is disallowed. Grace periods or kernel reference counts alone are lifetime tools, not authorization-order proofs [S4].
- **SMP contention:** multiple grants in one MOVE vector plus endpoint owner/lineage mutations require an atomic abstract ordering. Acquiring many locks in arbitrary user-provided order could deadlock; a future design must compare canonical ordering, transactional validation, bounded retries or epoch/version approaches. **No algorithm is selected**.
- **Resource exhaustion:** q admission record Q and queue/lineage/ref credits must be reserved ahead of K-S, before any irreversible MOVE. If reserves fail, no K-S. M0 recovery retains independent bounded CPU/IPC/memory/cleanup.
- **Latency under hostile Thread:** A(q) may be delayed on Pager even before commit; with CO-S it confers no owner pin. CO-E-C should fence/abort rather than wait on a faulted user buffer while retaining global locks.
- **Historical receipts:** ACK/GC releases evidence only under declared retention; source/queued grant and revocation fence liabilities remain separately charged until actually settled.

No actual latency, lock contention, throughput, memory use or availability has been measured.

## 10. Candidate decisions CO-D01…CO-D12 — ALL Proposed

- **CO-D01:** Research **CO-S strict B-before-K-S deny** as first E1 SEND profile; A(q) is never a perpetual authorization lease.
- **CO-D02:** Research **CO-E-C cancel/settle-before-E-M** first for one-owner Endpoint transfer; alternatives CO-E-A/R remain open.
- **CO-D03:** Preserve distinct A, K-S, B, E-M, receipt report and later R1/R2-F/R2-O cuts.
- **CO-D04:** Clarify `NOT_COMMITTED` retains original source entries **from this SEND** only; independent close, MOVE or revoke may alter usability/possession.
- **CO-D05:** Treat queued Endpoint as ownerless-to-Tasks InTransit until actual K-I, without equating a kernel ref with second owner.
- **CO-D06:** Serialize complete source Grant vector ownership against other SEND, Close, Revoke and Endpoint MOVE, never a half-send.
- **CO-D07:** Commit-time checks derive from actual Task/Grant/Endpoint objects and generations, not user Handle integers alone or frozen caller strings.
- **CO-D08:** Charge all receipt/queue/lineage and committed grant resources independently; no Pager calls in protected commit section.
- **CO-D09:** B controls future selected-lineage authorization; prior physical CPU/DMA side effects require independent R1/R2-F/O evidence.
- **CO-D10:** Receiver HPA-B+ publication and unknown resource/SABI effect are a separate unsolved proof area; don't claim K-S settles them.
- **CO-D11:** Model and falsify CO-S / CO-E-C first on paper before any executable formal spec, and document counterexample expectations.
- **CO-D12:** Keep all proposed changes design-only, with no accepted ADR, stable KABI, kernel implementation or proof result.

## 11. Blocking questions CO-O01…CO-O16 — all OPEN

| ID | Priority | Exact question | Owner |
|---|---|---|---|
| CO-O01 | P0 | Can K-S + B + E-M + multi-Grant source MOVE refine one protected serializable order under SMP? | 002/003 |
| CO-O02 | P0 | Which minimal non-faultable commit critical section and memory ordering is sufficient without global Pager deadlock? | 001/003/004 |
| CO-O03 | P0 | Can CO-E-C cancel every not-yet-K-S call before E-M in bounded time including Task death? | 003/005/006 |
| CO-O04 | P0 | Is E-M defined as public source removal at K-S, arrival K-I, or two-phase source→InTransit→receiver ownership? | 002/003/009 |
| CO-O05 | P0 | Does a bounded old invocation ref remain valid *only* for memory/lifetime, with no unauthorized post-B SEND? | 002/003 |
| CO-O06 | P0 | Specify grant/Endpoint scope, independent roots, revocation barrier epoch and authorization snapshot binding | 002/003/018 |
| CO-O07 | P0 | Exact destination queue charge and multi-source vector lock ordering; zero partial sends under failure | 002/003/005 |
| CO-O08 | P0 | Define loss of SEND result when B/close independently affects original source Handle usability | 002/009 |
| CO-O09 | P0 | Can PENDING→FINAL_NOT_COMMITTED be proved against asynchronous cancellation and late K-S worker? | 003/009 |
| CO-O10 | P0 | Specify how queued revoked grants settle at K-I/terminal after B without reconstructing sender grants | 002/003 |
| CO-O11 | P0 | Prove Task death/Endpoint InTransit/queue cycle/lineage charges don't starve M0 | 002/006 |
| CO-O12 | P0 | ReceiveClaim/V/K-I/Handle publication cross-cut still open in separate FM-O07/08 review | 003/004/009 |
| CO-O13 | P1 | Evaluate CO-E-C versus CO-E-R latency/availability under bounded preflight/commit | 003/005 |
| CO-O14 | P1 | Compare batch revoke impact, independent root fairness and per-Task sponsor budget | 002/005 |
| CO-O15 | P1 | Specify future scoped status and guest errno mappings without pretending revoked source still usable | 009/010/018 |
| CO-O16 | P1 | Complete independent consistency audit of sender-source retention language across RFC-002/003/009 | cross-RFC |

## 12. Next formal-model-preparation handshake — still paper-only

**Suggested finite research configuration:** one sender Task with two Threads, one E1 Endpoint and peer, two distinct source CapGrants, one scoped revoker, one competing source Close/MOVE, one KR-M2 q with pre-reserved receipt, one Endpoint payload MOVE to recipient Task, one Task death and one queue record. Enumerate K-S<B, B<K-S, K-S<E-M, E-M<K-S, Close<K-S and lost-return permutations.

**Required future proof witnesses:**
1. B-before-K-S returns true FINAL_NOT_COMMITTED without pretending revoked grant remains usable.
2. K-S-before-B remains historical COMMITTED_KS with exactly one queue custodian and no sender restoration.
3. Two competing SENDs with same source vector cannot both commit; source duplication, close and endpoint handoff races cannot produce split vectors.
4. Endpoint owner transition is unique; ownerless InTransit is modeled and old/new K-S FIFO explicitly ordered.
5. Result copyout fault and retained q do not alter kernel ownership or grant fresh authority.
6. Mutating an earlier preflight snapshot or faulting usercopy cannot bypass current effective owner or B.
7. Protected M0 credits and retained queued lineage survive Task death and asynchronous revocation cleanup.

**Gate CO-R1 = OPEN / NO-GO.** Neither CO-S nor CO-E-C can become an Accepted contract until source-identity/linearization/liveness proof, realistic memory/lock budget and cross-RFC receive/revoke evidence exist. This paper does **not run** a checker, exercise hardware or authorize actual code.

**Priority after this review:** a second paper-only **Receive V / K-I / public Handle publication semantic-cut study** for FM-O07/08, then a combined no-code trace-to-RFC consistency register before any separately authorized executable model.

## 13. Official reference URLs and limits

- **[S1] Fuchsia Channel object — one owner, in-flight MOVE order, queued Handle references:** https://fuchsia.dev/fuchsia-src/reference/kernel_objects/channel
- **[S2] Fuchsia zx_channel_write_etc — MOVE/DUPLICATE transfer and errors:** https://fuchsia.dev/reference/syscalls/channel_write_etc
- **[S3] seL4 official CNode Revoke API:** https://docs.sel4.systems/projects/sel4/api-doc.html
- **[S4] Linux kernel RCU requirements — readers and grace-period limits:** https://docs.kernel.org/RCU/Design/Requirements/Requirements.html
- **[S5] UniOS primary RFCs and model papers:** links above (Proposed specifications).

**Method and evidence boundary:** inspected current UniOS Git main design contracts, official Fuchsia/seL4/Linux primary manuals, and paper-reasoned conflicting operation cuts. References describe other systems; they provide analogies, *not* assurance that UniOS's proposed B, CO-S, CO-E-C or K-S have been implemented or formally verified.

---

**History:** 2026-10-25 Proposed v0.1 independent FM-O01/SC-O02 handoff. CO-S and CO-E-C are conditional research priorities; exact SEND source retention vs independent close/revoke clarified. **28 paper-only counterexamples, 26 unproved invariants, 12 candidate decisions, 16 open blockers.** No executable model, OS code or ABI approval.

## 14. Unified SEND–RECEIVE handoff — 2026-10-25, Proposed

A follow-on [CAPIPC Unified SEND–RECEIVE Semantic Contract v0.1](../contracts/capipc-unified-send-receive-semantic-contract-v0.1.md) joins CO-S/CO-E-C native sender commit order with receiver audience-bound K-C and potential disclosure Vm/Vp/Vt before all-or-none K-I. Actual K-S committed source→kernel queue custody is historical and cannot be undone if a queued grant is later B-revoked and the recipient K-I must deny whole-vector installation. The sender's qS and receiver's qR receipts are distinct and external effects require SABI/backend reconciliation. CO-S/CO-E-C remain research candidates, not accepted kernel algorithms, stable ABI or formally verified properties.

## 15. 2026-10-26 A/C/E joint cut and in-flight disclosure race — Proposed

[CAPIPC Joint A/C/E Semantic-Cut Resolution v0.1](capipc-joint-semantic-cut-resolution-a-c-e-v0.1.md) retains CO-S/CO-E-C as SEND paper candidates and asks whether their E1 owner handoff/issuer PolicyEpoch cuts also safely fence **preauthorized but fault-blocked receive payload/token publication**. A(qS) still isn't blanket post-B SEND authority; K-S is full Grant-vector MOVE or no-cut, source return truth unaffected by recipient failure. K-I-only rights checks cannot undo or prevent all in-flight usercopy disclosure after an ownership/audience cut. Bounded publisher epoch/drain versus restricted receive output is OPEN. No new Accepted ADR, implementation, checker or frozen ABI.

# UniOS CAPIPC — Joint A/C/E Semantic-Cut Resolution v0.1

> **Status:** Proposed, design-only proof-target narrowing — 2026-10-26 NZDT. **NOT Accepted, executable, implemented, checked, mechanically proved, tested, benchmarked, or ABI-frozen.**
> **Scope:** Consolidate draft **ADR-CAPIPC-A** (K-S/revoke/Endpoint ownership), **ADR-CAPIPC-C** (K-C/Vm/Vp/Vt/K-I/Handle publication) and **ADR-CAPIPC-E** (quota/Pager/M0 cleanup). This paper selects **candidate semantics for the next paper review only**, identifies unsafe alternatives and states evidence needed to decide actual feasibility. It does **not** establish runnable concurrency, performance bounds or physical/hardware correctness.
> **Upstream:** [P0 Conflict Resolution & ADR Readiness](../reviews/capipc-p0-conflict-resolution-adr-readiness-v0.1.md), [Unified SEND–RECEIVE Semantic Contract](../contracts/capipc-unified-send-receive-semantic-contract-v0.1.md), [CO-S/CO-E-C research](capipc-commit-revocation-ordering-v0.1.md), [Receive Exposure/K-I/HPA-B+ research](capipc-receive-exposure-ki-handle-publication-v0.1.md), [Formal Model Preparation](capipc-formal-model-preparation-v0.1.md).
> **RFC ownership:** [RFC-001 TCB](../rfcs/rfc-001-kernel-boundary-and-tcb.md), [RFC-002 Capability](../rfcs/rfc-002-object-capability-lifetime-delegation-revocation.md), [RFC-003 IPC](../rfcs/rfc-003-ipc-wire-protocol-and-synchronization.md), [RFC-004 Pager/OOM](../rfcs/rfc-004-virtual-memory-pager-cache-and-oom.md), [RFC-005 Scheduler](../rfcs/rfc-005-scheduling-preemption-and-cpu-budget.md), [RFC-006 M0 Recovery](../rfcs/rfc-006-boot-trust-chain-service-startup-and-recovery.md), [RFC-009 KABI](../rfcs/rfc-009-native-abi-and-system-call-contract.md), [RFC-018 SABI](../rfcs/rfc-018-versioned-idl-abi-evolution-and-compatibility-contracts.md); physical [RFC-007 DMA](../rfcs/rfc-007-user-space-drivers-dma-and-iommu.md) remains independent.
> **Governance:** Draft ADR-A/C/E remain **D0 / OPEN**, not Accepted D1/D2. Paper decisions in this file are not approval to create even an executable formal model, much less OS code.

## 0. Executive judgment: a tighter candidate, and a new hard publication barrier

**Research baseline A0 + C0 + E0 (conditional):**

- **A0 strict sender cut:** a caller TaskGeneration preidentifies sender qS and charges protected R_S(qS) at admission A(qS). It snapshots bytes and exact GrantIDs. K-S, scoped revoke B, source Close/other MOVE, E1 effective owner E-M and Task death have a coherent protected conflict order. A(qS) alone permits *no* stale post-B/owner-MOVE Commit. K-S atomically enqueues the full message/grant vector, retires sender tokens and publishes R_S=COMMITTED_KS; before-cut rejection can become final no-K-S after future commit is fenced.
- **C0 T1 inert-token-first receive candidate:** receiver TaskGeneration qR/receipt reservation before exclusive K-C; bounded message staging and all destination slots/tokens prepared first; **authorized metadata Vm**, then **payload possible exposure Vp** and its faultable copy, then **possibly exposed inert tokens Vt** and token copy, then a protected all-or-none K-I of the exact whole native Grant vector + queue retirement + R_R=COMMITTED_KI. This **specific Vp→Vt→K-I order is a study target, not an accepted ABI**. A zero-byte payload/no-cap message omits inapplicable cuts. Other orderings remain comparators.
- **E0 failure-safety and progress boundary:** every irreversible effect has required sponsor credit/receipt reserved before its cut; usercopy and Pager waits occur **outside any lock the Pager/M0 path may need**. Commit and revoke/exposure publication gates are short and non-faultable in abstract; failing to obtain quota denies *before* the corresponding native cut. Receipt garbage collection never releases still-live message/Claim/cap-lineage/DMA obligations. M0 protected CPU/IPC/frame/IRQ/stack/cleanup credits are separate.
- **Core unresolved problem (JC-P0):** a Vp/Vt exposure authorization checked before a faultable usercopy may become stale if Endpoint E-M or revoke B occurs **during** the publication. Revalidating at **K-I is too late to revoke already-disclosed bytes**. The future design must define a publication-in-flight fence/epoch and its interaction with E-M/B, or narrow the ABI so a bounded safe publication phase is provable. **Neither A0+C0+E0 nor HPA-B+ alone solves this.**

**Not a solved protocol.** Selecting a logical event order does not prove that a given lock-free or non-blocking kernel implementation can realize it. If the publication fence cannot be bounded without a Pager/M0 cycle, this **first candidate must be rejected or narrowed**, not approved by weakening authority/disclosure invariants.

## 1. Three trust/cut layers that must not be collapsed

| Layer | Strict owner/evidence | Cut that matters | No inference allowed |
|---|---|---|---|
| Receipt identity | exact TaskGeneration; KR-M2 q/C/Used and separate protected R[q] | A(q), X(q), GC of *final evidence* | admission ≠ permission, ≠ completion, ≠ remote effect |
| Native capability custody | exact GrantID/lineage/Task/Endpoint object and kernel queue/Claim custodian | K-S (source→queue), K-I (queue→receiver), B deny-new-use | whole MOVE ≠ byte-atomic usercopy, ≠ current invokability after later close/revoke |
| Information publication | issuer audience, receiver actual Task/Egen, Vm/Vp/Vt history | authorization/current audience then possible first store/map; independently terminal | no K-I ≠ no disclosure; completed copy ≠ only initiating Thread saw data |
| Resource and recovery | sponsor ledgers/lineage/physical pins and independent M0 | precharge, terminal custody, real R1/R2-F/R2-O | receipt ACK, Task death or CPU grace period ≠ DMA fenced |
| External service | service policy / backend ledger | SABI method and durable/external effect cut | native COMMITTED_KS/KI ≠ service exactly-once |

**Kernel line:** the kernel authenticates actual Task/Endpoint/Grant and issuer-configured recipient audience; it does not trust a raw ServiceID, SABI role label, guest PID/FD or untrusted receiver's self-reported method approval. **T1** native bearer rights become Task-wide at K-I. **T2** method-gated native rights require a genuinely separate verifier/issuer and K-U; do not inject that promise into ordinary C0 T1.

## 2. Candidate coherent transition ledger (natural language, NOT runnable pseudocode)

Each action names the *logical linearization point* whose feasibility must be demonstrated. No compound row silently treats user memory stores as one atomic cut.

| ID | Action and cut | Guard / pre-reservation | Required effect or truthful failure |
|---|---|---|---|
| **JC-T01** | sender immutable Prepare | bounded copied bytes/descriptor vector; snapshot actual current grant identity, not just user token integers | fault/reject has no native source MOVE; no long-held commit locks |
| **JC-T02** | sender A(qS) | authentic Task receipt scope; q available; protected Pending R_S and credit allocated as one atomic admission | consume q once, not capability authority; failure before admission leaves no new SEND |
| **JC-T03** | K-S full-vector Commit | current valid E1 SEND owner, exact all GrantIDs with TRANSFER/lineage, applicable B/E-M/Close ordered, queue+grant+R credits | full source→single kernel m+R_S committed atomically; or no source MOVE by this SEND |
| **JC-T04** | applicable B/Close/E-M | true authorized controller/current holder; lock/epoch conflict with T03 | whichever wins excludes the other's incompatible *new* cut; no rollback of prior K-S |
| **JC-T05** | sender final no-K-S settlement | future T03 conclusively impossible for this qS | R_S=FINAL_NOT_COMMITTED; original tokens not invalidated *by this SEND*; independent Close/B may have acted |
| **JC-T06** | receiver A(qR) | actual receiver TaskGeneration; preknown qR, separate protected Pending R_R and quota | consumed receiver q exactly once; not linked by numeric equality with sender qS |
| **JC-T07** | K-C exclusive Claim | queued actual m after K-S; authorized receiving E1 half, Task/Egen/issuer audience and sponsor | single claimant; queue still owns native vector, receiver has no transferred live caps |
| **JC-T08** | bounded destination Prepare | whole slot/vector capacity; public TokenIDs inert; clone immutable native grant snapshot, classify Vm projection | no sensitive publication until authorization; if rejected unexposed, safe claim settlement per policy |
| **JC-T09** | authorized Vm disclosure | current claim/audience/policy and classification of size/count/type projection | mark metadata possibly observed before returning secrets; nonsecret projection only if explicitly approved |
| **JC-T10** | Vp guarded first-payload publication | current claim/Task/Egen/audience, publication fence/epoch and a bounded writable destination; no forbidden Pager lock | record possibly-exposed **before** first store/map; fault after this does not erase Vp |
| **JC-T11** | Vt guarded inert TokenID publication | whole reserved Task-local nonrebind IDs; current publication policy/fence; remaining output pages may fault | record possibly-published ID before output; all pre-K-I TokenIDs **inert**, even if caller guesses them |
| **JC-T12** | K-I full-vector Commit | current claim/Task/Egen, every actual Grant valid and installable at cut, queue+slots+R_R/sponsor prepared; no active foreign publication | atomically retire m and install all N grants + R_R=COMMITTED_KI; if a mandatory grant revoked, **no** N−1 success |
| **JC-T13** | TerminalNoKI/possible exposure | T12 conclusively prevented; V flags and sponsor debt preserved | no cap install; no foreign same-m redelivery after relevant exposure; no sender source restoration |
| **JC-T14** | outcome/receipt Query or result lost | actual scoped TaskGeneration query right; final evidence may expire | history only, no cap remint; UNKNOWN or EXPIRED can't be converted into definitive no cut |
| **JC-T15** | Task death, stale Epoch/Endpoint owner | active claim/publisher and independent in-flight references checked | revoke ordinary Task rights; complete/stop publisher safely before foreign requeue; cleanup native custody and debt |
| **JC-T16** | protected E/M0 cleanup | all queued/claim/lineage/pin debts charged independently; fault wait graph bounded | iterative terminal/quarantine with honest fail-stop; do not refund DMA before physical R2-F |
| **JC-T17** | optional trusted K-U / backend effect | **separate** authorized T2 verifier or service effect issuer | not inferred from native K-I, no built-in app exactly-once |

**No hidden claim that T08 is a one-shot user-memory stability guarantee.** A concurrent malicious B Thread may remap/overwrite its own output or share pages; we can guarantee only the native grant/Task ownership and conservative exposure classification under explicit buffer/epoch assumptions.

## 3. A-cut: strict K-S versus revoke and Endpoint handoff

### 3.1 Conflict decision table (Proposed)

| Protected cut order | Historical native SEND result | Can later do what? |
|---|---|---|
| **K-S < B** relevant grant lineage | COMMITTED_KS | queued grant remains kernel-custodied; B denies new forbidden uses, may make later K-I deny entire vector |
| **B < K-S** | FINAL_NOT_COMMITTED *once future SEND fenced* | that SEND moved none; current source token could be present but revoked/unusable |
| **K-S < E-M** old E1 owner | COMMITTED_KS | old message stays committed; new owner acquisition later is independent |
| **E-M < K-S** | FINAL_NOT_COMMITTED *once future SEND fenced* | old owner cannot send under old Endpoint; no source restoration from prior independent actions |
| **Close/other MOVE(g) < K-S** | FINAL_NOT_COMMITTED after safe settlement | no stale raw Handle resolves to a different actual Grant |
| **K-S < Close/other MOVE(g)** | COMMITTED_KS | later source action sees invalid source; no double MOVE |
| **K-S < Task death** | COMMITTED_KS | queued m/lineage and backend possible effects outlive sender |
| **Task death < K-S** | safely cancel/no-K-S only if no old protected commit won | Task cannot be reincarnated by ServiceID to continue stale work |

**A(qS) is not listed as owner-cut victory.** It merely makes qS unavailable to replay. Even after earlier admitted qS, B/E-M/Close can win and yield truthful FINAL_NOT_COMMITTED; this is not KR-M2 SKIP, which only consumes never-admitted q.

### 3.2 Exact multi-Grant whole-vector requirement

For native vector [g1,g2,…gN], an actual K-S transition:
1. validates every **distinct** actual source GrantID, TRANSFER right and permitted rights attenuation; duplicate Transfer references to same source must be rejected or governed by a separately reviewed disposition;
2. orders the needed GrantLineage revocation and E1 owner epochs with all other conflicting send/close actions, not a stale user integer snapshot;
3. publishes **one** queue message m holding full vector + immutable bytes and removes **all** corresponding source table entries, plus protected R_S COMMITTED_KS, as a single abstract ownership cut.

If any precondition fails, this specific SEND moves **none**, even if another Thread closed or revoked an original source in the meantime. No post-K-S source restoration is permitted.

**Open feasibility tradeoff:** one global serialization lock across all Grant lineages might make the model simple but introduce unacceptable contention and Pager recursion. Ordered fine-grained locks, epoch validation or limited transactional retry are comparators; **none chosen**. A future proof must show which actual kernel synchronization makes T03 indivisible on SMP without faultable work or unbounded retries.

### 3.3 Two Endpoint operations must be distinguished

- **E1 owner handoff of the SEND Endpoint itself:** CO-E-C **candidate** — after precommit old invocations are canceled or committed, an effective owner handoff occurs; no old-owner new K-S cut after it. Pending faultable input preflight does not pin owner forever.
- **A ChannelEndpoint carried as an ordinary transferred cap in m:** its old Task owner is removed at original K-S and it remains **Task-ownerless InTransit** until some authorized K-I assigns new public owner. If receipt/task failures terminalize it, do not invent an owner restoration.

Both are required. “one effective public owner” allows *zero* public owners during kernel transit. Any queued Endpoint/cap dependency cycles require E0 safe cleanup rather than recursive destruction.

## 4. C-cut: a narrower HPA-B+ token-first candidate (still not approved)

### 4.1 One proposed explicit order for T1 with nonempty payload+grant vector

**C0 strict order for paper counterexamples only:**

1. A(qR), protected Pending R_R and charge precede any receive irreversible output.
2. K-C binds exact m to live TaskGeneration/Egen/issuer audience; grant custody remains kernel queue/claim.
3. Reserve *entire* grant slots, immutable staged payload, TokenIDs and budget; TokenIDs are **inert and nonrebind**.
4. Authorize/classify Vm; publish only explicitly authorized metadata if requested; mark Vm possible before observable projection.
5. **Vp before any payload store/map**, then attempt bounded payload copy without holding Pager/IPC global locks.
6. **Vt before any public token-number store**, then attempt inert public TokenID array copy in a similarly nonfaulting-lock-free path.
7. Revalidate exact Task/Egen/audience/Grant lineage B at protected K-I; install **all** currently permitted grants and retire exactly m+R_R COMMITTED_KI, or no K-I and terminal/same-audience safe disposition.
8. Result may be lost; protected Query reports historic actual cut or honest Expired/Unknown, **never reissues grants**.

This precise **Vp→Vt→K-I** order narrows the formal state space but is not magically preferable for all workloads. Metadata might be public or omitted; no-grant receive omits Vt/K-I cap-vector install details as explicitly profile-defined. HPA-C self-AcceptCommit adds latency/held claims and is not a separate trust boundary. A T2 method trust requirement demands an independently authorized verifier/issuer and distinct K-U.

**Important observation:** installing native TokenIDs at K-I makes them Task-wide usable **before the initiating receiver Thread necessarily knows it succeeded**. This is fine for actual T1 bearer rights, not for “method validation before any native use.” User arrays can remain partial even when K-I commits; the kernel status must not claim “every byte/token observed.”

### 4.2 The publication-rights race — a NEW explicit D1 blocker

Example:

~~~text
R Task B holds exclusive claim m and is authorized at Vp.
B1 begins faultable payload copy; B1 blocks on user Pager after part of output.
Controller revokes issuer audience or E1 receive Endpoint MOVEs B→C.
B1 resumes with stale preflight and writes more confidential payload bytes.
~~~

**K-I revalidation alone cannot correct the information already written after the owner/policy cut.** Three comparators:

| Comparator | Authorization and publication behavior | Danger |
|---|---|---|
| **P-A publication window/fence** (first theoretical candidate) | current audience/epoch authorizes a **bounded in-flight chunk**; barrier B/E-M prevents *new* chunks and accounts for permitted previously-started chunks before claiming disclosure quiescence | bounded chunk/memory stability, active publisher reference and barrier completion must be defined; Pager fault cannot hold active global lock |
| P-B lifetime-long exposure permit from original K-C | even after E-M/PolicyEpoch change, old B can finish usercopy | **unsafe** if owner/revoke semantics promise immediate stop for new publication; a confidentiality exception would require explicit issuer agreement |
| P-C abort old publisher at B/E-M without synchronization | return error after stale pointer validation | **unsafe** if faulted work resumes after barrier and writes bytes; cancellation requires a demonstrable fence against already-running stores |
| P-D restrict T1 receive outputs to bounded register-only or validated fixed resident buffers | smaller or no pageable disclosure window | limits general desktop IPC message/cap payload scope; still needs owner/Task epoch and SMP cut proof |

**Research preference:** investigate P-A *only as a proof obligation*, together with P-D fallback. The candidate must state exactly when a chunk is considered “authorized before barrier,” how the revoker/handoff reports **barrier established** versus **all in-flight byte publication drained**, how Task death terminates pending writes, and how to avoid indefinite Pager waiting. If the latter is infeasible with bounded budget, choose P-D or another explicitly restricted receive profile rather than falsely promise revocation of already published secrets.

**Scope discipline:** Capability lineage B is a deny-*new native use* barrier, not automatic erasure of already read plaintext. We must say whether confidentiality publication is issuer-audience PolicyEpoch governed rather than incorrectly assuming Capability B revokes knowledge. Any independent issuer E-M/audience epoch should have a separate marker; B and PolicyEpoch may coincide in a policy path but are not universally identical.

### 4.3 Revoked mandatory grant between K-S and K-I

A committed m contains [g1,g2]. B(g2) publishes after K-S but before K-I. In C0 first research variant, K-I cannot install g1 while reporting success for [g1,g2]. It must deny **whole K-I** if the grant would be newly invokable contrary to current B, then terminal/no-K-I with truthful Vm/Vp/Vt history and complete queued g1/g2 custodial cleanup. The original sender R_S stays COMMITTED_KS. A “revoked placeholder token” alternative alters ABI semantics and is **not assumed**.

If B happens **after actual K-I**, the historic receiver R_R stays COMMITTED_KI but subsequent use may be denied; hardware/memory R1/R2-F/O is separate.

### 4.4 Truthful receive outcomes under C0

| Actual history | Native status | Information status | Retry / cap consequences |
|---|---|---|---|
| valid K-C no V, safely canceled | no K-I | verified no secret V | **potentially** requeue after independent policy check |
| Vm public count only, no secret | no K-I | approved public metadata only | audience/requeue policy explicit, not inferred from count |
| Vp then usercopy fault | TERMINAL_NO_KI if conclusively fenced | payload MAY_HAVE_BEEN_OBSERVED | no ordinary foreign requeue; source grant not restored |
| Vp+Vt, revoke wins before K-I | TERMINAL_NO_KI after fenced no future K-I | payload and TokenIDs potentially exposed | token IDs retired, no N−1 delivery |
| Vp+Vt then K-I, status lost | COMMITTED_KI actual, observer UNKNOWN | possible payload/token observation | B Task grants may be used/closed by another Thread; Query never remints |
| K-I then Task death | historic COMMITTED_KI | existing exposure persists | authority cleanup per Task, backend effect not undone |
| R[q] finalized then GC | Expired evidence on later historical query | disclosure custody retained if still relevant | old q replay-fenced; no remint or clean external retry |

No status layout, ABI errno, copyout strategy or receipt retention TTL is selected.

## 5. E-cut: finite resource and wait-for safety contract

### 5.1 Precharge and conservation — symbolic, not sizes

Let the following **distinct** available budgets be independently accounted per admissible principal/Task/Sponsor/DeviceGroup:

| Symbol | Reservation/liability | Earliest cut that must have full credit |
|---|---|---|
| Qs / Qr | protected sender and receiver R Pending records | sender/receiver A(q) respectively |
| Bq / Bg | immutable queued bytes, whole native Grant vector refs | before K-S |
| Cl | exclusive Claim and message/Endpoint generation refs | before K-C |
| Hn | entire receive cap-table slot vector plus possibly burned public TokenIDs | before any Vt; complete slots before K-I |
| Ev | conservative metadata/payload/token exposure ledger and audience epoch | before first relevant V |
| Wf | bounded Pager FaultWaiter/mapping/Task-generation references | before allowing fault path with outstanding claim |
| Lg | lineage/revocation anchor and sponsor/cycle cleanup debt | before moving/transferring references |
| Df | physical DMA pin/IOVA/device fence/quarantine liability | before relevant device use; cannot be freed on IPC receipt alone |
| M0 | wired emergency CPU/IRQ/IPC/frame/stack/cleanup reserve | **never** ordinary borrowable; survive local Task/service outages |

These are **logical ledgers**, not additive byte counts: one Grant may hold several independent references, and \`Hn\` TokenID exhaustion is a lifetime/identity resource, not merely temporary memory. A finite Qs/Qr by itself does not bound Claim, queued messages, tombstones, metadata exposure records, Endpoint-cycle cleanup or DMA.

**General admissibility condition (not numerical):** every proposed irreversible cut has authenticated current authority, no incompatible pending cut, and full protected accounting for its worst-case associated **mandatory** successor liabilities. If the required liability has no reserved safe terminal home, reject *before* that cut. Neither “just GC the receipt” nor a caller-supplied deadline creates new M0 memory.

### 5.2 Allowed waiting and forbidden lock graph

| Phase | May fault / wait on Pager? | Hold global IPC/cap/VM/revoke/cleanup lock while waiting? |
|---|---|---|
| preflight copyin to bounded kernel snapshot | yes | **NO** |
| protected A/R reservation and q fence | not in actual atomic cut | **NO** |
| K-S whole Grant ownership commit | **NO** | bounded nonfaulting synchronization only; no unbounded retry inside protected section |
| K-C/claim or publication-epoch authorization | not in protected cut | **NO** |
| payload/token usercopy after Vm/Vp/Vt | yes, with conservative exposure history | **NO** |
| K-I whole Grant install+queue retire | **NO** | bounded nonfaulting synchronization only |
| Pager supply/revoke/mapping cutoff | depends on Pager path | must use independent M0-admitted dependencies as RFC-004/005/006 require |
| terminal cleanup / DeviceGroup quarantine | may defer work | don't recursively block while holding global hot-path locks; separate actual hardware fences |

**Counterexample wait cycle:**
IPC usercopy holds lock L→Pager fault→Pager Task requires Channel Send/Receive acquiring L→fault waits forever. A higher CPU priority alone doesn't break the cycle. Similarly a queue Claim that consumes the only IPC credit needed by its own verifier/recovery path is an admission dependency deadlock.

**Required E0 proof artifact before D1:** a wait-for DAG whose nodes include Pager, M0, Claim, sender, receiver, sponsor, device cleanup and CPU/IRQ budget; each potential wait edge names *what lock/resource is held* and *what independently reserved resource permits progress*. The startup DAG is not the runtime wait-for graph.

### 5.3 Cancellation, terminal handling and physical debt

- After preflight/q admission but before K-S, a revoke/owner/death winner can final-abort only when no later K-S worker can commit. This requires bounded synchronization/cancellation evidence; no false \`FINAL_NOT_COMMITTED\`.
- After first relevant V but before K-I, failure may require **terminalizing m** rather than foreign redelivery. At terminal, queued grants close or quarantine under actual lineage; token IDs possibly published remain nonrebind; disclosure/audience history survives as long as needed to enforce no-requeue.
- A committed K-S remains historical after Task death, revoked in-transit member or terminal receive. Kernel refs and sponsor obligations cannot be auto-refunded merely due to R GC or Task lifecycle.
- Destroying recursively nested Endpoint/Message cycles cannot rely on unbounded destructor stack or endless synchronous reclaim. A bounded iterative work queue/quarantine with declared fail-stop when exhausted is a research candidate, **not shown feasible**.
- Physical DMA quiescence is not established by K-I, GC, object refcount, \`synchronize_rcu()\`, Task death or B alone. Scoped actual R2-F fence evidence is required before physical pin/frame reuse; independent roots require R2-O analysis.

## 6. Synthesis — three cuts and one stronger shared condition

| Interleaving | A-cut obligation | C-cut obligation | E-cut obligation |
|---|---|---|---|
| B before sender K-S | no new unauthorized SEND/Grant MOVE | no queue m for that SEND | pending receipt safe final settlement without magic rollback |
| Sender K-S before B, B before receiver K-I | K-S remains COMMITTED | all-or-none no-K-I/terminal for newly revoked mandatory member | retain whole queued lineage and exposure/sponsor until cleanup |
| B/E-M during faultable payload publication | prior K-S remains truth | publication fence determines permitted in-flight writes; no old Claim migration | no wait on locked Pager/M0 path, bounded drain or fail-stop |
| Token array partially copied | sender original Grant still in queue/Claim | TokenIDs inert until K-I and nonrebind if maybe exposed | charge burn/Claim/terminal credits, finite ID exhaustion |
| K-I commits before caller sees result | no sender resurrection | exactly full Grant install, Task-wide T1 rights, R_R historical | result-loss Query doesn't remint or release external effect debts |
| Task dies with claimed m and DMA pin | queued/received custody not magically returned to sender | preserve possible V and exact KI/no-KI result if settled | iterative cleanup/quarantine, hardware R2-F not inferred |
| W/Q exhausted and q hole remains | no fake A/skip/commit | refuse new cuts or truthfully terminal | separate M0 reserve; admit availability loss, not unsafe progress |

**New proposed compound invariant JI:** “For every live m and each mandatory transferred Grant, all native ownership cuts K-S/K-I, visibility cuts Vm/Vp/Vt, applicable revocation/owner epochs and mandatory per-cut credits have one auditable history such that no unauthorized *new* native grant is activated, no foreign same-m disclosure is induced after relevant V, and no uncharged live custody is freed.” **JI is a proof goal, NOT shown to hold.** It cannot guarantee plaintext erasure or external service exactly-once.

## 7. Adversarial paper schedules (JC-X01…JC-X24) — NONE EXECUTED

| ID | Interleaving | Unsafe countermodel / boundary |
|---|---|---|
| JC-X01 | sender A(qS) then B(g) then K-S | A mistaken for post-B grant |
| JC-X02 | K-S full [g1,g2] then B(g2) then K-I | partial N−1 install falsely full |
| JC-X03 | competing K-S of same source g across Threads | duplicate custody |
| JC-X04 | source Close(g) between snapshot and K-S | stale public token/GrantID confusion |
| JC-X05 | E-M owner wins while old preflight Pager-blocked | old owner sends afterward |
| JC-X06 | Endpoint e moved as payload and recipient not yet installed | two effective owners / unauthorized SEND in transit |
| JC-X07 | sender result copyout fault after K-S | source restored or R_S false no-cut |
| JC-X08 | qS C-frontier passes still Pending R_S | improper GC and late commit |
| JC-X09 | receiver qR equals sender numeric qS, distinct Tasks | receipt alias / cap authority confusion |
| JC-X10 | K-C before R_R reserved or qR known | lost Claim bootstrap after V |
| JC-X11 | confidential metadata count queried by nonrecipient | issuer audience oracle |
| JC-X12 | Vp mark then first payload copy faults | possible V but false NO_DISCLOSURE |
| JC-X13 | Vp prefix observed then Pager stalls, E-M B→C | post-handoff old B writes secret without publication cut ordering |
| JC-X14 | issuer PolicyEpoch changes while Vp copy suspended | stale preflight authorizes resumed sensitive copy |
| JC-X15 | B lineage deny occurs during Vt TokenID usercopy | later K-I bypasses new revoke policy |
| JC-X16 | Vt TokenID first entry maybe visible then second faults | TokenID rebound to unrelated grant |
| JC-X17 | Vp Vt complete, K-I wins, caller's return lost | remint after Query/retry |
| JC-X18 | K-I vs Task death vs Claim terminal race | cap vector partly installed or queue erroneously retired |
| JC-X19 | K-I then another B Thread closes Handle | historical receipt falsely promises current usability |
| JC-X20 | invalid user buffer preflight while holding cap global lock | Pager/IPC deadlock |
| JC-X21 | repeated failed Vt burns Task TokenIDs | unbounded token budget/M0 reserve exhaustion |
| JC-X22 | Terminal after V, receiver B2 same ServiceID reclaims m | foreign disclosure despite prior V |
| JC-X23 | Task dies with Endpoint message cycle | unbounded recursive kernel destruction |
| JC-X24 | native receipt GC, DMA still active, frame reused | premature R2-F/object free |

## 8. Required invariants (JC-I01…JC-I24) — ALL UNPROVED

| ID | Candidate property |
|---|---|
| JC-I01 | A(q) is identity admission with protected Pending receipt, never immutable permission to commit later. |
| JC-I02 | K-S and relevant B/E-M/Close/TaskDeath order as one coherent conflicting-cut state. |
| JC-I03 | Full K-S vector+queue+source removal+R_S status is all-or-none, never double Grant custody. |
| JC-I04 | Precommit failed SEND preserves sources **from itself only**, independent close/revoke stays effective. |
| JC-I05 | Only one effective E1 Task owner (or zero while InTransit), FIFO by actual K-S. |
| JC-I06 | Sender qS, receiver qR and kernel message m are distinct generation-authenticated identities. |
| JC-I07 | Receiver R_R and capacity exist before exclusive K-C and any potentially irreversible V/K-I. |
| JC-I08 | K-C is bound to actual live Task/Egen/issuer audience, queue retains custody until full K-I. |
| JC-I09 | Confidential Vm projection is authorized; Vm/Vp/Vt flags become monotone before first possible store/map. |
| JC-I10 | Audience/Endpoint owner/PolicyEpoch cut cannot authorize fresh stale postbarrier Vp/Vt stores. |
| JC-I11 | Preauthorized in-flight publication either drains under an explicit bounded rule or prevents a successful handoff/quiescence claim. |
| JC-I12 | Partial/possibly published TokenID is inert before K-I and never rebinds to unrelated Grant in same TaskGeneration. |
| JC-I13 | K-I installs exactly N legitimate GrantIDs and retires exactly m and R_R as one coherent cut, or installs none. |
| JC-I14 | A queued mandatory Grant revoked before K-I cannot silently become active or yield reported N−1 full success. |
| JC-I15 | K-I/Terminal/TaskDeath/queue Close cannot all win; exactly one valid custody disposition. |
| JC-I16 | After relevant V, same m isn't normally delivered to unrelated Task/ServiceIncarnation. |
| JC-I17 | Terminal NO_KI can coexist with POSSIBLE_DISCLOSURE and nonempty independent cleanup debt. |
| JC-I18 | Protected Query/GC never remints a CapGrant or admits old q into fresh native work. |
| JC-I19 | Committed R_R history never promises currently live token or calling-Thread-exclusive bearer rights. |
| JC-I20 | Usercopy and Pager waits never hold global locks/credits required by Pager/M0 recovery. |
| JC-I21 | Every irreversible cut has precharged mandatory worst-case liabilities including tombstones and safe terminal disposition. |
| JC-I22 | Ordinary Task/Q/W/Claim/Token budgets cannot consume separately reserved M0 capacity. |
| JC-I23 | Close/Task death/receipt ACK/GC don't treat physical DMA obligations as already fenced; R2-F distinct. |
| JC-I24 | Native receipt status never proves or denies SABI backend effect/storage durability without independent evidence. |

**These predicates deliberately include a difficult conditional-liveness side:** without a bounded publication lease/drain, E-M can remain pending forever while a pager-faulted publisher holds an in-flight privilege. Conversely, letting E-M commit and merely trusting an unfenced stalled copy breaks the owner/audience promise. Neither is an acceptable unqualified “always progresses” claim.

## 9. Alternatives that should be rejected or held conditionally

| Option | Current paper status | Reason / needed evidence |
|---|---|---|
| **A0 CO-S + CO-E-C** | first **research** candidate | strict authority/owner order, but SMP multi-vector atomicity and bounded cancel open |
| A-lease: all A(q) authorize later K-S after B | **reject blanket** | admission q isn't a current grant; missing revocation boundary |
| **C0 HPA-B+ Vp→Vt→K-I** | first **paper-model candidate only** | inert tokens/whole K-I, but publication epoch versus pager stall is unproved |
| C-install-first K-I→V | comparator | installed-but-unreported orphan live grants and caller result loss |
| C-HPA-C user Stage→self AcceptCommit | conditional comparator | Task self-consent doesn't add security principal, more held queue/credits |
| C-register-only bounded IPC | **fallback research** | shrink pageable exposure; still cost and feature scope research |
| **E0 precharged + M0 segregated + iterative cleanup** | required proof target | exact worst-case charges, lock graph, cycle/destruction bounds and hardware fences open |
| E-unbounded global lock around usercopy | **reject** | may fault/Pager deadlock and starve M0 |
| E-refcount/RCU grace period as revocation guarantee | **reject as sufficient alone** | memory reclamation lifetime ≠ current authorization barrier or physical DMA fence |
| T2 self-report verified | **reject security claim** | untrusted B has no separate issuer authorization; needs real T2/K-U |
| External exactly-once from native K-S/K-I | **reject** | independent service effect and media evidence required |

## 10. D1 semantic readiness and future proof budget

**Draft A/C/E are still D0 / OPEN.** For any of them to be even **D1 semantic-review-ready**, demand the following **paper artifacts** — not executed checker results:

- **A-D1:** precise Grant/Endpoint conflict graph, revoke-scope/epoch, multi-Grant commit winner order and bounded old invocation settle semantics; known failure path never consumes sources *by that failed SEND*.
- **C-D1:** one selected current-audience **publication-in-flight** protocol defining Vp/Vt versus E-M/PolicyEpoch B, explicit positive/negative result after partial bytes/token exposure, whole N-vector K-I at protected cut, Task-wide token invokability and no historical remint.
- **E-D1:** per-cut sponsor credit conservation matrix including Claim, R, queue, anti-ABA tombstones, timeout/terminal, Endpoint cycles and DeviceGroup pins, plus runtime Pager/M0 wait-for edges. Define honest fail-stop if a credit/fence cannot be supplied.
- **A+C+E joint D1:** no time-of-check-to-time-of-use gap where actual confidential user stores can cross a revocation/owner/publication barrier unauthorized; no unilateral “resolve via later K-I revalidation” shortcut.
- **Scope D1:** explicitly choose general T1 bearer promise versus optional independently trusted T2 method-before-use; different trust models cannot share an underspecified SUCCESS.
- **Review evidence:** source-to-claim mapping and counterexample expectations for each guard (e.g. deliberately remove authorization guard, verify on paper a violating schedule exists). No source, code or runtime checker is authorized.

**Separately authorized later D2:** bounded TLC/Apalache/Alloy or equivalent with reachable good paths and negative controls, after deciding the exact paper contract. **D3/D4** would require implementation-specific SMP memory-order/VM/refinement and physical hardware evidence. None achieved.

### Suggested next design-only milestone

**CAPIPC Publication Barrier & Pager-Safe Copyout Feasibility v0.1.** Study candidate P-A's bounded guarded user-memory publication and Endpoint/issuer epoch transitions **against** a narrowed register-only/fully bounded resident-output alternative. This is the decisive blocker shared by A/C/E; it must be settled in a Proposed paper before claiming any exact C0 micro-order is semantically viable. No executable model or OS implementation is implied.

## 11. Official external references and limits

- **[S1] Fuchsia Channel kernel object (whole-message Handle transfer, owner handoff, queue destructor caveat):** https://fuchsia.dev/fuchsia-src/reference/kernel_objects/channel
- **[S2] Fuchsia zx_channel_read / typed read_etc (API whole-message and byte/Handle output order):** https://fuchsia.dev/reference/syscalls/channel_read and https://fuchsia.dev/reference/syscalls/channel_read_etc
- **[S3] Linux kernel usercopy guidelines (usercopy may sleep/fault; don't hold spinlocks):** https://docs.kernel.org/kernel-hacking/hacking.html
- **[S4] Linux RCU reference (removal and reclamation distinct, reader grace period not authorization barrier):** https://docs.kernel.org/RCU/whatisRCU.html
- **[S5] seL4 IPC tutorial (different synchronous grant and receiver CSpace semantics):** https://docs.sel4.systems/Tutorials/ipc
- **[S6] UniOS current main-branch architecture/RFC and prior model papers**, linked at beginning.

**Evidence limit:** written candidate contracts, adversarial paper reasoning and official system manuals only. None of these third-party systems is asserted to implement UniOS's proposed publication barrier, q receipt ledger or Revocation semantics. No future correctness, feasibility or latency figure is yet measured.

---

**History:** 2026-10-26 v0.1 D0 joint A/C/E research narrowing. Proposed a specific paper-only T1 Vp→Vt→K-I ordering, a new explicit publication-in-flight barrier/Pager race and A0+C0+E0 failure truth. **17** candidate transitions, **24** unexecuted adversarial schedules, **24** unproved invariants. No accepted ADR, executable formal model, code or ABI.

## 12. 2026-10-26 Publication barrier and Pager-safe copyout feasibility follow-up — Proposed

[CAPIPC Publication Barrier & Pager-Safe Copyout Feasibility v0.1](capipc-publication-barrier-pager-safe-copyout-feasibility-v0.1.md) deepens the earlier **JC-P0** issue: P-DENY blocks new issuer/Endpoint audience publication permits while P-DRAIN additionally proves older publishers cannot write any more bytes. A postcopy epoch retry cannot unpublish earlier bytes. For general T1 HPA-B+ the new **conditional P1 paper comparator** requires pre-resolved bounded nonfaulting output slices, actual MappingGeneration and writable backing/alias authorization, serialized issuer Epoch/E-M and precharged terminal/M0 resources. **P0 register-only** is a narrower fallback, **P2 controlled mailbox** a separate VM comparator, and indefinitely Pager-faultable P3 cannot claim bounded strong drain. Vp→Vt→K-I remains a logical research order, not an established SMP-safe implementation or Accepted ADR.

## 13. 2026-10-26 Publication Permit & Destination Memory follow-up — Proposed

The further [CAPIPC Publication Permit & Destination Memory Contract v0.1](capipc-publication-permit-destination-memory-contract-v0.1.md) separates receiver-destination page lifetime, (TaskGeneration, AddressSpaceGeneration, MappingGeneration, VA/backing) binding correctness and issuer PolicyEpoch/P-DENY publication authorization. Proposed P1-MA pin-backed writes may reach a now-unmapped old frame rather than caller's *current VA*; P1-MB mapping lease/freeze needs an explicit invalidate-vs-publisher fence and scheduler liveness proof. P0-R bounded register returns reduce Pager surface but still require a ReturnRelease authorization cut. No underlying CO-S/HPA-B+ paper profile becomes an Accepted or demonstrated-realizable algorithm.

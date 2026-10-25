# UniOS — Unified SEND–RECEIVE Semantic Contract v0.1

> **Status:** Proposed Design-Only Joint Contract and Feasibility Research — 2026-10-25 NZDT. **NOT Accepted, implemented, formally proved, model-checked, benchmarked, hardware-validated or ABI-frozen.**
> **Purpose:** Reconcile separate SEND, RECEIVE, revocation, public Handle publication and kernel receipt design papers into one **candidate** end-to-end *native IPC* message-lifetime contract and an explicit P0 decision/traceability register. This is **not** application exactly-once delivery, service processing, storage durability or a guarantee the receiver observed all bytes.
> **No implementation authorization:** No source code, runnable TLA+/Alloy model, numeric KABI, stable protocol, syscalls, SDK, runtime tests, implementation, CI jobs or kernel modifications in this study.
> **Controlling research:** [Architecture v0.2](../architecture/unios-architecture-v0.2.md) (eight broad directions only); [CO Commit/Revocation/Endpoint Ordering v0.1](../models/capipc-commit-revocation-ordering-v0.1.md); [RP Receive V/K-I/Public Token Ordering v0.1](../models/capipc-receive-exposure-ki-handle-publication-v0.1.md); [KR-M2 Receipt Admission v0.1](../models/capipc-ordered-vs-sliding-window-receipt-admission-v0.1.md); [HPA-B+ Re-Review v0.2](../models/capipc-handle-publication-activation-v0.2.md); [Formal Model Preparation v0.1](../models/capipc-formal-model-preparation-v0.1.md); [Cross-RFC Safety Case v0.1](../reviews/capipc-cross-rfc-safety-case-v0.1.md); [CAPIPC–SABI Contract v0.2](capability-ipc-sabi-integration-contract-v0.2.md).
> **Owning RFCs:** [RFC-001 TCB](../rfcs/rfc-001-kernel-boundary-and-tcb.md), [RFC-002 Capability](../rfcs/rfc-002-object-capability-lifetime-delegation-revocation.md), [RFC-003 IPC](../rfcs/rfc-003-ipc-wire-protocol-and-synchronization.md), [RFC-004 VM/Pager](../rfcs/rfc-004-virtual-memory-pager-cache-and-oom.md), [RFC-005 Scheduling](../rfcs/rfc-005-scheduling-preemption-and-cpu-budget.md), [RFC-006 Boot/M0](../rfcs/rfc-006-boot-trust-chain-service-startup-and-recovery.md), [RFC-009 Native ABI](../rfcs/rfc-009-native-abi-and-system-call-contract.md), [RFC-018 SABI](../rfcs/rfc-018-versioned-idl-abi-evolution-and-compatibility-contracts.md). Boundary [RFC-007 DMA](../rfcs/rfc-007-user-space-drivers-dma-and-iommu.md), [RFC-008 Storage](../rfcs/rfc-008-storage-filesystems-and-crash-durability.md), [RFC-010 Linux GABI](../rfcs/rfc-010-linux-personality-and-binary-compatibility.md).

## 0. Executive verdict — alignment of contracts, NOT completed assurance

**Recommended for further proof research only:** E1 exclusive Channel Endpoint ownership and atomic K-S all-or-none sender-to-queue Grant MOVE, with **CO-S** (actual K-S ordered against scoped revoke barrier B) and **CO-E-C** (settle old precommit Endpoint calls before effective owner handoff) as current SEND research candidates. For ordinary **T1 bearer RECEIVE**, compare **HPA-B+** first: bounded exclusive Claim, prior Task-scoped receipt identity, conservative potentially observable metadata/payload/token markers, inactive reserved public IDs before K-I, and one atomic full-vector **K-I** install+message retirement+protected result. Optional HPA-C/T2 escrow applies only to an independently justified deferred/trusted activation profile.

**Important caveat:** Even together, these are **candidate semantic contracts**. The exact public token copyout versus K-I micro-order, Pager-safe linearization, resource bounds and multi-CPU memory-order proof are OPEN. Prior studies' `CO-S/CO-E-C/HPA-B+` ranking is not an Accepted ADR, a runnable state machine, or demonstrated feasible implementation.

**Five independently tracked realities:**
1. **Custody:** native GrantID is held by sender, one kernel queue/claim custodian, one recipient or terminal safe custodian — never copied by MOVE.
2. **Information:** Vm/Vp/Vt indicate *possible* metadata, payload and public token exposure; copying a prefix cannot be undone.
3. **Kernel operation identity:** sender Task qS and receiver Task qR are separate KR-M2 admission identities/receipts, even if numeric values match.
4. **Current native authority:** E1 owner/Grant lineage/revocation/TaskGeneration determine new operation rights; historical K-S/K-I does not preserve present invokability.
5. **External outcome:** SABI method admit, backend operation, media durability, hardware fence and client knowledge are independently established, if at all.

**New integrated finding 1 — queued revoked vector:** K-S can be genuinely committed before B, while one member of its N-Grant vector becomes unusable before K-I. **K-I cannot “succeed with N−1 rights” under the all-or-none vector contract.** It must settle as no full-vector K-I (potentially with earlier Vm/Vp/Vt exposure), or invoke a separately specified safe denial/terminal path that accounts **all N grants**. Neither original source restoration nor “delivered all usable grants” is truthful. This is a **P0 cross-RFC contract**, not a newly discovered running-system bug.

**New integrated finding 2 — no magic atomic user memory:** K-S and K-I can each be one protected kernel cut, but Vm/Vp/Vt output writes and the caller's subsequent observation do **not** join that cut into an all-or-none user-memory transaction. Final statuses must represent **native receipt phase × possible disclosure × current authority** separately.

**New integrated finding 3 — a claimed message remains a kernel-custodied message:** A Claim grants *exclusive receive/disclosure opportunity* to an authorized Task; it is **not** an early full-vector Capability ownership transfer. A terminal outcome after V can legitimately cause *message loss for availability* while preventing secondary-recipient information disclosure; custody/lineage/DMA charges remain independently until safe resolution.

## 1. Primary references: transfer principles, not imported semantics

| Source | Documented external fact | Exactly what it cannot prove for UniOS |
|---|---|---|
| [S1] Fuchsia Channel kernel object | Two Endpoints, ordered queued messages; channel Endpoint has exclusive owner and documented transfer ordering; native handle transfer uses whole-message cuts | UniOS's CO-S revoked-after-admission ordering, Task receipt identity or partially published user memory |
| [S2] Fuchsia channel_read/read_etc | Channel read is an entire logical message; bytes and Handle buffer have documented copy order, `read_etc` supplies kernel object type/rights | UniOS's output publication/visibility is byte-atomic, or a failed Fuchsia read actually leaked bytes |
| [S3] Fuchsia channel_write_etc | Source handles have explicit MOVE/DUPLICATE dispositions, different failure/ownership semantics from UniOS's proposed NOT_COMMITTED | Failed UniOS SEND consumes handles in the same way |
| [S4] seL4 IPC tutorial | Typed capability transfer is tied to receiver-side capability slot/path configuration | UniOS's queued E1 and optional escrow have seL4-like formal guarantees |
| [S5] Linux `copy_to_user` reference | Copy may fault or sleep; returned uncopied amount and incompatible lock contexts are documented | Native user output of multiple buffers can be treated as an atomic memory transaction |
| [S6] UniOS Proposed documents | Extensive written race matrices and logical cut contracts | Independently executed proof, correct source code or hardware availability |

**Evidence supplied here:** current GitHub main-branch document comparison and upstream technical docs. No experiments or copied third-party code.

## 2. Namespaces and who owns each identity

| Identifier | Authority/owner | Scope and never-equate-with |
|---|---|---|
| TaskGeneration S, R | kernel actual Task admission/termination | display TaskID, old Task reused PID, logical ServiceID |
| E1 Endpoint `E` and generation | kernel owner Task or kernel transit custodian | identical service name or a second Endpoint owner |
| MessageIdentity `m` | kernel at actual K-S queue publication | sender qS, receiver qR, SABI RequestId |
| GrantID `g_i`, GrantLineage | kernel typed capability + sponsor/revoke anchor | raw Handle integer, SABI role string |
| Native CapHandle `h` | current Task-local public nonrebind token | Linux guest FD/OFD, ReceiptSlot or permission from guessing |
| Sender (S,qS) | kernel protected KR-M2 receipt namespace | receiver (R,qR), service business request |
| Receiver (R,qR) | **separate** receiver Task receipt | source grant, message identity or transfer authorization |
| Claim (m,R,Egen,Audience) | kernel exclusively reserved receiver opportunity | currently-installed destination CapGrant |
| Vm/Vp/Vt history | kernel conservative possible observation | confirmed successful read of all message bytes |
| R[qS]/R[qR] | kernel protected historical phase evidence while retained | actual live cap availability, external durability |
| SABI request/effect ID | authorized service-specific state | qS, qR, E1 FIFO, kernel receipt |
| B / R1 / R2-F / R2-O | lineage-specific barrier / CPU / hardware / object lifetime | a single success bool or receipt ACK |

**Custody and status are separate:** An Endpoint or Grant embedded *in* a message is held by the queue/claim custodian while pending. It may have **zero effective public Task owners** in transit; this is not a violation of **at most one** effective owner. Message claim exclusivity doesn't imply recipient grant installation.

## 3. Unified message/cap custody graph (abstract, paper only)

```text
Source Task S: exact CapGrant vector [g1 ... gN]
     │  Input snapshot + pre-reserve + A(qS), protected R_S=PENDING
     │  [CO-S contest: current SEND/TRANSFER, E1 owner, B, close, M0 quota]
     │
     ├── Pre-K-S abort/final no-cut ──> no transfer by THIS SEND
     │    (independent revoke/close/MOVE may still change current source rights)
     │
     └── K-S atomic cut ──> Message m queued with whole [g1 ... gN]
                               R_S=COMMITTED_KS; S's moved public handles dead
                               │
                               ├── Queue abort / endpoint death ──> terminal close/quarantine
                               │    (no restoration to original S)
                               │
                               └── Receiver Task R authentic A(qR) / protected R_R=PENDING
                                      ├── K-C(m,R,Egen,Audience) exclusive
                                      ├── reserve whole slots + inert token IDs
                                      ├── Vm, Vp, Vt (independent possible publication)
                                      │    ├── copyout fault/revoke/Endpoint MOVE
                                      │    │    └── same-authorized-context recovery
                                      │    │        OR terminal/no foreign requeue after V
                                      │    │
                                      │    └── K-I coherent cap vector + dequeue + R_R
                                      │         └── R receives ALL g_i (or none)
                                      │              tokens may be live before caller knows
                                      └── NO_KI terminal with truthful exposure flags

Later: K-U only if independent T2 trusted verifier; service S-A/effect;
       B/R1/R2-F/R2-O, queued refs, sponsor and durability all independent.
```

**Source of truth:** K-S commits a single immutable queue message snapshot, not a promise of delivery or SABI parsing. K-C is an exclusive claim *on that queue message*. K-I transfers it to a Task exactly once. A terminal/cancel path destroys or quarantines transferred rights **without** recreating S's previously moved source entries.

### 3.1 Important adjacency constraints

- Native SEND/RECEIVE each must have a distinct preknown/authorized receipt q and protected Pending record before their own first irreversible cuts; KR-M2 `C/Used` means **identity consumed**, not message complete.
- **No K-C before K-S** for the same m. No K-I without actual K-C/Grant custody. No K-I twice. No K-S replay under same q after receipt GC.
- K-C is not an early ownership transfer: while claimed and not K-I'd, grant custodianship remains **kernel queue/claim**, not a second holder in R.
- Vm/Vp/Vt are *possible publication events*, not required to occur in every profile; exact allowed ordering and policy classification are still P0.
- No user-visible V of confidential content before current receiver authorization and audience eligibility. A receiving Endpoint owner change/revoke during preparation must be ordered with V and K-I, not only K-C.
- If K-I happened, later usercopy/report failures cannot undo installation or queue retirement; receiver may independently close/revoke/MOVE tokens.
- If V happened and K-I cannot safely happen, ordinary cross-recipient requeue of that **same m** is forbidden; terminal may sacrifice delivery but retains exposure truth and independent sponsor/effect debt.
- An unexposed aborted claim can potentially requeue to an eligible R2 **only after** proving all Vm/Vp/Vt projections were safe and no kernel effect grants leaked. “No full payload copied” is not sufficient.
- Duplicate source GrantID in MOVE vector must be explicitly rejected or have an independently defined legal disposition; **never double-move** the same GrantID. Direct E1 self-send Endpoint in its own payload remains excluded in the initial candidate.

## 4. Combined cut partial-order and conflict register

| Candidate cut | Must precede / must follow | Can race with | When it loses the race |
|---|---|---|---|
| Snapshot/queue/receipt reservation | before A/K-S and first irreversible publication | usercopy faults, Task death, close, B | fail safely before relevant cut; no stale rights inherited |
| Sender A(qS) | before K-S for qS | B, E1 owner MOVE, source Close, another A/X | consumed q can final no-K-S; A is not perpetual rights grant |
| Sender K-S | after sender A and precharged exact vector; **before m exists in queue** | applicable B, E1 owner E-M, source Close/MOVE, Task death | if competing barrier wins, whole no-cut settlement; original rights may independently change |
| Scoped revoke B | independently ordered against K-S and later K-I/K-U | source SEND, queue claimant, Pager/DMA | does not retroactively un-send m or erase V; affects later new rights |
| Endpoint E-M | before new owner's K-S / K-C under actual Egen | old K-S, receive Claim, Task death | CO-E-C settle old precommit before effective handoff; if claim exposed, no reassignment |
| Receiver A(qR), K-C | after m queued; pre-irreversible receiver V/K-I | another Claim, B, Endpoint E-M, Q exhaustion | no second Claim, no implicit cap install |
| K-Vm (metadata) | after issuer-approved projection/audience authorization | owner/PolicyEpoch, usercopy, Task death | possible metadata already observed may restrict future redelivery |
| K-Vp (payload) | after exclusive K-C/current audience; before first possible sensitive store/map | Pager fault, owner E-M, revoke, terminal | exposure monotone, no alternate Task's receive of same m by ordinary requeue |
| K-Vt (token number) | after pre-reserving inert IDs; before any token bytes possibly visible | fault/alias, K-I, other receiver Thread | maybe-published IDs must not rebind unrelated grant in live R |
| Receiver K-I | after K-C, prepared complete destination slots and prior protected R; actual authorization at cut | B, E-M, source queue close, Task death, competing K-I | none of vector installed; terminal/same-recipient path, earlier V remains |
| Optional trusted K-U | only after actual inert escrow and independent verifier release | B, PolicyEpoch change, new Task/Endpoint | refuse activation; no falsely durable service effect |
| Result Query / GC | after any A, with applicable rights; final GC only after exact native settlement | return-loss, close, Task death, independent resource cleanup | return Pending/Expired/Unknown, never re-execute old q/remint cap |

**One consistent order for each conflicting shared object is required**, not a universal total order of all events in the OS. A receive in another Task uses qR independent of qS, and two different Endpoint directions may make progress concurrently. Future formalization must show the actual shared Grant/message/Endpoint cuts compose without introducing a hidden lock/Pager cycle.

### 4.1 Revoke between K-S and K-I: whole-vector failure is not sender rollback

A sends m with `[g1, g2]`; K-S fully commits and invalidates S's exact handles. Before R's K-I, a controller publishes B for `g2`. Under the research-first “no new revoked-right install” candidate, K-I **cannot** simply install `g1` and silently drop `g2` while claiming all-cap success. It may **deny the entire K-I**, settle the message/claim and close or quarantine all queued grants under legitimate sponsor and revoke cleanup, or adopt another **separately reviewed atomic disposition**. If R already saw payload Vp, no normal redelivery of that m to C. Sender receipt remains COMMITTED_KS. Independent root grants are not revoked merely because g2 was. **Receipt phase, revoke phase and current native invokability must not be merged.**

**OPEN design fork:** Is it permitted to install a *revoked unusable placeholder* as a typed native Handle for diagnostic/revocation status without granting invokability? Doing so changes all-vector semantics and token/claim budget. **Not approved**; first proof should use a clean no-K-I terminal path rather than invent implicit partial success.

### 4.2 Endpoint-as-payload and in-transit owners

If m contains an E1 Endpoint e, original holder S loses its public e handle at K-S. Until K-I installs e in R, the Endpoint has **no public Task owner** and is kernel-owned InTransit. Other independently owned Endpoint halves may still have prior queued messages. Uncommitted old e SENDs must be canceled/settled before e's effective ownership removal under provisional CO-E-C; they must not later perform new K-S from old S. When R gets e, actual Egen/current rights govern new calls. A receiving Endpoint used for K-C may **itself** MOVE while m is claimed; after possible V, don't retarget disclosure or Claim to new owner automatically.

**OPEN cycle/resource requirement:** Endpoint messages may carry Endpoints/Grant lineage cycles and survive sender death; kernel queue custody accounting and bounded iterative cleanup cannot assume reference counts alone collect cycles. No source Grant restored simply because receiver never completes.

### 4.3 Copyout, alias and token publication remain separate from K-I

Even with complete bytes/cap snapshot and pre-reserved public Handle IDs, writing N IDs to arbitrary user memory may expose the first ID while the second faults or be concurrently observed by another same-Task Thread. HPA-B+ research requires IDs **inert until K-I**; after K-I Task-wide rights may become usable before the original Thread sees completion. Token numbers possibly visible cannot be rebound to unrelated grants during same TaskGeneration (HN-A nonreuse or HN-B generation candidate, no selected widths). `COMMITTED_KI` is historic cap-vector installation, not a promise that all public Handle outputs were read, remained valid or were fully delivered.

## 5. Unified terminal and error-outcome truth table

**Three independent output/knowledge dimensions:** (1) source/queue/receiver cap custody, (2) possible Vm/Vp/Vt information publication and (3) observer knowledge of native receipt. Backend effects/durable state are **fourth independent ledgers**.

| Case / actual cut history | Authoritative kernel custody | Exposure truth | Honest status/recovery |
|---|---|---|---|
| Failure before any sender A/K-S | unchanged **by this SEND** | none from SEND transport | definitive no new message; unrelated source close/revoke still possible |
| Sender A then applicable B/owner E-M before K-S | no m created by this SEND; sender entries not consumed by it | sender no receive disclosure | R_S FINAL_NOT_COMMITTED if safely settled; current source may be revoked/closed |
| Sender K-S, result lost | entire vector in kernel m, S originals removed | receiver not necessarily exposed | historic R_S COMMITTED_KS; caller UNKNOWN until query; no blind resend |
| Sender K-S then peer closes without K-C | m terminal destroyed/quarantined per queue policy | none under this receive claim | no sender rollback; queue/lineage safe cleanup independent |
| K-C acquired, buffers too small before sensitive V | m still kernel-owned, one claimant or released safely | only approved metadata | NO_KI, possible retriable unexposed receive after verified claim settlement |
| K-C then confidential Vm without full payload | m still kernel claimed | metadata maybe seen | restrict cross-audience redelivery per issuer policy, not “no exposure” |
| K-C/Vp then output fault before K-I | m kernel claim/terminal custodian, **not** R's new caps | payload maybe seen | TERMINAL_NO_KI + POSSIBLE_DISCLOSURE; same-R continuation only if proven |
| K-C/Vt partial token output, before K-I | kernel still owns vector | some public token IDs maybe seen; inert | NO_KI with token tombstones; no unrelated ID reuse; conservative terminal |
| K-I succeeds, reply lost | R owns full vector, m retired | payload/token may be visible or partly unseen | historic R_R COMMITTED_KI; observer UNKNOWN until query, never reinstall |
| K-I succeeds, R's another Thread closes g1 | R owned vector at K-I, g1 now closed | independent | COMMITTED_KI historical; no promise h1 remains invokable |
| K-S then relevant B before K-I | kernel still owns m or terminal disposition | Vm/Vp/Vt may have occurred | sender COMMITTED_KS, receiver DENIED_NO_KI/terminal, no partial install |
| K-C/Vp, R Task dies, new incarnation arrives | old claim ends; independent kernel queue/grant custodian until terminal | old R may have forwarded bytes | no foreign requeue; B2 receives no old claim/receipt by name |
| Native q result GC after final while lineage/DMA persists | custody/effects unchanged | exposure history needed for live m must persist | replay identity fenced; historic result may be EXPIRED/UNKNOWN |
| Backend effect after payload observation before K-I | new transferred caps still not installed | payload already seen | effect MAY_APPLY independently via preexisting B rights; no native “no effect” |
| Hardware DMA still active after Task close | independent pin/quarantine custodian | independent | B/R1/R2-F/R2-O evidence required; receipt/Task death not enough |

**`NO_KI` means no native *installation by that receive***, not “no information,” “no remote processing,” “sender grant restored” or “no grant remains in the kernel.” **`COMMITTED_KS` means sender-to-queue custody transfer, not receiver success.** Neither `UNKNOWN` nor `EXPIRED` may be translated to a definitive precommit safe-retry status. These are **logical taxonomy labels, not ABI numbers**.

## 6. Resource-owner conservation and protected M0

| Resource / debt | Protected custodian | Release condition |
|---|---|---|
| Sender KR-M2 qS / R_S pending | sender Task/charged kernel result | final cut determined; ACK/GC policy, consumed q fence persists while Task lives |
| Immutable source cap vector and send queue charge | sender precommit; kernel queue after K-S | transfer whole or fail-before-cut, terminal message closure or completed K-I |
| Cap lineage anchor / revoke marker | kernel object family and sponsor | descendants/effects and protected epochs safely retired |
| Message queue record m | kernel E1 queue | K-I retirement or terminal handling; Claim alone is not dequeue |
| Receive Claim and qR/R_R | kernel protected receiver Task/claim sponsor | final K-I/no-K-I state, exposure-safe custody and bounded GC |
| Receiver cap slots and public token values | Task cap table reservation/kernel ledger | installed grants transfer charge; possibly published tokens retired nonrebind until actual TaskGeneration end |
| Metadata/payload disclosure history | kernel claim/message terminal record while relevant | recipient / redelivery policy conclusively settled, not merely native receipt TTL |
| Pager FaultWaiter/usercopy pin | VM/Thread/Mapping/PagerGeneration | actual fault/cancel and safe reference termination |
| MemoryObject/DMA pins/IOVA/quarantine | actual DeviceGroup/physical sponsor | R2-F/hardware fence and separate object-wide R2-O |
| ReplyToken/SABI session/BackendEffect | authorized service/backend | service contract/durable reconciliation, not kernel K-S ACK |
| M0 protected CPU/IRQ/IPC/frame/cleanup | boot/recovery reserve | never ordinary Task borrowable; precharged finite maximum of essential recovery |

**Conservation target (abstract):** For each transferred GrantID and its descendants, no two independently effective custody owners exist after MOVE; queue/Claim are a single kernel custody domain, not two copies. Charges need conservation per underlying object/ref/sponsor as separately specified; avoid naive “one grant implies one unit of every charge” when mappings/pins/in-transit references have independent costs.

**No general normal-work availability promise:** finite W/Q/Handle token space, permanently missing q, stalled receive head, Pager dependency, DeviceGroup quarantine and essential M0 reserves can force explicit denial or terminal message loss. A refusal is safer than silently allowing duplicate grants or unauthorized cross-recipient disclosure. M0 cleanup may itself fail-stop when its actual protected bound is exhausted.

## 7. Cross-RFC integration matrix and contract friction

| Item | Source of truth | Aligned design intention | Still-open semantic friction |
|---|---|---|---|
| **UN-C01** E1 public owner | RFC-002/003; CO | <=1 effective owner, queue/Endpoint transit | exact E-M cut when transferring Endpoint as another message payload |
| **UN-C02** K-S vector commit | RFC-002/003; CO | all-or-none source→queue + R_S | multi-Grant + B/Close + E-M coherent SMP commit algorithm |
| **UN-C03** Precommit source retention | RFC-002/009; CO | **this SEND** does not consume source | older prose might imply restoration after unrelated close/revoke; addendum clarifies |
| **UN-C04** KR-M2 receipt | KR-M2, RFC-009 | consumed q identity separate from receipt result | scoped Task/lane, lost admission response, finite W/Q and GC |
| **UN-C05** K-C audience and ownership | RFC-003/018; RP | one bounded claim to actual authorized Task/Egen | Endpoint MOVE, ServiceIncarnation/PolicyEpoch and secrecy of Vm |
| **UN-C06** V disclosure | RFC-003/004; RP | conservative visibility marker before first possible publication | Vm projection, partial usercopy/backing aliases, permanent audience fence |
| **UN-C07** K-I whole vector | RFC-002/003; RP | all-or-none install+dequeue+R_R | revoke of one queued g mid-transfer: whole-vector no-K-I policy; race/terminal |
| **UN-C08** Handle publication | RFC-002/009; RP/HPA | inert token IDs pre-K-I, no external stale rebind | exact Vt/K-I micro-order, finite ID exhaustion, alias race, lost result |
| **UN-C09** T1 vs T2 | RFC-018; HPA | Task-wide bearer T1; optional trusted K-U | independent verifier/trusted original immutable bytes + actual Grant binding |
| **UN-C10** Task restart | RFC-006/009 | new generations/Endpoint grants, no blind retry | postmortem receipts, sponsor old queue/claim cleanup |
| **UN-C11** Physical/VM fences | RFC-002/004/007 | B/R1/R2-F/R2-O separated | actual hardware path, mapping cutoff, safe pin release |
| **UN-C12** Service effect/durability | RFC-008/018; RFC-010 boundary | method outcome, storage and native IPC are distinct | guest Linux errno/FD and durable transaction/RequestId handshakes |
| **UN-C13** M0 reserve/wait graphs | RFC-004/005/006 | Pager/IRQ/cleanup own protected budgets | joint bounded admission/lock ordering/progress across SEND and RECEIVE |

**Finding classification:** No new **confirmed direct contradictory textual requirement** has been identified in the currently examined updated RFC addenda. UN-C03 is a **wording ambiguity already addressed by Proposed addenda**; UN-C01/02/05/07/08 are genuine **unresolved combined semantics**, not mechanically demonstrated safety failures. No category marked ALIGNED is a proven implemented property.

## 8. Integrated adversarial schedule register UN-X01…UN-X32 — NONE EXECUTED

| ID | Scenario | Required safe result or falsifier |
|---|---|---|
| UN-X01 | S A(qS), B revoke wins before K-S | whole no-K-S, safe pending settlement |
| UN-X02 | K-S wins before B, caller return lost | committed source MOVE unchanged; query honest |
| UN-X03 | independent Close(source g) wins before K-S | that SEND no-cut, no magical restored current handle |
| UN-X04 | competing sends same GrantID/duplicate handles in one vector | no two queue custodians; reject ambiguous vector |
| UN-X05 | E1 owner E-M wins after A but before old K-S | old Task cannot perform new SEND |
| UN-X06 | Endpoint itself payload in other m during InTransit | no public Task owner; no old/new simultaneous SEND |
| UN-X07 | K-S committed; sender Task dies; E1 queue survives | no phantom source restoration or sponsor refund |
| UN-X08 | K-S qS reused after ACK/GC | replay fence prevents second SEND |
| UN-X09 | qS and qR equal numeric, different TaskGenerations | no receipt or Grant owner alias |
| UN-X10 | two receiver Threads claim same m | one exclusive claimant |
| UN-X11 | K-C R before SEND m actually exists | prohibited phantom claim |
| UN-X12 | unauthorized size/type probe before K-C | no secret metadata oracle |
| UN-X13 | authorized confidential Vm then policy/owner changes | mark possible disclosure; restrict foreign redelivery |
| UN-X14 | Vp first byte then Pager fault | possible exposure, never assert full rollback |
| UN-X15 | Vt first token copied, second token faults | inert IDs; maybe-published nonrebind |
| UN-X16 | B second Thread sees token; K-I wins; initiator loses return | legitimate Task-wide invocation, historical receipt |
| UN-X17 | K-I before token copyout, return lost (R-A alternative) | orphan grants recognized/charged; not false no-K-I |
| UN-X18 | one of N grants revoked after K-S, before K-I | full K-I fails/terminals, no N-1 partial success |
| UN-X19 | B wins *after* K-I, cap closed later | history committed, current rights may fail |
| UN-X20 | K-C sees m; receiving E1 Endpoint moves B→C before V | abort/revalidate; no audience confusion |
| UN-X21 | E1 moves B→C after V before K-I | no claim migration/foreign requeue |
| UN-X22 | Task B dies after V; B2 same ServiceID appears | old claim can't retarget, disclosure history survives terminal |
| UN-X23 | K-I and queue close/destruction race | exactly one custody terminal/receive disposition |
| UN-X24 | duplicate K-I or result query attempts cap reinstall | no duplicate transferred native Grant |
| UN-X25 | GC receiver R_R with queued/terminal exposure debt | q fenced; independent claim/disclosure persists while needed |
| UN-X26 | Task death/GC while DeviceCap DMA pins continue | no premature physical frame reuse/R2-F lie |
| UN-X27 | T2 verifier accepts mutable/stale SABI role under old policy | no live K-U without trusted original Grant binding |
| UN-X28 | R returns NO_KI after V; B already writes via existing network cap | backend effect remains possible |
| UN-X29 | Result UNKNOWN mapped by guest to success or clean retry | invalid semantic translation rejected |
| UN-X30 | W full, malicious B claims head indefinitely, M0 blocked by same credits | bounded failure/terminal, protected M0 progresses if assumptions hold |
| UN-X31 | output ranges distinct VA but physically aliased or concurrently remapped | no kernel grant identity confusion; exposure status honest |
| UN-X32 | receiver status result lost after K-I; different q attempts “retry” | no same-message second K-I, possible new business effect requires external idempotency |

## 9. Proposed safety properties UN-I01…UN-I30 — ALL UNPROVED

| ID | Required invariant (abstract) |
|---|---|
| UN-I01 | Every qS/qR action authenticates actual TaskGeneration and scoped right; q alone is not a bearer Grant. |
| UN-I02 | Sender qS A/X fence permits at most one original K-S, even after receipt GC. |
| UN-I03 | K-S is one all-or-none message+whole Grant MOVE+protected R_S phase. |
| UN-I04 | K-S/B and K-S/E-M have coherent conflicting-cut order, no post-B unauthorized new action. |
| UN-I05 | Failed pre-K-S SEND cannot consume sources *by that SEND* or restore independently closed/revoked grants. |
| UN-I06 | Each source exact GrantID has no two simultaneous independent MOVE custodians. |
| UN-I07 | At most one effective public owner per E1 Endpoint; zero while kernel InTransit allowed. |
| UN-I08 | E1 direction FIFO derives from K-S queue-commit order, not q/thread/return order. |
| UN-I09 | K-C never precedes existence of same queued m and is exclusive for m. |
| UN-I10 | Receiver Claim binds authenticated TaskGeneration/Egen/current issuer audience. |
| UN-I11 | Before sensitive Vm/Vp/Vt, current recipient authorization and appropriate projection are checked. |
| UN-I12 | Vm/Vp/Vt recorded before first possible publication, never reset by fault/death/ACK. |
| UN-I13 | After relevant V, same m is not normally redelivered to unrelated Task or ServiceIncarnation. |
| UN-I14 | K-I applies full exact N-Grant vector at most once or none, retires exactly same m and R_R coherently. |
| UN-I15 | Revoke of any mandatory grant before K-I never yields disguised N-1 success or revoked usable rights. |
| UN-I16 | K-I respects Claim/Endpoint owner, Task death, queue close and relevant lineage barrier ordering. |
| UN-I17 | Maybe-published public token values stay inert before K-I and cannot later authorize unrelated grants in same TaskGeneration. |
| UN-I18 | Historic COMMITTED_KI query cannot reinstall a native Grant or promise its current invokability. |
| UN-I19 | Native T1 transferred grants after K-I belong to Task boundary, not exclusively initial receiver Thread. |
| UN-I20 | True independent T2 authorization binds immutable message/native GrantIDs/current policy at K-U. |
| UN-I21 | NO_KI or TERMINAL doesn't automatically assert no sensitive data exposure or no remote effect. |
| UN-I22 | Committed sender K-S never undone by failed receive, Task death or revoked queued descendant. |
| UN-I23 | Actual R_S and R_R status reflect their independent cuts, observer UNKNOWN remains separate. |
| UN-I24 | Pending receipts are not GCed while future cut can still commit; final GC never re-enables q. |
| UN-I25 | Queue/Claim/exposure/tokens/lineage/DMA obligations survive receipt/Task lifetime independently. |
| UN-I26 | Revocation B, R1, R2-F and R2-O remain distinct authorized physical/memory lifetime evidence. |
| UN-I27 | Backend method effects and FS durability aren't inferred from native K-S/K-I. |
| UN-I28 | Kernel copyin/copyout/Pager/cleanup avoids forbidden wait-for cycles with locked IPC and M0. |
| UN-I29 | Ordinary Task capacity use is bounded and cannot borrow designated M0 reserves. |
| UN-I30 | New TaskGeneration/ServiceIncarnation cannot read/use old native receipts, claims or transferred rights by display ID alone. |

**Reminder:** These are formal-model targets, **not properties that a checker has evaluated**. Some need ghost variables for past disclosure and external effect uncertainty, and a separate implementation refinement proof later.

## 10. Priority P0/P1 semantic blockers UN-O01…UN-O18 — ALL OPEN

| ID | Priority | Single blocking question and proof deliverable | Owners |
|---|---|---|---|
| UN-O01 | P0 | Atomic K-S with multiple GrantIDs/Endpoint generation and simultaneous B/Close/MOVE, no faultable global lock | 002/003/004 |
| UN-O02 | P0 | Resolve E1 Endpoint owner removal/ownerless transit/reacquisition and CO-E-C bounded old-call settlement under K-S/K-I | 002/003/006 |
| UN-O03 | P0 | Define after-K-S queue destructor and self-send/Endpoint reference cycle safe terminal path | 002/003/006 |
| UN-O04 | P0 | Provide preknown qS/qR + protected receipt admission+Pending storage under results lost and finite Q/W | 003/009 |
| UN-O05 | P0 | Specify K-C exact Task+Egen+issuer Audience/ServiceIncarnation binding and policy/revoke race | 003/018 |
| UN-O06 | P0 | Determine Vm projection secrecy and precisely when requeue to distinct authorized receiver remains legal | 003/018 |
| UN-O07 | P0 | Select/prove Vp/Vt publication vs K-I, full protected anti-ABA token issuance with alias/fault safety | 002/003/004/009 |
| UN-O08 | P0 | Revoked member of vector after K-S: all-or-none K-I denial, terminal handling, charges and no partial grants | 002/003 |
| UN-O09 | P0 | Explain terminal NO_KI vs possible exposure and preserve historic true COMMITTED_KS/COMMITTED_KI | 003/009 |
| UN-O10 | P0 | Prove K-I commit, Claim, queue retirement and protected R_R atomically win SMP races against close/B/Task death | 002/003/009 |
| UN-O11 | P0 | Prove no-loss/no-rebind token publication under caller result loss and task-wide concurrent use | 002/009 |
| UN-O12 | P0 | Bound all Q/W/queue/Claim/token/spin/reference/lineage sponsor costs with M0 independent emergency credits | 004/005/006 |
| UN-O13 | P0 | Prove Pager/copyout lock graph, CPU scheduling and cleanup liveness without recursive wait | 003/004/005/006 |
| UN-O14 | P0 | Real independently trusted T2 verifier/issuer immutable original message + Grant provenance where chosen | 002/018 |
| UN-O15 | P0 | Actual R1 CPU map and R2-F DMA/DeviceGroup lifecycle separate from read/receive ACK | 002/004/007 |
| UN-O16 | P1 | Service idempotency, durable effect ledger and truthful Linux guest errno/fd translation | 008/009/010/018 |
| UN-O17 | P1 | Compare KR-M1/KR-M2/lane and HPA-B+/C/register profiles on target bounded workload and kernel TCB | 001/003/005 |
| UN-O18 | P1 | Formal cut-to-implementation refinement criteria and selected bounded state search coverage | research/001/009 |

**Acceptance dependency order:** UN-O01/02/04 → 05/06 → 07/08/09/10/11 → 03/12/13/15 → remaining policy/guest/performance/refinement. These are dependencies for **design research**, not a software delivery schedule or approval.

## 11. Proposed joint decisions UN-D01…UN-D12 — NONE Accepted

- **UN-D01:** Keep E1 one-effective-Task-owner and MOVE-only source-to-queue custody as initial native IPC research topology.
- **UN-D02:** Explore CO-S K-S-versus-B strict cut first, with CO-E-C precommit settle-before-Endpoint handoff; neither is validated or Accepted.
- **UN-D03:** Keep sender A(qS)/R_S and receiver A(qR)/R_R distinct, even when same q numeric; link only by kernel message ID.
- **UN-D04:** Distinguish kernel custody graph, possible Vm/Vp/Vt information release, caller knowledge and external method effect.
- **UN-D05:** Research HPA-B+ inert public token T1 receive as first ordinary bearer baseline; preserve HPA-C/T2 as conditional comparators.
- **UN-D06:** Require exclusive audience-bound K-C, whole N-Grant K-I/queue retirement, and no foreign requeue after relevant V.
- **UN-D07:** A revoked mandatory vector member before K-I must not silently yield partial success; prefer whole no-K-I terminal for first proof case.
- **UN-D08:** Retain strong Task-lifetime public Handle nonrebind of possibly exposed tokens; fail closed at exhaustion without selecting width.
- **UN-D09:** Terminal NO_KI and result UNKNOWN never entail no data disclosure, current Handle usability or external no-effect.
- **UN-D10:** Keep revocation B/R1/R2-F/R2-O, M0 sponsorship and service effect/storage durability strictly separate from receipts.
- **UN-D11:** Require bounded, non-vacuous joint model checks and independent implementation/hardware refinement before changing any P0 gate.
- **UN-D12:** All decisions remain *Proposed research*; no numeric ABI, syscall choice, executable model, kernel code or test is authorized.

## 12. Future formal proof-plan traceability — paper only

**Small combined model candidates:** S with two Threads, R with two Threads, C as another eligible Task, old/new TaskGeneration, one E1 Channel/Endpoint transferred as payload, two distinct source GrantIDs, one message m, one scoped revoker, one Pager/usercopy fault, one Claim, qS/qR distinct; W=2/Q=2 as illustrative finite values, not production limits.

**Future witness classes (only after separate explicit model authorization):**
1. **Good:** S K-S, R K-C/Vp/Vt/K-I, historical R_S and R_R both correctly final, grants each at only one effective custodian.
2. **Revoked before K-S:** qS final no-cut, no sender MOVE caused by this SEND; independent revoke prevents current use.
3. **Revoked between K-S and K-I:** sender receipt committed; receiver all-vector denies/no-K-I terminal, possible V reported honestly.
4. **Result lost:** qS/qR known protected phase, caller observer UNKNOWN; no native repeat or grant remint.
5. **Audience race:** E-M or Task death after V, no claim migration/foreign redisclosure.
6. **Token ABA:** exposed inert ID retired on fault; never becomes another Grant.
7. **Resources:** pending claim and grant/DMA debt survive terminal receipt GC without M0 starvation.
8. **Negative controls:** intentionally break exactly one guard each (partial K-I; skip over consumed q; requeue after V; stale token rebind; K-S after B; lost result interpreted as no-cut), and require an independently understandable counterexample.

**Go/No-Go gate UN-R1: OPEN / NO-GO.** Document alignment is not proof. Approval requires one precisely fixed micro-order for HPA-B+ or a consciously narrower alternative, a bounded coherent SMP K-S/K-I design, non-vacuous checked interleavings, recovery sponsor/wait graph proof and later concrete implementation/hardware refinement. The **next recommended design-only research** is a **P0 Conflict Resolution / ADR Readiness Matrix** focusing on UN-O01/02 (K-S/E-M/B), UN-O07/08 (public token/V vs K-I/revoked vector) and UN-O12/13 (M0/Pager), with rejected alternatives and explicit proof budget. No implementation is required or permitted at this stage.

## 13. Source references, methods and limitations

- **[S1] Fuchsia kernel Channel object:** https://fuchsia.dev/fuchsia-src/reference/kernel_objects/channel
- **[S2] Fuchsia channel_read + typed read_etc:** https://fuchsia.dev/reference/syscalls/channel_read and https://fuchsia.dev/reference/syscalls/channel_read_etc
- **[S3] Fuchsia channel_write_etc:** https://fuchsia.dev/reference/syscalls/channel_write_etc
- **[S4] seL4 official IPC tutorial:** https://docs.sel4.systems/Tutorials/ipc
- **[S5] Linux `copy_to_user` constraints:** https://docs.kernel.org/kernel-hacking/hacking.html
- **[S6] All cited UniOS v0.2/v0.1 research and RFCs linked in header**, current main-branch documents.

**Evidence limitations:** This contract is an independent written cross-comparison of candidate semantics and official external precedents. It **does not** claim tests, proof, executable state model, working kernels or a settled syscall ABI. Real Fuchsia/seL4/Linux behaviors are not asserted to be reproduced in UniOS.

---

**Document history:** 2026-10-25 Proposed Unified SEND–RECEIVE v0.1. **13** joint ownership/friction categories, **32** adversarial paper schedules, **30** unproved invariants, **18** open blockers, **12** Proposed joint decisions; consolidated K-S→Queue→Claim→K-I/Terminal, Vm/Vp/Vt, ownerless E1 Endpoint transit, revoked multi-Grant vector and distinct source/receiver receipts. No OS implementation or formally verified design.

## 14. Follow-up P0 conflict triage and draft ADR readiness — 2026-10-25 Proposed

[CAPIPC P0 Conflict Resolution & ADR Readiness Matrix v0.1](../reviews/capipc-p0-conflict-resolution-adr-readiness-v0.1.md) maps all 18 UN-O blockers to eight **draft-only** ADR-CAPIPC-A…H packages, assigns heuristic design-priority scores and lays out explicit D0 study vs D1 semantic review vs later separately authorized verification/refinement/production gates. First coupled risk chain: K-S/B/E-M whole Grant vector, K-I/public Vm/Vp/Vt+revoked-in-transit N-vector, and Pager/M0/queue-cycle bounded cleanup. HPA-B+ remains a **T1 candidate family, not an approved token/K-I micro-order**; optional independently trusted T2 does not silently become mandatory baseline. The original UN-O statuses remain OPEN and the Unified contract remains Proposed; no simulation, runtime, source code, accepted ADR or numerical ABI.

## 15. 2026-10-26 A/C/E joint cut resolution candidate — Proposed

The [CAPIPC Joint A/C/E Semantic-Cut Resolution v0.1](../models/capipc-joint-semantic-cut-resolution-a-c-e-v0.1.md) provides a more precise paper-only candidate: sender K-S/B/E-M protected order and no half-vector MOVE; receiver Task qR, exclusive audience Claim and **Vp→Vt→K-I** with inert public IDs prior to all-or-none install; independent per-cut R/Claim/queue/token/lineage/Pager/DMA accounting and M0 reserve. It **does not prove this ordering can be executed safely on SMP**. A Pager-faulted publisher may resume *after* Endpoint owner/issuer audience authority changes, so the publication epoch/fence itself is a new P0 semantic/liveness burden; checking rights only at K-I is insufficient for data confidentiality. Earlier UN-O statuses remain OPEN; this synthesis is not an Accepted contract or an executable specification.

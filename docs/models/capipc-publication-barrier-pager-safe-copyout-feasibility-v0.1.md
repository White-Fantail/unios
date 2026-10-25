# UniOS CAPIPC — Publication Barrier & Pager-Safe Copyout Feasibility v0.1

> **Status:** Proposed independent design-and-feasibility research — **2026-10-26 NZDT**. **NOT Accepted, implemented, executable, formally proved, model-checked, tested, benchmarked, or ABI-frozen.**
> **Question:** Can UniOS offer confidential T1 IPC receive copyout from a kernel-owned message into receiver-owned userspace memory, while E1 Endpoint ownership, issuer audience PolicyEpoch, grant revocation, Pager faults and Task death occur concurrently, **without** stale publication, double native Grant ownership, M0 deadlock or unbounded kernel-held resources?
> **Design-stage limitation:** Research and paper failure schedules only. No .tla/.als model, pseudocode intended for execution, kernel source, numerical sizes, ABI structs, physical experiments, CI, SDK or implementation.
> **Immediate predecessors:** [Joint A/C/E Semantic-Cut Resolution v0.1](capipc-joint-semantic-cut-resolution-a-c-e-v0.1.md), [P0 ADR Readiness Matrix](../reviews/capipc-p0-conflict-resolution-adr-readiness-v0.1.md), [Unified SEND–RECEIVE Contract](../contracts/capipc-unified-send-receive-semantic-contract-v0.1.md), [Receive Exposure & HPA-B+ Study](capipc-receive-exposure-ki-handle-publication-v0.1.md), [Commit/Revoke/E1 Ordering](capipc-commit-revocation-ordering-v0.1.md), [Formal Model Preparation](capipc-formal-model-preparation-v0.1.md).
> **Contract owners:** [RFC-002](../rfcs/rfc-002-object-capability-lifetime-delegation-revocation.md) Grant and Endpoint owners; [RFC-003](../rfcs/rfc-003-ipc-wire-protocol-and-synchronization.md) Claim, Vm/Vp/Vt, K-I; [RFC-004](../rfcs/rfc-004-virtual-memory-pager-cache-and-oom.md) mapping/fault; [RFC-005](../rfcs/rfc-005-scheduling-preemption-and-cpu-budget.md) preemption/M0 progress; [RFC-006](../rfcs/rfc-006-boot-trust-chain-service-startup-and-recovery.md) recovery/reserve; [RFC-009](../rfcs/rfc-009-native-abi-and-system-call-contract.md) output/receipt truth; [RFC-018](../rfcs/rfc-018-versioned-idl-abi-evolution-and-compatibility-contracts.md) issuer metadata policy. Hardware DMA [RFC-007](../rfcs/rfc-007-user-space-drivers-dma-and-iommu.md) separately evidenced.

## 0. Executive verdict: narrow the publication operation, not its safety promise

**Conditional feasibility judgment:** A *bounded non-faulting publication slice after preparatory VM work* appears a more viable **paper proof target** than an arbitrarily long Pager-faultable copy holding an in-flight authorization across owner handoff. **However, no implementation or unbounded concurrency proof exists.** If it cannot be made genuinely non-faulting, bounded, properly synchronized with MappingGeneration and audience Epoch, **a smaller register-only or separately controlled mailbox output profile must be considered** before accepting HPA-B+ copyout for general IPC.

The key semantic distinction is between:

- **P-DENY (barrier established):** a scoped owner/issuer audience epoch transition has made **new publication permits** for the old authority unavailable. This does **not** prove that prior permits stopped copying, that prior bytes were unread, that all bytes are visible, or that physical quiescence occurred.
- **P-DRAIN (publisher quiesced):** every older authorized publication slice has been **observably completed or prevented from performing any future store** under its old permit. It does not erase already published bytes or revoke legitimate previously issued native caps.
- **K-I (native receive commit):** independently installs the full exact CapGrant vector and retires the queue message + protected R_R once. Even P-DRAIN says nothing about K-I, and K-I alone does not prove P-DRAIN.

**Strongest security boundary actually supported by the proposal:** no *new unauthorized publication slice* can start after P-DENY for its exact owner/audience/PolicyEpoch, and no *old-permit publication store* can occur after a successfully reported P-DRAIN for that scope. If the product instead promises “no previously permitted byte can become visible immediately when P-DENY is issued,” that is a **different, stronger and possibly unrealizable contract**, requiring a safe drain before the reported completion cut. Pre-barrier copied plaintext may remain observable forever.

**Critical new subtlety:** A plain sequence-counter **retry after copying does not undo user-visible writes** made under the stale epoch. Likewise *pin_user_pages*, memory residency, a validated user pointer or an MMU-notifier-style generation stamp are **not by themselves** a publication authorization mechanism, stable user VA→frame binding, prevention of aliasing across Tasks, or proof that copyout has no fault.

**Recommendation for next comparative design:** **P1 — staged, fully charged finite non-faulting slices with an issuer-scoped publisher fence**, **P0 — register-only bounded baseline**, and **P2 — isolated receiver mailbox** as longer-term research comparator. Avoid automatic approval of faultable lease/drain, global locks, or unbounded pinned buffers. No ranking is an Accepted ADR.

## 1. Threat and trust model

Authorized principals:

- A real receiver **TaskGeneration R**, its current E1 receive **EndpointGeneration E**, exact kernel **Claim(m,R,E,Audience)** and matching preidentified Task receipt qR. Service display name or SABI label is not authority.
- A specific **issuer audience/PolicyEpoch P** governing **sensitive metadata and payload publication**. It may be changed by authorized issuer policy, and its change is *not automatically identical* to CapGrant lineage revoke B. Kernel must know how to authenticate the issuer/configured audience grant, without parsing arbitrary untrusted IDL method strings.
- Exact destination **AddressSpace / MappingGeneration M / readable-or-writable frame backing**, and a **bounded current publisher permit**. Userland may concurrently remap its VA, share writable backing, race another Thread, exit, and observe bytes at any time.
- **Pager** can be crashed, out of CPU/IPC/frame credit, or recursively blocked by an IPC lock. M0 recovery must not depend on normal Pager, filesystem or ordinary Task quota for essential fail-safe work.
- **Native Handle TokenIDs** reserved before optional Vt remain inert until real all-or-none K-I. They may be visible to any Thread in receiving Task while inert and become Task-wide bearer capabilities after K-I.

Adversary can: race owner MOVE/issuer epoch cutoff/revoke with faulting copy, modify or alias output buffers, induce repeated bad pages/partial TokenID publication, saturate Q/W/Claim, kill/restart a Task while a publisher is active, or force an intentionally never-completing Pager. Do not assume friendly same-Task Thread cooperation when arguing kernel-native grants or quotas.

**What cannot be promised by a kernel copy primitive:** confidentiality *against a Task already legitimately authorized to receive its own plaintext*, retroactive erasure of previously exposed bytes, prevention of SABI/backend effects using its **preexisting** capabilities, constant-time resistance to every speculative side channel, or userland output stability after another Thread writes to the same memory. These are different threat/contract layers.

## 2. Formal meanings: four cuts, two fences, three histories

| Symbol | Candidate state/cut | Authorized claim / disallowed inference |
|---|---|---|
| K-C | exclusive Claim to actual R/E/Audience for m | Claim reserves receiver opportunity, **not** native transferred Grant custody |
| Vm | potentially observable metadata projection | authorized secret size/type/right projection is its own disclosure |
| Vp | first possible payload publication marker **before** a store/map | may-have-been-disclosed, not all bytes definitely delivered |
| Vt | first possibly visible public TokenID marker **before** output | ID may be inert; may-be-visible ID cannot rebind to another Grant within live Task |
| **PermitStart(m,R,E,P,M,slice)** | protected transition to active bounded publisher slice | requires live issuer PolicyEpoch, E1 owner, authorized Claim, valid current mapping/backing, reserved quota and no cutoff |
| **P-DENY(scope,oldP)** | scoped *deny-new-permits* cut | earlier issued permits may still be in progress; reporter must not say publication quiesced |
| **PermitFinish/Abort** | cut proving a particular slice can issue no further write | does not assert consumer didn't observe earlier bytes |
| **P-DRAIN(scope,oldP)** | after all old permits finish or are safely fenced against further stores | only then can controller claim **no further old-permit stores**; prior plaintext may persist |
| K-I | whole native Grant vector installation + m retirement + R_R committed | native cap grant truth; no implication of complete usercopy or service effect |
| B/R1/R2-F/R2-O | lineage deny, CPU, physical drain, object-wide reclamation | **distinct** from issuer P-DENY/P-DRAIN and K-I |
| R_R and observer | protected actual receipt vs caller-known outcome | UNKNOWN/EXPIRED is not definitive no-KI or no disclosure |

**Ordering convention:** all '<' notation in this document is a **candidate protected linearization relation**, not wall-clock observation on another CPU. User memory writes may propagate to observers later; a publication rule cannot pretend to control when a legitimate Task's CPU caches or downstream copies expose a byte already written.

### 2.1 P-DENY, effective owner handoff and acknowledgement choices

**Option HB1 — dual-stage handoff (research preference):** controller establishes P-DENY for the old audience first, waits/proves P-DRAIN before reporting *strong publication quiescence* or before any **owner handoff response that promises old publication stopped**. New owner permits require separately authenticated new Egen/issuer audience. An owner-transfer *request* and a fully effective owner-transfer *completion acknowledgement* need not be the same event.

**Option HB2 — ownership changes immediately, old slices may drain later:** possible only if public contract explicitly allows authorized pre-handoff in-flight stores, bounds and charges them, never retargets their bytes to the new owner, and distinguishes *owner cut acknowledged* from *disclosure quiescence pending*. This is a larger trust/observability burden and **not presumed safe by an E1 owner uniqueness proof alone**.

**Option HB3 — immediate cutoff of every old store without drain:** **reject as a promise** without a demonstrable CPU/store fence and safe task cancellation. Marking a bool canceled doesn't revoke an already entered copy routine's CPU stores.

### 2.2 Publication cutoff is not Capability revoke

If a CapGrant lineage B occurs without an issuer audience change, B denies the specified **new native operations**. Whether B also disallows **future payload copyout** depends on the explicit issuer publication policy. Conversely P-DENY may stop bytes even if a prior S→queue K-S remains committed, and a later K-I may separately be denied by grant revocation. Avoid mislabeling one epoch as proof of all these effects.

## 3. Four concrete comparator families — still design-only

| Profile | Pre-publication action | Publication cut | Main upside | Primary proof debt |
|---|---|---|---|---|
| **P1: prepared non-faulting slices** | bound bytes and TokenIDs; pre-resolve/charge writable pages, mapping generation, private/share scope, finite pin/ref window | per-slice permit with bounded non-faulting store then Finish under an explicit publication-fence contract | preserves general-purpose T1 receive with flexible staged payload | VM remap/alias, mapping revocation, store/epoch ordering, bounded CPU segment and pin-cost proof |
| **P0: register-only bounded response** | stage minimal native response in kernel registers/Thread return state | one small return/publication permission cut per authorized Thread/Task | avoids arbitrary pageable usercopy and some copyout/Pager lock windows | severely limits payload size, ABI/thread return loss and interrupt/owner-cut linearization remain |
| **P2: sealed receiver mailbox / mapped message** | kernel-owned immutable message, authenticated controlled destination view | explicitly authorized mapping/access release cut, no bulk usercopy | potentially scalable large data and cleaner payload lifetime | page-table/TLB shootdown, shared alias, mapping Task ownership, disclosure via early alias, extra MemoryObject + service TCB |
| **P3: faultable epoch lease** | keep Claim/permit active across arbitrary usercopy and Pager faults | permit acquired once; revoke/owner cut waits/cancels | convenient copy API | unbounded Pager wait, deadlock/availability denial, stale stores after unsynchronized cancel |

**Initial comparative result:** P1 is a **conditional feasibility candidate**, not best-proven. P0 is a genuine smaller-scope fallback. P2 deserves a separate MemoryObject/mapping RFC review; mapping release isn't automatically atomic across CPUs. P3 cannot support a **bounded handoff** guarantee without changing its fault path and thus losing its apparent simplicity.

### 3.1 P1 — non-faulting slice prerequisites (not execution steps)

Before any PermitStart, candidate proof must establish all of:

1. **Staged immutable bytes and typed Grant snapshot** for the exact m, with receiver Task qR/R_R, current Claim/Egen/issuer P and sponsor quota.
2. **Destination eligibility:** address range arithmetic valid and bounded; effective audience authorized to see plaintext *through every writable alias* of chosen backing. A pinned physical frame may also be shared with another Task. **Pinning isn't isolation**.
3. **Mapping identity:** the copied-to backing must be tied to actual Task/AddressSpace/MappingGeneration and survive across the slice or be invalidated in a synchronized way. A raw VA revalidated earlier is insufficient; CPU mapping change and TLB shootdown need explicit proof.
4. **Fault independence:** permission/writability, COW resolution, page residency and any kernel mapping are completed before the protected PermitStart so that **the slice itself cannot Pager-fault or allocate**; a speculative user fault after acquiring a permit is a safety failure of the candidate.
5. **Finite maximal work:** bytes/TokenIDs per slice, number of concurrent active slices, CPU nonpreemptible window and references are bounded symbolically (numbers deliberately unselected). No holding arbitrary VM/IPC global locks across usercopy or Pager.
6. **Issuer/Endpoint epoch serialization:** PermitStart and P-DENY share a protected conflict ordering. P-DRAIN only after no old permit remains able to write, verified across CPUs and cleanup paths. Mere postcopy seqcount retry isn't sufficient.
7. **Precharged terminal debt:** failure after Vp/Vt still has funded message/claim/disclosure/tombstone/sponsor cleanup, so a canceled receive cannot force unsafe redelivery to another Task or consume protected M0 reserves.

**Open cost:** pre-resolving writable output may itself page-fault, COW-break, pin or allocate. It must be performed *without* holding a publication permit or locks that the Pager needs; after preparation, hostile remapping/reclaim must not invalidate a supposedly nonfaulting slice without an ordered conflict. If that requires long-lived pinned mappings or hostile Task cooperation, P1 may be economically or architecturally inferior to P0/P2.

**A malicious same-Task Thread can read/modify a receiving Task's authorized output even before K-I.** This does not contradict T1; the kernel must still prevent out-of-audience backing/Task disclosure. P1 can't promise per-Thread confidentiality inside a Task.

### 3.2 P0 — register-only restricted IPC

For a strictly bounded return payload with a bounded number of typed native results, a register/state-return interface can avoid pageable copyout in the critical window. An issuer-authorized **return publication cut** must still decide whether the Thread can receive plaintext after E1 owner/PolicyEpoch changes. If that cut occurred first, later scheduling/context switch does not erase the staged register values; if barrier won first, stale plaintext must not be introduced as a fresh permitted return.

Do not conflate “register-only” with unrestricted handle transfer: public TokenIDs are still Task-scoped bearer identities, real CapGrants must install coherently at K-I, and lost return leaves actual committed grants/receipt truth potentially UNKNOWN to caller.

A truncated message must be **explicitly refused or handled by a separately documented continuation protocol**; do not silently break whole-message or N-Grant atomicity. This is the most defensible **smaller initial proof model** if P1 isn't tractable, but may not satisfy full native desktop IPC requirements.

### 3.3 P2 — sealed mailbox / view-release comparator

A kernel-owned or privileged protected immutable message can be attached to a receiver Task only after an authorized view-release cut. Avoiding *bulk copyout* does not avoid disclosure: installing a mapping, propagating page table changes/TLB shootdown, preexisting writable shared aliases and later revoke map access all need scoped evidence. An attacker can copy bytes during an earlier valid map before revoke. MemoryObject provenance, frame sponsorship, pinned/device DMA and page-cache/storage durability remain independent.

**No automatic zero-copy guarantee:** copying/staging may still occur to obtain immutable backing and avoid shared writable aliases. Mapped mailbox often shifts, rather than removes, the security burden to RFC-004 MemoryObject, R1 CPU cutoff and ResourceBudget.

### 3.4 P3 — why ordinary seqlock recheck fails for information

~~~text
Epoch p read at preflight
copy_to_user makes some confidential bytes visible
issuer epoch changes p→p+1
other copy_to_user writes more bytes
postcopy epoch check detects mismatch; returns error
~~~

The returned error can be truthful about K-I/no-KI but **cannot undo byte disclosure**. Sequence checks that guard *read-only private computation* do not automatically authorize irreversible writes to an observer's memory. Linux's mmu interval notifier collision-retry concept concerns consistent VA-range invalidation observations; it is **not a license** to write to stale Task user buffers after another principal's publication barrier.

## 4. Paper-only protected transition candidates PB-T01…PB-T18

| ID | Candidate transition | Guard and proof obligation |
|---|---|---|
| PB-T01 | Preknown receiver qR/Pending R_R | authenticated TaskGeneration, quota before any Claim/V |
| PB-T02 | K-C | queued m, exclusive actual Claim, receiver Egen/issuer audience |
| PB-T03 | Immutable staging | actual kernel m/grants/bytes, not mutable user-provided policy |
| PB-T04 | ResolveDestination | Task/AS/MappingGeneration/private-or-shared authorized backing, bounded writable pages |
| PB-T05 | Pin/charge bounded lifetime | frame/mapping/CPU and M0-independent quota; pin alone is not confidentiality |
| PB-T06 | Classify authorized metadata Vm | no secret count/type/rights leak through generic size probes |
| PB-T07 | PermitStart(payload slice) | current Egen/PolicyEpoch, no P-DENY, bound prepared destination, active-publisher credit |
| PB-T08 | Vp first possible store | monotone maybe-exposed marker before first copy |
| PB-T09 | SliceCopyNoFault | bounded work, no Pager/heap allocation, no incompatible global lock |
| PB-T10 | PermitFinish / cancel after write fence | prove no further old-permit stores; release publisher credit |
| PB-T11 | P-DENY | disallow new permits for old exact scope and epoch atomically with PB-T07 |
| PB-T12 | P-DRAIN | require all old live permits retired and CPU visibility of stopped publishing; otherwise pending/fail-stop |
| PB-T13 | PermitStart(token slice), Vt | reserved inert nonrebind IDs, audience and mapped destination as in payload |
| PB-T14 | Whole K-I | Claim+Task/Egen/current lineage valid; whole N grants installed and m retired with R_R committed atomically |
| PB-T15 | TerminalNoKI after V | confirmed future K-I impossible, possible disclosure/TokenIDs retained as independent debts |
| PB-T16 | Task death/Endpoint handoff | no stale Claim/permit to new TaskGeneration/ServiceIncarnation; in-flight old permit fenced |
| PB-T17 | Pager failure/OOM after preparation started | cancel without active permit; or honest blocked state, no global wait cycle |
| PB-T18 | Query/GC and M0 safe cleanup | receipt history never remints; bounded credits and hardware R2-F separately evidenced |

**Atomicity disclaimer:** PB-T07 and PB-T11 are required to share an ordering, but this document does **not** prescribe a mutex, RCU, seqlock, or physical CPU memory-order algorithm. PB-T09/10 cannot be treated as trivially atomic on SMP; a later accepted model must split their possible interleavings as appropriate. 

## 5. Two-stage barrier outcome table — P-DENY vs P-DRAIN

| History | New old-audience PermitStart? | May preauthorized byte stores still happen? | Honest reply |
|---|---|---|---|
| no P-DENY, current Egen/P | permitted if all guards hold | yes | no barrier asserted |
| P-DENY wins before PermitStart | **NO** | none from that rejected permit | DENY_ESTABLISHED, not promise previous plaintext erased |
| PermitStart wins then P-DENY | **NO new** | **YES**, strictly preauthorized bounded slice(s) still active | DENY_ESTABLISHED; QUIESCENCE_PENDING |
| all old slices finished after P-DENY | NO | NO further old-permit stores | DRAIN_CONFIRMED, never “plaintext unread” |
| Pager fault blocks in allegedly no-fault slice | invalid P1 assumptions | unknown/stalled | **NO drain success**; reject P1 feasibility until fixed |
| Task killed with active copied-data permit | NO new permitted Task work | old CPU/kernel publisher must be actually fenced | QUIESCENCE_PENDING or proven DRAIN; not mere Task-death assertion |
| Endpoint owner handoff to new Egen | new owner separately authorized | old stores only if contract explicitly allows undrained overlap | E-M completion semantics must say DENY versus DRAIN; don't imply both |
| physical DMA keeps a pinned page | independent | ordinary CPU publisher may be drained | R2-F/DeviceGroup safety still unproved |

**Public status note:** DENY_ESTABLISHED/QUIESCENCE_PENDING/DRAIN_CONFIRMED are **logical review terms**, not new syscall status integers, ABIs, or guarantees for all endpoint/lineage cuts.

**If the required product contract needs an immediately complete old-publication shutdown at handoff:** the controller must hold its strong handoff acknowledgement until P-DRAIN is evidenced, or explicitly fail/degrade. There is no universal finite response time if an authorized publisher is permitted to fault and stay blocked forever.

## 6. Concurrency and alias-specific failure register PB-X01…PB-X24 — NONE EXECUTED

| ID | Adversarial interleaving | Failure to exclude |
|---|---|---|
| PB-X01 | receiver Claim then issuer epoch cut before PermitStart | stale audience still begins publication |
| PB-X02 | receiver Payload PermitStart then P-DENY | barrier lies that old stores already drained |
| PB-X03 | new PermitStart concurrently with P-DENY on second CPU | both win incompatible order |
| PB-X04 | P-DENY established then old publisher sees cached epoch | new stale slice starts |
| PB-X05 | faultable copy stalls while permit held | unbounded owner handoff/quiescence |
| PB-X06 | content copied before postcopy epoch mismatch | retry hides actual exposure |
| PB-X07 | copy marks Vp only after first user store | false NO_DISCLOSURE |
| PB-X08 | first TokenID Vt output succeeds, second faults | stale public ID later authorizes unrelated grant |
| PB-X09 | old Task remaps VA during pre-resolve→slice gap | write goes to wrong/new mapping |
| PB-X10 | pinned frame aliased by unauthorized foreign Task | pin confuses backing stability with confidentiality |
| PB-X11 | COW not resolved before supposedly nonfaulting slice | Pager fault under publisher fence |
| PB-X12 | CPU page-table generation changed, stale TLB path used | unauthorized destination across mapping epoch |
| PB-X13 | private intended buffer shares writable backing with another Task | receiver audience validation insufficient |
| PB-X14 | hostile Thread corrupts B's output while kernel copies | claimed semantic byte integrity beyond kernel rights |
| PB-X15 | B Endpoint MOVEs to C while B active slice held | old new-client cross-audience confusion |
| PB-X16 | PolicyEpoch changed, K-I still succeeds on old grant | installed revoked native rights despite new cut |
| PB-X17 | K-S committed, g2 revoked in transit, K-I attempts N−1 | false all-cap atomic delivery |
| PB-X18 | P-DRAIN reports success before old CPU store fully fenced | post-ack old secret store |
| PB-X19 | Task dies after Vp; new B2 same ServiceID claimed | old message redisclosed to new Task |
| PB-X20 | Pager needs queue lock held by publication wait | M0/recovery deadlock |
| PB-X21 | repeated bad pages / token burns exhaust Q and M0 | unbounded resource debt |
| PB-X22 | register-only receipt result lost after K-I | guest/app retries and remints Grant |
| PB-X23 | mapped mailbox granted while old shared alias exists | premature foreign disclosure |
| PB-X24 | receipt GC frees pinned DMA frame before R2-F | physical use-after-recycle |

## 7. Candidate proof predicates PB-I01…PB-I24 — NONE PROVED

| ID | Unproved safety/feasibility obligation |
|---|---|
| PB-I01 | Every publisher permit authenticates current TaskGeneration, Egen, issuer audience P and Claim. |
| PB-I02 | P-DENY and PermitStart for same exact scope serialize without both winning in inconsistent order. |
| PB-I03 | Once P-DENY succeeds, no new old-scope permit can begin. |
| PB-I04 | P-DRAIN may succeed only if no old permit may write again, across all CPUs and cancellation paths. |
| PB-I05 | Vp/Vt flags are conservatively set before first possible relevant observable store. |
| PB-I06 | After relevant V, ordinary foreign same-message redelivery is forbidden. |
| PB-I07 | No PermitStart with stale MappingGeneration or unsafe unauthorized shared backing. |
| PB-I08 | P1 slice is genuinely nonfaulting/nonallocating and bounded after permit begins. |
| PB-I09 | Pre-resolve/Copy/Pager waits don't hold global lock required for Pager/M0. |
| PB-I10 | Preauthorized old publisher cannot be retargeted silently to new Egen/Task/ServiceIncarnation. |
| PB-I11 | P-DRAIN does not claim plaintext erasure or absence of previous disclosure. |
| PB-I12 | P-DENY/PolicyEpoch publication rules and lineage B/R1/R2-F remain distinct. |
| PB-I13 | Pinned frames/mapped pages cannot be used as authorization by pointer/token guessing. |
| PB-I14 | Current authorized audience includes the selected shared/backing visibility policy. |
| PB-I15 | Reserved inert TokenIDs become native invokable only under actual whole-vector K-I. |
| PB-I16 | Maybe-published TokenIDs never rebind unrelated native Grant in live TaskGeneration. |
| PB-I17 | K-I and R_R protected result are coherent with exact queue m and Claim. |
| PB-I18 | Revoked mandatory Grant before K-I cannot produce active unauthorized or N−1 reported success. |
| PB-I19 | No TerminalNoKI/result-loss/GC can resurrect a Grant or undo committed K-S. |
| PB-I20 | Receipt qS/qR, message ID and public TokenID namespaces do not alias by numerical equality. |
| PB-I21 | Each PermitStart consumes precharged publisher/Claim/VM/pin/terminal debt, not unbounded general reserves. |
| PB-I22 | Normal Task load cannot borrow M0 wired CPU/IRQ/frame/IPC/stack/cleanup reserve. |
| PB-I23 | Task death/supervisor recovery can't report publication drained without actual publisher cancellation proof. |
| PB-I24 | DMA/FS/remote effect truth is not inferred from P-DRAIN or native K-I. |

**Liveness is explicitly conditional:** if page faults are allowed *after* acquiring an indefinitely live permit, an attacker-controlled Pager can hold the permit forever. Scheduler fairness cannot make a blocked Pager respond. If a finite deadline is required, either P1 nonfaulting bounded slices must be established with actual assumptions, or the product must fail-stop / choose a different output mechanism. No proof of guaranteed bounded quiescence is supplied.

## 8. Feasibility risk evaluation and fallback gates

| Comparator | Confidentiality risk | Liveness/resource risk | ABI/TCB cost | Current conclusion |
|---|---|---|---|---|
| **P1 prepared no-fault slices** | medium, conditional on exact backing/epoch/slice serialization | potentially bounded if no faults and finite slices; preparation cost unknown | kernel pin/map/permit/ref accounting expands TCB | **preferred full-payload proof comparator, OPEN** |
| **P0 register-only bounded** | narrower exposure window, still needs issuer-cut order | fewer Pager dependencies, bounded payload | constrained API and adaptation costs | **credible initial fallback, OPEN** |
| **P2 sealed mapping mailbox** | mapping/alias and revoke/TLB remain | long-lived mapped object/pin quota | larger VM/MemoryObject design | **longer-term comparison, OPEN** |
| **P3 arbitrary faultable lease** | epoch staleness and stale writes | indefinite Pager stall/permission hold | superficially easy API, bad recovery coupling | **reject for bounded strong barrier contract** |

**Stop P1 if** its pin/residency/mapping-retention assumptions cannot be enforced against untrusted Task remap/COW, or it needs a lock held over faultable usercopy, or active publisher work cannot be bounded under SMP/interrupts. A proof about a fixed two-page model cannot justify arbitrary-sized output or a production resource budget.

**Stop P0 if** the actual targeted minimal native IPC workload cannot be represented without silent truncation/splitting of an atomic message/grant vector. Limit the advertised product scope honestly, or explicitly design a continuation/reassembly layer with separate receipts.

**Stop P2 if** it inherits hidden accessible aliases to plaintext or its “atomic view release” is only a label without mapped-page/TLB coherence and Task owner evidence.

## 9. Research candidate decisions PB-D01…PB-D12 — ALL PROPOSED

- **PB-D01:** Separate scoped P-DENY (no new old permits) and P-DRAIN (all prior old publishers unable to write further).
- **PB-D02:** Link E1 handoff acknowledgement to the precise publication guarantee it makes; strong quiescence requires P-DRAIN.
- **PB-D03:** Scope issuer audience/PolicyEpoch independently from native CapGrant B/R1/R2-F/R2-O.
- **PB-D04:** Compare pre-resolved bounded non-faulting P1 slices first for full payloads, never claim already feasible.
- **PB-D05:** Retain register-only P0 as first restricted receive feasibility fallback, preserving whole native cap vector semantics.
- **PB-D06:** Study protected mailbox P2 later with RFC-004 map/alias/TLB proof, not as effortless zero-copy.
- **PB-D07:** Reject postcopy epoch retries as an information-confidentiality rollback technique.
- **PB-D08:** Treat physical pinning and correct Task/MappingGeneration/audience validation as independent requirements.
- **PB-D09:** Keep Vm/Vp/Vt possible exposure monotone and after-V no-foreign-same-message rule.
- **PB-D10:** Keep K-I whole N-grant install and protected receipt independently coherent; result query never remints.
- **PB-D11:** Enforce independent per-cut credit/publisher/tombstone/Claim quotas and protected M0 reserve, fail closed.
- **PB-D12:** Retain D0/OPEN/Proposed status; no model execution, Accepted ADR, KABI or implementation now.

## 10. P0 open questions PB-O01…PB-O16 and design proof exit gate

| ID | Priority | Blocking question / evidence demanded |
|---|---|---|
| PB-O01 | P0 | Which issuer has publication policy authority; how does its PolicyEpoch differ from B/E-M and service name? |
| PB-O02 | P0 | Define exact PermitStart vs P-DENY protected linearization and CPU-visible ordering. |
| PB-O03 | P0 | Define active-publisher Finish/Abort and what prevents any later CPU store before P-DRAIN. |
| PB-O04 | P0 | Prove finite nonfaulting prepared slice even with Task remap, COW, TLB, preemption and CPU hotplug. |
| PB-O05 | P0 | Prove backing visibility/alias authorization, including shared writable pages across Tasks. |
| PB-O06 | P0 | Identify bounded chunk size/concurrency and precharge costs without inventing ABI widths. |
| PB-O07 | P0 | Define Claim/E1 handoff weak-vs-strong acknowledgement; no false old-publication quiescence. |
| PB-O08 | P0 | Integrate Vp/Vt and inert public TokenID anti-ABA across partial output/Task death. |
| PB-O09 | P0 | Guarantee whole-vector K-I when a mandatory Grant B races receive/publication. |
| PB-O10 | P0 | Specify secure same-recipient continuation or terminal after partial V, and disclosure retention. |
| PB-O11 | P0 | Prove no Pager/M0 wait-for cycles during prepare, publication, handoff or cleanup. |
| PB-O12 | P0 | Define exhaustive per-cut Claim/queue/R/token/pin/sponsor/DMA credits, fail-stop ceiling. |
| PB-O13 | P0 | Establish TaskDeath/ServiceRestart cancellation of in-flight publisher before old Claim reassignment. |
| PB-O14 | P1 | Evaluate register-only restricted ABI coverage against minimum UniOS developer desktop workflows. |
| PB-O15 | P1 | Independently evaluate sealed mailbox/MemoryObject view release and R1/TLB scope. |
| PB-O16 | P1 | Specify checker abstraction/negative controls and later implementation memory-model refinement. |

**Gate PB-R1 = OPEN / NO-GO for accepting general pageable HPA-B+ copyout.** Before even D1 semantic readiness, name one exact issuer/audience/publisher fence contract and show paper interleavings for concurrent E-M/P-DENY vs prepared/suspended slice. Supply a bounded no-fault VM/backing model, safe terminal and M0 quotas or narrow the initial ABI. Future D2 would need separately authorized finite state checking and refinement; D3/D4 implementation/hardware evidence is not available.

**Recommended next design-only milestone:** **CAPIPC Publication Permit & Destination Memory Contract v0.1**. Compare detailed P1 private/pinned backing and P0 register-only semantics under identical publication-epoch and error-status guarantees, including Task/MappingGeneration, shared alias restrictions and actual M0 cut budget. Keep status Proposed until independent proof artifacts exist.

## 11. Primary references and precise applicability limits

- **[S1] Linux kernel usercopy: may sleep/return uncopied bytes; prohibited spinlock/IRQ-off faultable use:** https://docs.kernel.org/kernel-hacking/hacking.html
- **[S2] Linux Memory Management API: pagefault-disabled access vs pageable access / MMU notifier API:** https://docs.kernel.org/core-api/mm-api.html
- **[S3] Linux pin_user_pages: physical pin types, duration and synchronization constraints:** https://docs.kernel.org/core-api/pin_user_pages.html
- **[S4] Linux mmu_interval read/retry: VA-range invalidation observation is not automatic publication permission:** https://docs.kernel.org/core-api/mm-api.html
- **[S5] Linux sequence counters and limitations of postcopy retry:** https://docs.kernel.org/locking/seqlock.html
- **[S6] Fuchsia zx_channel_read: whole logical message, bytes buffer before handles buffer:** https://fuchsia.dev/reference/syscalls/channel_read
- **[S7] Fuchsia Channel object / E1-like owner and queue precedents:** https://fuchsia.dev/fuchsia-src/reference/kernel_objects/channel
- **[S8] UniOS preceding Proposed RFC/model papers:** linked above.

**Evidence boundary:** External kernel documentation is precedent only; Linux's pinned-page API, mmu notifier, seqlock and Fuchsia's Channel syscall **are not UniOS algorithms and are not being imported**. No source indicates UniOS has built this, and no usercopy scheme, latency number, bit allocation or CPU memory fence has been experimentally validated.

---

**History:** 2026-10-26 first Proposed PB v0.1. Two-stage P-DENY/P-DRAIN fence, P1 finite non-faulting slices versus P0 register-only/P2 sealed mailbox/P3 faultable lease; 18 proposed action cuts, 24 paper-only attack schedules, 24 unproved predicates, 12 proposed decisions, 16 open questions. No implementation or checker.

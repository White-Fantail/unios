# UniOS — CAPIPC Destination Memory & Publication Barrier Cross-RFC Decision Table v0.1

> **Status:** Proposed, design-only cross-RFC decision and feasibility research — 2026-10-26 NZDT. **NOT Accepted, proven, model-checked, executable, implemented, tested, benchmarked or ABI-frozen.**
> **Scope:** Compare output identity (**current Task VA vs typed MemoryObject vs bounded Thread registers**), VM invalidation, issuer/Endpoint publication gates, native Capability install, resource accounting and recovery. Provide **reviewable choices**, not a source-level solution.
> **Immediate upstream:** [Publication Permit & Destination Memory Contract v0.1](../models/capipc-publication-permit-destination-memory-contract-v0.1.md); [Publication Barrier & Pager-Safe Copyout Feasibility v0.1](../models/capipc-publication-barrier-pager-safe-copyout-feasibility-v0.1.md); [Joint A/C/E Cut Study](../models/capipc-joint-semantic-cut-resolution-a-c-e-v0.1.md); [Unified SEND–RECEIVE Semantic Contract](../contracts/capipc-unified-send-receive-semantic-contract-v0.1.md); [P0 ADR Readiness](capipc-p0-conflict-resolution-adr-readiness-v0.1.md).
> **Primary responsibility:** [RFC-003 IPC](../rfcs/rfc-003-ipc-wire-protocol-and-synchronization.md) owns message m, receiver qR, audience-bound exclusive K-C, Vm/Vp/Vt, whole-vector K-I and receipt; [RFC-004 VM/Pager](../rfcs/rfc-004-virtual-memory-pager-cache-and-oom.md) owns actual Task/AddressSpace/MappingGeneration, write backing, COW, invalidation and pins. [RFC-002](../rfcs/rfc-002-object-capability-lifetime-delegation-revocation.md) owns typed grants/Endpoint, [RFC-005](../rfcs/rfc-005-scheduling-preemption-and-cpu-budget.md) CPU/IRQ admission, [RFC-006](../rfcs/rfc-006-boot-trust-chain-service-startup-and-recovery.md) wired M0, [RFC-009](../rfcs/rfc-009-native-abi-and-system-call-contract.md) result truth, [RFC-018](../rfcs/rfc-018-versioned-idl-abi-evolution-and-compatibility-contracts.md) issuer/SABI audience, and [RFC-007](../rfcs/rfc-007-user-space-drivers-dma-and-iommu.md) independent physical DMA.
> **Terminology warning:** *M0 recovery reserve* (RFC-006) is distinct from RFC-004's research **VM stage M0**; do not silently merge availability budgets or versioned VM stages.

## 0. Executive judgment: prefer a split initial profile, not one universal copyout mechanism

**Candidate architecture for subsequent design-only review:**
1. **IPC-CTRL-R (narrow P0-R):** bounded register/Thread-return publication for a separately scoped first-profile native control, Pager fault and M0 recovery transport. Preserve native whole-message/Grant semantics, qR receipts and a **ReturnRelease** information cut. Do not claim arbitrary desktop messages fit.
2. **IPC-VA-B (conditional P1-MB):** a **fixed, short, pre-resolved resident destination VA binding** for bounded body copy; a VM write binding/MappingGeneration and an issuer PermitStart must both be live. VM M-INV and issuer P-DENY may block new work, and strong P-DRAIN requires all prior stores fenced. No faultable Pager work or long-lived global locks inside the protected slice. **D0 OPEN; not yet viable by proof.**
3. **IPC-OBJ (P1-MA/P2 comparator):** deliver to a **typed kernel-attested backing MemoryObject** rather than promising an arbitrary current VA. Copy-to-object, immutable message view or explicit MappingRelease are different subprofiles; the choice can avoid a stale-VA *contract*, but creates MemoryObject alias/share/TLB/VM sponsor obligations and does **not** automatically secure disclosure. **D0 OPEN**, likely appropriate to study for larger transfers.

**No global winner is declared.** The recommendation is **profile separation as a research organizing principle**. General-purpose HPA-B+ \`Vp→Vt→K-I\` remains a candidate logical ordering only, not evidence of a realizable copyout implementation.

**Most consequential unresolved cross-RFC issue:** RFC-003 cannot alone declare a destination “safe to publish,” and RFC-004 cannot alone declare an output “authorized for this receiver message.” An abstract **paired (M,P) publication authority** must refine both the VM's *mapping/backing write eligibility* and IPC issuer's *audience/Endpoint/Claim permission* at one conflict-consistent start. Its lifetime and revocation have two separate quiescence obligations. A timestamp/seqcounter checked *after* writing cannot repair an unauthorized disclosure.

**Review outcome:** **NO-GO for Accepted D1 semantics or an executable state model yet.** Establish a minimal purpose-built output profile and a reproducible, paper-level state/action ordering including preemption/TaskDeath first.

## 1. External evidence: precise relevance, not assumed UniOS behavior

| Source | Documented behavior | What it does NOT prove |
|---|---|---|
| [S1] Linux pin_user_pages | Physical-page pinning for writes has explicit lifetime/synchronization constraints | current user VA stays mapped to the pinned frame, alias privacy or publication authorization |
| [S2] Linux MMU notifier | Secondary mapping/TLB invalidation ordering matters during COW/replacement | UniOS has those synchronization rules; old readable plaintext is erased |
| [S3] Linux usercopy | General access to user memory may fault/sleep and must avoid incompatible atomic locking | an arbitrary public buffer can be copied under a short nonfaulting lock by declaration |
| [S4] Linux seqcount | Sequence checks can observe concurrent modification; pointer lifetime/progress assumptions still apply | post-publication retry undoes earlier stores |
| [S5] Fuchsia Channel read | A whole logical message and defined bytes/Handle outputs; public API reports buffer insufficiency | CPU-byte writes are globally atomic or UniOS cap installation is copied from Zircon |

These are **first-party external reference contracts only**. No external kernel's internal code or implementation assurance is represented as proof of proposed UniOS actions.

## 2. The three candidate output identity promises

| Output identity | Success promises about destination | Must avoid claiming | Main RFC owner |
|---|---|---|---|
| **VA-targeted (V)** | at the defined protected delivery cut, staged bytes were written through a binding that still denotes receiver R's exact requested VA interval / authorized backing, with separately stated possible observer disclosure | raw pointer still targets same frame after any later unmap; user Task didn't race its own writable bytes | RFC-004 mapping record + RFC-003 Claim |
| **Object-targeted (O)** | bytes logically delivered to an **authenticated exact MemoryObject/version/range**, not necessarily available at any caller VA yet | caller's originally supplied VA unchanged; any preexisting alias cannot see it; mapping/CPU R1/TLB revocation is immediate | RFC-002 typed grant + RFC-004 MemoryObject/alias policy |
| **Register-targeted (R)** | small defined output accepted for publication at a coherent Thread/Task **ReturnRelease** cut, even if the initiator is delayed and never reads it | result values are unread until instruction retires; new CapGrants are Thread-only or fully known to caller | RFC-001/005/009 + RFC-003 |

**Do not mix promises.** A P1-MA kernel-alias write to a pinned old frame can be memory-safe and correctly Object-targeted while **violating** a claimed current-VA delivery. Conversely freezing a mapping to satisfy VA-targeting may increase TCB and risk M0 progress. Register-targeting avoids user-buffer alias during publication but not the policy/Endpoint cut or lost K-I result.

### 2.1 What "private" can legitimately mean

- **PrivateAtRelease:** during an issuer-defined release cut, backing has no unauthorized readable/writable cross-Task aliases *under an enforced MemoryObject sharing policy*. It does not prevent authorized R from forwarding/plaintext-copying later.
- **ExplicitSharedAudience(A):** issuer has approved a declared set of Tasks/sharing realms that may see backing via aliases; do not silently advertise Task-private secrecy.
- **UnknownBackingAudience:** reject private-publication claims; either fail closed, require issuer-approved shared scope, or deliver into a separately secured typed MemoryObject.
- Racy full-system reverse-map enumeration is not a substitute for a typed sharing/invalidation contract. Immutable backing provenance and authorized future alias creation need separate review.

**Specific limitation:** an authorized R can always read legitimately released plaintext then invoke its *preexisting* IPC/file/network rights; a native OS cannot guarantee indefinite downstream secrecy against that trusted audience simply because the initial page was private.

## 3. Cross-RFC decision table CRDM-01…CRDM-18 — all UNRESOLVED candidates

| ID | Boundary / question | Research-leading option | Rejected or conditional alternative | Required evidence / owner |
|---|---|---|---|---|
| **CRDM-01** | VA vs backing identity | **Profile split:** R for control, V limited bounded, O for typed bulk | universal raw-pointer receive for every size | RFC-003/004/009 choose exact success definition |
| **CRDM-02** | Writable frame lifetime | protected bounded backing reference with sponsor | \`pin\` presumed to freeze VA or issuer rights | RFC-004/002 lifetime and terminal |
| **CRDM-03** | MappingGeneration during stores | M-INV coordinates binding and every actual store | optimistic postcopy version retry | RFC-004 SMP mapping correctness |
| **CRDM-04** | COW/huge split/remap | resolve before live permit or fail before cut; actual invalidation ordering | "prefault once" then arbitrary reuse | RFC-004 VM/TLB |
| **CRDM-05** | Cross-Task alias audience | typed PrivateAtRelease or ExplicitSharedAudience grant | only check writable R VA | RFC-002/004/018 issuer sharing authority |
| **CRDM-06** | Issuer PolicyEpoch vs CapGrant B | explicit P-DENY for publisher and distinct B for native caps | equate revoke B with plaintext erasure | RFC-002/003/018 scoped epochs |
| **CRDM-07** | Endpoint E-M vs publisher | deny new old-owner permits; old-slice drain for strong handoff ack | merely switch owner pointer, let stale CPU write | RFC-002/003/006 |
| **CRDM-08** | Paired (M,P) authority | protected logical PermitStart validates both current VM binding and current issuer Claim/Egen/P | two unrelated unlocked checks | RFC-003/004 one conflict-consistent cut |
| **CRDM-09** | Destination M-INV | separate M-DENY then M-DRAIN before backing contract retired | recycle pinned page when Task ID dies | RFC-004 VM lifetime and CPU/TLB ordering |
| **CRDM-10** | Copyout faults and waiting | do Pager work **before** finite active permitted write; no incompatible global locks held | faultable long lease with unbounded quiescence | RFC-003/004/005/006 |
| **CRDM-11** | Public TokenID publication | inactive exact token until full K-I, no rebind if maybe visible | assume public ID copyout success is atomic | RFC-002/003/009 |
| **CRDM-12** | Whole Grant K-I | all authorized N GrantIDs + queue retire + protected R_R exactly once | N−1 after one mandatory B-revoked Grant | RFC-002/003/009 |
| **CRDM-13** | Terminal after possible V | preserve possible disclosure and custodian/sponsor debt; no foreign same-m requeue | failed copy means no bytes seen | RFC-003/006 |
| **CRDM-14** | Register ReturnRelease | explicit Thread/Task/issuer/Egen return-publication cut | assume prepared trapframe is always unobservable or cap-holder only one Thread | RFC-001/005/009 |
| **CRDM-15** | Attacker stalls active publisher | bound actual CPU/IRQ critical execution or honest pending/fail-stop | finite byte length implies bounded wall time | RFC-005/006 |
| **CRDM-16** | Pager/IPC wait cycles | separate admitted Pager/M0 reserves, canonical dependency DAG | global IPC/VM lock around faultable usercopy | RFC-004/005/006 |
| **CRDM-17** | Receipt identity and GC | qS/qR Task-generation scoped replay fence, result history separate | expired status means safe native rerun | RFC-003/009 |
| **CRDM-18** | hardware and service effects | DMA R2-F, mapping R1, backend durable outcome distinct | infer any from K-I or P-DRAIN | RFC-007/008/018 |

**Status interpretation:** each row states a Proposed candidate and a concrete alternative/error. This is **not** a voted ADR, confirmation that current RFC code is internally contradictory, or a mechanically completed proof. The most critical undecided row is **CRDM-08**, because two independently correct epoch checks might not compose into one safe publish cut.

## 4. Paired publication cut and mapping invalidation ordering

### 4.1 Minimum shared state (abstract, not executable)

Let m be actual queued MessageID; r=(TaskGeneration, AddressSpaceGeneration, receive EndpointGeneration, ClaimGeneration); p=(issuer-authorized Audience, PolicyEpoch); d=(DestinationBindingID, MappingGeneration, VA-or-MemoryObject identity, backing-sharing class, writable/present/COW-resolved state); and k=(TokenID reservation, GrantID vector, R[qR]).

- A permitted **PublishStart(m,r,p,d,slice)** requires **both** current authorized r/p *and* current d lifetime/write/audience contract, plus precharged bounded active-publisher and terminal liability.
- **M-DENY(d)** disallows *new* writes through old d; **M-DRAIN(d)** allows retirement/reuse only after all live publishers with d can no longer write.
- **P-DENY(p)** disallows *new* old-audience permits; **P-DRAIN(p)** proves all old permits cannot write further. **M-DRAIN and P-DRAIN are orthogonal**: drain of one scope isn't proof the other has released safe references.
- Every actual byte/TokenID store associated with a permit must be bound to the permitted exact destination and issuer scope, and be followed by protected PublishFinish/Fence with no future store. "Granted at start" is an allowed prebarrier old-slice exception only if the public contract explicitly recognizes it.
- Only then may strong acknowledged E-M/issuer handoff or old destination retirement claim **its exact promised quiescence**, not global erasure of historical plaintext.
- K-I is a **separate** coherent kernel CapGrant/queue/R_R cut; a queue message may terminalize after V with no native rights installed.

**A future candidate synchronization must have a consistent outcome when M-DENY and P-DENY race PublishStart.** There need not be a universal global lock or total order for unrelated Tasks/objects. The difficult proof is that **no stale old-scope permit can win against either deny cut while the other preflight observation is out of date**, and that the Finish/Fence condition truly holds on SMP.

### 4.2 Conflicting-cut matrix

| Relative cuts | May old writer begin? | Possible future old stores? | Status and custodian |
|---|---|---|---|
| P-DENY before PublishStart | NO | none from rejected new permit | honest DENY, message remains claimed or terminals |
| M-DENY before PublishStart | NO | none through rejected binding | reprepare destination or safe terminal, no false current-VA success |
| PublishStart before P-DENY | existing permitted slice only | until protected Finish/Fence, if explicitly allowed | P-DENY acknowledged, P-DRAIN **pending** |
| PublishStart before M-DENY | existing permitted slice only | until protected Finish/Fence | M-DENY acknowledged, M-DRAIN **pending** |
| Both P-DENY/M-DENY after PublishStart | no further new permits | existing slice must finish/fence under **both** debts | neither DRAIN may lie; TerminalNoKI may retain possible V |
| TaskDeath before new permit | NO | none from new permit | old publisher must be actually fenced before backing recycle |
| K-I wins before later B | whole N GrantIDs installed | publication history independent | R_R COMMITTED_KI historical; later rights may change |
| B on mandatory queued Grant before K-I | no unauthorized whole-vector install | earlier V remains | TerminalNoKI/Claim settlement; original K-S still committed |
| R0 ReturnRelease then P-DENY | register output considered maybe exposed | no pageable store to undo | later Task/Thread death doesn't unrelease data |
| P-DENY before R0 ReturnRelease | no old-authority register release | any prepared secret registers require safe discard before untrusted return | cannot call return successful merely from prepared trapframe |

### 4.3 A four-object deadlock candidate and the forbidden shortcut

**Paper wait-for cycle:** old publisher holds an M-binding permit; mapping invalidator holds a VM management lock while waiting for M-DRAIN; publisher's preemption/PageFault path needs a Pager service; Pager's IPC setup requires the same VM/IPC lock or ordinary credit exhausted by the Claim. **Such an edge set is forbidden for a bounded M0 promise.**

Merely using a monotonically increasing epoch and checking it after writing is **not** a solution. A spinlock around a faultable copy also isn't a solution. A future option may need staged resident/private backing and short writer lifetime, a narrower register-only profile, or object-release semantics with separate MappingGeneration/TLB fencing. The exact lock/ref/memory-barrier algorithm is **not chosen here**.

### 4.4 Delivery time and retained plaintext nuance

The contract must name which cut establishes "delivered to VA" (actual protected write completion) versus "possibly observed" (before first store) versus "new native rights installed" (K-I). A CPU store that completed before the P-DRAIN cut may become visible to an observer later. P-DRAIN guarantees *no further old-authority writes* under the stated CPU/mapping model, **not** "no further observations of old bytes." Do not promise plaintext disappearance after receiver has validly learned it.

## 5. Profile decision gate: control R versus fixed resident V versus object O

| Check | IPC-CTRL-R | IPC-VA-B | IPC-OBJ |
|---|---|---|---|
| Main output identity | Thread register release | exact current Task VA+binding during protected delivery | exact typed MemoryObject version/range |
| First use candidate | fault, control, recovery and tiny RPC | bounded resident private data | bigger or shared bulk payloads |
| Copy to arbitrary pageable user buffer | absent for return payload | **not** in proposed first V profile | not asserted; object/map release separate |
| Pager during publication | no bulk user-buffer write | nofault slice **if** preparation actually sufficient | page mapping/access may be deferred; still needs R1/TLB contract |
| Audience/Endpoint revoke | ReturnRelease vs P-DENY | PublishStart/P-DENY/P-DRAIN | ObjectViewRelease vs issuer/Endpoint, separate mapping cutoff |
| Destination alias problem | no user buffer for return registers | private-at-release typed backing required to promise privacy | deliberate shared audience/alias permissions explicitly specified |
| Native K-I | whole vector+queue/R_R | same | same; object view never silently equals cap install |
| Size semantics | explicit small upper envelope / no silent truncation | fixed resident window; any continuation separate | immutable object range/ownership credits |
| Resource sponsor | Thread qR/receipt, TokenID/Grant vector | plus VM binding, pins, publisher permits, terminals | plus object memory/map ref, possible TLB + page-cache debt |
| Status | **D0 OPEN** | **D0 OPEN** | **D0 OPEN** |

**Conditional recommendation:** advance R and a **strictly fixed resident/private V** to precise paper guard studies first. Study O in parallel as a **separate bulk-data MemoryObject contract**, not as magic zero-copy. Avoid turning all desktop native receive operations into long-lived frozen user pages. If V cannot satisfy CRDM-03/04/05/08/09/10/15/16 without expanding the trusted computing base unacceptably, de-scope it from the first release profile and do not violate kernel integrity to preserve a broad advertised API.

### 5.1 P0-R minimal functionality research only

Candidate P0-R workloads: Pager fault/status notification, scoped Service startup/control, bounded Wait/Event notification, tiny ReplyToken methods, recovery/diagnostic small records. **All need a real workload inventory before choosing ABI sizes.** They may carry a small exact Grant vector only if the chosen register-return and protected K-I contracts can represent every grant without unreported installation or silent truncation. No magic rollback after K-I and lost register return.

### 5.2 Object output is not an implicit dynamic current-VA promise

A typed MemoryObject identity may be stable while its user mappings change or become absent. If kernel writes to the named object, success can be valid at the object cut and subsequent mapping may fail without contradicting **that** delivery promise. However memory immutability/versioning, private/shared alias policy, issuer visibility, kernel Map/Unmap rights, fault behavior, page contents, mapping TLB/grants and sponsor transfer still need own guards. Native CapGrant installation K-I and releasing readable message view are potentially separate cuts; a different Task must not gain access by guessing an ObjectID.

## 6. Joint adversarial paper schedules CRDM-X01…CRDM-X26 — NONE EXECUTED

| ID | Schedule / adversary | Required rejection or truthful outcome |
|---|---|---|
| CRDM-X01 | pinned R frame, VA unmapped/rebound; old frame written | not falsely report current-VA delivery |
| CRDM-X02 | object-targeted delivery succeeds; no current R mapping | do not falsely report user-VA delivery requirement |
| CRDM-X03 | unauthorized read-only alias exists to pinned frame | private-at-release claim denied, not merely writable-alias check |
| CRDM-X04 | sharing explicitly authorized to C; reverse-map detects C | don't overclaim private or incorrectly forbid agreed shared audience |
| CRDM-X05 | COW after preflight before PublishStart | stale backing/mapping cannot win |
| CRDM-X06 | hugepage split/migration between page pin and PublishStart | actual refs, mapping and TLB must remain coherent |
| CRDM-X07 | PublishStart checks issuer P first, M invalidation wins before M check | two unlocked checks cannot permit stale write |
| CRDM-X08 | PublishStart checks M first, P-DENY wins before P check | issuer authority stale despite fresh mapping |
| CRDM-X09 | PublishStart wins then both M-DENY and P-DENY | both drains remain pending until safe Finish/Fence |
| CRDM-X10 | publisher CPU preempted indefinitely after permit acquired | finite slice length not enough to promise finite P-DRAIN |
| CRDM-X11 | Pager invoked under IPC lock while M-DRAIN waits | fatal wait-for cycle; no M0 success claim |
| CRDM-X12 | TaskDeath frees frame before old publisher fenced | no premature frame reuse |
| CRDM-X13 | source K-S committed, g2 revoked before receiver K-I | no partial N−1 native cap delivery |
| CRDM-X14 | public token number copied before K-I, receiver faults | token inert; no later unrelated Grant rebind |
| CRDM-X15 | native K-I commits, receiver return lost | historical result truthful, no remint/retry |
| CRDM-X16 | Endpoint E-M after Vp; old Claim forwarded to new Task | foreign redelivery prohibited |
| CRDM-X17 | issuer policy changes during register result preparation | current ReturnRelease gate needed |
| CRDM-X18 | ReturnRelease first, thread dies before actually reading | maybe-exposed history, no retroactive secrecy |
| CRDM-X19 | R0 message exceeds register cap, implicit split | native whole-message/Grant promise not preserved |
| CRDM-X20 | object published to Task C via stale alias before authorized view release | unauthorized recipient disclosure |
| CRDM-X21 | active native K-I but mapped payload later unmaps | historic cap receipt not equal ongoing mapped view |
| CRDM-X22 | timer for publisher drain expires but CPU cannot execute cancel | don't report DRAIN_CONFIRMED |
| CRDM-X23 | issuer P-DENY completed but bytes already in R caches | can't claim previous plaintext erased |
| CRDM-X24 | final receipt GC with active binding/pin/Claim | custody/charge persists, q replay fence intact |
| CRDM-X25 | Task/Service restarts with reused display ID and old memory object token | no cross-generation Claim/Grant adoption |
| CRDM-X26 | DMA still owns physical frame while CPU publication is drained | separate R2-F quarantine before physical recycle |

## 7. Unproved cross-RFC invariants CRDM-I01…CRDM-I24 — NONE PROVED

| ID | Future falsifiable property |
|---|---|
| CRDM-I01 | At native publication, destination identity is explicitly one of V/O/R and status truth uses only that promise. |
| CRDM-I02 | A frame ref or pin alone never establishes current VA binding or issuer publication permission. |
| CRDM-I03 | Every V permit is tied to current actual Task/AS/MappingGeneration and authorized backing/sharing scope. |
| CRDM-I04 | Every O output names exact typed MemoryObject identity/version/range, never guesses another Task's object. |
| CRDM-I05 | Every R ReturnRelease authenticates actual Thread/TaskGeneration/Endpoint/issuer cut. |
| CRDM-I06 | Paired (M,P) PublishStart cannot win after either M-DENY or P-DENY for same exact scope. |
| CRDM-I07 | Every old permitted writer becomes unable to store before its scope reports M-DRAIN/P-DRAIN complete. |
| CRDM-I08 | M-DRAIN and P-DRAIN are independent; neither certifies past plaintext erased or physical DMA fenced. |
| CRDM-I09 | Authorized private/shared audience classification cannot be faked by a racy writable-VA check or mere page pin. |
| CRDM-I10 | No kernel publication occurs to unauthorized foreign readable or writable aliases under a PrivateAtRelease claim. |
| CRDM-I11 | Legitimate receiver plaintext forwarding is not treated as a native kernel permission/security proof violation. |
| CRDM-I12 | Vp/Vt/Vm possible exposure markers precede first potentially observable user write/view/return release. |
| CRDM-I13 | After relevant exposure, same m not automatically requeued to unrelated Task/ServiceIncarnation. |
| CRDM-I14 | Whole transferred native Grant vector installed at most once at K-I or not installed, with queue/R_R coherence. |
| CRDM-I15 | Relevant B before K-I on a mandatory Grant prevents unauthorized N−1 partial success. |
| CRDM-I16 | Maybe published TokenIDs never authorize different native Grant in same live TaskGeneration. |
| CRDM-I17 | qR receipt history and current Handle authority differ; Query never remints rights or replays K-I. |
| CRDM-I18 | V and O publication preflight can PageFault only outside forbidden Pager/VM/IPC global locks. |
| CRDM-I19 | An active V slice is genuinely bounded/nofault with admitted CPU/IRQ/preemption assumptions or honest pending fail. |
| CRDM-I20 | VM binding, issuer policy, publisher, Claim and TokenID liabilities remain charged until real terminal/fence. |
| CRDM-I21 | M0 wired CPU/frame/IPC/IRQ/stack/cleanup reserve cannot be borrowed by ordinary Task/Claim saturation. |
| CRDM-I22 | Endpoint owner change/TaskDeath cannot silently continue old publisher or adopt old claim by ServiceID. |
| CRDM-I23 | Guest FD/errno and backend durability/effect are not inferred from native K-I/ReturnRelease. |
| CRDM-I24 | R1 CPU cutoff, R2-F DMA/device fence and R2-O object-wide reclamation remain independently evidenced. |

**Caveat:** these are proposed invariants. A finite checker that never reaches a legitimate PublishStart or K-I could report no violations vacuously; future reviews must demand positive success witnesses and negative-control mutants, when separately authorized.

## 8. Decision proposals CRDM-D01…CRDM-D12 — ALL PROPOSED

- **CRDM-D01:** Separate V current-VA, O typed MemoryObject and R register destination identity instead of claiming one universal output success.
- **CRDM-D02:** Study bounded R control/Recovery and strictly fixed resident/private V in the first small-state semantic research profile.
- **CRDM-D03:** Keep O immutable/controlled MemoryObject release as a distinct larger-payload comparator; do not presume transparent zero-copy.
- **CRDM-D04:** Require L/M/P lifetime/mapping/publication authority to be checked together for every protected V slice.
- **CRDM-D05:** Keep VM M-DENY/M-DRAIN distinct from issuer P-DENY/P-DRAIN, and both from B/R1/R2-F.
- **CRDM-D06:** Strong owner handoff acknowledgement must state which exact publication and mapping scopes have actually drained.
- **CRDM-D07:** A current-VA output promise is invalid if the binding was only a pinned old frame after unmap/rebind.
- **CRDM-D08:** PrivateAtRelease is a temporary enforced alias policy, not permanent secrecy against authorized R forwarding.
- **CRDM-D09:** Keep whole K-S and K-I/native Capability move and inert TokenID rules independent from output delivery.
- **CRDM-D10:** Restrict faultable preflight and Pager waits to phases without locks/resources needed by the same Pager/M0.
- **CRDM-D11:** Never claim finite P-DRAIN from a byte-count bound alone; CPU/IRQ/preemption and termination are separate requirements.
- **CRDM-D12:** Retain all open Proposed/D0 statuses; no accepted ADR, formal-checker execution, numeric ABI or OS implementation.

## 9. Cross-RFC blockers CRDM-O01…CRDM-O16 — ALL OPEN

| ID | Priority | Exact unresolved question | Owner |
|---|---|---|---|
| CRDM-O01 | P0 | Is first-profile native receive V, O, R or an explicit combination? Which workload/size envelope justifies it? | RFC-003/009 |
| CRDM-O02 | P0 | At what cut is a VA-targeted successful delivery established despite unmap/remap and later reader observation? | RFC-003/004 |
| CRDM-O03 | P0 | Can VM provide bounded binding writes and M-DENY/M-DRAIN without Pager or unbounded invalidation wait? | RFC-004/005 |
| CRDM-O04 | P0 | Can issuer/Endpoint P-DENY and VM M-DENY serialize with a paired permit without global blocking lock? | RFC-002/003/004 |
| CRDM-O05 | P0 | Which typed MemoryObject/Map authority enforces PrivateAtRelease versus explicitly shared audience? | RFC-002/004/018 |
| CRDM-O06 | P0 | How are COW, hugepage split, pin, TLB and writable alias changes represented in actual mapping epochs? | RFC-004 |
| CRDM-O07 | P0 | Can R0 ReturnRelease be defined under preemption, TaskDeath and E-M with honest lost result? | RFC-001/005/009 |
| CRDM-O08 | P0 | Is full native N Grant K-I possible with R0 small return without orphan live grants or invisible HandleIDs? | RFC-002/003/009 |
| CRDM-O09 | P0 | How does revoke of a mandatory queued Grant before K-I terminalize with possible prior Vm/Vp/Vt exposure? | RFC-002/003 |
| CRDM-O10 | P0 | Does every action have finite precharged terminal/cancel/pin/lineage/receipt/token debt and M0 reserve? | RFC-002/004/005/006 |
| CRDM-O11 | P0 | What runtime wait-for graph proves no Pager/VM/IPC/recovery credit cycle? | RFC-003/004/005/006 |
| CRDM-O12 | P0 | Can Task/ServiceRestart safely stop old publishers before strong E1 handoff response or VM backing reclaim? | RFC-002/003/006 |
| CRDM-O13 | P1 | What initial desktop workloads can actually be carried by R without silent truncation? | RFC-003/009/018 |
| CRDM-O14 | P1 | Does O support immutable messages under user-driven mapping/share changes without a new TCB-heavy broker? | RFC-002/004 |
| CRDM-O15 | P1 | How should future finite model represent map/alias changes and weak versus strong cutoff acknowledgements? | Research/FM |
| CRDM-O16 | P1 | What hardware/SMP x86_64 TLB, scheduler/IRQ, DMA and refinement evidence will be needed later? | RFC-001/004/005/007 |

## 10. Paper-only D1 gate, follow-on study and safeguards

**Gate CRDM-R1: OPEN / NO-GO**, for both a general pageable native receive ABI and acceptance of any of draft ADR-CAPIPC-A/C/E. Reaching *paper-only D1 review readiness* requires:

1. **Exact user-facing output promise:** V current VA at actual store/finish cut, O typed MemoryObject version/range at object commit/release cut, or R Thread register ReturnRelease; no ambiguous dual meaning.
2. **One well-formed coherent paired M/P publish contract**, including competing denials, active publisher drain and bounded VM/backend ref charge, with refusal paths.
3. **Explicit private/shared alias policy:** who may declare shared delivery, who can create new aliases while release occurs and whether "private" lasts only at release; disallow unprovable perpetual confidentiality promises.
4. **No hidden atomic usercopy:** Vm/Vp/Vt and native full K-I are separate; NO_KI and possible disclosure can coexist.
5. **Canonical lock/wait/resource obligations:** Publisher, VM invalidation, Pager, Task scheduler/IRQ, M0, Claim, TokenID and DMA charges, under an honest finite or pending/fail-stop outcome.
6. **R0 envelope and ReturnRelease:** single bounded control/Recovery message with actual N-Grant capacity, status loss, TaskDeath and future non-truncating continuation assumptions specified.
7. **Paper trace matrix against each RFC owner:** demonstrate reachable safe success and exhibit unsafe interleavings if guards dropped; mere checklist completion isn't a proof.

**Recommended next design-only study:** **CAPIPC Paired Mapping–Publication Cut & Invalidation Ordering v0.1**, scoped to CRDM-O02/03/04/05 and a small P0-R ReturnRelease comparator. Prioritize a state-transition ordering with explicit hostile mapping invalidation and E1/issuer cutoff races *before* touching ABI choices or writing runnable models.

**No artifacts beyond documentation authorized:** no code, executable formal specification, kernel test, benchmark, numeric handle bit layout, ABI freeze or production assurance should follow automatically from this review.

## 11. Primary references and verification limits

- **[S1] Linux pin_user_pages:** https://docs.kernel.org/core-api/pin_user_pages.html
- **[S2] Linux MMU notifier replacement/COW ordering:** https://docs.kernel.org/next/mm/mmu_notifier.html
- **[S3] Linux kernel usercopy constraints:** https://docs.kernel.org/kernel-hacking/hacking.html
- **[S4] Linux sequence counters:** https://docs.kernel.org/locking/seqlock.html
- **[S5] Fuchsia zx_channel_read:** https://fuchsia.dev/reference/syscalls/channel_read
- **[S6] UniOS internal Proposed RFC and CAPIPC model papers**, linked above.

**Methods:** compared current GitHub main architecture/RFC-003/004/005/009 and successor CAPIPC research against first-party external technical manuals; specified paper-only alternatives, counterexample obligations, owner cuts and refusal criteria. No model checker or kernel execution, no tested copyout path, no measured latency and no evidence of physical quiescence. Sources are precedents, not imported implementation or security proofs.

---

**History:** 2026-10-26 initial Proposed Cross-RFC Decision Table v0.1. **18** cross-RFC decision rows, **26** paper-only adversarial schedules, **24** unproved predicates, **12** proposed research decisions, **16** OPEN blockers. No Accepted ADR or runnable implementation.

## 12. Follow-up: paired (M,P) logical Start and implementation readiness — 2026-10-26, Proposed

[CAPIPC Paired Mapping–Publication Cut & Invalidation Ordering v0.1](../models/capipc-paired-mapping-publication-invalidation-ordering-v0.1.md) specifies the **paper-only U-J paired Start** requirement: one coherent permission for both VM M and issuer P, per-scope charged active publisher and separate M-DENY/M-DRAIN and P-DENY/P-DRAIN. Sequential unlocked checks and postcopy retries are not substitutes; U-T two-ticket acquisition may deadlock without nonblocking rollback/ordered admission. The restricted register ReturnRelease U-R avoids dynamic user-buffer output but not issuer/current Task generation or whole-K-I receipt truth. See [UniOS Implementation Entry & Readiness Gate](unios-implementation-entry-readiness-gate-v0.1.md) for a separate **L1 experimental boot** vs **L2 native IPC** implementation-entry assessment, both currently NO-GO. No CRDM blocker marked closed, no code/model/ABI approved.

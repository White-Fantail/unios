# UniOS CAPIPC — Publication Permit & Destination Memory Contract v0.1

> **Status:** Proposed independent design/feasibility study — **2026-10-26 NZDT**. **NOT Accepted, formally verified, model-checked, implemented, benchmarked, tested, security-certified, or ABI-frozen.**
> **Constraint:** UniOS remains in research/design stage. This is **not** executable pseudocode, a working kernel algorithm, a chosen syscall ABI, an authorized TLA+/Alloy model, a test suite or an implementation request.
> **Purpose:** Deepen [Publication Barrier & Pager-Safe Copyout Feasibility v0.1](capipc-publication-barrier-pager-safe-copyout-feasibility-v0.1.md)'s P1 protected non-faulting slices versus P0 restricted register returns. Explicitly separate memory lifetime, logical receiver MappingGeneration, issuer audience/PolicyEpoch, publication PermitStart/P-DENY/P-DRAIN, native K-I and M0 resources.
> **Primary predecessors:** [Joint A/C/E Semantic-Cut Resolution](capipc-joint-semantic-cut-resolution-a-c-e-v0.1.md), [Unified SEND–RECEIVE Contract](../contracts/capipc-unified-send-receive-semantic-contract-v0.1.md), [Receive V/K-I/Public Token Study](capipc-receive-exposure-ki-handle-publication-v0.1.md), [Formal Model Preparation](capipc-formal-model-preparation-v0.1.md), [CAPIPC P0 ADR Readiness](../reviews/capipc-p0-conflict-resolution-adr-readiness-v0.1.md).
> **Owners:** [RFC-001](../rfcs/rfc-001-kernel-boundary-and-tcb.md) K/TCB, [RFC-002](../rfcs/rfc-002-object-capability-lifetime-delegation-revocation.md) Task/CapGrant/AddressSpace, [RFC-003](../rfcs/rfc-003-ipc-wire-protocol-and-synchronization.md) Claim/V/K-I, [RFC-004](../rfcs/rfc-004-virtual-memory-pager-cache-and-oom.md) mapping/pinning/COW/TLB, [RFC-005](../rfcs/rfc-005-scheduling-preemption-and-cpu-budget.md) bounded scheduling, [RFC-006](../rfcs/rfc-006-boot-trust-chain-service-startup-and-recovery.md) M0, [RFC-009](../rfcs/rfc-009-native-abi-and-system-call-contract.md) receipt/native error, [RFC-018](../rfcs/rfc-018-versioned-idl-abi-evolution-and-compatibility-contracts.md) issuer audience, plus [RFC-007](../rfcs/rfc-007-user-space-drivers-dma-and-iommu.md) for any independent physical DMA guarantees.

## 0. Executive decision research — keep semantics narrow and auditable

**Preferred paper candidate P1-M:** receiver-specific **short-lived, charged, read/write-eligible DestinationBinding** coupled with an audience/Endpoint-scoped **PublicationPermit for one finite, no-fault slice**. VM invalidation of that binding must serialize with actual access; issuer PolicyEpoch/Endpoint P-DENY blocks new permits; P-DRAIN is truthful only after every older publisher can perform **no additional store**. Entire claim/exposure/TokenID/Grant custody and K-I remain independent ledgers. **P1-M is plausible as a proof target, not demonstrated feasible on SMP**.

**Fallback paper candidate P0-R:** bounded **register-only** payload/Handle-ID return profile, with a single explicit result-publication authorization cut in the syscall/Thread-return contract. P0-R reduces pager-related risk but **does not remove** issuer/Task cut ordering, signal/preemption/return ambiguity, Task-wide K-I authority or public token/replay rules. It may not meet a general desktop's arbitrary-sized IPC API.

**Key feasibility discovery 1 — pinning ≠ binding:** pinning a page's physical lifetime does **not** pin the receiver's virtual-address-to-frame mapping, ensure COW has already resolved, prove the destination belongs to an authorized audience, prevent another Task from already mapping that backing, or make an arbitrary future usercopy nonfaulting. A DestinationBinding needs an independently reviewed, synchronized VM mapping/invalidation contract and access-path proof.

**Key feasibility discovery 2 — postcopy validation ≠ secrecy:** after writing possibly observable bytes, detecting a stale epoch and returning an error does not undo disclosure. This is particularly important when E1 Endpoint owner/issuer audience changes during copying.

**Key feasibility discovery 3 — nonfaulting ≠ bounded wall time:** even a no-fault finite instruction slice can be indefinitely preempted, interrupted, or parked if its CPU/IRQ/scheduler assumptions aren't bounded. A strong P-DRAIN response therefore requires an actual, admitted finite publisher progress/abort guarantee, **or must remain PENDING / fail safely**, not claim an unconditional deadline.

**Key feasibility discovery 4 — no confidentiality against authorized receiver:** If Task R is deliberately entitled to receive plaintext, it may read/copy/share that data using its other existing rights. The kernel cannot guarantee R never voluntarily or maliciously relays the bytes. Its native promise is **no publication caused by the kernel to an unintended Task/backing/epoch** under a stated shareability policy, not perfect downstream information-flow control.

**Verdict:** General-purpose P1-M is **D0 / OPEN, not D1-ready**, pending actual MemoryObject/MappingGeneration pin+invalidation rules. P0-R is a meaningful narrower architectural fallback but also OPEN. Neither is an Accepted ADR or ABI selection.

## 1. Source precedent versus unproved UniOS guarantees

| Official source | Factual precedent | Cannot be inferred for UniOS |
|---|---|---|
| [S1] Linux pin_user_pages() | Pinning pages has distinct short/long-duration reference and dirty/write implications; plain writable page operations require proper pin management | A pinned frame freezes Task virtual mapping, rejects shared aliases, or grants an information-release right |
| [S2] Linux MMU notifier/interval invalidation | Coordination of virtual-range invalidation and secondary mappings needs explicit notification and ordering, including COW/replacement cases | A version check after bytes are written undoes exposure; UniOS supports that specific Linux API |
| [S3] Linux usercopy/Memory Management API | User writes can fault/sleep; copying a simple value differs from bulk copy | arbitrary private user buffers can be written atomically with caps and Pager-free for free |
| [S4] Linux seqlock/seqcount | Sequence readers can detect concurrent writers but dereferencing invalidated pointers requires separate lifetime protection; preemption has caveats | A stale seq retry alone protects irreversible user stores or guarantees quiescence |
| [S5] Fuchsia zx_channel_read | Entire logical message, supplied bytes/handle outputs and defined copy order | UniOS gets Fuchsia's receive implementation, Handle object behavior, or its safety proof |

No third-party kernel API is proposed to be ported verbatim. The official documentation provides a **risk taxonomy and comparison**, not empirical UniOS validation.

## 2. Three separate rights to verify for every destination

| Component | Exact protected object/scope | Why pin or pointer check alone is insufficient |
|---|---|---|
| **L — lifetime** | exact frame/MemoryObject backing reference, COW identity, ownership and charge while kernel writes | pin may keep old frame alive but user VA can point elsewhere |
| **M — mapping** | (TaskGeneration, AddressSpaceGeneration, MappingGeneration, VA interval, actual backing set, write permission, alias policy) | virtual address validation precedes unmap/rebind, page split, COW and TLB changes |
| **P — publication authority** | (kernel m, current Claim, issuer Audience, PolicyEpoch, E1 EndpointGeneration, allowed payload class/slice) | VM writability is not authorization to publish message bytes |

**All three must hold for a permitted slice.** An AddressSpace MAP right belongs to VM administration; it does not grant arbitrary E1 message disclosure. A CapGrant lineage B denies selected new native rights but isn't the same as issuer PolicyEpoch P-DENY. Likewise P-DRAIN neither means the old plaintext was erased nor that actual DMA R2-F has completed.

### 2.1 The receiver-alias threat model is not an impossible non-sharing promise

A Task with valid native authority may deliberately map a shared writable MemoryObject with another Task, use preexisting IPC or write a network file to transmit its plaintext. UniOS cannot meaningfully guarantee no third party ever observes data after intentionally publishing to that Task. The **enforceable narrower publication contract** is:

1. The kernel writes only to exact backing currently authorized for R under the issuer's declared *delivery audience class*.
2. If the issuer requires **private destination backing at delivery**, reject shared/cross-Task writable aliases or copy to a newly private MemoryObject under a separately proven allocation and COW policy.
3. If shared delivery is explicitly authorized, record the broader audience as intentional; do **not** claim Task-private secrecy.
4. A malicious R that copies received bytes elsewhere is outside the kernel's at-most-once native capability transfer guarantee; separate sandbox policy and effect mediation may limit, not erase, that capability.

**Privacy caveat:** kernel scanning every possible mapping of a frame may be prohibitively expensive and racy. A stronger proposed option is a *typed private output MemoryObject contract* with controlled sharing and epoch reference, rather than an unchecked global reverse-map search. Such an object profile would add new VM semantics and must be reviewed independently.

## 3. Candidate state variables (paper contract, NOT code)

| Name | Logical domain | Owner | Meaning |
|---|---|---|---|
| TGen, ASGen, MappingGen | generation identities and alive/revoked state | Task/VM kernel | avoid stale PID/AddressSpace/VA aliases |
| m, ClaimGen, R[qR] | one native message and protected pending/final receiver receipt | IPC kernel | ensures exactly one authoritative Claim and result |
| PolicyEpoch, Audience | issuer-authorized publication scope | policy owner/kernel verifier | excludes stale old audience permissions |
| Eowner, Egen | at most one effective Task owner for E1 Endpoint | capability/IPC kernel | old Claim cannot migrate by ServiceID |
| DestBindingID | reserved exact VA interval, frame/backing identity and mapping generation | VM kernel | a typed stable destination relation, not just pointer |
| BackingSharingClass | PrivateToR / ExplicitlySharedToSet / Unknown | MemoryObject/VM | rules for allowed destination audience; unknown must not be silently private |
| PinRef, COWResolved | bounded lifetime/prepared-frame reference and actual write backing | VM/sponsor | pin does not imply mapping stable or alias authorized |
| PublicationPermit | NotIssued / Active(slice,epoch,binding) / Finished / Fenced | IPC+VM+issuer | exact bounded old-authority ability to write |
| P-DENY(scope), P-DRAIN(scope) | barrier-established vs old-publisher quiescence | policy/Endpoint controller | two different completion facts |
| Vp, Vt, Vm | monotone maybe-observable payload, TokenID, metadata | IPC kernel | first possible publication recorded before output |
| TokenState | ReservedInert / MaybePublishedInert / ActiveExactGrant / Retired | cap table/Task | native TokenIDs cannot rebind unrelated grants during live TaskGeneration |
| K-I phase, NativeGrantCustody | NoKI/Pending/CommittedKI/Terminal and Queue→R exact Grants | IPC/cap kernel | whole vector atomic independent of output copy |
| BoundCharge and M0Reserve | per-sponsor destination/publisher/pin/tombstone/claim and critical budgets | resource/boot kernel | old receipt GC never refunds live liabilities |
| Observation | caller knows / doesn't know actual result | caller vs protected receipt | lost output isn't automatic rollback |

**Invalid interpretation:** “Published” is not a proof any particular Thread fully read the buffer. It is a conservative irreversible possibility; the claimed consumer may observe only a prefix or nothing.

## 4. P1-M: two VM destination-binding alternative models

### 4.1 P1-MA: kernel write via pinned backing

The kernel resolves R's writable user pages, holds bounded backing pins and writes a staged message slice via an independently mapped kernel alias to those exact frames, without faulting in the publication section.

**Potential advantage:** the protected publication slice need not follow a user virtual mapping that can fault mid-store.

**Dangers:** it might write to *the old pinned frame* after R unmaps/rebinds its intended virtual range. The bytes might never reach the intended R VA, or could be visible through an already authorized/unauthorized shared alias. A pin alone prevents physical reuse only under the chosen pin lifetime assumptions; it does not block Task remap, COW or cross-Task sharing. Without synchronized MappingGeneration and allowed-backings agreement, the user-facing contract “copy to this R buffer” cannot be claimed.

**Necessary semantic condition:** if mapping changes during binding, either (a) invalidation prevents the slice from starting and/or fences ongoing writes before the mapping/binding is retired, or (b) a separately agreed contract delivers into a particular reserved MemoryObject rather than promising a stable numeric VA. Which is intended must be explicit at D1.

### 4.2 P1-MB: VM mapping lease/freeze for bounded slice

The VM authority coordinates an actual short-lived destination binding with unmap/COW/write-protect/map-replacement operations. A bounded publisher operates on a precisely identified mapping/backing until a Finish/Fence event; invalidators must either precede PermitStart or wait/fence existing stores before saying binding retired.

**Potential advantage:** stronger “this exact R VA interval maps to these writable frames” claim at the publication cut.

**Dangers:** malicious R can request conflicting unmap while a publisher is preempted; VM invalidation waiting on a Pager-blocked or preempted writer can create starvation. A lease/freeze is not proven finite merely because bytes-per-slice is small. Forcing a long nonpreemptible copy risks IRQ/M0 latency and SMP contention. VM needs per-binding accounting, canonical conflict order and an admitted maximum publisher hold interval.

**First paper research priority:** compare **P1-MB for a limited fixed-size resident/private destination** versus P1-MA with **MemoryObject-bound, not VA-bound, delivery**. Neither branch is selected or regarded as feasible until mapping/alias/progress guards are specified.

### 4.3 Shared backing / COW / hugepage / DMA corner cases

- **COW:** read-only/shared page may be writable only after a fault, replacement or private copy. “Pre-faulted once” cannot imply subsequent coherent writable backing unless the prepared binding serializes against COW invalidation.
- **Hugepage split / migration / compression:** physical backing set may change even while logical VA appears stable; corresponding VM generation/pin/ref and TLB invalidation must account for it.
- **Unmap and remap to foreign-backed page:** stale address translations must not redirect a protected usercopy into another Task's private object without authorization.
- **Writable aliases:** a pinned frame can have other aliases. “No unintended kernel write” requires accurate issuer allowed-audience class; “no receiver ever leaks plaintext” is outside native IPC's possible guarantee.
- **Device/IOMMU alias:** physical DMA and R2-F need independent DeviceGroup rights/fences. CPU publication P-DRAIN is not a hardware drain.
- **Task death:** an in-flight binding must not be deallocated or adopted by a new TaskGeneration before all old writers are fenced; stale display PID/ServiceID equality provides no authority.

## 5. P1-M publication, VM invalidation and issuer-barrier ordering

### 5.1 Distinct ordering requirements

1. **Prepare state**: faultable staging and destination resolution/charges occur before Protected PermitStart; no IPC/global VM lock is held across Pager waits.
2. **Mapping invalidation gate M-INV:** VM invalidation of the same DestBinding may start, but old binding cannot retire its frame/VA semantic contract while an authorized publisher might still write. It must serialize/safely fence actual stores; revalidate generation at start alone is not enough.
3. **P-DENY(scope):** blocks *new* old-Audience/Endpoint/PolicyEpoch permits. It can be acknowledged separately from the full drain state.
4. **Publication slice:** Vp or Vt is conservatively marked before first possibly observable write; short non-faulting store operation, bound to both destination M and audience P; never initiates Pager/ordinary allocation.
5. **PermitFinish/Fence:** proves no further writes from that permit, including other CPU work/interrupt-preempt paths, before releasing active count/reference.
6. **P-DRAIN(scope):** only after all old permits are fenced. A strong Endpoint/issuer handoff acknowledgement that promises no old writes must depend on P-DRAIN, otherwise it must explicitly report drain pending.
7. **K-I:** independent coherent all-N-Grant install+message retirement+receiver protected receipt; current native rights and Grant revoke B rechecked at its own cut. P-DRAIN is not K-I.

**Potential ordering cycle:** waiting for P-DRAIN while holding a VM or IPC global lock that a pending publisher/Pager/M0 needs is prohibited. If a publisher is preempted indefinitely after acquiring a permit, even zero faults do not guarantee bounded P-DRAIN. Research must specify scheduler/preemption/IRQ assumptions, or admit unbounded pending/fail-stop.

### 5.2 Publication after Endpoint owner changes

If E1 owner moves B→C while B already has an exclusive Claim, **the Claim must not migrate** merely because C now owns the Endpoint. Old B may have partially observed secret payload. A clean handoff contract must distinguish:

- owner-transition requested;
- **P-DENY** on old audience/Endpoint;
- old published/copying slices **P-DRAIN** confirmed (or honestly pending);
- previous Claim same-audience continuation or terminal cleanup;
- C's new authenticated ability to claim later messages under its own Task/Egen and PolicyEpoch.

An existing source K-S is not rolled back by this receive failure. The same m is not ordinarily redelivered to C after relevant V. New C may receive other messages, assuming queue and ownership rules permit it.

### 5.3 In-flight permission is not retroactive secrecy

If a slice was authorized and partially wrote bytes **before** P-DENY, those bytes may already have been read, forwarded or copied. Even if all remaining stores are successfully fenced, P-DRAIN does not claim prior disclosure erased. If the product requires a stronger “from the instant revoke is requested, even prior slices never expose further bytes,” that needs a separate immediately synchronous effect contract and likely must delay return acknowledgement until actual drain, without asserting finite completion in hostile Pager/CPU conditions.

## 6. P0-R: register-only return compared under same trust contract

| Dimension | P1-M fixed resident/private output | P0-R bounded register/Thread-state output |
|---|---|---|
| destination lifetime | frame/MemoryObject pin and mapping generation proof | Task/Thread context and CPU trap-return state lifetime |
| memory remap/COW dependency | substantial; can be reduced only by exact DestBinding | none for output bytes if really all returned in registers |
| issuer audience and E1 owner | current Claim/Egen/PolicyEpoch and publication PermitStart | **still required** at the actual result-release cut |
| publication under preemption | bounded store slice needs scheduler/IRQ bound | preemption between preparing saved registers and user resume needs defined release cut |
| payload size / HandleIDs | finite slices could support larger bounded messages under accounting | hard count/width limitation, no silent truncation of native whole-message contract |
| K-I and public bearer grants | all-or-none install distinct from usercopy | **same** whole-vector K-I; other Task Threads can use grants before caller sees register return |
| lost result and qR receipt | R[qR] actual vs observer UNKNOWN | **same** result-loss/receipt reconciliation, no remint on query |
| M0/Pager dependency | pre-resolve can page-fault outside permit; prepared publish ideally nofault | much smaller receive output Pager surface; general service requests still use memory elsewhere |
| future complexity | VM binding, pin, alias, publication fence and cleanup | ABI/register pressure, continuation/message-size policy, scheduler return-state proof |
| status | **Proposed, unproved** | **Proposed, unproved** |

### 6.1 The subtle return boundary for P0-R

A kernel may write values to a saved Thread trapframe before that Thread resumes. The candidate must choose when these values count as **authorized for user observation** relative to issuer P-DENY and Endpoint E-M.

**Comparator R0-A (candidate):** designate a protected \`ReturnRelease\` cut that authenticates R's actual Task/Thread generation, issuer audience and Egen. After release, values are treated as **potentially disclosed**, even if return to userspace is delayed or the Thread is terminated before reading registers. Later P-DENY cannot retract already-released information. Release must be coherent with saved register identity and task death, not just a user-space instruction that races revocation.

**Comparator R0-B:** try to revoke already prepared yet not returned register values after policy change, up until the CPU actually resumes execution. This requires an additional per-CPU/Thread return gate and cross-core ordering, and may be expensive or impossible to promise without a stronger scheduler/CPU contract. **No design approved.**

A short register return path narrows memory hazards but does not eliminate *information publication ordering* or the fact that native K-I grants are Task-wide before the caller observes the return.

### 6.2 Honest feature limitations

If target IPC messages exceed R0's eventual fixed ABI return capacity, do **not** split a single all-or-none native Grant vector into independently committed half-messages without a new explicit continuation/reassembly and receipt protocol. A payload continuation can copy data under separate audience publication events but should not masquerade as a single atomic user-visible receive. R0-R may be suitable for initial control messages, faults, small RPC and recovery paths; it cannot be claimed to support large general-purpose desktop IPC without workload evidence.

## 7. Candidate abstract transitions DM-T01…DM-T20 (NOT executable code)

| ID | Action | Guard / required result |
|---|---|---|
| DM-T01 | Receiver A(qR)/R_R reservation | authenticate actual TaskGeneration, qR, quota before V/K-I |
| DM-T02 | K-C audience Claim | exact queued m, E1 EndpointGeneration, issuer PolicyEpoch, exclusive Claim and sponsor |
| DM-T03 | Stage immutable message | real bytes/typed GrantIDs; no trusted sender-authored SABI rights |
| DM-T04 | Resolve writable destination | TaskGen/ASGen/MappingGen/VA/backing/COW rights and any shared-alias policy |
| DM-T05 | Charge pinned/resident binding | finite frames, mapping, Task death and M0 independence |
| DM-T06 | Competing VM M-INV | serialize with active DestBinding; no silent frame recycle or wrong-VA promise |
| DM-T07 | P-DENY old audience | no *new* old permit after protected issuer/Endpoint cut |
| DM-T08 | Acquire paired (M,P) permit | both destination binding and issuer scope current, active-publisher budget |
| DM-T09 | Record Vm/Vp/Vt as relevant | conservative before first possible store/return publication |
| DM-T10 | Bounded no-fault slice | actual backing bound, no allocation/fault/globally blocking lock |
| DM-T11 | Finish/Fence old publisher | no future stores by old permit, including SMP/scheduler paths |
| DM-T12 | P-DRAIN | all older permits fenced, not a plaintext erasure guarantee |
| DM-T13 | Release DestBinding | after active writers stopped + VM refs/charge safe, not just Task-death assertion |
| DM-T14 | Abort/fault before permit | fail/settle without publication, or retain accurate previously exposed flags |
| DM-T15 | Vt inert TokenID output | exact Task-local TokenIDs nonrebind after possible exposure |
| DM-T16 | K-I exact whole-vector install | current live Claim/Egen/Grant/revoke; one queue retire+R_R commit |
| DM-T17 | TerminalNoKI | future K-I prevented; exposure/lineage/sponsor retained as necessary, no foreign m requeue after V |
| DM-T18 | P0-R ReturnRelease | audience-authorized Thread/Task register-result publication cut, independent of K-I |
| DM-T19 | TaskDeath/E-M/ServiceRestart | old Claim/publisher fenced, no new Task name-based adoption |
| DM-T20 | Protected Query/GC/M0 cleanup | honest UNKNOWN/EXPIRED, no cap remint, independent DMA R2-F and sponsor charges |

**Future refinement challenge:** DM-T08 versus T06/T07 must serialize coherently *together* with DM-T10/T11 for the exact Task/backing and PolicyEpoch. A checker treating the whole sequence as one atomic action would hide the very race this paper is intended to investigate.

## 8. Adversarial schedules DM-X01…DM-X28 (paper-only, NONE EXECUTED)

| ID | Scenario | Unsound behavior to reject |
|---|---|---|
| DM-X01 | pin page, R unmaps VA, maps different backing, P1 copy by old frame | claims success at requested VA even though bytes went elsewhere |
| DM-X02 | pin frame shared with unauthorized C, kernel writes confidential data | pin treated as secrecy despite alias |
| DM-X03 | COW resolves after precheck into new frame | stale physical destination or fault in claimed no-fault slice |
| DM-X04 | hugepage split/migration during active DestBinding | stale backing/TLB use |
| DM-X05 | M-INV wins after checking MappingGen but before PermitStart | stale mapping accepted |
| DM-X06 | P-DENY wins before PermitStart but old CPU cache sees stale epoch | new forbidden slice permitted |
| DM-X07 | PermitStart wins, P-DENY occurs, strong handoff returns before Finish | FALSE P-DRAIN |
| DM-X08 | TokenID first element published, second faults | old token rebinds unrelated Grant |
| DM-X09 | page present at prepare, paged away after it, copy faults while permit active | no-fault premise broken |
| DM-X10 | another Task already maps writable backing via shared MemoryObject | kernel incorrectly promises only R ever sees it |
| DM-X11 | R intentionally sends bytes using existing IPC after valid publication | wrong assumption kernel can guarantee no onward disclosure |
| DM-X12 | VM invalidation waits for publisher while publisher waits Pager holding VM lock | circular wait/M0 deadlock |
| DM-X13 | CPU preempts a nonfaulting publisher forever | bounded byte count falsely implies bounded wall-clock drain |
| DM-X14 | IRQ storm prevents publisher finish; M0 assumed automatically available | incorrect unbounded progress guarantee |
| DM-X15 | issuer P-DENY cuts while R Task is dying, old kernel writer resumes | post-cut old-permit store without legitimate drain |
| DM-X16 | Endpoint B→C while original B Claim exposed Vp | m automatically retargeted to C |
| DM-X17 | K-S committed vector [g1,g2], g2 revoked, later K-I | partial N−1 success or active revoked grant |
| DM-X18 | K-I succeeds, result lost, querying qR | duplicate cap vector installation |
| DM-X19 | P0 register values prepared, issuer P-DENY, Thread later resumes | unmodeled ReturnRelease ordering |
| DM-X20 | P0 ReturnRelease commits, Thread terminated before observing | false assertion no data possibly disclosed |
| DM-X21 | R0 payload too large for registers; kernel silently truncates | whole-message semantics broken |
| DM-X22 | Task R shares backing with C and policy expressly permits it | overly strong rejection mismatches explicit allowed-audience contract |
| DM-X23 | private destination promise relies on racy global reverse mapping lookup | new alias opens during “proved private” interval |
| DM-X24 | TokenID output stale, TaskGeneration R2 with same display ID | cross-generation grant confusion |
| DM-X25 | qR receipt GC while active Claim/DestBinding/pin charge persists | prematurely freed debt or reactivated q |
| DM-X26 | TaskDeath frees pinned frame before P-DRAIN or DMA R2-F | use-after-recycle |
| DM-X27 | wall-clock timer expires while scheduler cannot run cancellation | falsely completed publication drain |
| DM-X28 | read-only alias to another Task sees copied plaintext via shared backing | privacy claim excludes read-only alias incorrectly |

## 9. Unproved invariants DM-I01…DM-I26

| ID | Formalizable intended property |
|---|---|
| DM-I01 | DestinationBinding always identifies actual TaskGeneration/ASGen/MappingGen and exact backing+allowed visibility. |
| DM-I02 | A frame pin/MemoryObject ref never alone authorizes K-C publication or guarantees VA identity. |
| DM-I03 | Every pending publisher holds protected lifetime credits for its exact backing until verified Finish/Fence. |
| DM-I04 | VM M-INV and permitted memory writes have one consistent ordering: no stale binding store after a completed invalidation fence. |
| DM-I05 | PermitStart and issuer P-DENY serialize for the same actual Claim/Audience/PolicyEpoch. |
| DM-I06 | No new old-scope publication permit is issued after P-DENY. |
| DM-I07 | P-DRAIN means no existing old permit can execute another store; earlier bytes may remain known. |
| DM-I08 | Vm/Vp/Vt may-exposure is monotone and marked prior to the first relevant output operation. |
| DM-I09 | Current destination allowed-audience policy is not falsely inferred from writable permission or physical pin. |
| DM-I10 | The private-to-R guarantee, if claimed, has active enforcement against foreign writable **and readable** aliases within stated scope. |
| DM-I11 | Authorized R may relay legitimately seen bytes; no kernel all-future secrecy claim is made. |
| DM-I12 | P1 public proof profile performs no Pager-fault or new allocator dependency inside active bounded slice. |
| DM-I13 | A slice's actual CPU/preemption/IRQ progress assumptions are explicit; byte bound alone isn't wall-clock bound. |
| DM-I14 | No global IPC/VM/Publisher lock used by Pager/M0 is held across faultable preflight or waiting drain. |
| DM-I15 | P-DENY is not mistaken for CapGrant B or actual CPU map R1/DMA R2-F. |
| DM-I16 | A strong owner-handoff acknowledgement promising old publication stopped needs actual P-DRAIN. |
| DM-I17 | Old Claim and publication authority do not migrate by Task/ServiceIncarnation display name. |
| DM-I18 | K-I installs all exact currently authorized N GrantIDs or none; receipt/queue history coherent. |
| DM-I19 | Pre-K-I TokenIDs remain inert; possibly exposed IDs never rebind unrelated Grant in that TaskGeneration. |
| DM-I20 | NO_KI can coexist with possible Vm/Vp/Vt exposure; observer UNKNOWN never entails clean retry. |
| DM-I21 | R0 ReturnRelease authority is independently ordered against P-DENY, TaskDeath and Endpoint E-M. |
| DM-I22 | R0 limited payload cannot silently truncate or split one promised whole native message/grant vector. |
| DM-I23 | Claim/pin/TokenID/receipt/destruction/DMA charges survive final GC/Task death until actual settlement. |
| DM-I24 | Ordinary Task quotas cannot borrow independently wired M0 CPU/frame/IPC/IRQ/stack/cleanup budgets. |
| DM-I25 | Task-held bearer native CapGrants after K-I may be used by its other Threads; receipt does not promise current h invokability. |
| DM-I26 | P-DRAIN/K-I/Q receipt cannot certify backend effect, physical DMA fence or storage durability. |

**None of these invariants have been mechanically checked, and some require a richer physical mapping/CPU history abstraction than the earlier paper finite models.** Do not weaken mapping/alias assumptions silently just to make a small model pass.

## 10. Comparative feasibility and Trust/TCB audit

| Criterion | P1-MA pinned backing | P1-MB mapping lease/freeze | P0-R registers | P2 protected mailbox |
|---|---|---|---|---|
| VA identity fidelity | **weak without explicit MemoryObject-bound ABI** | potentially stronger if invalidation serialized | not applicable to output bytes | user mapping/view semantics shift burden |
| backing lifetime | pinned frame refs required | pinned/protected binding plus mapping refs | register/Thread state | kernel-owned immutable pages/refs |
| explicit issuer publication cut | required | required | required at ReturnRelease | required at view release |
| user Pager during final output | avoidable if kernel alias truly prepared | avoidable if binding and COW stable | absent for register payload | may fault later on authorized mapping access |
| destination shared alias exposure | a major issue | explicit share-policy enforcement necessary | no user-buffer alias issue for returned bits | share-map and preexisting aliases remain |
| bounded strong P-DRAIN | conditional on finite slice+CPU fences | conditional on lease+VM/scheduler proof | simpler small return but release/preemption race | mapping R1/TLB cutoff is distinct |
| quote/credit cost | pin/VM refs and tombstones | map-invalidators/pins/ref and lock DAG | restricted ABI/receipt and token namespace | VM object sponsor and mapping/TLB |
| app/desktop message size | flexible bounded slices | flexible bounded slices | very limited | potentially larger immutable payload |
| readiness now | **D0 OPEN** | **D0 OPEN** | **D0 OPEN** | **D0 OPEN** |

**First design paper recommendation:** investigate **P1-MB** with a *small fixed resident/private buffer contract* first and concurrently assess **P0-R** for native control/Recovery IPC. P1-MA may be useful for an explicitly MemoryObject-identity-targeted API; it must not misrepresent a stale pinned page as the caller's currently requested VA. Keep P2 for a separate RFC-004 typed MemoryObject policy review. No numerical performance/latency or ABI evidence currently exists to declare one “best”.

### 10.1 Relative trade-off estimate (qualitative only)

- **P1-MB** increases VM/IPC TCB coupling and risks waiting on hostile invalidation/cancellation; it may support general native IPC if bounded resident output is workable.
- **P0-R** substantially reduces the pageable-copyout proof surface, but register pressure and all-or-none message limits may force an explicit new protocol. A microkernel often needs more than a few machine words per message.
- **P2 mailbox** can avoid certain copies but complicates frame ownership, TLB coherency, immutability and data sponsorship; not free “zero-copy”.
- **P1-MA** has a particularly subtle **delivery destination truth** problem when R unmaps/rebinds after pin. It may be correct for a MemoryObject-targeted contract but wrong for an “exact current VA” promise.
- **No numerical scoring can replace a real bounded execution/MemoryObject proof**, so the alternatives remain unranked for deployment.

## 11. Candidate decisions DM-D01…DM-D12 — NONE Accepted

- **DM-D01:** Distinguish physical lifetime L, exact logical mapping M and issuer publication P as independent mandatory conditions.
- **DM-D02:** Keep P-DENY new permit cutoff distinct from P-DRAIN older writer quiescence and from native B/R1/R2-F/O.
- **DM-D03:** Define DestinationBinding as actual Task/AS/MappingGeneration, allowed backing audience and bounded sponsor/ref, not raw VA or pin alone.
- **DM-D04:** Study P1-MB mapping lease/freeze for fixed resident/private buffers first; no implementation feasibility claimed.
- **DM-D05:** Permit P1-MA only under an explicit backing-MemoryObject delivery contract or equivalent VA-binding proof.
- **DM-D06:** Keep P0-R register-only bounded IPC as independent first control/Recovery proof fallback.
- **DM-D07:** Make P0-R ReturnRelease a genuine publication cut; preparing saved CPU registers does not automatically settle issuer cutoff races.
- **DM-D08:** If receiver knowingly shares its buffer or forwards plaintext, kernel can't claim permanent confidentiality inside a legitimately authorized Task.
- **DM-D09:** Keep all-N K-I, inert TokenIDs, Task-global bearer T1 grants, protected qR and truthful result-loss status independent of copyout.
- **DM-D10:** Require actual CPU/IRQ scheduling and VM invalidation fences for bounded drain; no “nofault means never preempted” shortcut.
- **DM-D11:** Preserve M0 critical reserves and all Claim/TokenID/pin/DMA sponsor liabilities across Task/receipt GC.
- **DM-D12:** All variants remain Proposed, D0/OPEN; no accepted ADR, tool run, syscall ABI or kernel implementation.

## 12. Unresolved questions DM-O01…DM-O16 — ALL OPEN

| ID | Priority | Blocking issue | Owner |
|---|---|---|---|
| DM-O01 | P0 | Who issues binding and audience authority; what protected grant allows checking allowed data-sharing recipients? | RFC-002/003/018 |
| DM-O02 | P0 | Does P1 native ABI promise delivery to a VA range or to a typed immutable MemoryObject backing? | RFC-003/004/009 |
| DM-O03 | P0 | Can a bounded VM lease/freeze actually guarantee nonfaulting writes under COW/unmap/split/TLB? | RFC-004 |
| DM-O04 | P0 | What shares/aliases are allowed and how are private/explicitly-shared guarantees enforced without racy reverse lookup? | RFC-002/004 |
| DM-O05 | P0 | How exactly do M-INV, PermitStart, P-DENY and P-DRAIN serialize across CPUs? | RFC-002/003/004/005 |
| DM-O06 | P0 | What scheduler/preemption/IRQ bounds let the system report P-DRAIN, even if a publisher stalls or CPU hotplugs? | RFC-005/006 |
| DM-O07 | P0 | Which preparatory operations may Pager-fault and which live permit operations provably cannot? | RFC-004/006 |
| DM-O08 | P0 | Can K-I whole native Grant install occur without requiring a faulting output or leaking Task-wide rights too early for T2? | RFC-002/003/009/018 |
| DM-O09 | P0 | Which owner/PolicyEpoch change promises DENY only versus strong DRAIN acknowledgement? | RFC-002/003/018 |
| DM-O10 | P0 | Are enough finite credits precharged for Claim/pin/DestBinding/publisher/tombstone/receipt/terminal and M0? | RFC-002/004/005/006 |
| DM-O11 | P0 | Can TaskDeath/fault/Endpoint E-M terminalize an exposed m without old-publisher store or foreign redelivery? | RFC-003/004/006 |
| DM-O12 | P0 | How does P0-R bind ReturnRelease to Thread return/trap context and issuer epoch without arbitrary code/memory pollution? | RFC-001/005/009 |
| DM-O13 | P1 | Which initial UniOS control/Recovery messages fit P0-R while preserving whole-message/Grant semantics? | RFC-003/006/009 |
| DM-O14 | P1 | Is MemoryObject-targeted delivery or protected mailbox a better eventual large-payload path than freezing user buffers? | RFC-003/004 |
| DM-O15 | P1 | What compact finite-model abstraction catches memory alias/wrong VA/store-after-drain counterexamples without unjustified atomicity? | Formal research |
| DM-O16 | P1 | How will future concrete x86_64 SMP/TLB/CPU fences be validated against the written abstract bindings? | RFC-001/004/005 |

## 13. D1 semantic review gate and next research

**Current gate DM-R1: OPEN / NO-GO.** The following *paper* artifacts are required before any D1-ready candidate or accepted later ABI:

1. A defined native receive output contract: **exact VA or exact MemoryObject backing identity**, plus issuer's audience/shared mapping policy; reject accidental mixing of both promises.
2. For P1-MB, explicit VM binding/lease invalidation ordering, COW/alias/reclaim behavior and bounded protected refs without Pager locks.
3. For P1-MA, demonstrate why stale VA after pinned-frame publication is not a violation **under the selected MemoryObject-based output API**, or reject it for exact VA delivery.
4. A two-stage P-DENY/P-DRAIN plus VM M-INV correctness matrix including SMP preemption, TLB and TaskDeath/ServiceRestart.
5. A P0-R small-message envelope and register ReturnRelease, K-I/receipt/fault/lost-result case table without numerical syscall ABI allocation.
6. A proof-level wait-for/resource-credit ledger separating ordinary receiver, Pager fault waiter, issuer handoff, M0 and DeviceGroup DMA drain.
7. Explicit negative-control interleavings: remove mapping generation check, allow old-permit store after P-DRAIN, allow unauthorized shared alias, let revoked grant K-I succeed, or alias stale TokenID to new GrantID.
8. Before **any** executable model/checker or kernel source, get separate user authorization; formal run pass alone is not an x86_64 kernel implementation proof.

**Strong recommended next design-only milestone:** **CAPIPC Destination Memory & Publication Barrier Cross-RFC Decision Table v0.1**, concentrating on RFC-004/003 boundaries: “VA-targeted vs MemoryObject-targeted output”, private vs explicitly shared audiences, M-INV/P-DRAIN and P0-R ReturnRelease under Task/Endpoint generation change. That is the smallest next paper capable of clarifying which contracts can even be separately modeled and checked without hiding a semantic contradiction.

## 14. First-party source links and evidence limits

- **[S1] Linux pin_user_pages and physical page lifetime (pin ≠ mapping/authorization):** https://docs.kernel.org/core-api/pin_user_pages.html
- **[S2] Linux MMU notifier and COW/secondary mapping ordering:** https://docs.kernel.org/next/mm/mmu_notifier.html
- **[S3] Linux Memory Management APIs/usercopy and faultable put_user:** https://docs.kernel.org/core-api/mm-api.html
- **[S4] Linux sequence counters limitations (invalidated pointers, writer scheduling):** https://docs.kernel.org/locking/seqlock.html
- **[S5] Fuchsia Channel read logical whole message and separate buffer outputs:** https://fuchsia.dev/reference/syscalls/channel_read
- **[S6] UniOS Proposed RFCs and model studies:** direct repository links in header.

**Methods and limits:** reviewed UniOS current main-branch proposals and official published first-party kernel manuals, then performed adversarial reasoning on abstract contracts. No actual kernel model/tool, experimental latency, code, source portability or implementation evidence. External Linux/Fuchsia mechanisms are analogies, not guarantees available in UniOS.

---

**History:** 2026-10-26 initial Proposed Destination Memory Contract v0.1. Candidate P1-MA (physical backing) versus P1-MB (mapping lease), P0-R register ReturnRelease, P2 mailbox; three independent rights L/M/P; **20** candidate transitions, **28** paper-only failure schedules, **26** unproved invariants, **12** Proposed decisions and **16** open P0/P1 questions. No implementation or accepted ABI.

## 15. 2026-10-26 Cross-RFC Destination/Publication Decision Table — Proposed

[CAPIPC Destination Memory & Publication Barrier Cross-RFC Decision Table v0.1](../reviews/capipc-destination-memory-publication-cross-rfc-decision-table-v0.1.md) refines DM-O02/03/04/05 into a three-profile research split: R register-only bounded control with an explicit ReturnRelease, V fixed resident/private VA-to-backing mapping lease, and O typed MemoryObject output rather than an implicit current-VA guarantee. It introduces separate VM M-DENY/M-DRAIN and issuer P-DENY/P-DRAIN plus a **paired (M,P) PublishStart** whose protected permission must not outlive either current mapping or issuer audience epoch. Page pin alone isn't an exact VA or private-audience proof. 18 decision rows and 26 paper counterexamples add **no verified property**, accepted ADR, executable model, source code or numeric ABI; this paper and DM-R1 remain Proposed/Open.

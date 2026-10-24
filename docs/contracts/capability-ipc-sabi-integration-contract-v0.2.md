# UniOS — Capability · IPC · SABI Joint Integration Contract v0.2

> **Status:** Proposed, independent adversarial re-review (2026-10-25 NZDT). **Not Accepted, Stable, implemented, tested, model-checked or formally verified.**
> **Supersedes for future research:** [CAPIPC v0.1](capability-ipc-sabi-integration-contract-v0.1.md); v0.1 remains available as an immutable historical design document in Git.
> **Authoritative proposed source RFCs, all still Proposed:** [RFC-002 Capability](../rfcs/rfc-002-object-capability-lifetime-delegation-revocation.md), [RFC-003 IPC](../rfcs/rfc-003-ipc-wire-protocol-and-synchronization.md), [RFC-009 Native ABI](../rfcs/rfc-009-native-abi-and-system-call-contract.md), [RFC-018 Versioned IDL](../rfcs/rfc-018-versioned-idl-abi-evolution-and-compatibility-contracts.md).
> **Co-reviewed boundaries:** [RFC-004 Pager](../rfcs/rfc-004-virtual-memory-pager-cache-and-oom.md), [RFC-005 Scheduling](../rfcs/rfc-005-scheduling-preemption-and-cpu-budget.md), [RFC-006 Boot/M0](../rfcs/rfc-006-boot-trust-chain-service-startup-and-recovery.md), [RFC-007 Device/DMA](../rfcs/rfc-007-user-space-drivers-dma-and-iommu.md), [RFC-008 Storage](../rfcs/rfc-008-storage-filesystems-and-crash-durability.md), [RFC-010 Linux guest](../rfcs/rfc-010-linux-personality-and-binary-compatibility.md), [RFC-017 Verified Boot](../rfcs/rfc-017-verified-boot-updates-and-rollback-protection.md); [Cross-RFC audit](../reviews/cross-rfc-architecture-consistency-audit-v0.1.md).
> **STRICT SCOPE:** research and text only. No kernel, IDL, SDK, IPC, cap, test, TLA+, proof model, emulator, OS implementation or binary compatibility support is authorized. This document is a **design hypothesis and proof obligation registry**, not an implementation plan.

## 0. Executive outcome — CAPIPC-A survives, receive contract needs a stronger cut

**Overall:** retain the conditional CAPIPC-A family — type/rights-checked Task-local caps, one E1 owner per Endpoint, bounded move-only kernel transport, versioned userspace SABI — but v0.1's statement that an exclusive ReceiveClaim suffices to guard against unauthorized disclosure was **too weak**.

**Critical new v0.2 conclusion:** the security-relevant event is **the FIRST VISIBILITY of any sensitive message byte to untrusted receiver-controlled user memory**, not merely the final cap-install/queue-removal linearization. User-memory disclosure is irreversible and can be observed by any concurrently executing Thread or shared mapping alias. If bytes have been published to B, the message MUST NOT subsequently become claimable by a *different* Task C under a naive rollback/requeue path. Either prove that no bytes could have been exposed, or terminalize/discard/quarantine the message with accounted resources and honest failure. An exclusive ReceiveClaim prevents concurrent claimants; it does **not** erase observations after abort.

**Other v0.2 tightening:**
- An E1 Endpoint MOVE cannot overlap an in-flight SEND/RECEIVE without a chosen **invocation admission/order cut**. A different Task can't become an effective owner while a previous Task creates new operations outside its admitted scope.
- A valid cap transferred into receiver custody is a **resource with authority**, not innocuous bytes. If schema parse happens after handle install, prevent unexpected handles being usable outside admitted method policy, or fail/settle in bounded fashion; a kernel parser cannot infer all SABI resource roles.
- Post-SEND COMMITTED caller-report loss can never be fixed merely by assigning a **fresh** operation ticket in the response that was itself lost. Either a pre-identified authenticated operation record is discoverable by a bounded method, or caller outcome stays UNKNOWN/fail-closed.
- Revocation B can race with SEND prepare, commit, ReceiveClaim, install and SABI method admission. The selected cut must specify what is denied, admitted, inert, closed and independently hardware-fenced, rather than assuming all forms of revoke are a rollback.
- A sender must not be allowed to form an unbounded **self-referential Channel/cap graph** via a message's transferred Endpoint unless cycles have precise cleanup/sponsorship; Fuchsia directly forbids sending the endpoint used for a write [S2]. Reverse/peer ownership and broader cycles are a separate question.
- Bytes buffer and output handle/status buffer **overlap** can overwrite or expose unexpected data; simply checking each pointer separately is inadequate [S3]. Permit overlap only if the exact observable contract is proven; initially propose rejection of dangerous overlap.
- Cancelling the receiver, client or service after a partial publication is not equivalent to revoking disclosure, remote effect or already moved cap authority.

**Version policy:** v0.2 is a new Proposed *companion/re-review artifact*, not a claim that all source RFCs have accepted its stronger invariants. Original RFC-003 E1 DERIVE text inconsistency was already editorially corrected; concurrency and receive safety proof remain unresolved.

## 1. Official comparison and caveats

| Primary external source | Documented behavior (not UniOS) | Re-review consequence |
|---|---|---|
| **[S1] Fuchsia Channel objects** | nonduplicable endpoint, unique owning handle, ordered messages across transfer; unread queue destruction can recursively close handles | E1 single-owner and possible cap/Channel reference cycles require explicit owner/order/drop rules |
| **[S2] Fuchsia zx_channel_write** | channel write moves handles (Fuchsia basic API may consume even on error), rejects source endpoint in its own transfer vector | avoid silent adoption of failure consumption; define self-referential message policy |
| **[S3] Fuchsia zx_channel_read** | message must be read as a whole from queue; bytes are written before output handle array and overlapping buffers can overwrite bytes | "one dequeued message" is not proof that all receiver memory writes are atomic or nonobservable before success |
| **[S4] Fuchsia zx_channel_read_etc** | supplies received handle object type and rights to help untrusted-peer validation, reports counts and BUFFER_TOO_SMALL/discard behavior | cap role/type checks need trusted live metadata and a strict size/claim rule |
| **[S5] Fuchsia FIDL wire format** | encoded handle placeholders and separate vector; exact handle count and buffer traversal validation | IDL payload ordinal cannot substitute for native cap-vector accounting |
| **[S6] Fuchsia FIDL unknown-field RFC** | unknown handles must be processed, default close under that RFC; flexible resource and strict type behavior matter | never skip unknown moved cap with a normal optional scalar decoder |
| **[S7] seL4 IPC tutorial** | shared badged endpoint caps, per-Thread IPC buffer, synchronous rendezvous, reply capability | a legitimate alternative design with different multi-owner and progress guarantees; badges alone don't authenticate a human |
| **[S8] Fuchsia kernel object lifetime** | queued messages hold handle refs alive despite no active Task handle; lifetime/cycle distinctions | sponsor/ancestry and object/handle count differ; endpoint-close cascade must be bounded |
| **[S9] Fuchsia ChannelWriteEtc** | separately described MOVE versus DUPLICATE dispositions in Zircon transport | UniOS E1 deliberate MOVE-only is **not** equivalence with a borrowed Zircon transaction |
| **[S10] Fuchsia handle replacement** | replacing a handle changes the caller's lookup token and invalidates original | public ABA and source ownership are more than copying numeric bits |

Caveat: Fuchsia syscall pages document Fuchsia's kernel-owned atomic queue/read semantics; they **do not prove** that UniOS's proposed ReceiveClaim can atomically publish an arbitrary user byte buffer plus a handle table. Conversely, seL4 intentionally uses a different endpoint ownership model. The comparison is feasibility research, not proof or license approval.

## 2. Contract vocabulary and four independent graphs

- **Grant (G)**: live kernel capability to an underlying object with rights/scope/lineage and revocation state; more than one independently authorized Grant may reference the same Object.
- **PublicHandle h**: Task-local opaque lookup token for a Grant. No stale token in a living Task may authorize a newly unrelated grant. Kernel invocation refs are not additional user-visible owning E1 handles.
- **Source/Transit/Claim/Receiver custody**: who presently holds or accounts for the **moved specific grant**, not the whole object's independent capability roots.
- **Sponsor**: ResourceBudget charged for active handle, queue record, escrow, pending message, pinned memory, crash ledger and orphan cleanup; not implied same as holder.
- **Endpoint E**: kernel object, at most one effective public owning Task handle in E1. ServiceID is a different logical name; ServiceIncarnation and PolicyEpoch are authorized userspace states.
- **SEND cut K-S**: one atomic queue append and cap-vector MOVE that changes custody of all moved source grants or none. Kernel transport success is NOT method success.
- **ReceiveClaim K-C**: exclusive kernel claim/escrow for one message with bound receiver Task/Endpoint generation, charges and deadline/cancel policy. It prevents competing claimants, **not** already observed user bytes.
- **Visibility cut V**: first potential user-observable publication of any sensitive payload byte or handle-derived metadata to an untrusted address space. Visibility is irreversible even if subsequent copyout faults or the receiver dies.
- **Receive install cut K-I**: complete cap-vector installation plus queue/claim retirement under kernel bookkeeping; not necessarily simultaneous with V.
- **SABI admission cut S-A**: trusted server accepts protocol+caller/current PolicyEpoch/resource roles and authorizes a **new** effect; unknown schema/resource must fail closed/settle.
- **Physical/persistent effect P-E**: external VFS/device/network work, with optional scoped durable receipt; may outlive Channel/Task, caller timeout, or cap close.

Four graphs must never be collapsed: (1) capability lineage and revoke family, (2) sponsor/charge graph, (3) object/invocation/queue/physical lifetime graph, (4) information-flow/exposure graph. **Correct handle movement does not imply no information disclosure** and **correct confidentiality does not imply a file mutation was not applied**.

### 2.1 Product-independent trust ceiling

The kernel checks real cap type/rights/scope and protected native Task/Thread identity. SABI service checks current authorized method scope. A bearer Endpoint doesn't assert which human gave consent; forwarded proxy and true caller differ. An IDL resource role cannot mint cap ownership or grant a missing privilege. Guest Linux UID0/FD and a ServiceID string are not kernel grants.

## 3. Sender SEND linearization (v0.2 sharper pre/post contract)

### 3.1 Abstract steps, not a syscall implementation

~~~text
A0: Caller A holds source vector G=[g1,..gn], valid E1 Endpoint owner hE.
A1: Snapshot bounded bytes and handle descriptors under safe usercopy.
A2: Validate distinct source grants, TRANSFER rights, lineage B, full bytes,
    sender/queue credits, cap metadata and target Endpoint generation.
A3: Reserve all queue record + sponsor charges + cap-list transit nodes.
K-S: Revalidate atomic active rights/ownership/revoke and commit ENTIRE bytes
     + ENTIRE moved cap vector to bounded queue; invalidate source handles.
A4: Result attempted to A; may be observed, lost, or A may be terminated.
~~~

**Invariant proposal:** A1/A2 alone do not authorize K-S after another Thread closes or MOVEs the handle, transfers the Endpoint or controller publishes revoke B. K-S must recheck protected live cap state atomically against competing cuts. Holding locks across pageable A1 usercopy is not permitted; prepare/snapshot in bounded memory first, then reserve/validate under separately proved lock order/charge protocol. Partial byte copy before K-S stays only in kernel staging and must not disclose to another receiver.

**All-or-none:** no subset of the n source grants moves if K-S aborts. Duplicate entries referencing the **same source Handle** in one MOVE are rejected (unless an independently permitted derivation produces distinct grants beforehand, and E1 Endpoint cannot be DERIVEd); object identity may legitimately be shared by distinct authorized grants. Cap vector and bytes snapshot are one self-consistent message. A revoked Grant cannot be authorized merely because preflight saw it valid.

**Endpoint-specific hazards:** (i) moving hE while its own SEND is in flight, (ii) including hE in its own outgoing cap vector, (iii) carrying the opposite Channel Endpoint or a nested Channel in a cyclic containment graph, (iv) closing the last E1 endpoint with a queued cap holding a reference to a Channel. Default candidate bans **direct self-attachment** of the exact sending Endpoint. For opposite-endpoint and indirect cycles, compare explicit prohibition versus type-safe bounded cycle collection/quarantine; never claim zero-leak teardown until chosen.

### 3.2 A4 lost result, status-known vs state-known

A result lost *after* K-S is not kernel rollback. A result lost *before* K-S still does not permit the caller to infer precommit unless it has an authoritative record. Distinguish:
- kernel actual **NOT_COMMITTED / COMMITTED** state;
- caller epistemic **UNKNOWN_COMMIT** if it can't reliably observe the result;
- separate SABI effect **NOT_APPLIED / APPLIED / UNKNOWN_EFFECT** that depends on actual backend journal.

**Harder than v0.1:** a newly created ticket sent only through A4 is useless if A4 is lost. Candidate **T1-A**: authorize a Task-scoped, preidentified request slot **before** K-S, with reserved memory/metadata/budget and a durable *kernel-lifetime* status record. Candidate **T1-B**: bounded authenticated enumeration of outstanding Task IPC results without knowing a lost ticket ID. Both have anti-ABA, Task death, privacy, starvation, GC and context-switch obligations. A fixed register-only result or fail-closed ambiguous-call state remain alternatives; nothing selected.

### 3.3 Endpoint ownership and in-flight admission ordering

With one E1 owner, concurrency must choose an ordering, not rely on a mutable pointer labelled owner. Compare:
- **S-END-A quiescence-first:** the Task cannot MOVE E1 Endpoint until all kernel-admitted SEND/RECEIVE invocations for it are finished, canceled or fenced. Simpler to reason about, may delay MOVE forever without bounded timeouts.
- **S-END-B admission-reference cut:** calls admitted before Endpoint MOVE may complete within a fixed predeclared scope, but are serialized against MOVE in a kernel sequence order; calls admitted after MOVE are refused for old Task. Requires queue order evidence and correct resource sponsorship under old incarnation.
- **S-END-C cancel-on-move:** all old in-flight uncommitted invocations fail/settle before transfer; side effects committed earlier still stand, and result-loss UNKNOWN applies.

**Re-review preference:** study S-END-C or bounded S-END-A first to minimize confusing owner transitions. But forced cancellation after an actual K-S cannot pretend to undo the committed message. Compare against Fuchsia's explicit before/after transfer ordering [S1]; do not silently assume its implementation proof.


## 4. ReceiveClaim, FIRST VISIBILITY and terminalization — primary v0.2 P0

### 4.1 Failure of naive requeue

~~~text
Q owns Message M=(secret bytes P, moved grant vector G)
B exclusively claims M                                           [K-C]
kernel starts copying P into writable B user buffer               [V]
another Thread in B copies first 32 bytes                          [visible]
the next user page faults, before kernel cap install               [no K-I]
naive kernel aborts B claim and requeues M
C becomes owner or next authorized receiver and obtains M again
=> B and C have seen P despite "only one successful dequeue."
~~~

**Core insight:** a ReceiveClaim is exclusive but cannot erase earlier observations from another Thread, shared mapping alias or already submitted external I/O. The definition of confidentiality needs an information-flow boundary: which recipient Task/ServiceIncarnation was authorized to observe the payload, and can that payload be seen by a second distinct recipient after abort? **Successful ownership transfer and one logical queue dequeue are not the same thing as at-most-one-recipient disclosure.**

**Candidate RCV-T:** mark a conservative, irreversible Exposed(M) flag **before attempting the first user-visible payload write**, not only after the byte copy completes. A failed write may never actually expose a byte, but conservative terminalization is safe. If Exposed(M), the queue record MUST NOT return to an unrelated Task, new Endpoint owner or new ServiceIncarnation via normal claim rollback. It can only continue with the same explicitly authorized receiver context where meaningful, or become TerminalSettlement (drop/close/quarantine, sponsor accounted, no foreign requeue). Terminal settlement doesn't erase any already exfiltrated bytes or already applied remote effects.

### 4.2 Competing receive approaches — NONE chosen

| Family | Mechanism | Benefit | Undischarged obligation |
|---|---|---|---|
| **RCV-T1** | exclusive claim + kernel byte staging + Exposed before copyout + terminal after failure | consistent no-foreign-requeue safety; keeps E1 small | user copyout can partially modify memory; message may be lost after disclosure |
| **RCV-T2** | separate inert kernel cap escrow; user-space decoder sees data then explicitly admits actual cap role vector | prevents unintended grants becoming immediately usable before method validation | extra state/TCB, authenticated release, malicious stalled claimant, recovery |
| **RCV-T3** | sealed transferred MemoryObject and one-shot cap escrow | may simplify bounded staging | shared aliases, pin, TLB/revoke, Pager and performance proof |
| **RCV-T4** | register-only tiny receiver path | avoids bulk usercopy on fast path | not an arbitrary SABI payload+cap atomic delivery |
| **RCV-R** | requeue after first possible user-visible write as if no receive occurred | simplifies retry only | **rejected default**: duplicates secret disclosure to new recipient |

**Research priority:** compare RCV-T1 with RCV-T2 in a small formal state space. RCV-T2 has stronger separation between data decode and cap usability but greater kernel/reserve complexity. Neither can claim true rollback of arbitrary user-visible byte writes.

### 4.3 Explicit receive state table

| Phase | Current custody | Exposure | Allowed next step |
|---|---|---|---|
| Queued_Unclaimed | Channel kernel queue owns cap vector | no | only authorized B Claim or close |
| Claimed_Private | bounded exclusive claim, B identity/generation pinned | no | abort and requeue if truly unexposed, or prepare |
| Prepared | staged bytes, reserved output slots/budget, Task/Endpoint revalidated | no | mark Exposed BEFORE write |
| Exposed_BoundToB | claim/escrow remains, same B context | possible/irreversible | same B continuation or terminal settlement, **not unrelated C** |
| Installed_B | B cap table owns all actual caps; queue retired | may have happened | service authorization/close unknown cap, no A ownership restore |
| Terminal_Settlement | bounded cleanup/charge/quarantine record | may have happened | no foreign requeue, honest failure/disclosure state |
| Requeued_Unexposed | kernel queue owns same caps | proven none | only if claimant/rights/version/revocation still eligible |
| Failed_HardwareFence | unresolved DeviceGroup DMA/quarantine | possibly | no R2-F or pin-refund claim |

**Different cuts:** K-C exclusively claims, V first possibly writes to user-owned memory, K-I atomically transfers whole cap vector to B. They are **not** assumed one transaction. Same-Task Threads may read while a syscall is copying; failed K-I doesn't make earlier V nonexistent. After B death, the kernel cannot prove that visible bytes were not already forwarded elsewhere.

### 4.4 Copyout buffer overlap and hostile remapping

Fuchsia channel read documents that it writes bytes before handles, and overlapping output buffers can overwrite prior bytes [S3]; this is precedent for **why separate pointer checks are not enough**, not UniOS's desired semantics. Default proposal: reject overlapping byte/handle/count/status output ranges, invalid/unmapped pointers, checked integer arithmetic overflow and clearly unsupported aliases before K-C. Virtual nonoverlap is insufficient where writable shared backing aliases the same pages; avoid relying on buffer contents not changing concurrently.

BUFFER_TOO_SMALL should be decidable under an authorized count/size query or preflight without exposing message bytes, unless an explicit read-discard option and custody contract is separately accepted. After a partial user copyout fault, status cannot truthfully assert that no data was delivered. Snapshot user parameters/handle descriptors in kernel-owned bounded memory before validation; avoid global locks while usercopy may page fault.

### 4.5 Cancellation, claimant death and queue timeouts

An exclusive claim binds actual kernel ReceiverTask+EndpointGeneration, charge and owner, not merely an untrusted SABI request_id. A malicious receiver may stall; enforce bounded claim age, cap escrow/bytes budget and protected M0 cleanup. Aborting a claim before V can requeue only if policy and active rights still allow it. **Aborting after V cannot requeue to a new Task/ServiceIncarnation**, even when receiver has died. A terminally dropped message might have been observed by B and led to an external effect: do not report it as certainly no-effect.

## 5. Cap install, schema validation and revocation

### 5.1 Handle usability can precede method admission

Kernel can validate actual transferred cap type/rights/lineage, but cannot decode arbitrary evolving user-space SABI resource role schemas without violating RFC-001/009 small TCB. If caps become immediately invokable in B's cap table at K-I, a malicious B can use a granted authority **before it validates the higher-level method**. Whether that is a security violation depends on whether receiving a cap is itself authorized by Endpoint ownership, or product policy promises that only method admission unlocks a resource.

Two explicit security models:
- **CAP-BEARER:** successful receiving grants the actual kernel authority to trusted B, followed by userspace method validation; B's granted rights are the policy ceiling. A malformed method can still expose live caps to an already authorized B. Never claim method-level permission gates those caps.
- **CAP-ESCROW:** actual transferred grants remain non-invokable in kernel custody until trusted role-check and authenticated release; malformed/unknown resource claims close/settle without becoming arbitrary B rights. Needs separate kernel escrow object, release authorization, quota, claimant death policy and no reliance on raw IDL string. **Not selected**.

A third alternative uses a narrow, separately trusted router/broker to receive and attenuate caps before less-trusted service workers. That broker's own TCB and provenance require validation, and its user-space parser is only as secure as the caps it holds.

### 5.2 Cap vector, revoke B and admission cut

A single K-I may install the whole vector or none. Byte copyout V is separately observable. If V preceded K-I, an all-or-none cap install cannot imply "no data leaked". If K-I preceded V, a service might use native authority before a subsequent failed payload copy. CAP-ESCROW might prevent immediate use while still leaving already visible bytes irreversible.

If revocation barrier B publishes during queued transit or ReceiveClaim, newly admitted cap operations under that revoked lineage must fail regardless of whether the source had transfer rights at earlier SEND preflight. A cap that was legitimately moved before B can remain in transit but not be newly usable after B. An in-flight operation admitted before B needs separate continuation, CPU R1 and DMA R2-F rules, not blanket rollback.

### 5.3 Proposed logical outcome vocabulary (not numeric ABI)

| Receive status class | Truthful statement |
|---|---|
| RECV_NOT_CLAIMED | no successful claim, no user-visible publication |
| RECV_CLAIMED_UNEXPOSED | exclusive kernel claim active, no V yet |
| RECV_PARTIAL_VISIBLE | any user-visible write could have occurred; new-recipient requeue prohibited |
| RECV_CAPS_INSTALLED | actual live cap vector installed for receiver in one kernel cut |
| RECV_TERMINAL_SETTLED | queue record no longer claimable by foreign receiver; caps may be closed/quarantined |
| RECV_UNKNOWN | observer lacks trustworthy evidence of current phase; no speculative retry |

These names are proposed design categories, not C types/syscall numbers. A dead Task may never observe RECV_PARTIAL_VISIBLE; kernel must nonetheless preserve custody and cleanup truth.

## 6. Endpoint ownership, message cycles and hardware cleanup

E1 MOVE of endpoint ownership races with old Task's in-flight SEND/RECEIVE. A consistent design must order admitted old operations before transfer or cancel/fence them before new owner becomes active, without undoing a committed SEND. A kernel invocation reference may outlive a Task's public handle but it is not a second effective owner; its permitted work and budget are bounded. Fuchsia Channel explicitly specifies before/after Endpoint transfer ordering [S1]; UniOS does not inherit this proof.

Queue-cap containment can form cycles and recursive destruction. Fuchsia Channel docs warn about messages owning handles and recursive close effects [S1]; its ChannelWrite rejects including writing Endpoint as a moved handle [S2]. **Research initial rule:** reject direct self-send of actual sending Endpoint; examine sending peer Endpoint and indirect cycles under explicit reject/iterative bounded GC/quarantine policies. Never rely on recursive unbounded kernel stack or unlimited cleanup under a global lock.

Channel peer-closed or queue-full **before SEND cut** returns true NOT_COMMITTED with original source grants still owned by A (UniOS's candidate E1, unlike Fuchsia's consume-on-error write). After SEND cut, peer death can cause queue cleanup and close of transit caps but **does not restore original sender handle**, stop DMA or undo a remote VFS effect. When R2-F cannot be proved, keep device/pinned frames charged/quarantined, not reclaimed simply because final receiver cap is gone.


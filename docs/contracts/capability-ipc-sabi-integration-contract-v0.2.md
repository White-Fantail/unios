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


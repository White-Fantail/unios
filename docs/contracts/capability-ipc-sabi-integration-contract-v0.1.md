# UniOS — Capability · IPC · SABI Joint Integration Contract v0.1

> **Status:** Proposed Joint Contract — Design-Only Research, 2026-10-25 NZDT. **NOT Accepted**, not a stable KABI/IDL/API, not implemented, modeled, fuzzed, benchmarked, formally verified or tested on hardware.
> **Parent:** [Cross-RFC Architecture Consistency Audit v0.1](../reviews/cross-rfc-architecture-consistency-audit-v0.1.md), findings CX-01–04/05–06/15–17 and research action R1.
> **Joint owners:** [RFC-002 Capabilities](../rfcs/rfc-002-object-capability-lifetime-delegation-revocation.md), [RFC-003 IPC](../rfcs/rfc-003-ipc-wire-protocol-and-synchronization.md), [RFC-009 Native KABI/SABI](../rfcs/rfc-009-native-abi-and-system-call-contract.md), [RFC-018 IDL](../rfcs/rfc-018-versioned-idl-abi-evolution-and-compatibility-contracts.md).
> **Mandatory co-dependencies:** RFC-004 FaultWaiter/Pager, RFC-005 CPU budgets, RFC-006 verified service admission/RecoveryAuthority, RFC-007 R2-F hardware fence, RFC-008 durable effect, RFC-010 guest status and capabilities, RFC-016 consent and RFC-017 Root/Floor.
> **Baseline:** UniOS architecture v0.2 broad D1–D8 directions accepted, all individual RFC choices remain Proposed.
> **Restrictions:** no kernel IPC or cap code, syscall headers/number assignments, IDL compiler, SDK, service registry implementation, Linux ABI bridge, model checker, test, hardware access, application support claim or code branch change is authorized. This is a written contract candidate.

## 0. Executive design position

**CAPIPC-A is the conditional default integration candidate:** a small kernel controls type-checked Task-local capability ownership and revocation, bounded asynchronous E1 Channel transfers opaque bytes plus real MOVEd kernel capabilities, and user-space versioned SABI services decode methods and enforce application policy. Native user-service protocols and guest Linux ABI never elevate user-provided strings, integers or object IDs into kernel grants. Recovery M0 must not depend on an unbounded pageable IDL runtime.

A complete integration requires seven **separate** commitments, none proven:
1. **E1 Endpoint ownership:** one effective owning handle per ChannelEndpoint object at one time; same Task Threads may access it under kernel invocation references. E1 endpoints cannot DUPLICATE or DERIVE; Task-to-Task transfer is MOVE only. Multiple client connections mean **different channel objects** or explicitly sequential Endpoint MOVEs. E2 multiowner is NOT implied.
2. **Cap provenance:** kernel Task-local public handle identifies one right+scope+lineage and preserves anti-ABA; issuer, controller, current holder, sender, current actual caller, human principal and sponsor are not interchangeable.
3. **SEND proof:** one atomic kernel commit switches MOVEd cap custody from Task to kernel queue. Genuine precommit NOT_COMMITTED retains all source handles; after COMMITTED even decode failure/receiver death cannot rewind ownership. If report is lost, caller state is UNKNOWN until protected reconciliation.
4. **RECEIVE proof:** one claimant and bounded accounting guard against unauthorized bytes disclosed to losing receivers; whole cap-vector receiver install + message removal is atomic at its own linearization point, but user-memory byte copyout is **not** automatically atomically reversible.
5. **Service auth:** Endpoint possession proves only an issued bearer right. Identity-sensitive operations require independent trusted per-call authority/provenance; SABI version and ServiceID strings are not capabilities.
6. **Policy epoch:** a negotiated SABI session is tied to exact Endpoint/grant generation, actual ServiceIncarnation, required features/resource roles and current authorization-policy epoch. Revocation and policy strengthening require revalidation at each effect admission.
7. **Result truth:** kernel queue commit, received caps, method admission, remote effect, hardware fence, file/directory durability and observed result are distinct. A timeout/old ReplyToken has no automatic rollback or exactly-once semantics.

**Direct conflict addressed as editorial only:** RFC-003 §4 currently lists ChannelEndpoint DERIVE in the generic table while immediately forbidding E1 Endpoint DERIVE/DUPLICATE and in IPC-D13. The intended E1 comparison is nonduplicable. The exact right-table correction is specified in §2; this new document does not by itself override the original RFC.

### 0.1 Trade-off countermodels

| Option | Technical trade-off | Early recommendation |
|---|---|---|
| **CAPIPC-A:** E1 exclusive Endpoint and separate per-client channels | simple precise one-owner transfer and peer generation, per-client connection/dispatcher overhead | **provisional primary baseline** |
| CAPIPC-B: E2 shared multi-Task endpoint | easier single service listener with many callers, but sender origin/permission fairness, concurrent revoke and receiver selection far more complex | defer pending joint threat model |
| CAPIPC-C: seL4-style shared synchronous endpoint / badges | clear badge/cap examples and rendezvous discipline but changes queue/progress/fault model, badge not original human | serious alternative only, **not equivalent E1** |
| CAPIPC-D: Zircon-compatible Channel transfer | mature owner endpoint pattern, but zx_channel_write consumes handles on failure; conflicts with UniOS's proposed NOT_COMMITTED source-retention | reference, **not drop-in semantics** |
| CAPIPC-E: one native full IDL parser in Ring 0 | reduces some crossings, expands privileged parser/untrusted length and schema TCB | not preferred without exceptional proof |

External precedent: Fuchsia Channel endpoints are nonduplicable and ownership moves with ordering semantics [S1]; its basic channel write consumes transferred handles even on failure [S2]. seL4 permits badged capabilities referring to one shared Endpoint and provides separate one-shot reply capabilities [S3]; this is a **different** connection-ownership model. FIDL's value/resource and strict/flexible distinction informs user-space schema only [S4]. None is already UniOS.

## 1. Objects, identity domains and authoritative components

### 1.1 Precise nouns

| Term | Meaning | Authority/source |
|---|---|---|
| Kernel Task | isolated AddressSpace/cap table, generation and protected Native/GuestDomain | kernel |
| CapGrant | typed object reference + allowable rights/scope + lineage/revoke state + charge metadata | kernel and authorized issuer |
| Public Handle | caller Task-local opaque lookup token, including nonreusing public-generation defense | kernel; user can copy token bytes but not authority to other Tasks |
| Holder | Task with active handle, or named kernel transit/receiver escrow custodian | kernel |
| Issuer | entity authorized to CREATE/DERIVE a scoped grant | authorization path, not raw ServiceID |
| Controller | scoped RevocationController; may outlive issuer's data handle | kernel capability lineage |
| Sponsor | ResourceBudget owner charged for handle/transit/page/pin/claim use, not necessarily Holder | budget ledger and authorized ADOPT |
| Current caller | invoking Task/Thread proven by kernel for this syscall; proxy may originate the actual backend call | kernel per-invocation state, *not* original user automatically |
| Human principal | authenticated user/consent subject, if operation truly depends on it | trusted identity/consent issuer; independent subject proof needed |
| Endpoint | kernel-owned half of one Channel, containing one direction-pair queue identity and generation | kernel |
| Connection | two Endpoint halves (client/server) plus authorized issuance and policy context | launcher/service broker; kernel owns actual handles |
| ServiceID | stable logical registry name, not cap token | userspace manifest/registry |
| ServiceIncarnation | specific authorized executing Task/image/service deployment generation | trusted Supervisor/issuer |
| PolicyEpoch | version of issuer/server mandatory authorization rules, not SABI minor version | trusted policy authority |
| Session | agreed protocol/features/resource-role/mandatory security binding for one issued connection | service/issuer checks bound to Endpoint and policy epoch |
| ReplyToken | one-shot kernel response right tied to precise Call+Task/Endpoint generation | kernel, not raw IDL request_id |
| RequestId | userspace correlation/optional durable-effect key scoped to authenticated actor, method and backend | service state, not kernel authentication |
| DeviceGroup | physical DMA/IRQ isolation unit and epoch, not just a DeviceCap name | RFC-007 privileged device authority |
| DurableReceipt | scoped File/Directory generation proof under actual device assumptions | RFC-008 trusted volume/ledger service |

### 1.2 Three independent authority/data graphs

**G-A Kernel capability graph:** issuer/revocation ancestry, rights/scope attenuation, active holder, moved transit record, independent roots, sponsors and object lifetime.

**G-B Service/policy graph:** ServiceID, approved executable digest/image, ServiceIncarnation, Endpoint issuance, PolicyEpoch, identity-bearing consent/provenance, negotiated SABI method/feature/resource role. It can narrow G-A rights but **cannot mint G-A rights from labels**.

**G-C Effect/persistence graph:** application RequestId, file/volume dirty generation, VFS durable transaction, StorageHost+DeviceGroup/IOVA epochs, physical FLUSH/FUA and receipt/unknown observation. G-A send success is neither G-B method approval nor G-C completion.

The kernel never needs to parse a full ServiceManifest/IDL, Linux ELF metadata or trusted signing policy just to validate a native handle invocation. The loader/M0/Supervisor and ordinary services retain separately bounded trusted responsibilities.

### 1.3 Precedence and invariant conservation

Effective allowed operation = **actual kernel cap type, rights, scope, lineage, current revoke barrier** intersected with **current trusted service policy and required features**, and with valid independent consent/identity evidence if and only if the operation demands it. Never union kernel rights due to client features, SDK types, a signed app name or an endpoint badge. Signed executable identity may constrain issuer admission but does not grant universal root.

The single-owner rule concerns **one E1 Endpoint object**, not all services, not every typed Cap object and not all E1 connections globally. In-flight invocation references do not create a second public owning Endpoint handle but do require owner/transfer linearization.

## 2. E1 rights erratum and multi-client connection contract

### 2.1 Exact RFC-003 editorial collision and correction candidate

The RFC-003 §4 table currently says ChannelEndpoint rights INCLUDE DERIVE; the prose and IPC-D13 say E1 DERIVE/DUPLICATE forbidden. The smallest coherent E1-specific row proposal is:

~~~text
E1 ChannelEndpoint | SEND, RECEIVE, WAIT, TRANSFER, QUERY_BASIC
                    NO DUPLICATE, NO DERIVE (including attenuated alias copies)
                    E2 multiowner / derivation = different future profile
~~~

This is a correction of a **Proposed** table, not approval of a final right bitmask. SEND/RECEIVE may also need per-operation direction/rights validation; existing task-level concurrency is not proof safe transfer. RFC-002 generic DERIVE stays type-specific and does not force a ChannelEndpoint duplicate.

### 2.2 Connection establishment without hidden global Endpoint copy

~~~text
Trusted Task Startup grants Client a scoped bootstrap/broker Endpoint
Client --(authorized request to connect to ServiceID)--> Connection Broker
Broker checks current issuer policy + resource/sponsor budget + selected service
Broker creates E1 Channel pair (ClientHalf, ServerHalf) as typed kernel objects
Broker MOVEs ClientHalf through Client's EXISTING authorized bootstrap Channel
Broker MOVEs ServerHalf through Service's EXISTING authorized control Channel
Service binds new per-connection Session to ServerHalf under its current incarnation
Client binds ClientHalf only after verified peer/policy negotiation (Ready)
Client 1: [C1 Endpoint] <== channel #1 ==> [S1 Endpoint] : Service Task
Client 2: [C2 Endpoint] <== channel #2 ==> [S2 Endpoint] : Service Task
No Task concurrently owns the SAME C1 or S1 Endpoint as another Task in E1.
~~~

**Bootstrap is not a free-standing global lookup that mints caps.** The Client must already hold an authorized initial broker connection, provided through its verified/task admission path; Service Task likewise holds a scoped control endpoint/manifest-authorized grant. If either cap MOVE fails after the other succeeded, broker must settle the partial connection with bounded cleanup and PEER_CLOSED semantics, not pretend atomic distributed delivery. No user app gets a raw global ServiceRoot cap merely from knowing ServiceID.

**ConnectionFactory is a logical userspace protocol role**, not a new approved kernel object or syscall number. A possible *alternative* has caller create a pair using scoped CREATE_ENDPOINT budget and request that trusted launcher attach its ServerHalf; that still needs actual issuance/authorizations and cannot allow spoofed service identity.

### 2.3 E1 transfer and concurrent invocation

At most one **effective public owner handle** per endpoint. For a MOVE from Task A to Task B:
- new operations begun after MOVE linearization must be rejected on A; older invocation references may complete only within defined pre-admission scope;
- FIFO direction/order is by kernel Channel send commit, never global ordering across multiple connections;
- peer identity/generation may change owner; an already authorized request is not automatically a new owner's request; no sender identity token is manufactured by merely moving the endpoint;
- after B's Task death the channel and in-transit caps require bounded teardown or explicit drain; handles never "snap back" into A;
- E2 must be a new explicitly reviewed family if a single shared endpoint is later desired for many independent client Tasks.

**Request origin limitation:** unique Endpoint ownership can identify the current *Task owner at kernel entry* under a proven move/forward model; it cannot authenticate a human principal, authorizer who originally issued a delegated cap, or a proxied guest. For any stronger identity-sensitive API, see §7.

## 3. Unified cap custody: SEND, RECEIVE and local status

### 3.1 Conservation invariant — one live custodian at a time

For one MOVEd CapGrant g, candidate abstract states:

~~~text
Active(Task A, h) -> Prepared(A, h, reserved queue credit)
  -> [one SEND commit] InTransit(Channel queue record, lineage, sponsor)
  -> Claimed(Receiver B, exclusive ReceiveClaim; kernel escrow)
  -> [one RECEIVE commit] Active(Task B, new non-ABA public handle)
  -> Close/Revocation/Fence/Retire per type

Pre-SEND failure -> Active(Task A, original h)
Post-SEND lost status -> actual state is still either Active(A) or InTransit;
                         caller knowledge UNKNOWN; no blind retry
Claim abort/decode fail -> kernel transit/escrow settlement or cap close/quarantine;
                           NEVER silently recreate old A public handle
~~~

A method may contain **multiple caps**. They transfer as one kernel SEND transaction (all source entries removed or all preserved before commit) and one independent RECEIVE installation transaction (all destination slots installed or none). Claim/escrow may be in the middle. All ownership states are distinct from object references, grant ancestry, in-flight operations and sponsors.

### 3.2 Required boundary table

| Phase | Cap custodian and charge | Bytes visible? | What can be truthfully reported? |
|---|---|---|---|
| P0 preflight | A retains handles; temporary credits reserved | only A's own data | NOT_COMMITTED if failure before cut |
| P1 SEND_COMMITTED | bounded kernel queue record owns cap vector and bytes | kernel snapshot; receiver not yet | sender's old handles invalidated; still NOT processed |
| P2 SEND_REPORT_LOST | same true kernel state; A has incomplete knowledge | no new guarantee | UNKNOWN; no speculative source ownership |
| P3 ReceiveClaim | kernel escrow exclusively claimed for B, bounded by quota and timeout | no unapproved B/C disclosure before authorized staging | B not yet general public owner |
| P4 partial user byte copyout | B may have seen some bytes if copyout made them visible | **yes, non-rollbackable** | must not say confidentiality preserved or receive globally atomic |
| P5 RECEIVE_COMMIT | kernel moves all installed caps and retires queue | authorized B sees declared bytes subject to actual copyout policy | cap ownership B, not method success |
| P6 SABI_DECODE/ADMISSION | B owns or settles any caps, applies trusted rights/method checks | B may parse values | unknown critical/resource errors may reject but cannot undo P1/P5 |
| P7 SERVICE_EFFECT | service/driver/VFS may mutate external state | app-specific | effect evidence or UNKNOWN, not automatic file durability |
| P8 REPLY OBSERVED | caller sees one-shot ReplyToken result | returned bytes available | only reported verified stage; timeout not cancellation proof |

**Hard problem still open:** how to publish complete bytes and installed cap handles to a malicious B without copyout leaking before committed authority, with mutable user memory and Pager faults. RFC-003 RCV-A exclusive escrow is a promising **hypothesis**, not yet atomic or confidentiality proof. Register-only small response and pre-pinned immutable staging remain alternatives with cost/availability tradeoffs.


## 4. Kernel operation receipt, UNKNOWN and abort ownership

### 4.1 Sender cannot infer actual commit from a lost output

Kernel queue mutation and caller-visible status are different cut points, even when the nominal syscall uses one register return. A cancelled or killed caller, CPU exception during syscall return, signal race or user copyout fault can leave the caller without an authoritative result. **The committed state must still have exactly one kernel owner.**

| Candidate | Advantage | Undischarged P0 cost |
|---|---|---|
| **T1 protected operation record/ticket** | can query SEND commit truth without replaying MOVE | ticket ID itself may be lost; Task-scoped enumeration, authenticated query, quota and GC needed |
| **T2 preallocated immutable result slot** | reports to prepared shared slot | slot remap/alias/fault or user DMA, Pager deadlock, continuous atomicity unproven |
| **T3 fail-closed ambiguous caller** | avoids claiming false rollback | may kill/quarantine a benign caller; pending effects and sponsor must still be settled |
| Register-only atomic result candidate | fewer user-buffer copyout faults | must prove completion after irreversible commit when Thread/context is killed or signal interrupts; narrow ABI/architecture constraint |

**Provisional preference:** T1 as a design hypothesis **with explicit discoverability**, not a selected syscall. An operation ticket issued in the failed return path is useless if the caller loses the ticket number. Explore preadmitted Task-scoped operation slot whose identity exists *before* send and cannot be forged, plus bounded enumeration permitted only within the actual Task/service scope. A ticket shall not reveal another application's service endpoint, FileCap or user data. If no safe reconciliation channel exists, expose UNKNOWN and prevent unsafe retries.

### 4.2 A lost reply is not a lost request

A one-shot ReplyToken bound by kernel to CallerTask/Thread/RequestGeneration/EndpointObject cannot be forged from request_id bytes. A SABI ServiceIncarnation may change independently. Timeout, CancelLocal, PeerClosed and ReplyToken invalidation only affect local wait/right to deliver reply. They do not cancel a filesystem rename already admitted or a device command physically submitted. A committed request may later return an application result or a lost reply; no generic exactly-once file/network guarantee.

Kernel REPLY commitment, application effect receipt and client RESULT_OBSERVED are separate. SABI idempotency keys are not protected kernel operation tickets. Avoid numeric collision across Caller/Endpoint/ServiceIncarnation and guest process PID reuse. Request reissue after UNKNOWN requires backend durable reconciliation or a method-specific proven idempotent effect.

### 4.3 ReceiveClaim abort and unknown-cap policy

When service B decodes an unknown resource-bearing field *after* kernel P1 SEND_COMMITTED, default action is **reject and settle all already-transferred resources** under known kernel custody rules. Allowed outcomes are (i) B never installed caps, then kernel claim returns to carefully bounded queued custody or explicitly drops/revokes under declared policy; (ii) B installed caps, then B closes/revokes according to cap type and sponsor accounting; or (iii) a trusted authorized proxy receives and re-MOVEs a **newly constrained** resource to an allowed peer. No automatic return to A's pre-SEND ownership. Closure of DeviceCap does not imply physical hardware R2-F. Unknown scalar bytes may be discarded only if their omission is genuinely security-neutral and does not erase mandatory fields under a proxy transformation.

**Separate error names needed (logical only, no numeric ABI):** TRANSPORT_NOT_COMMITTED, TRANSPORT_COMMITTED, TRANSPORT_UNKNOWN, RECEIVE_CLAIM_FAILED, RECEIVE_PARTIAL_BYTES_VISIBLE, BAD_RESOURCE_ROLE, UNKNOWN_REQUIRED_SECURITY_FIELD, METHOD_UNSUPPORTED, POLICY_EPOCH_STALE, SERVICE_INSTANCE_STALE, EFFECT_NO_EFFECT, EFFECT_APPLIED, EFFECT_UNKNOWN and SYNC_SCOPED. Names are research semantics, not a promise each is observable from every existing Linux syscall.

## 5. Cap type, right attenuation, handle ABA and revocation

### 5.1 Typed right table (E1-only candidate)

| Resource class | Permitted scoped operation | False assumption rejected |
|---|---|---|
| E1 ChannelEndpoint | SEND, RECEIVE, WAIT, TRANSFER, QUERY_BASIC subject to direction/profile | **DERIVE and DUPLICATE prohibited** for E1 |
| ReplyToken | one-shot REPLY, optional bounded worker handoff under issuer policy | arbitrary Task reply or replay after service restart |
| CapDeriveAuthority | DERIVE only those object/right types explicitly permitted by issuer | all caps duplicate by default |
| Scoped ServiceConnectionRequest | request broker to issue exactly one new authorized E1 pair/connection | ServiceID string mints endpoint |
| DirectoryCap/FileCap | scoped native file/dir methods under current VFS/portal policy | guest Linux UID0 or path string broadens cap |
| MemoryObject/DmaPin | authorized virtual/physical mapping with explicit R1/R2-F limits | Task close means DMA stopped |
| ResourceBudget | reserve/adopt/charge explicit objects/queues and orphaned transit | MOVE automatically transfers sponsor or refunds RAM |
| RevocationController | revoke exact grant family and await specified phase | revoke B immediately erases shared independent root effects |
| DeviceGroup/IRQ | privileged hardware control and masked interrupts under validated topology | SABI DeviceCap ordinal grants raw PCI/MMIO/TPM |

An overall type permission is **not** a promise that every individual handle has that right. Rights and scope must be checked from the kernel grant on invocation; services also enforce current method-specific and consent policy. The RFC-002 abstract DERIVE right is type-specific; no code generator may accidentally derive an exclusive ChannelEndpoint by mechanically applying a generic handle clone routine.

### 5.2 Public opaque Handle and stale caller bits

An external Handle token that has become invalid in a Task may be copied into old stack or register memory. The **exact same externally visible token** must not later authorize a different grant while that stale copy can still be presented in the same Task lifetime. Merely storing an internal generation but reusing identical public bits is insufficient. Candidates are monotonic Task-scoped public tokens or versioned nonwrapping public token with fail-closed namespace exhaustion. Format/layout, bit widths and public SDK C type are still undecided; this is not a fixed 64-bit ABI mandate.

Endpoint and ReplyToken kernel generation, SABI ServiceIncarnation, PolicyEpoch and guest FD/PID/OpenFileDescription generation must not be collapsed to one unchecked integer. If a Process/Task dies, stale asynchronous callback and ticket cannot be rebound by name/number reuse. However, an invocation reference admitted before Handle close can remain live until its *own* commit/fence cut and associated charge settles.

### 5.3 Independent R0/B/R1/R2-F/R2-O phases

~~~text
R0: close one Task handle (other grants/operations can survive)
 B: reject new actions in the selected revocation lineage
R1: grant-derived CPU mappings no longer usable after proper TLB fence
R2-F: named family hardware DMA/IRQ and admitted work physically quiesced
R2-O: ALL independent roots and references drained, memory sanitized/reclaimable
~~~

A scoped revoke is not necessarily global object revoke. A call admitted *before* B may continue within its defined reference/sponsor scope, and disclosure to user memory or external file/network side effect cannot be undone. Even if cap reference count drops, DMA pin may persist and require quarantine. A path-dependent FileCap is often an application-level lease policy, not a promise of kernel-hardware R2-F on the file's prior writes.

**Race that must be modeled:** Invocation operation enters using grant G, revocation B is published, operation blocks on Pager, another Thread tries to DERIVE/MOVE G and a DeviceGroup still has an IOVA alias. New derivative and new work after B denied; old invocation continuation and physical fence handled according to its admitted cut. No automatic CPU/DMA reclamation from one close.

## 6. Budget, sponsored resource conservation and bounded failure

### 6.1 Charges by each custody state

| Resource and phase | Sponsor obligation | Failure/reclamation |
|---|---|---|
| Kernel cap/grant and lineage anchor | typed handle slot, ancestry metadata and object refs | retained while descendants/effects still exist after original issuer closes |
| SEND preflight | provisional queue, cap transit, metadata and protected result-slot reservation | release only for genuine NOT_COMMITTED |
| SEND_COMMITTED queue | message bytes, cap vector, queue slot and age/retention | may remain after sender Task dies; quota not simply refunded |
| ReceiveClaim | exclusive claim, cap slots, staging bytes, Task/Endpoint generation | claimant death/timeout has bounded cleanup or quarantine |
| Installed B handles | holder's handle/metadata budget plus any inherited sponsor obligation | sponsor switch only via explicit authorized ADOPT |
| Pending ReplyToken/WaitSet | protected callback/reply metadata/credits | bounded cancel/death/dedup; no stale new-instance wake |
| MemoryObject + dirty data | physical pages, COW writeback versions, Pager faults | clean/dirty/reclaim proof separate |
| Device DMA pin and IRQ | pinned PFNs/IOVAs/IRQ/ATC records and quarantine charge | release only after actual R2-F and independent reference test |
| Durable effect ledger | bounded per-volume generations/idempotency receipts | crash survival only if independently correctly persisted |

Sponsor need not be current holder. A Task dying does not necessarily refund a physically active DmaPin it formerly sponsored or a queued grant transit it originally funded. Sponsor ADOPT requires both authorized new charge/reservation and atomic old charge release; otherwise retain debt or refuse admission. Recycling handle slots/pins with a stale public token violates anti-ABA regardless of budget arithmetic.

### 6.2 Critical recovery and resource-pressure deadlocks

Kernel IPC must reserve minimal pages, queue slots, and bounded receipt/lineage cleanup; M0 RecoveryAuthority must have resident code+stack+page tables+timer/IRQ+critical CPU+IPC/Wait/error state. A service waiting for Pager while holding the only Pager-to-driver queue slot can deadlock regardless of a correct IDL. User-sponsored message parse complexity must not consume M0's reserved CPU/IRQ/clean-up capacity.

Examples:
- A malicious Client monopolizes all Channel queue records with valid small messages; Pager cannot request storage and M0 failover has no IPC reserve.
- Server B claims bytes for a large SABI resource union and never completes ReceiveClaim, pinning all cap transit.
- Task A closes a cap and sponsor exits, but a revocation lineage anchor or DeviceGroup remains Quarantined and every new service allocation incorrectly assumes the debt disappeared.

Require explicitly bounded worst-case admitted counts and critical lane reserve, not unproved microsecond guarantees. If actual hardware or root authority cannot progress, deterministic fail-closed/quarantine/fail-stop is a valid safety outcome; "M0 always repairs the device" is not.

## 7. Caller authentication, consent, forwarding and policy epochs

### 7.1 Bearer-only versus identity-sensitive method classes

**BEARER:** possession of a scoped Endpoint cap grants the allowed operations to the current holder, regardless of whether it is the original issuing user or a delegate. A Task moving the Endpoint explicitly delegates whatever access is allowed under issuance policy; E1 MOVE enforces exclusivity, not human identity.

**IDENTITY-SENSITIVE:** method authorizes on account/user/consent ownership, for example user-scoped file disclosure or an irreversible privileged setting. Endpoint bearer alone is insufficient. Need independently authorized per-call proof of principal/consent (issuer-stamped scoped proof cap/token or trusted kernel/portal metadata bound to actual Task, object, operation and generation). Whether an issuer should supply a transferable proof is a distinct confidentiality/delegation policy; user-provided strings or Linux UID0 are NOT that proof.

**FORWARDED:** an intermediary may hold a high-privilege native service cap but receives a less-privileged request from another Task. It must not exercise its own ambient grant as if that proves the original request's scope. The method's effective authority is constrained by a trusted delegated authorization and server policy. seL4-style badges are one useful kernel-minted grant label example [S3], but neither seL4 badge nor a mere issuer ID automatically proves a human login or current consent.

### 7.2 Time-of-check vs time-of-use of session features

One illustrative finite sequence:

~~~text
Caller C holds Endpoint E with scope R0 and ServerInstance S1
  -> trusted broker grants a Session(E,S1,PolicyEpoch=P10,IDL=V2,roles=F0)
  -> client negotiates mandatory future-feature F0 and sends Method M
  -> issuer revokes a right or server policy becomes P11
  -> Method M is about to admit a *new* scoped native operation
  -> service MUST check live cap and P11 floor; stale P10 handshake not enough
  -> service denies/rebinds or, for already-admitted effect, follows its
     explicitly defined pre/post-authorization linearization contract
~~~

An SABI request may include SessionId/MethodOrdinal/RoleIndex bytes, but only trusted Endpoint+issuer/server state binds them to actually granted scope. Never allow caller-provided SessionId or ServiceID to extend the cap's resource rights. Reconnect after service death gets fresh Endpoint and new SABI version/PolicyEpoch binding; old ReplyToken cannot refer to new ServiceInstance.

### 7.3 Strict unknown resource and version change

Unknown **value-only** optional data may be ignored if its default does not change authorization or effect. Unknown **resource-bearing** union/role requires counted kernel cap-vector settlement. Unknown mandatory security/durability/consent restriction MUST fail closed regardless of wire backward readability; older client cannot choose legacy default broad rights. Version negotiation is not token authentication; binary-to-JSON representation bridges may lose unknown Protobuf fields [S5], so proxy chains that depend on opaque retention of mandatory restrictions are denied.

**Key invariants:** a new field cannot change MOVE to COPY silently; absent consent cannot become granted consent; unknown SUCCESS cannot mean plain success; optional handle role cannot install root DeviceGroup; unknown result cannot be quietly mapped to an older Linux errno that implies no effect.

## 8. Response/effect status composition and guest translation

### 8.1 Multi-layer result lattice, not one catch-all STATUS_OK

| Truth owner | Candidate observation | What status does NOT assert |
|---|---|---|
| kernel IPC | SEND NOT_COMMITTED / COMMITTED / UNKNOWN | service admitted, completed, durable |
| kernel receiver | RECEIVE_CAP_INSTALLED/claim state | bytes published atomically, method authorized |
| SABI service | METHOD_DENIED / EFFECT_APPLIED / EFFECT_UNKNOWN | hardware DMA stopped, disk durable |
| DeviceGroup authority | R1 / R2-F / R2-O fence ticket | storage transaction undone, file key unsealed |
| VFS/durable ledger | BUFFERED_VISIBLE / SYNC_COMPLETED_SCOPED / RECEIPT_UNKNOWN | unrelated volume/media power-safe |
| boot trust authority | BootGood/Trial/RootFloor status | an app RPC magically grants BootFloorWriter |
| user SDK | RESULT_OBSERVED or unavailable | remote work automatically undone by timeout |

File/Network/DeviceService may already have side effects after caller deadline. Even a backend "idempotency key" is only useful when tied to authenticated caller, method, immutable request digest and a durable journal coordinated with the effect. Guarantee exactly-once across nontransactional external devices is not available from ReplyToken or IDL alone.

### 8.2 Native ABI and Linux guest status are different namespaces

RFC-009 native KABI logical statuses and RFC-018 user-space SABI result families are not Linux errno. A Linux-personality broker translating a POSIX-style write, fsync, close, futex, execve or epoll must honor the actual guest program's signal/FD/OFD/time/commit behavior: the native UNKNOWN for an already-sent network write cannot safely be made no-effect Linux EINTR. A guest FD integer is not a native cap token. Any behavior that cannot be represented truthfully is a **compatibility-tier refusal/research gate**, not an invented SUCCESS.

No numeric syscall, status enum, versioned wire byte layout, C struct shape, or specific Linux compatibility claim is fixed by this paper.


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


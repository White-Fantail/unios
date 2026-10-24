# UniOS — Cross-RFC Architecture Consistency Audit v0.1

> **Status:** Research Audit — Open Findings (2026-10-25 NZDT); **no RFC or ADR approved**, no stable ABI, no source implementation, formal proof or hardware test.
> **Scope:** [Architecture v0.2](../architecture/unios-architecture-v0.2.md), [RFC-002](../rfcs/rfc-002-object-capability-lifetime-delegation-revocation.md), [RFC-003](../rfcs/rfc-003-ipc-wire-protocol-and-synchronization.md), [RFC-007](../rfcs/rfc-007-user-space-drivers-dma-and-iommu.md), [RFC-008](../rfcs/rfc-008-storage-filesystems-and-crash-durability.md), [RFC-009](../rfcs/rfc-009-native-abi-and-system-call-contract.md), [RFC-010](../rfcs/rfc-010-linux-personality-and-binary-compatibility.md), [RFC-017](../rfcs/rfc-017-verified-boot-updates-and-rollback-protection.md), [RFC-018](../rfcs/rfc-018-versioned-idl-abi-evolution-and-compatibility-contracts.md).
> **Consulted for cross-boundary constraints:** [RFC-001](../rfcs/rfc-001-kernel-boundary-and-tcb.md) kernel TCB, [RFC-004](../rfcs/rfc-004-virtual-memory-pager-cache-and-oom.md) Pager/VM, [RFC-005](../rfcs/rfc-005-scheduling-preemption-and-cpu-budget.md) CPU/IRQ, [RFC-006](../rfcs/rfc-006-boot-trust-chain-service-startup-and-recovery.md) recovery/A-B.
> **Method:** read live Git main markdown, compare typed rights, authority owner, commit cut, resource sponsor, failure and restart/boot dependencies; reason about adversarial interleavings. No code search, test, model checking, device/crypto verification or end-to-end mathematical proof performed.

## 0. Audit verdict and evidence classes

**Overall: conditionally consistent design directions, but NOT ready to accept or freeze.** The eight principal RFCs agree on capability-based kernel minimality, IPC SEND/MOVE semantics, CPU/DMA cutoff separation, storage durability as hardware-conditional, no ambient Linux guest root, signer/floor/data-schema independence, and strict unknown resource semantics. These are mutually compatible **Proposed research contracts**, not verified engineering achievements.

**Confirmed contradiction:** RFC-003 §4 rights table lists DERIVE for ChannelEndpoint, but the immediately following E1 rule and IPC-D13 prohibit Endpoint DERIVE/DUPLICATE. **Cross-document ambiguity:** RFC-018 multi-client Endpoint examples could be misconstrued as multi-Task simultaneous ownership of one E1 Endpoint, whereas RFC-003/006 require distinct channels or an explicit E2 change. Neither finding is a discovered implementation bug: UniOS remains design-only.

**Critical jointly unproved integration:** authenticated actual caller vs bearer delegation; ReceiveClaim before secret byte publication; cap MOVE and unknown status; service incarnation/policy epoch versus kernel ReplyToken; R1/R2-F/device quarantine versus software restart; file writeback and crash-durable result; guest Linux futex/FD/process correctness; trusted verified boot Good/floor/data-schema rescue and canonical signed bytes.

| Label | Audit meaning | Handling |
|---|---|---|
| **C** | Document explicitly assigns inconsistent rules to same item | propose narrowly-scoped wording correction, approval required |
| **A** | Wording admits incompatible interpretations; no conclusive contract | specify actor/object/scope and authoritative owner |
| **G** | Shared P0 state machine or evidence still missing, even when RFCs agree | hold Go/No-Go and schedule joint proof |
| **D** | External feasibility/hardware/legal/product promise pending | no unsupported delivery or security claims |
| **OK** | Aligning proposed contracts | still no code/proof/Accepted decision |

**Authority:** Accepted D1–D8 are only high-level direction. All RFC-002/003/007/008/009/010/017/018 decision candidates remain Proposed; this audit adds no normative syscall, codec, signed format, hardware requirement or software implementation authority.

## 1. Scope, comparison and exact evidence points

| Document | Reviewed contract | Key source sections |
|---|---|---|
| RFC-002 v0.2 | Task-local public handle anti-ABA, lineage/controller/sponsor, SEND MOVE, R0/B/R1/R2-F/R2-O | §§7–11 and §20 |
| RFC-003 v0.2 | Endpoint owner/rights, E1 send commit, ReceiveEscrow, ReplyToken/restart | §§4–7,10,22 |
| RFC-007 v0.2 | true DeviceGroup vs ResetCouplingSet, DMA/IOVA/IRQ, R2-F, DriverHost incarnation | §§6–10,12,25 |
| RFC-008 v0.2 | durable receipt, dirty generation, hardware IO truth, A/B data, error ledger | §§5,8–12,15,25 |
| RFC-009 v0.2 | native/guest trap, small KABI vs SABI, lost result, status | §§4–7,15–16,25 |
| RFC-010 v0.2 | restricted Linux broker, atomic futex, FD/OFD, exec/epoll and guest errno | §§3,5–9,21 |
| RFC-017 v0.2 | authenticated boot, independent Root/floor, Goood state, physical rescue/key/schema | §§3,5–10,20 |
| RFC-018 v0.2 | delegated Endpoint/identity, cap-aware IDL, ServiceInstance/policy epoch, unknown fields/receipt/signed encoding | §§4–10,19 |
| RFC-001/004/005/006 | kernel/parser TCB, private Pager fault, M0 budget, service A/B and trust graph | consulted as boundary controls |

## 2. Confirmed conflict and boundary ambiguities

### CX-01 — ChannelEndpoint DERIVE both allowed and forbidden (C / P0)

**Text evidence:** RFC-003 §4 object's rights table lists ChannelEndpoint **SEND, RECEIVE, WAIT, TRANSFER, DERIVE, QUERY_BASIC**. Immediately below, §4 says **E1 ChannelEndpoint 자체는 DERIVE/DUPLICATE 불가 타입**. §5 declares one effective owner-handle per Endpoint and §20 IPC-D13 explicitly forbids DERIVE/DUPLICATE; E2 is a separate future study. RFC-002's DERIVE is type-dependent, so its generic rule does not override the E1 exception.

**Risk:** implementing the table literally would allow independently held Endpoint copies, invalidating E1 ownership/order and the assumed client provenance model.

**Proposed surgical correction, not applied here:** RFC-003 §4 **E1** ChannelEndpoint rights table should list **SEND, RECEIVE, WAIT, TRANSFER, QUERY_BASIC** with **NO DERIVE/DUPLICATE**. Keep E2 rights as future separately authorized profile. Cross-check whether both SEND and RECEIVE are allowed at one endpoint (type-specific direction). **Exit:** RFC-003+002 agree one owner, one MOVE linearization, and audit target updated with its own revision commit.

### CX-02 — several clients at a service vs one owning E1 Endpoint (A / P0)

**Evidence:** RFC-003 §5 says only one owning Task can hold the same E1 Endpoint; same Task's Threads can share it and Task-to-Task ownership moves. RFC-006 §14 **explicitly** requires separate client channels/dispatcher, not a globally duplicated Endpoint. RFC-018 §19.3 discusses two clients C1/C2 possessing Endpoint caps from a service broker without saying whether these are distinct channel objects. That example is consistent **only if they have distinct Endpoint objects**, or if ownership is sequential MOVE, not simultaneous E1 multi-Task ownership.

**Proposed design clarification:** a logical service supports many per-client E1 channels via an authorized connection factory/dispatcher, each with a **unique owning Task and scoped grant**; the shared ServiceID is a registry name, not an Endpoint object. E2 would require explicit separate design. **Exit:** document connection establishment, issuer, each Endpoint holder and sponsor, revocation/death/reconnect and worker transfers.

### CX-03 — actual caller identity vs delegated bearer cap (A / P0)

**Evidence:** RFC-003 §4 lists trusted badge/peer-label as a *candidate*, §7 binds kernel ReplyToken to Task/Thread/Endpoint identity; RFC-002 distinguishes issuer/controller/holder/sponsor. RFC-018 §§4/19 requires a trustworthy per-call identity for identity-sensitive methods and warns about confused deputy.

**Risk:** a bearer Endpoint is a capability to invoke operations, not proof of original human identity or consent. Proxy can act within its native grants but cannot assert that it *is* the original user.

**Exit:** choose bearer-only policy for each method OR specify kernel/issuer-attested authenticated caller/delegation chain (generation and privacy included). Do not infer an authenticated human from client bytes, PID/UID0 or ServiceID. No such complete call-origin KABI is fixed yet.

### CX-04 — SABI session policy epochs vs kernel ReplyToken scope (A / P0)

**Evidence:** RFC-003 §7 deliberately binds the kernel one-shot ReplyToken to kernel Endpoint/caller/request generation, not userspace ServiceIncarnation; RFC-006 §14 supplies fresh service endpoint per incarnation; RFC-018 §§4/19 pins negotiated features to current Endpoint issuance, ServiceInstance and issuer policy epoch.

**Risk:** kernel parsing user-written ServiceID/IDL epoch broadens Ring-0 TCB; a userspace cached handshake held past B revoke or changed server policy can reauthorize old permissions.

**Proposed owner split:** kernel enforces kernel object/generation/ReplyToken and cap scope; Supervisor/issuer enforces genuine ServiceInstance/policy epoch and new grants; service checks current live policy before *new* effect admission. Specify what happens to already admitted work. **Exit:** independent epoch writer, expiry/rotation, rebind and race linearization.

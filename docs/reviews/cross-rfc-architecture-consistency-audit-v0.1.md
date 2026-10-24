# UniOS — Cross-RFC Architecture Consistency Audit v0.1

> **Status:** Research Audit — Original Findings + Editorial Corrections (2026-10-25 NZDT); **no RFC or ADR approved**, no stable ABI, no source implementation, formal proof or hardware test.
> **Scope:** [Architecture v0.2](../architecture/unios-architecture-v0.2.md), [RFC-002](../rfcs/rfc-002-object-capability-lifetime-delegation-revocation.md), [RFC-003](../rfcs/rfc-003-ipc-wire-protocol-and-synchronization.md), [RFC-007](../rfcs/rfc-007-user-space-drivers-dma-and-iommu.md), [RFC-008](../rfcs/rfc-008-storage-filesystems-and-crash-durability.md), [RFC-009](../rfcs/rfc-009-native-abi-and-system-call-contract.md), [RFC-010](../rfcs/rfc-010-linux-personality-and-binary-compatibility.md), [RFC-017](../rfcs/rfc-017-verified-boot-updates-and-rollback-protection.md), [RFC-018](../rfcs/rfc-018-versioned-idl-abi-evolution-and-compatibility-contracts.md).
> **Consulted for cross-boundary constraints:** [RFC-001](../rfcs/rfc-001-kernel-boundary-and-tcb.md) kernel TCB, [RFC-004](../rfcs/rfc-004-virtual-memory-pager-cache-and-oom.md) Pager/VM, [RFC-005](../rfcs/rfc-005-scheduling-preemption-and-cpu-budget.md) CPU/IRQ, [RFC-006](../rfcs/rfc-006-boot-trust-chain-service-startup-and-recovery.md) recovery/A-B.
> **Method:** read live Git main markdown, compare typed rights, authority owner, commit cut, resource sponsor, failure and restart/boot dependencies; reason about adversarial interleavings. No code search, test, model checking, device/crypto verification or end-to-end mathematical proof performed.

## 0. Audit verdict and evidence classes

**Overall: conditionally consistent design directions, but NOT ready to accept or freeze.** The eight principal RFCs agree on capability-based kernel minimality, IPC SEND/MOVE semantics, CPU/DMA cutoff separation, storage durability as hardware-conditional, no ambient Linux guest root, signer/floor/data-schema independence, and strict unknown resource semantics. These are mutually compatible **Proposed research contracts**, not verified engineering achievements.

**Confirmed historical contradiction, now corrected in Proposed text:** RFC-003 §4 originally listed DERIVE although E1 prose and IPC-D13 forbade it. A subsequent design-only edit removed the conflicting entry; model proof remains OPEN. **Cross-document ambiguity:** RFC-018 multi-client Endpoint examples could be misconstrued as multi-Task simultaneous ownership of one E1 Endpoint, whereas RFC-003/006 require distinct channels or an explicit E2 change. Neither finding is a discovered implementation bug: UniOS remains design-only.

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
| RFC-017 v0.2 | authenticated boot, independent Root/floor, Good state, physical rescue/key/schema | §§3,5–10,20 |
| RFC-018 v0.2 | delegated Endpoint/identity, cap-aware IDL, ServiceInstance/policy epoch, unknown fields/receipt/signed encoding | §§4–10,19 |
| RFC-001/004/005/006 | kernel/parser TCB, private Pager fault, M0 budget, service A/B and trust graph | consulted as boundary controls |

## 2. Confirmed conflict and boundary ambiguities

### CX-01 — ChannelEndpoint DERIVE both allowed and forbidden (C / P0)

**Original Git text evidence (prior to correction):** RFC-003 §4 rights table listed **SEND, RECEIVE, WAIT, TRANSFER, DERIVE, QUERY_BASIC**. Immediately below, §4 says **E1 ChannelEndpoint 자체는 DERIVE/DUPLICATE 불가 타입**. §5 declares one effective owner-handle per Endpoint and §20 IPC-D13 explicitly forbids DERIVE/DUPLICATE; E2 is a separate future study. RFC-002's DERIVE is type-dependent, so its generic rule does not override the E1 exception.

**Risk:** implementing the table literally would allow independently held Endpoint copies, invalidating E1 ownership/order and the assumed client provenance model.

**Separate editorial correction applied:** RFC-003 §4 E1 ChannelEndpoint rights table now lists **SEND, RECEIVE, WAIT, TRANSFER, QUERY_BASIC** with **NO DERIVE/DUPLICATE**. Keep E2 rights as future separately authorized profile. Cross-check whether both SEND and RECEIVE are allowed at one endpoint (type-specific direction). **Exit:** RFC-003+002 agree one owner, one MOVE linearization, and audit target updated with its own revision commit.

### CX-02 — several clients at a service vs one owning E1 Endpoint (A / P0)

**Evidence:** RFC-003 §5 says only one owning Task can hold the same E1 Endpoint; same Task's Threads can share it and Task-to-Task ownership moves. RFC-006 §14 **explicitly** requires separate client channels/dispatcher, not a globally duplicated Endpoint. RFC-018 §19.3 originally discussed two clients C1/C2 with Endpoint caps from a service broker without clarifying object identity. A later documentation edit now states these are **distinct E1 Endpoint objects**. That example is consistent **only if they have distinct Endpoint objects**, or if ownership is sequential MOVE, not simultaneous E1 multi-Task ownership.

**Proposed design clarification:** a logical service supports many per-client E1 channels via an authorized connection factory/dispatcher, each with a **unique owning Task and scoped grant**; the shared ServiceID is a registry name, not an Endpoint object. E2 would require explicit separate design. **Exit:** document connection establishment, issuer, each Endpoint holder and sponsor, revocation/death/reconnect and worker transfers.

### CX-03 — actual caller identity vs delegated bearer cap (A / P0)

**Evidence:** RFC-003 §4 lists trusted badge/peer-label as a *candidate*, §7 binds kernel ReplyToken to Task/Thread/Endpoint identity; RFC-002 distinguishes issuer/controller/holder/sponsor. RFC-018 §§4/19 requires a trustworthy per-call identity for identity-sensitive methods and warns about confused deputy.

**Risk:** a bearer Endpoint is a capability to invoke operations, not proof of original human identity or consent. Proxy can act within its native grants but cannot assert that it *is* the original user.

**Exit:** choose bearer-only policy for each method OR specify kernel/issuer-attested authenticated caller/delegation chain (generation and privacy included). Do not infer an authenticated human from client bytes, PID/UID0 or ServiceID. No such complete call-origin KABI is fixed yet.

### CX-04 — SABI session policy epochs vs kernel ReplyToken scope (A / P0)

**Evidence:** RFC-003 §7 deliberately binds the kernel one-shot ReplyToken to kernel Endpoint/caller/request generation, not userspace ServiceIncarnation; RFC-006 §14 supplies fresh service endpoint per incarnation; RFC-018 §§4/19 pins negotiated features to current Endpoint issuance, ServiceInstance and issuer policy epoch.

**Risk:** kernel parsing user-written ServiceID/IDL epoch broadens Ring-0 TCB; a userspace cached handshake held past B revoke or changed server policy can reauthorize old permissions.

**Proposed owner split:** kernel enforces kernel object/generation/ReplyToken and cap scope; Supervisor/issuer enforces genuine ServiceInstance/policy epoch and new grants; service checks current live policy before *new* effect admission. Specify what happens to already admitted work. **Exit:** independent epoch writer, expiry/rotation, rebind and race linearization.


## 3. Unified capability, IPC, hardware and storage state ownership

**This is an audit candidate, not a new accepted protocol.** The key integration mistake would be to collapse four different owners and six different completion cuts into one "done" status.

| Phase | Authoritative state owner | Observed predicate | Specifically NOT guaranteed |
|---|---|---|---|
| Handle active | kernel Task-local cap table | right/type/scope/lineage + public token generation validated | original human identity, current device ready, no stale external effects |
| SEND_PREPARED | sending Task + kernel temporary reserve | all bytes/caps checked, sender still owns | receiver got a message |
| SEND_COMMITTED (E1) | kernel queue/in-transit cap custodian | enqueue+MOVE atomic; source gone | receiver saw/parsed data; remote side effect |
| SEND_RESULT_LOST | kernel record may have committed; caller ignorant | status **UNKNOWN** if reliable result unavailable | safe retry of moved handle |
| RECEIVE_CLAIMED | kernel-backed exclusive bounded escrow claimant | second claimant excluded; charge retained | atomic or confidential user-memory copyout without proof |
| RECEIVE_CAP_INSTALLED | receiver Task cap table | entire cap vector installed; queue retired | method decoded/authorized, no post-commit copyout fault |
| SABI_METHOD_ADMITTED | service/issuer policy | schema/actual rights/identity+PolicyEpoch verified | operation applied or durable |
| SABI_EFFECT | service/backend | scoped File/Network/Device operation may have occurred | result observed; media stable; DMA quiescent |
| FS_DEVICE_ACK | StorageHost/device | request completion under device semantics | flush/FUA, journal/namespace durability |
| FS_DURABLE_RECEIPT | trusted FileSystem/VFS ledger and media assumptions | named file/dir generation and persist-barrier met | universal SSD honesty, unrelated file/slot durable |
| REVOKE_B | kernel lineage/controller | no new operations through affected grant family | old mappings stopped, prior requests rolled back |
| CPU_R1 | kernel MMU/TLB state | affected mapped CPU use cut off | DMA no longer writes |
| DMA_R2-F | privileged DeviceGroup/hardware control | relevant group/IOVA/IRQ and submitted DMA genuinely fenced | old filesystem transaction rolled back |
| R2-O | kernel/global object lifetime/sponsor | **all** independent roots/refs/fences settled, object reclaimable | prior bytes copied to arbitrary external parties deleted |
| BOOT_GOOD | trusted BootSlotStateWriter after genuine HealthReceipt | selected image passed named trial checks | floor already advanced or data rollback works |
| FLOOR_ADVANCED | independently trusted FloorWriter/monotonic anchor | security rollback admission floor raised | a usable readable rescue exists unless independently checked |

**Custodian invariant:** every live capability, kernel queue entry, receiver claim, hardware pin, dirty page and durable error/transaction record requires exactly one accountable owning state (and charges). *Grant holder*, *grant issuer/controller*, *resource sponsor*, *current caller*, *software service incarnation*, *physical DeviceGroup owner* and *receipt issuer* are often different actors.

### 3.1 Expected unified send-to-file path (all steps hypothetical)

~~~text
Task A Handle active / sender-sponsored grant
  -> kernel E1 SEND_COMMITTED + cap InTransit
  -> B ReceiveClaim + bounded bytes + cap ownership disposition
  -> B method authorize(current rights, actual caller policy, schema epoch)
  -> VFS accepted mutable effect (file generation F, txn id T)
  -> StorageHost/DeviceGroup submitted DMA, pins retained
  -> DEVICE_ACK (might include prior writes in volatile controller cache)
  -> Flush/FUA + filesystem/root/DirectorySync dependencies actually met
  -> DurableReceipt(F,T) if hardware profile supports it
  -> ReplyToken reply COMMITTED (not necessarily observed by A)
  -> A result observed OR UNKNOWN + authenticated recovery query
~~~

At every boundary, Thread death, IPC timeout, Pager fault, FileSystemService crash, DriverHost reset, power loss, revocation or policy change can occur. None of the prior steps is automatically a transaction covering all later steps. **R2-F is about safe physical memory/DeviceGroup reuse, not retroactive durability or rollback of filesystem writes.**

## 4. Joint proof gaps and collision points (CX-05…CX-17)

### CX-05 — E1 receive cap install vs observable bytes (G / P0)

**Sources:** RFC-002 §9 permits all cap installs with receive message removal or none; RFC-003 §6 distinguishes sender/receiver commit and **RCV-A ReceiveClaim** as a candidate; RFC-009 §25 requires byte-publication and handle-install proof; RFC-018 §6/§19 rejects an unknown cap *after* a committed SEND.

**Counterexample:** B's user buffer sees secret bytes, then faults before cap install or message removal; message goes back to queue for C. B saw bytes although it never became the committed receiver.

**Joint required cut:** choose exclusive ReceiveClaim/escrow and exact authorization **before protected payload publication**; define what partial user-visible bytes can never be retracted, whether retry/requeue is permitted, which Task pays escrow quota and how to handle unknown resources and receiver death. Cap installs must remain all-or-none and sender's already-COMMITTED move **cannot be refunded**. A code generator cannot prove this kernel/VM invariant. P0 owner: 002+003+004+009+018.

### CX-06 — sender commit versus status-copyout failure and ticket discovery (G / P0)

**Sources:** RFC-002 §9 COMMITTED/UNKNOWN, RFC-003 §5.2 T1 ticket (unselected), RFC-009 §5.3 post-commit copyout and §25, RFC-018 §8.

**Counterexample:** kernel removes sender FileCap and enqueues a request, but output status page is unmapped before return. Caller sees an error or dies with no operation ticket, retries SEND thinking cap still present, or recreates an external nonidempotent operation.

**Required design:** either unambiguous register-return commit point with safe context preservation or pre-reserved kernel ticket whose **identity can be recovered without relying on a lost returned ID**, bounded Task-sponsored outstanding-record enumeration and proper privacy/GC, or explicit fail-closed UNKNOWN containment. Do not conflate kernel receipt with SABI transaction ID. P0 owner: 002+003+009+018.

### CX-07 — R2-F family safety versus R2-O whole-object reuse (G / P0)

**Sources:** RFC-002 §10 distinguishes R0/B/R1/R2-F/R2-O and independent roots; RFC-007 §§7/10/12 binds IOVA, pinned frames and DeviceGroup epoch; RFC-008 §§8/9 ties dirty file pages and controller state.

**Counterexample:** Task A revokes its DmaPin family, driver claims "done", but a different independent root B holds mapping of same MemoryObject or a stale ATS/IOVA translation continues to write. Freeing frame when A's R2-F is complete could corrupt unrelated Task B or next allocator owner.

**Required predicate:** exact grant lineage and *every* relevant independent pin/map/ref, CPU R1, hardware R2-F (IOTLB/ATC/IRQ/device command and reset evidence), sponsor ledger, then R2-O frame sanitization before reuse. If unproved, quarantine frame/DeviceGroup and do not claim cleanup complete. P0 owner: 002+004+007+008.

### CX-08 — DeviceGroup isolation vs ResetCouplingSet and M0 storage dependency (G / P0)

**Sources:** RFC-007 §§6/10 has DeviceGroup from IOMMU/ACS and a distinct ResetCouplingSet covering hardware affected by group reset; RFC-006 §§7–10 needs wired recovery; RFC-008 §9 IO/flush reset; RFC-017 §9 physical rescue.

**Counterexample:** restarting a failed NIC's DriverHost performs secondary-bus reset that also resets the only boot NVMe/Pager path, while services claim recovery is independent. The unrelated logical Task can restart, but physical M0 path may be broken.

**Required graph:** device isolation groups and reset-coupling sets are separate; constrain stop/reset to verified scope, model mandatory shared siblings/firmware mappings and reserve physically independent M0 timer/IRQ/storage recovery *or fail-stop*. A device of unknown ACS/IOMMU/interrupt-remap topology is not silently admitted to strict untrusted DMA profile. P0 owner: 006+007+008+017.

### CX-09 — Driver restart vs in-flight persistent effects and remote UNKNOWN (G / P0)

**Sources:** RFC-003 §7/10 ReplyToken and incarnation, RFC-007 §12 DriverHost epoch+DMA, RFC-008 §12 IO/transaction/dirty error, RFC-018 §8 idempotency.

**Counterexample:** StorageHost dies after controller has written journal commit but before emitting the reply. New StorageHost accepts old request ID, repeats rename and corrupts namespace. Masking IRQ and achieving R2-F stops DMA but does not rollback journal writes.

**Required contract:** kernel-cap/Endpoint/DriverGroup/IOVA/Volume transaction generations, durable per-volume transaction ledger with correct ordering and honest UNKNOWN when ledger missing. Old external effects remain old despite new ServiceID. Do not automatically replay nonidempotent writes or assert exactly-once across uncoordinated devices. P0 owner: 003+007+008+018.

### CX-10 — Pager/VM + broker/driver CPU budget + M0 reserve (G / P0)

**Sources:** RFC-004 §§7–12 private FaultWaiter and resident M0; RFC-005 §§7–11 independent protected CPU admission; RFC-006 §7 entire M0 reserve; RFC-007 §11 essential IRQ/DMA; RFC-009 §5 usercopy and RFC-010 §10 broker.

**Counterexample:** critical kernel copyin from a pageable broker faults; Pager needs a driver whose IRQ is throttled or whose service waits on the same endpoint; everyone holds an IPC claim. An otherwise correct syscall API cannot complete, including failure cleanup.

**Required proof:** bound graph of physical resident frames, fault dependencies, IPC/claim credits, CPU quota, timer/IRQ route, cleanup metadata and hardware group. Show that no critical path waits on untrusted/pageable broker or on resource already held by blocked caller. Under unresponsive hardware, finite fail-stop rather than promised recovery. P0 owner: 004+005+006+007+009+010+018.

### CX-11 — Linux futex atomicity vs microkernel minimal TCB (G / P0 choice)

**Sources:** RFC-001 KABI narrow; RFC-009 §13 and §25 warns two-RPC futex invalid; RFC-010 §6 describes F-A small kernel atomic compare-block/wake vs F-B emulator vs F-C unsupported.

**Counterexample:** Linux guest Thread reads futex value through broker, another Thread changes/wakes, then broker starts WAIT; lost wake causes indefinite hang. Introducing full Linux futex/PI semantics to Ring 0 also expands trusted TCB.

**Decision required:** small typed kernel MemoryObject-scoped compare-block/wake with race/revocation/priority budget proof **OR** proven equivalent emulator **OR** explicitly exclude threaded Linux target tier. No present RFC claims full Linux guest support. P0 owner: 001+004+005+009+010.

### CX-12 — native SABI new status versus Linux FD/signal/errno ABI (G / P0)

**Sources:** RFC-008 §12 durability/error epochs, RFC-009 §16 guest mapping, RFC-010 §21 close/dup2/epoll/exec/SA_RESTART and RFC-018 §§7–8 unknown result.

**Counterexample:** new Native VFS returns EFFECT_UNKNOWN after a buffered write. Guest shim treats it as Linux EINTR and repeats already emitted bytes; or treats close(EIO) as FD still open although Linux FD may already be released, closing reused FD upon retry.

**Required mapping:** per-Linux-syscall+flag+partial-progress state machine bound to FDEntry/OFD generations, true native transaction/receipt status, SIG/EINTR/SA_RESTART and process-image replacement. When precise guest semantics cannot be expressed safely, narrow supported binary profile or refuse. P0 owner: 008+009+010+018.

### CX-13 — authenticated trial Good/floor versus physically usable rescue and data/key schema (G / P0)

**Sources:** RFC-006 §11 four ledgers and recovery; RFC-008 §11 DataVolume migrations/Flush; RFC-017 §§5–6/9 Root/floor and actual rescue; RFC-018 §9 canonical manifest vs SABI.

**Counterexample:** B passes service health, B migrates user data/key to V2, TPM floor rises, B or only SSD then fails. A is still signed but below floor or unable to decrypt/read V2. A Good label, old SABI-compatible RPC and rescue image digest do not make fallback possible.

**Required joint predicate:** verified usable B; independently witnessed Good and durable slot state; actual physically present/readable bootable *floor-eligible* rescue with current drivers/key/data reader; all floor-vector monotonic writes/interruptions handled; SlotWriter and FloorWriter independent; rollback-aware DataVolume schema and key migration, persistent boot-attempt counter. **Strong data-restoring fallback** cannot be promised without independent storage. P0 owner: 006+008+017+018.

### CX-14 — signed manifest bytes vs parsed policy and live executable identity (G / P0)

**Sources:** RFC-017 §§3/6/20 independent pre-existing Root/floor, RFC-006 §4/§6 verified loader and bounded descriptor, RFC-018 §9/§19 restricted canonical format, RFC-004 use-time mapping and RFC-007 pre-IOMMU DMA.

**Counterexample:** signature authenticates a blob with duplicate CBOR keys or unknown mandatory security extension. Loader/parser A grants broad cap, kernel/parser B treats same bytes differently. Or image was hashed then changed through writable alias/DMA before use.

**Required chain:** independent trusted Root and monotonic floor, exact signed role/domain/scheme/byte representation, canonical or exact parser-equivalent interpretation, unknown-critical rejection, bounded loader→kernel typed descriptor and use-time immutable/rehash mapping; constrain pre-remapping DMA exposure or decline strong boot claims. No self-authenticated manifest trust root. P0 owner: 004+006+007+017+018.

### CX-15 — source/holder/controller/sponsor charge after delegated moves and deaths (G / P0)

**Sources:** RFC-002 §§6/10/11 lineage anchor and nonowner sponsor; RFC-003 §9 charge/queue; RFC-007 §12 quarantine and pin; RFC-008 §8 dirty pages; RFC-018 §6 unknown cap settlement.

**Counterexample:** A sponsors a FileCap/Device DMA frame, MOVEs it to B, then A Task dies. B crashes while queue retains descendant cap and pinned frame. Simply refunding all of A's charges makes quotas lie; charging only B can exceed B's budget without any approved adoption.

**Required invariant:** single accountable charge ledger for live queued/claimed/cap ancestry/pins/dirty pages, authorized sponsor ADOPT or retained inherited debt, deterministic bounded cleanup after death and physical fence before release. Unknown cap parser rejection doesn't erase physical outstanding work. P0 owner: 002+003+007+008+018.

### CX-16 — text identity vs trusted issuer, ServiceRegistry and schema downgrade (G / P0)

**Sources:** RFC-006 §§8/14 ServiceManifest and registry, RFC-009 §15 discovery/issuance, RFC-018 §§4/7/19 session binding and source badges.

**Counterexample:** Old client negotiates deprecated version with broad permission; new service starts under same ServiceID with tighter consent policy; stale session/Endpoint caches grant broader access, or a proxy asserts caller ID without proof.

**Required binding:** actual issued Endpoint+grant generation, signed/approved service/image and current policy epoch, independently authorized caller where necessary, server-side minimum permissions, strict unknown resource field and no silently downgraded mandatory security semantics. Kernel should not parse full SABI ServiceID or caller string to establish its own cap. P0 owner: 003+006+009+018 (plus RFC-016 portal in later study).

### CX-17 — non-reissued public handle tokens vs finite width, old callbacks (G / P0)

**Sources:** RFC-002 §7 Task-local public anti-ABA, RFC-009 §§6/25, RFC-003 wait/Reply generations, RFC-010 FD/PID generational tables.

**Counterexample:** kernel changes internal generation but reuses exact same 64-bit public handle value in the still-running Task. A stale integer copy becomes a new unrelated authority. Similarly a reused guest FD integer is not an old epoll subscription's OFD identity.

**Required rule:** true public token nonreuse while stale Task copies remain callable, fail-closed exhaustion, distinct kernel invocation pin and lifecycle generations for Endpoint/Reply/Wait/Device/GuestFD/OFD/PID. Do not treat "has generation" as sufficient unless the public token comparison or equivalent reference isolation actually protects against old caller bits. P0 owner: 002+003+009+010+018.


## 5. Canonical contract owners and normalized vocabulary

No giant kernel-global state machine is prescribed here. This table assigns the **authoritative design description** of each state; actual enforcement may span smaller trusted components.

| Object / phase | Canonical design owner | Consumers and necessary co-review |
|---|---|---|
| Task-local handle, lineage, issuer, controller, sponsor, anti-ABA | RFC-002 | 003/007/009/010/018 |
| E1 Endpoint unique ownership and MOVE, ReceiveClaim, ReplyToken | RFC-003 with 002 type rights | 004/006/009/018 |
| ServiceIncarnation, admission, policy grant, new Endpoint issuance | RFC-006 plus 018 protocol policy | 002/003/009/010 |
| Actual issuer-attested caller identity vs bearer capability | RFC-002 and future policy/portal, RFC-018 method requirements | 003/006/009 |
| CPU virtual memory R1 / PageFault / PagerBinding | RFC-004 | 002/005/007/008/009/010 |
| Hardware DeviceGroup, ResetCouplingSet, IRQ/IOMMU/IOVA, R2-F evidence | RFC-007 with 002 stage names | 004/006/008/017/018 |
| File and DirectorySync, DirtyGeneration, writeback-error epoch, durable receipt | RFC-008 | 003/007/009/010/018 |
| Native trap/SDK ABI, guest/native isolation | RFC-009 | 003/010/018 |
| Linux ELF, futex, FD/OFD, close/exec/signal errno | RFC-010 | 004/005/008/009/018 |
| Signed boot root, floor, physical rescue/key/data schema compatibility | RFC-017 + 006 bootstrap | 007/008/018 |
| SABI method/IDL version, actual cap-role binding, current policy epoch, effect outcomes | RFC-018 | 003/006/009/010/017 |

**Proposed glossary to unify later revisions:**

- Holder, issuer, revocation controller, budget sponsor and caller are separate identities, even when one Task originally filled them all.
- Kernel Endpoint is a typed object; ServiceID is a logical name; ServiceIncarnation is the authorized running-service generation; PolicyEpoch is current issuer/service security rules; Session is a per-granted Endpoint negotiated feature/role agreement.
- R0 means local handle close; B denies new scoped actions; R1 cuts grant-derived CPU mappings; R2-F physically fences the named family of DMA/IRQ/effects or safely quarantines unresolved hardware; R2-O permits object-wide sanitized reuse only after all independent roots/refs/fences retire.
- Kernel SEND_COMMITTED, RECEIVE_CAP_INSTALLED, SABI_METHOD_APPLIED, FS_DURABLE_SCOPED and RESULT_OBSERVED name different predicates, not synonyms for success.
- Boot image verified, independent witness Good, monotonic floor raised, and physically usable rescue are different evidence predicates. A valid signature does not prove all.

## 6. Bounded common-state research model (none executed)

Use a conceptual finite model with two senders, one receiving service, one rival receiver, one Pager, one DriverHost/DeviceGroup, one FileSystemService, wired M0, Boot SlotStateWriter and independent FloorWriter. Include one E1 Channel capacity 1, one MOVE FileCap, one ReceiveClaim, one ReplyToken, one DMA pin shared by two independent roots, one dirty file generation and sync, one service restart, one version/policy change and A/B DataVolume V1/V2.

**Adversarial matrix to model later (not simulations performed now):**

| Case | Interleaving | Safety property |
|---|---|---|
| JM-01 | SEND commits MOVE; sender loses result; old cap reused | no false NOT_COMMITTED or blind resend |
| JM-02 | ReceiveClaim; partial secret byte copyout; cap installation fails; rival later receives | no unwarranted atomic/confidential receive claim |
| JM-03 | Unknown DeviceCap decoded then handle closed; DMA continues | close is not physical R2-F |
| JM-04 | Revocation B races with queued transit cap and receive | receiver cannot gain new revoked authority |
| JM-05 | Old public Handle value reused for new object while Task alive | stale integer cannot authorize unrelated object |
| JM-06 | SABI negotiated v1; security policy changes; cached session used | no downgrading authorization through old handshake |
| JM-07 | Old StorageHost commits to journal then crashes; new instance sees same request | no duplicate/false exactly-once effect |
| JM-08 | Group A R2-F done; independent group B still pins shared frame | no premature R2-O or frame reuse |
| JM-09 | Driver reset of one group also destroys only wired M0 storage/IRQ path | no false independent recovery promise |
| JM-10 | Guest futex compare read; other guest Thread wakes; broker blocks later | no missed wake from split RPC |
| JM-11 | B Trial migrates data V2, floor advanced, only A signed rescue lacks key/schema | no incorrect guaranteed rollback |
| JM-12 | Signed BootPolicy duplicate critical key parsed differently; executable changed after verification | no signed-but-ambiguous grant or mutable boot code |

**Safety invariant candidates:** conserve live kernel cap ownership at every phase; never accept a revoked cap or recycle its old public token into fresh unrelated rights; never report hardware DMA/FS durability without hardware-relevant evidence; never import guest UID0 or schema version string as native rights; preserve current service PolicyEpoch; forbid security-floor promotion that strands every authorized usable rescue. **Conditional liveness** assumes actual CPU, timer/IRQ, Pager and hardware availability. This is a design model outline, **not** a completed formal model or proof.

## 7. Prioritized design change requests and proposed ADR package

| Priority | Suggested action | Authoritative files to reconcile |
|---|---|---|
| **P0 A-01** | Resolve CX-01 E1 ChannelEndpoint DERIVE rights table against no-DERIVE text; reserve E2 | RFC-003 §4 / RFC-002 type-grant policy |
| **P0 A-02** | Define per-client E1 channel factory/dispatcher, actual caller proof vs bearer permission, policy epoch | RFC-003/006/009/018 |
| **P0 A-03** | Unified SEND/ReceiveClaim/cap transit/copyout lost result and unknown-resource settlement state machine | RFC-002/003/004/009/018 |
| **P0 A-04** | Explicit operation-ticket or fail-closed UNKNOWN result, including lost ticket identity and authenticated query | RFC-003/009/008/018 |
| **P0 A-05** | Physical DeviceGroup versus ResetCouplingSet, R1/R2-F/R2-O, quota/quarantine and M0 independence | RFC-002/004/005/006/007/008 |
| **P0 A-06** | Cross-service VFS/Pager/StorageHost effect ledger, writeback epoch, sync scope and restart reconciliation | RFC-003/007/008/009/018 |
| **P0 A-07** | One A/B Good, slot retry, independent security floor, actual rescue/media, data format/key and power-cut graph | RFC-006/008/017/018 |
| **P0 A-08** | Define signable manifest canonical grammar, parser equivalence and use-time immutable image authority | RFC-004/006/007/017/018 |
| **P0 A-09** | Choose limited Linux guest futex kernel assist vs proved emulator vs explicitly unsupported threading | RFC-001/004/005/009/010 |
| **P0 A-10** | Close caller identity/ServiceInstance/PolicyEpoch TOCTOU and unknown native status mapping to guest errno | RFC-002/003/006/009/010/018 |
| **P1 A-11** | Harmonize cross-RFC glossary, status/effect naming and type-specific resource role descriptions | RFC-002/003/007/008/009/010/017/018 |
| **P1 A-12** | Maintain evidence matrix for abstract model, actual test, reference hardware, security/legal review and Accepted ADR | README/architecture/RFC index |

**Editorial exception:** CX-01 could be corrected immediately as a *separate, carefully reviewed design text change*. This audit deliberately **does not mutate any existing RFC**, so the original Proposed decisions remain traceable; do not silently upgrade a decision to Accepted.

## 8. Research order and combined Go/No-Go

| Stage | Cross-RFC work | Required exit (none yet met) |
|---|---|---|
| R1 — Ownership/IPC/IDL | 002+003+009+018: E1 endpoint, MOVE ticket, ReceiveClaim, unknown cap and actual caller | one complete cap/byte/claim/revoke/lost-output transition graph without double grant or unaccounted escrow |
| R2 — Hardware/memory/recovery | 002+004+005+006+007: M0 reserve and IRQ, DeviceGroup, ResetCouplingSet, DMA/IOVA and physical reuse | quantified reference-platform isolation assumptions and honest R2-F versus R2-O |
| R3 — Crash storage | 003+007+008+009+018: I/O, dirty generations, Device ACK, scoped File/DirectorySync and lost receipts | modeled crash-consistent state under named honest storage/flush hardware assumptions |
| R4 — Boot/root/floor | 006+008+017+018: slot Good, root rotation, firmware/TPM floor, data schema/keys and actual rescue | post-promotion valid boot branch or documented fail-stop in every defined power-cut state |
| R5 — Guest ABI | 009+010+018: trusted Linux trap/broker, futex, signal, FD/epoll and status translation | named exact guest compatibility tier, no unauthorized native grant or false app-semantic promise |
| R6 — Independent consistency pass | all affected specifications/ADRs re-read after each bounded revision | no conflicting rights tables, each P0 obligation assigned owner/evidence and separate approval |

**Decision: NO-GO** for stable syscall headers, native ABI v1, production secure-boot/recovery guarantees, default hardware driver safety or Linux binary support claims. **GO** for further written design research and targeted documentation corrections only.

## 9. Official context and audit limitations

Internal Git markdown is the authoritative **evidence of what the current UniOS proposals say**; its existence is not proof they are correct. Technical comparisons checked or already documented by prior RFC work:
- [Fuchsia Channel kernel-object contract](https://fuchsia.dev/fuchsia-src/reference/kernel_objects/channel) — nonduplicable endpoint ownership in Fuchsia, not proof UniOS implemented E1.
- [Fuchsia FIDL language](https://fuchsia.dev/fuchsia-src/reference/fidl/language/language) — unknown method strictness and value/resource types.
- [Linux futex(2)](https://man7.org/linux/man-pages/man2/futex.2.html) — atomic compare-and-block requirement.
- [Linux close(2)](https://man7.org/linux/man-pages/man2/close.2.html) — FD early release/error and retry hazard.
- [CBOR RFC 8949](https://www.rfc-editor.org/rfc/rfc8949.html) — deterministic encoding profile and map duplicate policy considerations.

**Scope limit:** cross-reading twelve large current Proposed RFCs is not an exhaustive formal consistency theorem. No kernel/source implementation was examined or modified, no generated artifacts/code, no QEMU or physical tests, no DMA or power-cut experiments, no proofs and no independent audit certification. All findings and recommended ADR packages remain research only.

## 10. Follow-up editorial reconciliation — 2026-10-25

This is a **design-document correction notice**, not a second formal audit or an Accepted ADR.

| Historic finding | Edit verified in Git | Still-open design work |
|---|---|---|
| CX-01 E1 Endpoint DERIVE table conflict | RFC-003 §4 corrected to no DERIVE/DUPLICATE on E1 (commit 239317e) | concurrent MOVE/ReceiveClaim/handle ABA remain P0 |
| CX-02 Client Endpoint ownership ambiguity | RFC-018 §19.3 clarified **distinct per-client E1 Endpoint objects**, not same simultaneously shared Endpoint (commit a176049) | connection factory partial delivery and caller identity remain P0 |
| CX-03/CX-04 Origin, principal and policy epoch | [CAPIPC Joint Contract v0.1](../contracts/capability-ipc-sabi-integration-contract-v0.1.md) defines their separation as Proposed | no trusted policy/caller kernel interface or validated session cutoff yet |
| CX-05–17 Joint storage/DMA/boot/guest proof gaps | CAPIPC includes scoped cap/IPC/SABI subset interleavings and research gates | all physical and durability proof gates OPEN |

The original audit remains useful as a **historical finding**; it must not be cited as evidence the conflicting E1 rights-table entry still exists. All RFCs remain Proposed and no implementation or test is authorized.

---

**Audit history:** 2026-10-25 v0.1 first cross-RFC audit. One confirmed internal spec-text contradiction (CX-01), three ambiguities (CX-02…04), thirteen joint gaps (CX-05…17). Twelve conceptual race cases, shared phase/custodian chart, canonical contract owner table and research roadmap. Initial audit changed no RFC; subsequent narrowly scoped RFC-003 and RFC-018 design-text clarifications were applied. No code changed.

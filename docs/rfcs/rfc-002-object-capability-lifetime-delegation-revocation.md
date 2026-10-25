# UniOS RFC-002 — Object, Capability, Lifetime, Delegation & Revocation

> **Status:** Proposed — Specification v0.2 (critical review revision; not accepted or validated)
> **Date:** 2026-10-24 (NZDT)
> **Baseline:** [Architecture v0.2](../architecture/unios-architecture-v0.2.md) and [RFC-001 v0.2](rfc-001-kernel-boundary-and-tcb.md).
> **Scope:** Kernel objects, handles, rights, creation authority, quotas, derivation, transfer, revocation, destruction, safety and verification.
> **Restriction:** Architecture research only; no implementation authorization, binary compatibility claim, stable ABI, security certification or hardware support assertion.

## 핵심 결정 요약 (한국어)

**RFC 상태는 Proposed v0.2이다.** 기존 D1~D8과 RFC-001의 커널 경계를 상세화하는 *설계 가설*이며, 커널이나 서비스의 구현·검증·ABI 동결을 뜻하지 않는다. 세부 계약이 명확해졌다는 이유만으로 아키텍처의 안전성이 입증되지는 않는다.

**1차 연구 권고: H1 — 실용적 혼합형 Capability 객체 모델.** 독립 커널에는 Task별 opaque handle table, type-specific 권한, 객체 생성 허가, 리소스 쿼터, 권한 파생 기록을 두고, VFS·권한 UI·네트워크·호환성 정책은 사용자 공간에 둔다. 다만 H1은 *유일한 최선*으로 확정하지 않는다. 자원 재사용과 회수의 일관성을 명시적으로 보장하는 seL4형(H0), 단순하고 성숙한 핸들/Job 모델의 Zircon형(H2), 서비스 RPC 프록시(H3)를 함께 비교한다.

**객체의 5가지 지위를 혼동하지 않는다:** (1) 생성한 Task, (2) 현재 핸들 보유자, (3) 실제 권한을 부여한 Grant, (4) 커널 메모리·IPC·페이지·DMA 비용을 부담하는 Sponsor, (5) 객체를 살아 있게 하는 모든 참조. 예를 들어 A가 MemoryObject를 만들고 B에 읽기 권한을 주더라도, B가 핸들을 받았다는 이유만으로 A의 리소스 비용이 사라지거나 B에게 권한 재위임권이 생겨서는 안 된다.

**권한 회수와 객체 전체 재사용은 구분한다.** R0=핸들 종료, B=신규 권한 행사 차단, R1=권한 계보 및 의존 CPU 매핑 접근 차단, R2-F=해당 계보의 기존 작업·DMA 안전 중단, R2-O=모든 독립 권한·참조까지 해제되어 객체 전체를 재사용할 수 있는 상태. **R2-F가 성립해도 다른 독립 권한이 남으면 R2-O는 불가능하다.** 이미 읽은 비밀과 완료한 I/O는 되돌릴 수 없다.

**권고 변경 가능 조건:** R1 강제 경로가 IPC 성능·커널 TCB를 과도하게 증가시키거나, 권한 파생 그래프의 대량 정리를 안전하게 수행할 수 없거나, 자원 쿼터 회계가 유실되는 반례가 발견되면 H1을 고수하지 않는다. H0/H2로 전환하거나, 일부 서비스에서는 커널 레벨 회수를 포기하고 서비스 수준 접근 중지만 제공할 수 있다. 그 경우 보안 목표·지원 범위 차이를 공개해야 한다.

**v0.2 주요 수정:** 원본 핸들 종료 후 파생 권한의 계보를 유지하는 Anchor/RevocationController, IPC prepare/commit/report와 결과 불명확성, R2-F vs R2-O, 공유 메모리 회계와 범위 축소까지 재정의했다.

**검증 순서:** CAP-01~22의 모델·반례 설계 → RFC-003 IPC 원자성 + RFC-004 매핑/페이지 + RFC-006 부팅/종료 공동 검토 → RFC-007 DMA 그룹 신뢰 조건 → RFC-009/010 Linux 제한 실행 → RFC-015 보안 게이트. 미해결 P0 11건이 남아 있으며 R1 설계 종료를 선언하지 않는다.

---

## 0. Executive recommendation

**Proposed comparative baseline H1:** task-local opaque handles to typed kernel objects, kernel-enforced fine-grained rights, explicit creation authority and hierarchical resource budgets, provenance-aware delegated grants, and a distinct revocation/physical-reclamation lifecycle.

This compares seL4's explicit authority/derivation with Zircon's process-local handle and job-policy design. **Zircon's jobs are not evidence of a universal UniOS per-object quota ledger; no proposed combination or performance outcome is validated**. It must be measured and model-checked against:
- **H0: seL4-like explicit untyped/retype** and CSpace/derivation capabilities.
- **H2: Zircon-like typed handles + Job/Quota** with no generic strong transitive kernel revoke.
- **H3: service-controlled RPC proxy/lease** for resources whose revocation needs no kernel mappings or hardware control.

H1 is provisionally recommended for research because a developer-first desktop needs convenient creation, user-space policy brokers, IPC handle delegation and robust service restart. It **adds kernel TCB complexity**. If its lineage checking, metadata accounting or memory/DMA revocation cannot be bounded and secured, reject H1, narrow the required revoke guarantee or adopt H0/H2 with a documented trade-off.

**Foundational separation:** holder of a handle ≠ creator of its object ≠ quota sponsor ≠ root of revocation authority ≠ final live reference. Handle close ≠ subtree revoke ≠ object destruction ≠ undoing a completed side effect.

## 1. Relation to architecture and threat model

- D5 / ADR-002: independent Practical Capability Microkernel with strictly reviewed exceptions.
- D4: Rust-first hybrid; Rust borrow checker does not enforce cross-process or hardware revocation.
- D7 / ADR-009: general-user threats affect initial design despite VM/developer first delivery.
- RFC-001 KB-02/03/04/05/06/08/09/10/11/14/15: type/rights, no privilege amplification, IPC atomicity, resource reserve, guest scope, DMA, restart, stale state and trusted bootstrap.
- RFC-003/004/005/006/007/009/010/015/016/018 must later co-review these contracts.

Threats: malicious sandboxed process, compromised service or user driver, adversarial handle-number guessing, stale handle reuse/ABA, racing threads, cap laundering across IPC, resource/queue denial of service, sponsor death, lost revoke barrier, page-table/TLB remanence, outstanding DMA, compatibility guest escalation, and boot authority overgranting.

**Non-goals:** final syscall numbers, 32/64-bit public ABI freeze, userspace VFS inode or Linux FD objects, GUI permission UX, definite allocator algorithm, virtual memory policy, full device support, actual proofs or running software.

## 2. Primary-source comparison

| Model | Verified documented behavior | Strength | Important limitation for UniOS |
|---|---|---|---|
| **seL4** [S1,S2,S3] | CSpace/CNode capability addressing; copy/mint/move; untyped retype; derived-cap Revoke | Explicit authority/provenance; resource manager can reclaim derived capabilities | CSpace and untyped management, fragmentation and large revoke complexity; not automatic desktop APIs |
| **Fuchsia Zircon** [S4–S8] | Typed process-local handles; rights/duplicate/replace; handle transit through channels; jobs/policies and refcounted lifetime | Strong developer usability; clear process ownership and kernel object semantics | Generic handle close does **not** equal descendant-tree revoke; handle integer reuse and race hazards |
| **H1 UniOS (proposal)** | Kernel lineage records plus typed handle table and sponsor budgets | Enables selective lineage revoke while retaining friendly object factories | More kernel metadata, quiescence and accounting complexity; requires proof of bounded behavior |
| **Proxy or leased server cap** | User service validates each RPC | Can avoid large kernel lineage for file-like semantics | Does not unmap already granted pages or stop DMA |
| **CHERI concept** [S9] | Hardware-enforced spatial tagged capabilities on supported hardware | Highlights authority attenuation and pointer validity | Not initial x86_64 feature; temporal revocation requires more than spatial bounds |

References are research evidence for **external designs**, not assurance or implementation evidence for UniOS.

## 3. Vocabulary and three independent graphs

**Object:** typed, kernel-managed resource identified by a nonauthorizing internal ObjectID; may be reachable through multiple capabilities.

**Capability/grant:** kernel-resident, unforgeable authority record referring to an object, rights, scope, lineage/revocation family and validity state. Serializing its metadata into user memory does not create authority.

**Handle:** task-local numeric reference to a capability in the kernel-maintained handle table. Values are **not secrets**. Equal handle numbers in different Tasks have no shared meaning.

**Holder:** task with access to handle; **Creator:** principal requesting initial creation; **Sponsor:** account charged for resource use; **Controller:** party holding type-specific control rights; **Revocation authority:** explicitly authorized parent/grant-family node able to revoke descendants. These roles may differ.

**Reference:** handles, queued in-transit capabilities, kernel invocation references, mappings, waits, reply tokens, pinned pages, interrupt bindings and DMA that keep objects or effects alive.

**Provenance lineage:** parent–child relationship for grants. Independently authorized roots can reference the same object; revoking one does not cancel the other. **RevocationAnchor:** kernel-held ancestry/control metadata that must survive ordinary parent-handle close while descendants or effects remain. **RevocationController:** separate, explicitly scoped authority over that lineage, not a global object-destruction right.

Maintain three separately auditable graphs:
1. **Authority/derivation graph** — whose grant is derived from which other authorized grant?
2. **Sponsor/charge graph** — which ResourceBudget pays for each live object, handle slot, metadata allocation, pinned page and queued transfer?
3. **Lifetime/dependency graph** — which references, mappings, operations and hardware state prevent safe free/reuse?

No security property may depend on a single owner pointer when these graphs disagree. Track a fourth relation: controller-to-lineage scope. Controller transfer and parent close must not silently expand or erase revoke authority.

## 4. Initial object taxonomy (candidate only)

| Object candidate | Kernel minimum responsibility | Required initial authority | Extra lifecycle constraints |
|---|---|---|---|
| Task | protection principal, handle namespace, teardown | typed TaskCreate + sponsor budget | close handles on exit, not globally revoke external grants |
| Thread | execution context, preemption, fault state | ThreadCreate scoped to Task | thread handle close is not thread termination |
| AddressSpace | page-table map/unmap enforcement | scoped MapControl | mapping provenance and TLB shootdown |
| MemoryObject | frame identity, shareable backing/pinning | MemoryCreate + budget | maps and DMA refs outlive handle |
| ChannelEndpoint | IPC ownership and bounded queue | IPC/Create + queue budget | transit grants and cycles |
| Event/WaitSet | wait/signal, registration | type-specific Create | cancel pending waits |
| Interrupt | owner, mask/ack/bind | isolated DeviceGroupResource | IRQ remap/resume semantics |
| DeviceGroupResource | MMIO/PCI/IOMMU-group resource | privileged boot device grants | DMA, IRQ and reset fence |
| IOMMUDomain / DMA binding | protected bus mappings | group authority + pinned memory scope | quiescence before reuse |
| ResourceBudget | charge reservation and child budget limits | BudgetSubdivide/Allocate | sponsor survives creator Task until charges settle |
| Reply/FaultToken | one-shot permission to resume specific blocked context | generated by authorized IPC/fault | invalidate on consume, timeout, task/service generation change |
| GuestRestrictedContext | per-thread restricted-mode state if RFC-001 G-R adopted | sandbox Task/Thread-specific launcher grant | never an arbitrary TaskControl grant |

Timer, scheduling context, graphics and hypervisor objects remain RFC-005/013/014 candidates. Kernel VFS inode, POSIX file descriptor, Linux PID, PE handle and Darwin port are **not** kernel-object defaults.

## 5. Type-specific rights and authority

Common candidate rights: QUERY_BASIC, WAIT, SIGNAL, READ, WRITE, MAP, EXECUTE_MAP, DUPLICATE, TRANSFER, DERIVE, CONTROL, CONFIGURE, BIND, REVOKE_DESCENDANTS, CREATE_SUBOBJECT. Their applicability and implied operations are **type-specific**; READ on a MemoryObject is not equivalent to ChannelEndpoint RECEIVE; CONTROL must not imply a global ADMIN privilege. Specialized rights (MMIO, DMA_MAP, IRQ_BIND, DEBUG_OTHER, TASK_START/STOP, BUDGET_DELEGATE) need separate typed authority.

For an **existing object's grant**, effective_rights(child) is a subset of effective_rights(parent) intersected with allowed_rights(type), AND effective_scope(child) is within effective_scope(parent) (Task, page range, device group and duration where applicable). Rights bits alone are insufficient. A holder can remove DUPLICATE/TRANSFER/DERIVE to prevent onward spreading of that grant; it cannot restore a dropped power. A new object is different: explicit Create authority + budget legitimately permits the kernel to issue an initial cap to **a new object**, rather than copying rights from an unrelated cap. This does not mint arbitrary capability access to existing objects.

**Proposed DERIVE vs TRANSFER:** DUPLICATE/DERIVE authorizes the creation of a distinct same-object grant under an attenuated rights set. TRANSFER authorizes moving possession without gaining new powers. Moving with attenuation may drop rights, but it must not erase ancestry. Combining these bits as one generic privilege needs explicit review.

**Not sufficient for native privilege:** guest UID 0, Windows Administrator, Darwin entitlement, policy prompt, file name, registry lookup, random ID, copied handle integer, filesystem ACL. Such data may guide a user-space broker decision but cannot bypass the kernel's existing grant chain.

## 6. Allocation authority and hierarchical sponsor accounting

### 6.1 Resource creation is not simply budget consumption

An object create requires **both** (a) a typed Create/Control capability for the relevant namespace/Task/device resource and (b) a ResourceBudget with available reserve. Money does not confer access: a memory quota alone does not grant MMIO or a foreign Task handle.

H0 approach: explicit untyped memory authority and retype to typed objects [S2]. H1 approach: kernel-managed typed creation with narrow creator cap, explicit hierarchical charge and a contained allocator. H1 should be rejected if bounded allocation/OOM behavior cannot be proven. H2 simpler Job policy remains a serious alternative.

### 6.2 Accountable resource categories

Proposal: charge for object instances, handle slots, derivation/family metadata, IPC queued bytes/messages/transit handles, page-table/mapping metadata, resident/pinned pages, wait registration and critical reserved operations. CPU budget and DMA throughput policies go to RFC-005/007. Subbudgets must not exceed ancestors' aggregate reservations and commitments.

- Each object/metadata allocation has an accountable primary sponsor and, as needed, **additional reference/holder/queue/mapping charges**. One shared/COW page sponsor does not mean all additional consumers are unmetered; conservative aggregate accounting is a P0 research requirement.
- A handle's holder can change without silently moving its sponsoring debt. An explicit, authorized and atomic **sponsorship adoption** is required for charge transfer.
- Moving a handle between Tasks requires an available receiver handle-slot charge on **atomic receive/install** or a separately documented earlier reservation. While the message is queued, its credits/bytes/grants remain charged to an explicit queue/sender sponsor; successful enqueue is not receiver installation.
- Creator Task death does **not** magically discharge still-live shared objects. Sponsor-budget nodes survive until their objects are reclaimed/adopted, or remain frozen with explicit accounting.
- An untrusted app cannot consume the wired boot/Pager recovery reserve by endless derive/create/queue operations.
- An allocation that exhausts a noncritical quota must fail with a bounded, non-panicking error; partial created objects and charges must roll back.
- Budget termination with external caps is not forced object destruction: propose Freeze → stop new charges → authorized revoke/quiescence **or** still-charged orphan sponsor → final cleanup.

**P0:** exact charging of shared pages, channel metadata, lineage nodes and asynchronously reclaimed hardware buffers needs a formal invariant and a worst-case overhead estimate.

## 7. Handle design and ABA / token reuse

**Proposed early x86_64 handle:** 64-bit per-Task opaque integer. Do not fix index/generation/tag bit layout or declare KABI stable. Kernel must validate ownership, type, rights, lifetime, revocation family and operation scope **on invocation**; user-manipulated integer does not carry authority by itself.

**Security requirement:** after handle h was closed in a Task, an old copy of h must not become a valid unrelated authority during that same Task lifetime through token reuse. Explore monotonic identifiers or a nonwrapping generation scheme with a fail-closed exhaustion path. Merely claiming generation protects against ABA without explaining wrap/reuse is unacceptable. Non-secret token encoding is fine; unguessability is not the security boundary.

A successful lookup may acquire an **invocation reference** pinning the object while a syscall proceeds. Revoke and close races have explicit before/after commit semantics; a reference pin must never let a newly created unauthorized operation start after the revoke barrier.

Debug ObjectIDs/koids can aid observability but must not be usable as alternative handles. QUERY_BASIC and tracing require metadata disclosure controls.

## 8. Capability/object state machines

### 8.1 Object

~~~text
Allocated -> Validated/Live
Live -> Quiescing (new resource uses constrained)
Quiescing -> Reclaimable (all refs/fences drained)
Reclaimable -> Destroyed (sanitized and safely freed)
Quiescing -> Quarantined (unfenced DMA, IRQ or other unsafe dependency)
Quarantined -> Reclaimable only after verified fence/reset or controlled reboot
~~~

A closed handle may leave a live object: other handles, IPC queues, mapped memory or kernel operations may reference it. Refcount-zero is neither equal to revocation nor a universal promise of synchronous destruction. Reuse across security owners requires physical frame/page zeroing or equivalent sanitization and correct MMU/IOMMU invalidation.

### 8.2 Capability

~~~text
Unpublished -> Active[Task A]
Active[Task A] -> InTransit[bounded IPC queue] -> Active[Task B]
Active -> Active parent + Active attenuated child (DERIVE)
Active -> Closed (local close or successful MOVE from source)
Active or InTransit -> Revoking (ancestor revoke barrier set)
Revoking -> Revoked (no new use, all required local mappings invalidated)
Revoking -> Quarantined (hardware/resource may remain unsafe)
~~~

Each queued moved handle **retains ancestry**. Revoking an ancestor blocks both in-table and in-transit descendants. Independent caps to the same object may stay live; object-wide shutdown requires a separate authority and a multi-family policy.

### 8.3 No conflation of object/effect lifetime

Kernel must track grant-derived effects such as a memory mapping, queued reply, wait registration, DMA pin and IOMMU map **even when the invoking handle is closed**. The owner of each effect, dependency to lineage and conditions for cancel are part of later RFC-003/004/007 contracts.

## 9. Operation matrix and commit ambiguity

| Operation | Preconditions | Commit boundary and failure rule |
|---|---|---|
| create(type,budget,scope) | typed Create + sponsor reserve | initial object + grant + charge published atomically; failed precommit reservation rolled back |
| duplicate/derive | permission, rights/scope attenuation, available destination slot | new child grant and lineage published atomically, or none |
| attenuate | current holder, narrowed rights and scope | atomic replacement; prior admitted effects tracked |
| ipc_send(MOVE) | endpoint SEND, each source TRANSFER, bounded bytes/queue charge | all bytes/grants enqueued and source handles removed **at one commit point**, or no change before commit |
| ipc_receive | RECEIVE and receiving handle-slot budget | all transferred handles installed and message removed, or none |
| close | valid Task-local handle | local entry invalidated; descendants and independently held references can survive |
| revoke_descendants | exact lineage-scoped controller | B deny barrier committed; R1/R2-F results observed separately |
| revoke_wait | observe ticket for a specified phase | only report completed when phase is true; timeout/UNKNOWN is not success |
| budget_adopt | source and new sponsor authority | reserve new charge, atomically switch sponsor, release old charge |
| object_release | all independent roots, refs and fences drained | only after R2-O can the allocator reclaim and sanitize |

### 9.1 Three separate notions of IPC success

**NOT_COMMITTED:** a pre-commit error means sender retains ALL handles of the transaction.

**COMMITTED:** sender handles were atomically moved to a bounded kernel queue; later recipient crash/discard cannot retroactively undo the send. Successful enqueue is not successful receive, processing or durable external side effect.

**UNKNOWN:** syscall result could be lost after commit (thread termination, result-copyout fault, signal or interrupted completion). A generic error must not be described as guaranteed rollback. The eventual ABI must either ensure an unambiguous result boundary or provide bounded, authenticated transaction identifiers and status reconciliation. The caller must not blindly retry a MOVE or non-idempotent operation after UNKNOWN.

The kernel must snapshot/copy user buffer contents safely before the commit as needed, define what happens when copyout fails after commit, and ensure transferred source handles cannot be reattached by a race. This is a P0 RFC-003/009 obligation. Service processing still needs an independent receiver acknowledgement/idempotency scheme; kernel enqueue cannot guarantee exactly-once service effects.

### 9.2 Difference from Zircon

Basic Zircon zx_channel_write consumes attached handles on success or failure [S7]. Zircon zx_channel_write_etc distinguishes MOVE (source closed) and DUPLICATE (source kept) [S12]. UniOS's NOT_COMMITTED preserve-all proposal is separate from both. Do not conflate transactional atomicity with rollback after a committed send.

## 10. Scoped revocation and safe reclamation

### 10.1 Distinct promises — R0, B, R1, R2-F and R2-O

| Stage | Meaning | May independent roots still exist? |
|---|---|---|
| R0 local close | delete one Task handle | Yes |
| B barrier | deny all *new* actions or derivations using the selected lineage | Yes; some previously mapped CPU accesses may still be effective |
| R1 access cutoff | descendant handles unusable **and all grant-dependent CPU mappings blocked**, with required TLB synchronization | Yes; previously admitted actions/DMA may still be pending |
| R2-F family quiesced | all specified lineage's admitted operations and relevant IRQ/DMA safely drained, or pending/quarantined | Yes |
| R2-O object reclaimable | all independent roots, references and mappings retired, IOMMU/TLB/device fences complete and memory safe for allocator reuse | **No**, not with a remaining legitimate independent root |

Revocation of family A does not revoke independent family B merely because both reference the same MemoryObject. **R2-F never grants object-wide reuse by itself.** Completed file/network effects and previously copied secret bytes cannot be undone.

### 10.2 Original issuer handle closes — lineage cannot vanish

Example: A derives g1 to B from g0, then closes g0. Unless a distinct cascading revoke policy was explicitly authorized, B still holds g1. The kernel must retain a **revocation anchor**/equivalent ancestry metadata until the descendants, mapped effects and relevant controllers are retired. If A retains a separate scoped RevocationController, A may revoke descendants of g0 but not g0's ancestors, siblings, or independent root B2. Moving a descendant must preserve ancestry. Deleting an issuer's only data handle neither erases the anchor nor invents a new globally privileged controller.

Possible policy options: controller capability distinct from data capability; explicit sandbox-shutdown cascade; or explicitly irrevocable delegated rights with a separately approved trust guarantee. The final choice, including sponsor charges for orphaned anchors, is unresolved P0.

### 10.3 Proposed phased algorithm; not validated

1. Confirm exact lineage/controller scope and publish B atomically. Operations committed earlier retain in-flight references and quiescence obligations; later invocations/derivations/transfers fail.
2. Iteratively drain descendant grants, in-transit queue entries and issued effects in bounded preemptible work units. No unlimited recursive destructor or giant unpreemptible lock.
3. Before R1, invalidate all dependent CPU page mappings, synchronize inter-core TLBs and deny remapping via concurrent pager faults. A revoked-root marker alone does not stop existing mapped memory access.
4. Before R2-F, also settle all admitted operations and lineage-dependent DMA/IRQ bindings. If device quiesce/reset or IOTLB/interrupt fence cannot be proven, mark Quarantined; no false completion.
5. Before R2-O, additionally require **every independent object's root/reference** and unsafe hardware access to disappear; sanitize before allocator reuse. Never equate family cleanup with this object-wide check.
6. Retain ancestry/controller metadata (and its charges) until no epoch reader, queued descendant or effect can resurrect rights. Deep-tree lookup complexity, fairness and SMP races are P0 blockers.

### 10.4 Honest per-object revocation scope

Strong kernel-level R1/R2-F for all ordinary file-like service endpoints may impose unjustified TCB cost. User-space VFS/Portal leases can instead promise *future RPC denial* where no CPU mapping/device authority has escaped. When a MemoryObject or DMA capability has already been granted, server-only denial cannot enforce page mapping or device teardown. Initial research must compare per-object revocation classes instead of claiming a universal strong-revoke operation.

### 10.5 Broader emergency authority

TaskKill, DeviceAdmin or ObjectWideRevoke require separate, strictly scoped high-risk grants. Guest root/Administrator/Darwin entitlement never confers them. Even a legitimate object-wide kill cannot safely reuse physical memory while hardware DMA remains unfenced.

## 11. Transfer, transit, IPC cycles, death/restart

- **MOVE vs DERIVE:** MOVE transfers original grant lineage and loses source handle after successful publish; DERIVE creates a new grant node with attenuated rights. Neither may elevate authority.
- **Message transit:** kernel owns cap records while queued; it retains lineage, bounds and sponsor accounting. A failed delivery must not create half-installed receiver handles. Receiver slot exhaustion leaves the complete message queued or rejects with an explicit no-side-effect policy.
- **Endpoint close:** drop unread queued grants under an explicit accounting/cancellation contract. Outstanding operations already committed are not undone.
- **Service crash:** a restarted service gets a **new incarnation**. Old reply/fault tokens and capability paths cannot be implicitly retargeted to the new service. Reconnection requires a fresh approved grant.
- **Queue reference cycles:** channels that contain endpoints may form unreachable cycles. Reference counts alone can leak, while recursive destructor chains may overflow the kernel stack. **P0:** model bounded iterative queue drain and roots/reachability, or prohibit cyclic endpoint transfer in the first profile. The model must not assume Task death automatically breaks all cycles.
- **Grant sponsor after move:** new holder may not be resource sponsor. Sponsor budget remains charged even if its original Task is dead; quota reclamation requires authorized adoption or true resource teardown.
- **Completion caution:** sender ownership is preserved ONLY before IPC commit; COMMITTED followed by peer crash remains committed; UNKNOWN requires reconciliation. **Exactly-once caution:** cancellation/retry of a service request does not prove a file write or external side effect happened exactly once. Tie server idempotence to RFC-003 and storage RFC-008.

## 12. Boot, process isolation and guest compatibility

Kernel seeds minimal root authority and resources after verified boot assumptions; a user-space Root Resource Authority receives explicitly scoped initial grants and distributes child budgets and service caps by manifest. Its broad authority is part of TCB-B/TCB-S and should be minimized or frozen after bootstrap, but cannot be promised to disappear entirely if recovery requires it (RFC-001/006).

Native Task creation starts with **no ambient broad cap inheritance**; an authorized launcher deliberately provides startup handles. Parentage and process tree are not capability inheritance. A native exec-like action requires an explicit handle-transfer manifest. POSIX fork/exec/file descriptors are implemented by a **user-space personality within its existing native grant envelope**; they are not a general native privilege escalation feature.

Restricted Linux supervisor (RFC-001 G-R) is trusted only for its selected guest sandbox and cannot inspect arbitrary Task handles. A raw guest syscall, Linux root, Windows admin, or Darwin entitlement must never become a native capability.

**Example:** user chooses one document → File Portal validates intent → gives the app a READ-only *service capability* without DERIVE/TRANSFER → VFS handles reads under its own storage scope. App cannot search Registry to obtain sibling files. Portal revoke blocks new operations, not already read bytes.

**Device example:** boot Device Authority assigns an actually isolatable device group + IRQ and scoped IOMMU grants to a DriverHost → DriverHost pins authorized MemoryObject buffers → user-space driver queues DMA. Driver crash triggers new DMA denial and fence/reset; unsafe hardware forces quarantine instead of buffer reuse (RFC-007).

## 13. Safety failure matrix

| Fault/race | Forbidden outcome | Expected disposition |
|---|---|---|
| forged/foreign/stale handle | cross-Task authority | invalid-handle/type/rights error |
| close vs invoke on another core | dangling object pointer | linearize or reject; maintain invocation reference |
| derive vs ancestor revoke | post-barrier valid child | child included before barrier or derive fails |
| moved grant vs receiver crash | duplicated/stranded privileged handle | explicit in-transit ownership and drain |
| invalid batched IPC handles | partial transfer | all-or-none; sender unchanged on NOT_COMMITTED pre-commit failure |
| quota/OOM during receive | half-installed handles | message remains queued or defined atomic cancellation |
| revoke of mapped page | stale valid CPU mapping | MMU invalidate/shootdown before mapping R1 complete |
| revoke during DMA | frame reused under active DMA | quarantine until hardware fence/reset |
| Task/sponsor death | unaccounted live object | charge persists until reclaim or authorized adoption |
| channel holds endpoint cycle | unbounded leak / hidden authority | bounded quota + explicit cycle cleanup or feature gate |
| exhausted handle namespace | old number reauthorizes object | fail closed without stale token resurrection |
| service restart | old completion used on new instance | incarnation-scoped token invalidation |
| policy/registry manipulation | native cap minted from name | kernel requires source authority |
| tracing/debugging | object inventory or secret leak | scoped privileged debug, redaction in user release |

## 14. Proposed security invariants — not yet verified

| ID | Invariant |
|---|---|
| CAP-01 | No Task creates a valid cap via untrusted numbers, copied bytes or foreign Task's table. |
| CAP-02 | Every operation validates type, rights, scope and live/revoked state at a documented commit point. |
| CAP-03 | Existing-object derivation never increases authority; typed new-object creation requires independent authorized create + budget. |
| CAP-04 | Moving caps never duplicates ownership, strips ancestry or silently changes sponsor. |
| CAP-05 | Pre-commit failures publish no partial objects/handles and restore reservations; post-commit errors preserve authoritative committed ownership and settle charges explicitly. |
| CAP-06 | After a grant-root deny barrier, no descendant can begin a newly authorized operation. |
| CAP-07 | R1 completion cannot occur while a forbidden descendant still has a grant-derived accessible CPU mapping. |
| CAP-08 | R2-F requires documented lineage-scoped in-flight/MMU/IOMMU/DMA/IRQ quiescence; unresolved hardware remains pending/quarantined. |
| CAP-09 | Local close, Task exit, object destruction and revoke are separate operations with correct lifetime references. |
| CAP-10 | A stale handle token never reauthorizes unrelated capability within the same Task lifetime; exhaustion fails closed. |
| CAP-11 | Resource charges remain conserved across allocation, IPC transit, lineage creation, sponsor death and adoption. |
| CAP-12 | Untrusted allocations and IPC floods cannot exhaust critical boot/Pager recovery reserve. |
| CAP-13 | Guest UID0/Administrator/entitlement never becomes native object authority without explicit grant. |
| CAP-14 | ObjectID, process ID, registry name and debug metadata are not capability credentials. |
| CAP-15 | Service incarnation change invalidates stale continuation/reply/authorization references. |
| CAP-16 | Revocation never falsely promises erasure of copied secrets or rollback of committed I/O. |
| CAP-17 | Unisolatable IOMMU groups and unfenced DMA cannot be passed off as safe untrusted driver objects. |
| CAP-18 | Cyclic IPC/resource dependency cannot consume unbounded resources outside sponsor quota, with bounded iterative cleanup semantics. |
| CAP-19 | Parent-handle close and issuer death must not erase ancestry required to enforce living descendants' revocation scope. |
| CAP-20 | Sender handles are preserved only for NOT_COMMITTED IPC failure; COMMITTED and UNKNOWN results cannot be mislabeled as rollback. |
| CAP-21 | Family quiescence R2-F must not release an object while independent roots/references or unsafe device access survive. |
| CAP-22 | Revocation controller authority must not grow via handle transfer, ancestry change, sponsorship adoption or namespace reuse. |

These are **design proof obligations**, not claims of formal verification. CAP-19 to CAP-22 address newly identified IPC/lineage/object lifetime gaps. CAP-06 through CAP-08 intentionally exceed basic Zircon close semantics and may disqualify H1 if safe implementations are too complex.

## 15. Comparison, feasibility and performance gates

| Question | H0 Untyped/CSpace | H1 Hybrid candidate | H2 Handles/Job |
|---|---|---|---|
| Origin/provenance | explicit retype + derivation | new trusted lineage metadata | object/job rights, no assumed generic descendant revoke |
| Dynamic developer UX | user allocator complexity | simple typed factory candidate | convenient typed factory |
| Quota accounting | source memory regions; separate other budgets | must account handles, derivations, pin, queue | job/resource policies, still needs full accounting review |
| Transitive revoke | documented CDT-style revoke | requires proposed barrier and bounded cleanup | not automatically present |
| Memory/DMA revocation | still needs mapping/device fence | same, with stronger family coupling | same |
| Ring 0 complexity | CSpace/retype/revoke | factory + quota + derivation + barrier | handle/refcount/job policy |
| Performance | must measure equivalent semantics | no validated metrics | weaker revoke cannot be compared as equivalent |

Future measurements *after separate implementation authorization* should include cap lookup/create/derive/close/move, 1/100/10k descendant revoke, kernel memory per handle/lineage node, multi-core race tail latency, IPC in-transit pressure, mapping TLB shootdown, DMA reassign, restart and OOM. Report p50/p95/p99, worst-case work, QEMU/physical separation and functional equivalence. No timing/LOC/SLA promise in this RFC.

## 16. Planned design verification (not implementation)

1. Finite-state model with multiple Tasks, grants, ancestor roots, sponsors, transit queues, CPU mappings, DMA pins and revoke barriers. Explore close-vs-invoke, derive-vs-revoke, transfer-vs-death, sponsor-death, quota rollback, independent grants to same object and queue cycles.
2. Prove or find counterexamples for CAP-01/03/04/05/06/07/08/10/11/19/20/21/22: no amplification, no partial transfer, no resurrection, bounded accounting, deny barrier, revocation-anchor survival, lost-reply recovery and family-vs-object reclaim safety.
3. Model-check with TLA+/PlusCal, Alloy or equivalent. Choice of modeling tool alone is not proof. Keep machine-readable test traces and reviewer notes as future research artifacts.
4. Specify typed rights matrix, each syscall candidate's pre/postcondition and linearization, Task handle table lifecycle, ref-count vs grant-tree interaction, and error categories before ABI v1.
5. Cross-review with RFC-003 (IPC transfer/reply), 004 (mapping/pager/OOM), 005 (CPU budget), 006 (boot/restart), 007 (IOMMU/DMA), 009/010 (native + restricted guest), 015 (threat testing), 016 (permissions) and 018 (versioning).

**R1 exit gate is not met** until the above contracts have been reviewed and the P0 unknowns resolved. Simulation/test plans are not test results.

## 17. Open decisions / next research blockers

| ID | Priority | Unresolved design decision | Owning RFC |
|---|---|---|---|
| O2-01 | P0 | H0 vs H1 vs H2: trust closure/complexity, allocation and revoke trade-offs | 001/002 |
| O2-02 | P0 | Provenance storage and fast deny check without unbounded kernel latency | 002/003 |
| O2-03 | P0 | Revoking mappings while cores race TLB access | 002/004 |
| O2-04 | P0 | Which object types require R1/R2 or can use R0+service proxy | 002/004/007 |
| O2-05 | P0 | IPC endpoint cycles, roots and bounded reclamation | 002/003/006 |
| O2-06 | P0 | Complete resource charge across shared frames, IPC and derivation nodes | 002/004 |
| O2-07 | P0 | Restricted guest and supervisor cannot launder native grants | 009/010 |
| O2-08 | P0 | Actual device group, MSI, reset and DMA fences on Reference PC | 007/015 |
| O2-09 | P1 | 64-bit nonreusing handle format and long-lived namespace exhaustion | 002/009/018 |
| O2-10 | P1 | Sponsor termination vs external shared grants, charge adoption | 002/006 |
| O2-11 | P1 | Type-right tables incl DEBUG and BUDGET_DELEGATE | 002/015 |
| O2-12 | P1 | R1/R2 revoke async status, deadline and cancellation | 002/003 |
| O2-13 | P1 | Distinct global emergency TaskKill/DeviceAdmin authority | 002/006/007 |
| O2-14 | P1 | User-approved lease/sandbox grants and expiry | 016 |
| O2-15 | P1 | IPC receive credit reservation and atomic sponsor balancing | 002/003 |
| O2-16 | P2 | Public stable ABI values and multi-ISA representation | 009/018 |
| O2-17 | P0 | Lost post-commit syscall result: transactional status/ticket or provable unambiguous return semantics? | 002/003/009 |
| O2-18 | P0 | Issuer last-handle close: anchor/control lifetime, scoped revoker and sponsor charging? | 002/003/006 |
| O2-19 | P0 | R1/R2-F/R2-O separation across independent roots, TLB/DMA and page faults? | 002/004/007 |
| O2-20 | P1 | Per-object kernel strong revoke vs user-service RPC lease, honest claims and mixed memory effects? | 002/016 |
| O2-21 | P1 | Shared/COW frames, queued handles and SMP reservations: conservative fair charge ledger? | 002/003/004 |

## 18. Proposed decision register

None of the following CAP-D entries are accepted architectural decisions.

- **CAP-D01**: H1 hybrid is a conditional research baseline; H0/H2 are active alternatives.
- **CAP-D02**: Keep holder, authority, sponsor and lifetime graphs distinct.
- **CAP-D03**: Attenuate existing-object powers; use typed Create+Budget for new objects.
- **CAP-D04**: Per-Task opaque, non-resurrecting handle semantics; 64-bit is a candidate, not a frozen ABI.
- **CAP-D05 [revised]**: IPC send has a single commit point; only pre-commit failure preserves source handles. Postcommit and UNKNOWN need separate status handling.
- **CAP-D06 [revised]**: Differentiate R0, B, R1 mapped access cutoff, R2-F family quiescence, R2-O object-wide safe reclamation.
- **CAP-D07**: Mapping provenance and physical DMA/IRQ fence must precede safe reclaim; quarantine on failure.
- **CAP-D08**: Audit all kernel memory/queued/handle/pinned charges independently of handle holder.
- **CAP-D09**: No ambient privilege on process start or Linux/Windows/Darwin guest identities.
- **CAP-D10**: Do not call the design secure, performant or R1-passed until cross-RFC tests and independent review.
- **CAP-D11 [new]**: Parent handle close must not discard ancestry of living descendants; revocation-controller scope requires explicit limits.
- **CAP-D12 [new]**: H1 acceptance depends on proving IPC result/commit, R2-F/O and resource accounting correctness or reducing its promised guarantees.

ADR-002 stays **Accepted — Direction**, without silently upgrading RFC-002 details. Any eventual detailed acceptance requires its own explicit review/ADR and later implementation Go/No-Go.

## 19. Official primary references

- **[S1] seL4 Capabilities tutorial:** https://docs.sel4.systems/Tutorials/capabilities.html
- **[S2] seL4 Untyped tutorial:** https://docs.sel4.systems/Tutorials/untyped.html
- **[S3] seL4 API — Copy/Mint/Move/Revoke/Retype:** https://docs.sel4.systems/projects/sel4/api-doc.html
- **[S4] Zircon kernel concepts:** https://fuchsia.dev/fuchsia-src/concepts/kernel/concepts
- **[S5] Zircon handles/lifetimes/reuse:** https://fuchsia.dev/fuchsia-src/concepts/kernel/handles
- **[S6] Zircon channel transfer/lifetime:** https://fuchsia.dev/fuchsia-src/reference/kernel_objects/channel
- **[S7] Zircon zx_channel_write documented failure behavior:** https://fuchsia.dev/reference/syscalls/channel_write
- **[S8] Fuchsia Jobs:** https://fuchsia.dev/fuchsia-src/concepts/process/jobs
- **[S9] CHERI Alliance capability limits:** https://cheri-alliance.github.io/build/technology/cheri-capabilities/capabilities/
- **[S10] seL4 MCS CPU budget contexts:** https://docs.sel4.systems/Tutorials/mcs.html
- **[S11] Linux VFIO IOMMU group boundary:** https://docs.kernel.org/driver-api/vfio.html
- **[S12] Zircon zx_channel_write_etc:** https://fuchsia.dev/reference/syscalls/channel_write_etc
- **[S13] Zircon zx_vmar_unmap:** https://fuchsia.dev/reference/syscalls/vmar_unmap

**Research limitation:** External documentation supports the comparison, not any claim of UniOS code, formal verification, measured performance or safe hardware behavior. Code adoption requires separate source/license review; no external kernel code was imported.

---

## 20. Critical review and corrections (2026-10-24)

This is a specification/counterexample review of cited official resources, not a completed formal proof, implementation, independent external audit or measured performance result.

| Defect in v0.1 | Correction in v0.2 | Unproved obligation |
|---|---|---|
| R2 family cleanup could be mistaken for object-wide free | R2-F and R2-O separate | multi-root lifetime safety |
| IPC failures assumed full rollback even after commit | NOT_COMMITTED/COMMITTED/UNKNOWN | syscall output-fault and lost-reply handling |
| Issuer closing its parent could destroy provenance | ancestry anchor and scoped controller | revoker/controller longevity and charges |
| Revocation call confused with CPU memory cutoff | B then R1 TLB/mapping fence | SMP/page fault/remap races |
| Shared COW/IPC accounting under-specified | primary plus queued/holder/mapping charges | conserved quota/fairness |
| Rights-bit attenuation ignored physical/Task scope | rights + scope monotonicity | typed scope semantics |
| Zircon Job policies treated as complete quota precedent | source distinctions | UniOS accounting model |
| Recursive channel cleanup hazard | bounded iterative drain or feature gate | queue cycles and roots |

**Still proposed:** H1 is only a comparative research baseline, not the proven optimal design. H0 and H2 remain alternatives; universal kernel lineage revocation may be too costly for a desktop. Pending P0s must be addressed before RFC acceptance.

**History:** v0.1 was registered 2026-10-24 and is retained in Git history. v0.2 updates only research documentation; D1–D8 and ADR-002 remain Accepted at direction level. No implementation or stable ABI is authorized.

## 21. 2026-10-25 Native Task Handle namespace and Receipt cross-RFC review — Proposed

**Non-normative handoff:** [CAPIPC Task Handle Namespace & Kernel Receive Receipts v0.1](../models/capipc-task-handle-namespace-and-receipt-v0.1.md). This review refines RFC-002 §7 public Token/ABA and §§9–11 MOVE/sponsor but **does not accept a Handle encoding, widen DERIVE rights, implement any OS code or prove existing invariants**.

- **External public ID vs internal slot:** a kernel-internal HandleSlot may be recycled, but an old Task-public value must not later authorize an unrelated grant within the same live TaskGeneration under current proposed CAP-10. Candidate HN-B encodes externally checked nonwrapping generation with fail-closed per-slot retirement; HN-A uses monotonic Task-public IDs. **No public numeric width/bit layout is approved.**
- **Crucial external comparison:** official Fuchsia Zircon Handle documentation explicitly says it **may reuse numeric values of closed Handles**. UniOS's Proposed no-stale-rebind is *stricter* and demands explicit finite turnover/availability proof; it must not be falsely attributed to Fuchsia. Same-Task Handle integer guessing does not manufacture a security boundary between Threads or plugins inside one Task.
- **Reservation visibility:** internally provisional IDs that provably were never live or exposed may be reclaimable after proof; possibly user-published IDs (including partial copyout attempts) cannot later bind unrelated CapGrants in the same Task. Random identifiers without lifetime issued-history do not prove deterministic nonreuse.
- **Receipt rights are distinct from source cap rights:** after SEND K-S original source CapHandle is invalid; a Task must be able to query status using **separately scoped, preissued** operation-result authority if reconciliation is promised. Caller nonce is correlation, not an issuer cap, and another Task/ServiceID cannot inspect the record from numeric bytes alone.
- **Resource sponsor:** reserved handle IDs, receipt/tombstone metadata, queue/claim and DMA pins each have independent accounting; receipt expiry/Task death is not proof that committed MOVE or device effect was rolled back.

**P0 still open:** HN-A vs HN-B and finite overflow/Task-lifetime quota, exact public/partial-copyout exposure proof, preidentified receipt bootstrap and per-Task generation, independent roots and B/R1/R2-F/R2-O. This is design documentation only; RFC-002 stays **Proposed v0.2**.

## 22. 2026-10-25 Finite public Handle budget and receipt GC follow-up — Proposed

This is a **design-only** cross-reference to [CAPIPC Finite Handle Namespace, Receipt Ledger & GC v0.1](../models/capipc-finite-handle-namespace-receipt-ledger-gc-v0.1.md); it does not amend any Accepted direction, accept Handle widths or authorize OS code.

- **Finite no-rebind:** HN-A monotonic issue budget and HN-B per-slot nonwrapping generation/slot retirement are coequal feasibility candidates. A hot slot can exhaust long before aggregate bit capacity; internal HandleSlot memory may still be recycled safely with current TaskGeneration+grant comparison.
- **Explicit cost:** hypothetical 32-bit HN-A at 1,000 issued IDs/s exhausts in ~49.7 days, while a 16-bit public generation field on one recycled HN-B slot at 1,000 generations/s exhausts in ~65.5 seconds. These are arithmetic stress examples, **not** selected KABI bit widths or measured workload.
- **No public token/receipt conflation:** a kernel Receipt high-water mark only fences duplicate operation admission; it does not replace native Handle anti-ABA or ensure persistent outcome evidence after final-record GC.

**P0 OPEN:** finite token budget, concentrated slot churn, retirement fairness, sponsor isolation, public token partial exposure and TaskGeneration closure. RFC-002 remains Proposed v0.2.

## 23. 2026-10-25 KR-M1 versus KR-M2 receipt admission boundary — Proposed research

**New reference:** [CAPIPC Ordered vs Sliding-Window Receipt Admission v0.1](../models/capipc-ordered-vs-sliding-window-receipt-admission-v0.1.md). **Design only; no rights-table, accepted threat model or Handle encoding changes.**

A proposed TaskGeneration-scoped receipt sequence q is **correlation, not a CapGrant**. KR-M2's protected frontier C and used bitmap B prevent a previously consumed (admitted or explicitly SKIPped) q from authorizing a new operation even after final evidence GC; unlike native Handle no-rebind, this only fences operation identity. Result-query/abandonment rights must be independently scoped, survive authorized source MOVE without rights amplification, and not expose another Task's cap metadata. SKIP can discard an unadmitted q, but **cannot undo committed MOVE or revoke existing rights**. Multi-Thread same-Task adversarial SKIP remains an availability risk unless an actual independent compartment/security principal is introduced.

**P0 OPEN:** scoped SKIP/right provenance, Task/receipt namespace privacy, quota/sponsor, exact CapGrant fingerprint and revocation ordering. RFC-002 remains Proposed v0.2.

## 24. 2026-10-25 CAPIPC Cross-RFC Safety Case — Proposed review

**Research-only:** [CAPIPC Cross-RFC Safety Case v0.1](../reviews/capipc-cross-rfc-safety-case-v0.1.md) reviews KR-M2 A/X/C/B/R against current GrantLineage, Endpoint ownership, revocation B/R1/R2-F/R2-O and Task identity. A consumed q is not a present CapGrant nor proof of completed SEND/K-I; receipt GC/Task death do not refund queued rights or physical DMA pins. Commit-time authorization after earlier admission, scoped SKIP/query grants, public Handle no-rebind and lineage/sponsor cleanup remain P0 and unproved. No rights-table, Accepted ADR, Handle format or implementation change.

## 25. 2026-10-25 CAPIPC formal-model preparation — Proposed cross-reference

**Design study only:** [CAPIPC Formal Model Preparation v0.1](../models/capipc-formal-model-preparation-v0.1.md) explicitly models TaskGeneration, native CapGrant and public Handle nonrebind, Endpoint E1 owner MOVE, lineaged revocation B, protected receipt query/SkipRight and independently charged queue/claim/physical debts. K-S authorization after earlier A(q) and concurrent revoke/owner MOVE remains an unresolved alternative: no blanket post-B right is granted by A. Raw q is not CapGrant, and historical CommittedKI must not resurrect one. All invariants stay unproved, RFC-002 status Proposed; no rights-table, ABI or kernel code changes.

## 26. 2026-10-25 Commit/Revocation/Endpoint owner order — Proposed research

**Design-only joint paper:** [CAPIPC Commit, Revocation Barrier & E1 Endpoint Ownership Ordering v0.1](../models/capipc-commit-revocation-ordering-v0.1.md). A(q) is a protected operation-identity admission, not a permanent native permission to MOVE after relevant scoped B or Endpoint E-M. Proposed CO-S orders *actual* K-S against B/E-M; if K-S wins, whole SEND+source MOVE remains committed, with later revocation governing future use and R1/R2-F/R2-O remaining separate. If B/E-M wins, **that SEND** must not move source grants and reaches FINAL_NOT_COMMITTED only after no future K-S is possible. **Clarification of §§9/11:** “source Handles retained” means no invalidation *due to the failed SEND*. Another Thread's successful CLOSE/MOVE or an independent controller's B may have invalidated or revoked them; `NOT_COMMITTED` must never falsely promise every Handle is still live or invokable. E1 Endpoint payload may be ownerless-to-Tasks InTransit before K-I, with no second public owner. These are unproved design constraints; no rights table, ABI or implementation change.

## 27. 2026-10-25 Receive Handle publication versus actual grant installation — Proposed

**Design-only source:** [CAPIPC Receive Exposure, K-I & Handle Publication Ordering v0.1](../models/capipc-receive-exposure-ki-handle-publication-v0.1.md). Receiving B may observe metadata, payload or inert public token bits before native CapGrants actually move into its Task at one all-or-none K-I. Maybe-published token values cannot rebind unrelated native authority in that live Task; query of historic COMMITTED_KI never remints grants. After K-I actual T1 bearer grants belong to B's **Task**, not only the calling Thread; method-level authorization would need a genuinely independent T2 verifier/escrow, not Task self-acceptance. B/R1/R2-F/R2-O, lineage and sponsor survive receipt/Claim closure as independently necessary. No rights change, Handle layout, ABI, model execution or proof.

## 28. 2026-10-25 Unified SEND–RECEIVE Grant custody and revoke — Proposed contract link

**Non-normative integration:** [CAPIPC Unified SEND–RECEIVE Semantic Contract v0.1](../contracts/capipc-unified-send-receive-semantic-contract-v0.1.md) treats an exact CapGrant vector as owned by sender before K-S, one kernel queue/Claim custodian after K-S, and recipient only after full K-I. Claim is not duplicate grant ownership; a transferred Endpoint may be Task-ownerless InTransit. If scoped revoke B wins after K-S but before K-I on any mandatory member, no new revoked usable grant or silent N−1 partial install may occur: settle full K-I as denied/terminal under a first proof branch, with all Grant/lineage charges accounted and sender historical COMMITTED_KS preserved. Revocation B/R1/R2-F/R2-O and independent object roots remain separate. No rights-table, accepted ADR, ABI or code selection.

## 29. 2026-10-25 CAPIPC P0 conflict / draft ADR readiness — Proposed

The independent [CAPIPC P0 Conflict Resolution & ADR Readiness Matrix v0.1](../reviews/capipc-p0-conflict-resolution-adr-readiness-v0.1.md) identifies conditional ADR-CAPIPC-A (whole source Grant MOVE vs B/Endpoint E-M and source Close), C (queued mandatory-grant B before K-I cannot secretly install N−1) and E (sponsor/lineage/DMA resource lifetime) as **open design packages, not approved ADRs**. A(q) is only receipt identity admission; no source grant can be restored after committed K-S, and no new unauthorized K-I may mint a revoked grant. Public Handle external nonrebind, queue/Claim custodianship, independent root revocation and actual R1/R2-F/O fencing remain P0. No accepted rights/Handle allocation/ABI/implementation update.

## 30. 2026-10-26 CAPIPC joint A/C/E semantic-cut study — Proposed

[CAPIPC Joint A/C/E Semantic-Cut Resolution v0.1](../models/capipc-joint-semantic-cut-resolution-a-c-e-v0.1.md) links K-S whole-grant MOVE versus B/Close/E1 owner E-M to receiver audience Claim, Vp/Vt public TokenIDs and whole K-I. Under the **paper-only** CO-S/CO-E-C candidate, A(q) is not future permission, source GrantIDs cannot be doubly moved, an Endpoint transferred as payload can be Task-ownerless InTransit, and B on a mandatory queued grant before K-I denies full new installation rather than yielding silent N−1 rights. The **new open** issue is in-flight confidential usercopy while issuer PolicyEpoch/Endpoint owner changes; B/effective owner handoff cannot imply already-written bytes are erased, nor is subsequent stale publication allowed without an explicitly scoped prebarrier fence. B/R1/R2-F/R2-O and current capability rights remain distinct. No rights/ABI/code/formal proof accepted.

## 31. 2026-10-26 Publisher permit versus native lineage revoke — Proposed

[CAPIPC Publication Barrier & Pager-Safe Copyout Feasibility v0.1](../models/capipc-publication-barrier-pager-safe-copyout-feasibility-v0.1.md) distinguishes native GrantLineage B/R1/R2-F/R2-O from **issuer/Endpoint-scoped user-data publication authority** and its P-DENY/P-DRAIN cut. A Task might have seen or stored confidential bytes before a later B, and K-I receipt truth cannot erase that. If an issuer/E1 owner transition also promises old publisher quiescence, it requires exact active-permit drain/fence evidence, not only B or eventual Task death. Pinning output frames neither grants access to foreign aliases nor proves no future stale copyout; whole Grant MOVE and public TokenID nonrebind remain mandatory. All mechanisms are paper candidates with P0 blockers; no modified rights encoding or ABI.

# UniOS RFC-002 — Object, Capability, Lifetime, Delegation & Revocation

> **Status:** Proposed — Specification v0.1 (research draft, not accepted or validated)
> **Date:** 2026-10-24 (NZDT)
> **Baseline:** [Architecture v0.2](../architecture/unios-architecture-v0.2.md) and [RFC-001 v0.2](rfc-001-kernel-boundary-and-tcb.md).
> **Scope:** Kernel objects, handles, rights, creation authority, quotas, derivation, transfer, revocation, destruction, safety and verification.
> **Restriction:** Architecture research only; no implementation authorization, binary compatibility claim, stable ABI, security certification or hardware support assertion.

## 핵심 결정 요약 (한국어)

**RFC 상태는 Proposed v0.1이다.** 기존 D1~D8과 RFC-001의 커널 경계를 상세화하는 *설계 가설*이며, 커널이나 서비스의 구현·검증·ABI 동결을 뜻하지 않는다. 세부 계약이 명확해졌다는 이유만으로 아키텍처의 안전성이 입증되지는 않는다.

**1차 연구 권고: H1 — 실용적 혼합형 Capability 객체 모델.** 독립 커널에는 Task별 opaque handle table, type-specific 권한, 객체 생성 허가, 리소스 쿼터, 권한 파생 기록을 두고, VFS·권한 UI·네트워크·호환성 정책은 사용자 공간에 둔다. 다만 H1은 *유일한 최선*으로 확정하지 않는다. 자원 재사용과 회수의 일관성을 명시적으로 보장하는 seL4형(H0), 단순하고 성숙한 핸들/Job 모델의 Zircon형(H2), 서비스 RPC 프록시(H3)를 함께 비교한다.

**객체의 5가지 지위를 혼동하지 않는다:** (1) 생성한 Task, (2) 현재 핸들 보유자, (3) 실제 권한을 부여한 Grant, (4) 커널 메모리·IPC·페이지·DMA 비용을 부담하는 Sponsor, (5) 객체를 살아 있게 하는 모든 참조. 예를 들어 A가 MemoryObject를 만들고 B에 읽기 권한을 주더라도, B가 핸들을 받았다는 이유만으로 A의 리소스 비용이 사라지거나 B에게 권한 재위임권이 생겨서는 안 된다.

**권한을 끊는 것과 안전하게 자원을 회수하는 것은 다르다.** R0는 로컬 핸들 닫기, R1은 파생 권한에 대한 *향후 접근 차단과 필요한 CPU 매핑 무효화 완료*, R2는 이미 승인된 작업·TLB·DMA/IRQ까지 안전하게 종료된 *자원 재사용 가능 상태*이다. 이미 읽은 데이터나 완료한 파일 저장은 회수할 수 없다. 장치 DMA를 안전하게 멈췄다고 확인할 수 없다면 해당 자원은 Quarantined로 두어야 한다.

**권고 변경 가능 조건:** R1 강제 경로가 IPC 성능·커널 TCB를 과도하게 증가시키거나, 권한 파생 그래프의 대량 정리를 안전하게 수행할 수 없거나, 자원 쿼터 회계가 유실되는 반례가 발견되면 H1을 고수하지 않는다. H0/H2로 전환하거나, 일부 서비스에서는 커널 레벨 회수를 포기하고 서비스 수준 접근 중지만 제공할 수 있다. 그 경우 보안 목표·지원 범위 차이를 공개해야 한다.

**검증 순서:** CAP-01~18의 모델·반례 설계 → RFC-003 IPC 원자성 + RFC-004 매핑/페이지 + RFC-006 부팅/종료 공동 검토 → RFC-007 DMA 그룹 신뢰 조건 → RFC-009/010 Linux 제한 실행 → RFC-015 보안 게이트. 미해결 P0 8건이 남아 있으며 R1 설계 종료를 선언하지 않는다.

---

## 0. Executive recommendation

**Proposed comparative baseline H1:** task-local opaque handles to typed kernel objects, kernel-enforced fine-grained rights, explicit creation authority and hierarchical resource budgets, provenance-aware delegated grants, and a distinct revocation/physical-reclamation lifecycle.

This combines useful aspects of seL4's explicit authority and derivation discipline with Zircon's practical per-process handles and resource groups; **neither combination nor its performance is validated**. It must be measured and model-checked against:
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

**Provenance lineage:** parent–child derivation relationship for grants. Two independent authority roots may legitimately reference the same object. Revoking one does not inherently cancel the other.

Maintain three separately auditable graphs:
1. **Authority/derivation graph** — whose grant is derived from which other authorized grant?
2. **Sponsor/charge graph** — which ResourceBudget pays for each live object, handle slot, metadata allocation, pinned page and queued transfer?
3. **Lifetime/dependency graph** — which references, mappings, operations and hardware state prevent safe free/reuse?

No security property may be based on a single mutable owner pointer when these graphs disagree.

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

For an **existing object's grant**, effective_rights(child) is a subset of effective_rights(parent) intersected with allowed_rights(type). A holder can remove DUPLICATE/TRANSFER/DERIVE to prevent onward spreading of that grant; it cannot restore a dropped power. A new object is different: explicit Create authority + budget legitimately permits the kernel to issue an initial cap to **a new object**, rather than copying rights from an unrelated cap. This does not mint arbitrary capability access to existing objects.

**Proposed DERIVE vs TRANSFER:** DUPLICATE/DERIVE authorizes the creation of a distinct same-object grant under an attenuated rights set. TRANSFER authorizes moving possession without gaining new powers. Moving with attenuation may drop rights, but it must not erase ancestry. Combining these bits as one generic privilege needs explicit review.

**Not sufficient for native privilege:** guest UID 0, Windows Administrator, Darwin entitlement, policy prompt, file name, registry lookup, random ID, copied handle integer, filesystem ACL. Such data may guide a user-space broker decision but cannot bypass the kernel's existing grant chain.

## 6. Allocation authority and hierarchical sponsor accounting

### 6.1 Resource creation is not simply budget consumption

An object create requires **both** (a) a typed Create/Control capability for the relevant namespace/Task/device resource and (b) a ResourceBudget with available reserve. Money does not confer access: a memory quota alone does not grant MMIO or a foreign Task handle.

H0 approach: explicit untyped memory authority and retype to typed objects [S2]. H1 approach: kernel-managed typed creation with narrow creator cap, explicit hierarchical charge and a contained allocator. H1 should be rejected if bounded allocation/OOM behavior cannot be proven. H2 simpler Job policy remains a serious alternative.

### 6.2 Accountable resource categories

Proposal: charge for object instances, handle slots, derivation/family metadata, IPC queued bytes/messages/transit handles, page-table/mapping metadata, resident/pinned pages, wait registration and critical reserved operations. CPU budget and DMA throughput policies go to RFC-005/007. Subbudgets must not exceed ancestors' aggregate reservations and commitments.

- Exactly one **primary sponsor** is accountable for each object and each kernel-internal metadata charge, with parent aggregate charging or an equivalent provably conservative accounting scheme.
- A handle's holder can change without silently moving its sponsoring debt. An explicit, authorized and atomic **sponsorship adoption** is required for charge transfer.
- When moving a handle to another Task, recipient handle-slot budget must be available; queued IPC is charged while queued.
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

## 9. Operation matrix — preconditions and linearization

| Candidate operation | Required authority | Atomic / rollback contract |
|---|---|---|
| create(type, budget, scope) | typed Create + BudgetAllocate | initial object, grant and charges appear together; failed request publishes none |
| duplicate/derive(h, rights_subset) | DERIVE or DUPLICATE | new child grant under parent lineage; deny if revoked, insufficient rights or quota |
| attenuate_in_place(h, rights_subset) | current owner and allowed rights reduction | replaces rights atomically; never exposes temporary expanded rights |
| cap_move / ipc_send | source TRANSFER + endpoint SEND | all attached grants & bytes become in-transit, and sender loses rights, **or nothing changes** |
| ipc_receive | endpoint RECEIVE + receiver slots/charge | all grants installed and message removed atomically, or message remains queued |
| close(h) | local handle holder | removes this table entry only; no guarantee all external descendants disappear |
| revoke_descendants(root) | specifically scoped REVOKE_DESCENDANTS | deny-new-use barrier linearizes before incremental cleanup |
| revoke_wait(ticket) | scoped wait/observe on revoke | success only when claimed revoke phase actually completed |
| destroy_object | specific object-admin + absence of unsafe refs | no premature reuse; may return quarantined/pending |
| budget_adopt(sponsor, new_sponsor) | both appropriate sponsorship authorities | reserve new charge before releasing old charge, or fail atomically |

**UniOS choice for IPC send failure:** preserve source handles if the complete transactional send fails; reject duplicate source entries atomically. This deliberately **differs** from Zircon basic zx_channel_write, whose documented API consumes supplied handles even when an error occurs [S7]. Never describe the UniOS policy as existing Zircon semantics.

The exact syscall and service protocol vocabulary is **not** frozen; RFC-003 will specify IPC, backpressure, cancellation, reply-token and transaction semantics.

## 10. Revocation levels and concurrency contracts

### 10.1 Precise guarantee classes

- **R0 (Close):** invalidate one handle. Does not automatically invalidate derived copies, memory mappings, outstanding operations or committed I/O.
- **R1 (Revoke authority family):** invalidate descendants of a specified grant lineage and block **future** use, derivation, transfer and related kernel-enforced memory access. Already completed or independent access cannot be undone. R1 completion is not just 'scheduled for cleanup'.
- **R2 (Strong quiescent reclaim):** only after relevant admitted invocations, TLB invalidations, waiter/IPC dependencies, DMA/IRQ and hardware state reach safe boundaries may the resource be reused. R2 is conditional on a reliable device fence/reset. An unfenced device remains Quarantined/Offline; a timeout is **not** successful reclamation.

A user's previously copied plaintext, already sent network packet or durable file write is beyond a generic kernel revocation guarantee.

### 10.2 Bounded, two-phase revoke proposal

1. Check REVOKE_DESCENDANTS authority and exact scope; identify the grant-family root. Publish a kernel-enforced, irreversible **deny-new-use barrier** (revocation epoch/root marker) at a defined linearization point.
2. Any concurrent lookup/derive/IPC transfer must either have legally committed **before** that barrier and be tracked for quiescence, or fail **after** the barrier. In-transit grants cannot escape by moving to another Task.
3. In bounded, preemptible batches retire descendants, cancel dependent waits/tokens, remove descendant grants from queues and task tables, and walk provenance for dependent CPU mappings and IOMMU bindings.
4. Make memory mappings inaccessible with correct MMU permission updates and inter-core TLB shootdown **before declaring R1 mapping completion**. An ancestor marker alone does not revoke already executing loads from an unchanged page table.
5. Resolve admitted-invocation completion/cancel. For a physical device, quiesce/reset and obtain DMA/IOTLB/interrupt-fence evidence before R2 completion. Unreliable device cleanup leaves the resource quarantined.
6. Retire metadata only after epoch readers/in-transit entries and unsafe dependencies have drained; never reuse a family token while old nodes remain.

A wide derivation graph cannot be traversed in one unbounded non-preemptible kernel critical section. **Open P0:** exact fast-path revocation check and ancestor traversal costs, incremental reclamation fairness, and how denial remains correct when a grant's CPU mapping exists. Full lineage revoke in H1 is a design obligation, **not** a validated feature.

### 10.3 Independent grants and privileged kill

Revoke(root A) invalidates descendants of A; it need not invalidate root B's independently issued grant to the same MemoryObject. A separate TaskKill/DeviceAdmin/ObjectWideRevoke authority may authorize wider shutdown. Arbitrary Linux UID0/Windows Administrator/Darwin entitlement has no such authority on its own.

### 10.4 Service proxies versus kernel enforced memory/device effects

A File Portal may issue a service endpoint whose **future RPC calls** can be rejected on revoke. This does not retract bytes the app already read or magically unmap pages previously granted through an independent MemoryObject cap. Conversely, a driver DMA grant requires true IOMMU/IRQ lifecycle enforcement [S11], not only service-level denial.

## 11. Transfer, transit, IPC cycles, death/restart

- **MOVE vs DERIVE:** MOVE transfers original grant lineage and loses source handle after successful publish; DERIVE creates a new grant node with attenuated rights. Neither may elevate authority.
- **Message transit:** kernel owns cap records while queued; it retains lineage, bounds and sponsor accounting. A failed delivery must not create half-installed receiver handles. Receiver slot exhaustion leaves the complete message queued or rejects with an explicit no-side-effect policy.
- **Endpoint close:** drop unread queued grants under an explicit accounting/cancellation contract. Outstanding operations already committed are not undone.
- **Service crash:** a restarted service gets a **new incarnation**. Old reply/fault tokens and capability paths cannot be implicitly retargeted to the new service. Reconnection requires a fresh approved grant.
- **Queue reference cycles:** channels that carry their own or each other's endpoints can form unreachable reference graphs. Refcounting alone may leak. **P0:** determine a bounded, safe reclamation scheme (e.g. explicitly revocable domain teardown with root liveness checks), or prohibit cyclic endpoint delegation in the earliest profile. The model must not assume Task death automatically breaks all cycles.
- **Grant sponsor after move:** new holder may not be resource sponsor. Sponsor budget remains charged even if its original Task is dead; quota reclamation requires authorized adoption or true resource teardown.
- **Exactly-once caution:** cancellation/retry of a service request does not prove a file write or external side effect happened exactly once. Tie server idempotence to RFC-003 and storage RFC-008.

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
| invalid batched IPC handles | partial transfer | all-or-none; sender unchanged on failure |
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
| CAP-05 | Failed operations publish no partial objects/handles and restore reserved charges. |
| CAP-06 | After a grant-root deny barrier, no descendant can begin a newly authorized operation. |
| CAP-07 | R1 completion cannot occur while a forbidden descendant still has a grant-derived accessible CPU mapping. |
| CAP-08 | R2 completion requires defined in-flight, MMU/IOMMU, DMA/IRQ quiescence or returns quarantined/pending. |
| CAP-09 | Local close, Task exit, object destruction and revoke are separate operations with correct lifetime references. |
| CAP-10 | A stale handle token never reauthorizes unrelated capability within the same Task lifetime; exhaustion fails closed. |
| CAP-11 | Resource charges remain conserved across allocation, IPC transit, lineage creation, sponsor death and adoption. |
| CAP-12 | Untrusted allocations and IPC floods cannot exhaust critical boot/Pager recovery reserve. |
| CAP-13 | Guest UID0/Administrator/entitlement never becomes native object authority without explicit grant. |
| CAP-14 | ObjectID, process ID, registry name and debug metadata are not capability credentials. |
| CAP-15 | Service incarnation change invalidates stale continuation/reply/authorization references. |
| CAP-16 | Revocation never falsely promises erasure of copied secrets or rollback of committed I/O. |
| CAP-17 | Unisolatable IOMMU groups and unfenced DMA cannot be passed off as safe untrusted driver objects. |
| CAP-18 | Cyclic IPC/resource dependency cannot consume unbounded resources outside sponsor quota, with explicit cleanup semantics. |

These are **design proof obligations**, not claims of formal verification. CAP-06 through CAP-08 intentionally exceed basic Zircon close semantics and may disqualify H1 if safe implementations are too complex.

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
2. Prove or find counterexamples for CAP-01/03/04/05/06/07/08/10/11: no amplification, no partial transfer, no resurrection, bounded accounting, deny barrier and safe reuse.
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

## 18. Proposed decision register

None of the following CAP-D entries are accepted architectural decisions.

- **CAP-D01**: H1 hybrid is a conditional research baseline; H0/H2 are active alternatives.
- **CAP-D02**: Keep holder, authority, sponsor and lifetime graphs distinct.
- **CAP-D03**: Attenuate existing-object powers; use typed Create+Budget for new objects.
- **CAP-D04**: Per-Task opaque, non-resurrecting handle semantics; 64-bit is a candidate, not a frozen ABI.
- **CAP-D05**: Transactional move/IPC and persistent revocation ancestry; sender retains handles on failed send.
- **CAP-D06**: Clearly distinguish R0 local close, R1 revoked-grant access denial and R2 strong quiescence.
- **CAP-D07**: Mapping provenance and physical DMA/IRQ fence must precede safe reclaim; quarantine on failure.
- **CAP-D08**: Audit all kernel memory/queued/handle/pinned charges independently of handle holder.
- **CAP-D09**: No ambient privilege on process start or Linux/Windows/Darwin guest identities.
- **CAP-D10**: Do not call the design secure, performant or R1-passed until cross-RFC tests and independent review.

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

**Research limitation:** External documentation supports the comparison, not any claim of UniOS code, formal verification, measured performance or safe hardware behavior. Code adoption requires separate source/license review; no external kernel code was imported.

---

**History:** 2026-10-24 — RFC-002 v0.1 research draft prepared from Architecture v0.2 and RFC-001 v0.2. No implementation, build or testing; pending independent RFC acceptance.

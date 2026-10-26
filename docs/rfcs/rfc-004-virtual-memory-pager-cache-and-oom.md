# UniOS RFC-004 — Virtual Memory, Pager, Cache & OOM Contract

> **Status:** Proposed — Specification v0.2 (심층 재검토·미승인·미검증)
> **Date:** 2026-10-24 NZDT
> **Baselines:** [Architecture v0.2](../architecture/unios-architecture-v0.2.md), [RFC-001 v0.2](rfc-001-kernel-boundary-and-tcb.md), [RFC-002 v0.2](rfc-002-object-capability-lifetime-delegation-revocation.md), [RFC-003 v0.2](rfc-003-ipc-wire-protocol-and-synchronization.md)
> **Scope:** AddressSpace, MemoryObject, frames, mapping, page fault/user Pager, sharing/COW, copyin/out, dirty/clean caches, OOM, DMA pins, revocation, boot dependency.
> **Restriction:** Research/design only. This does not authorize kernel, SDK or service implementation, ABI freeze, verification, hardware support or performance claims.

## 0. 요약 — 조건부 연구 결론

**VM-A를 우선 연구:** 커널은 메모리 보호의 최소 집행(MMU·page table·TLB·physical frame의 소유권·zeroing·MappingRecord·kernel-internal FaultWaiter·pin·비상 자원)을 담당한다. 사용자 공간 Pager/VFS는 파일 backing 식별, 콘텐츠 공급, 파일 캐시, 재요청, 실행 파일 검증, writeback 및 일반 OOM 희생 프로세스 정책을 담당하는 방향을 검토한다. 익명 메모리 demand-zero와 COW의 위치, generic clean eviction은 아직 결론이 없다.

- **VM-A: Minimal kernel enforcement + anonymous mechanism + user file Pager.** 현재 우선 연구 후보. 실행 가능성이 곧 최선의 증거는 아니다.
- **VM-B: seL4-like user-space VM manager.** user-space가 frame/resource와 fault를 광범위하게 통제하고 kernel은 hardware mapping primitive를 제공한다. 낮은 정책 침투 가능성 vs 복잡한 자원 관리·부팅.
- **VM-C: Zircon-like kernel VMO and reclaim.** kernel anonymous demand, VMO, generic reclaim/page aging 역할 증가. 데스크톱 가용성 및 구현 용이성 가능성 vs TCB 확대.

**단계적 설계 연구 순서:** M0 = Pager 없는 최소 wired boot/recovery + 명시적 commit/zeroed anonymous 및 scoped shared. M1 = read-only immutable file Pager/fault failure. M2 = demand-zero/COW/clean eviction. M3 = writable file mapping/dirty writeback. 각 단계는 미승인 연구 게이트이고 구현 일정이 아니다. Swap/compression, transparent POSIX fork, hugepage, broad dedup, unrestricted JIT/RWX 등은 별도 연구 전 범위 밖이다.

**섞지 말아야 할 여섯 가지:** virtual address 예약, RAM page commitment, CPU mapping, frame 권한, DMA pin, durable storage write. Handle close는 unmap이 아니며 family-level R2-F가 object-wide R2-O를 의미하지 않는다.

RFC-001/002/003와의 설계 충돌을 검토하되 그 문서의 Proposed 사항을 암묵적으로 Accepted로 변경하지 않는다. RFC-003+004+006 공동 검토는 필수다.

## 1. Official primary-source comparison

| External source | Documented design fact | UniOS implication and caution |
|---|---|---|
| seL4 [S1] | user-controlled VSpace/frame page-table cap mapping/unmapping | VM-B comparator; no automatic general desktop COW or file pager |
| Fuchsia address spaces / VMOs [S2,S3] | VMAR/VMO separation, physical allocator, demand-zero and shared memory | separates backing from virtual mappings; copying concepts is not validation |
| Fuchsia Pager [S4] | external Pager supplies missing pages to pager-backed VMO | user Pager contract is plausible, security and deadlock still UniOS work |
| Fuchsia pager detach [S5] | detach stops new requests; existing requests/dirty pages have distinct lifetime | detach cannot equal discard every page or wait forever |
| Fuchsia memory reclamation [S6] | kernel evicts verified/reproducible pager-backed clean pages | completely external eviction is not automatically simpler/better |
| Fuchsia pager-writeback RFC [S7] | accepted separate dirty/writeback design | writable mmap requires a data durability contract |
| Linux userfaultfd [S8] | missing/WP/minor fault modes and specific concurrency issues | access fault is not always a mere missing page |
| Linux pin_user_pages [S9] | ordinary frame reference and DMA long-term pin differ | ordinary handle reference cannot guarantee device safety |
| Linux VM/OOM policy [S10] | recovery/kill/panic behaviors depend on configuration and resource limits | no universal kill-one-process-and-recover promise |

**Version caveat:** older Fuchsia Pager prose can state an old writeback limitation while later Fuchsia RFC and current detach docs discuss dirty writeback. Do not treat outdated individual docs as universal current platform truth. Official external evidence supports design comparisons only, not proof of UniOS behavior, performance or license permissions.

## 2. Responsibility and trusted-computing base

**Kernel proposed MUST:** own final page tables, AS isolation, per-frame references and scrub, map/protect/unmap authorization, TLB coherence, reverse mapping provenance for revoke, minimum anonymous safety, memory resource accounting, scoped fault delivery, non-escalating PagerSupply, physical DMA pin/IOMMU mediation, and protected recovery reserves.

**User-space proposed SHOULD:** path/VFS/file backing, verified file image source, cache hints/policy, demand-page content supply, normal memory-pressure response and victim choices, writeback/fsync/storage journal, service failure recovery policy, guest POSIX/Windows/Darwin memory API emulation.

**Still open:** physical memory allocation policy, COW implementation split, page cache aging and generic eviction, dirty/write protection tracking, swap, memory overcommit and OOM last-resort victim policy. Code executing outside Ring 0 can still be asset-specific TCB-S for file integrity and recovery. Userspace Pager access to its own file content does not imply global TaskControl or mapping another Task's private frame.

Threats: malicious/buggy Pager/driver/user app, wrong or stale page contents, forged PagerRequest correlation or stale internal FaultWaiter, recycled VA frame mappings, SMP/TLB races, memory disclosure across Tasks, kernel usercopy fault recursion, queue-induced Pager death, OOM budget laundering, dirty-page data loss, DMA use-after-free and compatibility guest privilege escalation.

## 3. Object taxonomy and authority matrix

| Type / metadata | Scope and kernel role | Proposed rights |
|---|---|---|
| Task/AddressSpace | protection domain, page table root, VA range generation | MAP_CONTROL, UNMAP, PROTECT, QUERY for scoped Task |
| MemoryObject | backing identity/size, frames and anonymous/file/shared type | MAP_READ/WRITE/EXECUTE, PIN_IF_AUTHORIZED, QUERY, controlled DERIVE |
| MappingRecord | internal (AS generation, range, backing offset, rights, grant lineage) | not a global mutable user handle; source of revoke dependencies |
| Frame record | physical ownership, page-state refs, charge, zero/dirty/pin | allocated through typed resource authority, raw PFN not credential |
| PagerBinding | named pager instance backing a precise MemoryObject/range | SUPPLY_PAGE, FAIL_FAULT, DETACH, OBSERVE under that scope |
| FaultWaiter (kernel internal) | thread-specific blocked continuation + Task/AS/Mapping generations | Pager에게 Thread resume 권한을 노출하지 않음 |
| PagerRequest / correlation cookie (candidate) | one missing object page can represent multiple kernel FaultWaiters | non-authoritative notification; kernel checks generation |
| FaultToken (optional internal-only) | kernel-managed one-shot waiter retirement / continuation | no generic userspace task-resume grant |
| ResourceBudget | physical/frame/page-table/fault/queue/map charges | reserve, allocate, observe, authorized adopt |
| Device/IOMMUDomain | actual hardware group, DMA pins, IOTLB and reset fence | device/group-scoped DMA_MAP/PIN/IRQ |
| ExecuteGrant (candidate) | image identity and authorized executable mapping | exec maps only after policy/image validation |

**Authority:** effective mapping rights are the intersection of AS grant, MemoryObject grant, requested range/offset, lineage state (including revoke B), backing type, W^X/alias policy and hardware MMU permissions. File name, PID, guest UID0, ObjectID, arbitrary virtual/physical address never becomes a kernel capability.

**Four different graphs:** rights/provenance, sponsor/accounting, lifetime/mapping dependencies, DMA pin/IOMMU fencing. A single ownership pointer is insufficient. Two independent grants may map the same underlying MemoryObject. Closing the root handle does not free mappings; revoking one lineage must not cancel an independent root without separately scoped authority.

## 4. AddressSpace, map and CPU protection

1. Validate canonical user VA ranges, offset/length overflow, page alignment, page size, type/scope rights and present/revoked grant state.
2. Reserve enough mapping metadata/page tables/physical commitment if demanded by the operation, account against explicit sponsor budget.
3. Publish PTE and MappingRecord/provenance at one valid transaction boundary, or fail without unauthorized partial exposure.
4. PROTECT/UNMAP/revoke first establish denial of new mapped access and fault installs, then perform required CPU TLB invalidations.
5. **R1 complete** may only be returned after all relevant cores no longer access the revoked mapping (including PCID/cached translation state).
6. VA reuse creates new MappingRecord generation; old waiter/request generation or COW completion cannot silently target the newly mapped backing object.

**Candidate transition:**

~~~text
Unreserved -> Reserved(No Physical Frame Guaranteed)
  -> MappingPrepared -> MappedResident OR MappedNonResident
  -> Unmapping / Revoking / ChangingPermissions
  -> CPUAccessCutoff(TLB synchronized)
  -> Unmapped -> VA Reusable(with new generation)
~~~

**x86_64 assumptions to measure:** 4KiB initial research pages, 4-/5-level paging configuration, canonical address, NX, user/supervisor bits, PCID, KPTI, SMEP/SMAP, PAT/cache attributes, possible hardware hotplug, W^X aliases and CPU feature detection. No promise that all mitigations are available on QEMU or the Reference PC. ASLR effectiveness, code verification and executable permission need an explicit platform profile.

**W^X alias attack:** one VA mapped RX and another alias mapped RW to the same MemoryObject bypasses naive same-VA RWX checks. Per-object alias rights or explicitly scoped JIT policy required, with no ordinary JIT exception presumed.

## 5. Physical frames and secure reuse

~~~text
FirmwareReserved / DeviceReserved / KernelWired (not general free RAM)
Available -> Reserved -> OwnedZeroed -> MappedResident
MappedResident -> Shared / Dirty / Pinned
MappedResident -> Quiescing -> ScrubPending -> Reusable
Pinned with uncertain DMA -> Quarantined, NOT Reusable
~~~

Boot firmware RAM descriptors must be reconciled with UEFI reserved/runtime regions, MMIO/ACPI/device holes before any allocation; a plausible PFN is not authorization. Frames crossing security domains are scrubbed/zeroed, with no remaining stale CPU mapping or outstanding device access before reuse. The same object may have legitimate mappings in multiple ASes; per-map references and charge persist even after one handle closes.

Memory status should distinguish reserved VA from physical committed/resident bytes, dirty vs clean backing, pin vs normal ref, and quarantined frames. Kernel error paths cannot secretly borrow unlimited bytes from a protected boot/Pager recovery pool.

## 6. Anonymous, zero, COW and shared MemoryObject

| Memory class | Initial design status | Must guarantee |
|---|---|---|
| Boot/recovery wired memory | candidate required | no self-dependent page-in |
| Anonymous private | zero on first external read; optional demand-zero first-write | no old owner data |
| Shared MemoryObject | explicit scope, independent mapped refs | correct rights/sponsor and revoke |
| Copy-on-write | investigated for private copy and guest fork | original frame unmodified, charged private page on write |
| Read-only file Pager | first optional page-on-demand class | reproducible verified backing identity |
| Writable pager-backed mmap | postponed | dirty writeback/durability and concurrent write correctness |
| DMA/MMIO memory | separate privileged resource | pin/fence/cache correctness |

A permanent read-only zero page can reduce demand allocation only if first write always produces an authorized private zeroed frame or bounded failure. A COW fault needs reserve private frame, safely snapshot source, publish new PTE with generation check, correct shared-ref reduction, TLB/permission fences and rollback on OOM. Concurrent writers never double-charge or corrupt the common source. Clone/fork guest API behavior requires much more than one COW primitive (RFC-010).

Explicit shared mappings do not imply zero-copy superiority; copying can be necessary for isolation/data consistency. Shared buffer user-level indices are untrusted and subject to size/overflow/TOCTOU races. MAP revoke for one lineage must preserve an independent valid mapping.

## 7. Page fault and user-space Pager contract

**Fault classification:** missing anonymous, missing file backing, protection write/exec fault, revoked mapping, guard fault, invalid virtual address, exhausted charge/RAM, stale mapping or stale PagerBinding, backing data error and kernel-usercopy fault. Error enums/public ABI not frozen. Linux signals and Windows/Darwin exception semantics remain user personality responsibilities.

**v0.2 모델:** per-Thread FaultWaiter는 커널만 소유하고 Task/Thread/AS/Mapping/grant epoch와 책임 sponsor를 추적한다. PagerRequest는 특정 PagerBinding의 MemoryObject/range/backing version에 대한 관측 요청이며 **한 요청에 0~N명의 FaultWaiter를 합칠 수 있다**. Pager가 갖는 SUPPLY_PAGE/FAIL_RANGE는 지정된 object/range만 변경하며 **다른 Thread를 직접 Resume하는 권한이 아니다**. Correlation cookie는 필요시 stale request 방지와 결과 추적에 쓰지만 user-supplied 숫자는 kernel capability가 아니다.

**State model:**

~~~text
CPU Fault -> Classify Current AS Mapping and Rights
 invalid/protection/revoked -> scoped exception/fail
 anonymous -> reserve + zero/supply + map OR bounded OOM fail
 file missing -> FaultPending + authorized Pager request
    -> ScopedPageSupply | FailRange | PagerDetached | Dead | Timeout
    -> Recheck TaskGen, ASGen, MappingGen, BackingVersion, PagerGen, B
    -> KernelRevalidateEachWaiterAndMapOrRefault | CancelStale | KernelFailThread
    -> Release fault/wait/queue/metadata charges safely
~~~

PagerSupply grants contents only for the MemoryObject/range to which the service is bound, not arbitrary Task mapping authority or raw physical PFN control. **PageSupply success means backing pages available, not completion of a specific fault or resumed Thread.** Kernel independently revalidates and settles each FaultWaiter, including concurrent revoke, Task death, mapping replacement and duplicate faults. Already queued fault does **not** justify skipping new rights/epoch rechecks when page is installed. A single page may satisfy coalesced faults, but every waiter has its own Task/Mapping generation, lifecycle, cancellation and sponsor obligations.

**Pager detach/failure:** no new request into detached binding. Inflight faults terminate or transfer through a separately authorized rebind path; never hang forever simply because service disappeared. Already resident clean content can stay mapped where safe. **Dirty contents must not be advertised as durably preserved by mere detach.** In Fuchsia's writeback RFC [S7], detach allows pending dirty writeback, but Pager object destruction can permit dirty pages to be dropped. UniOS must separately decide what to do on Pager authority destruction, OOM or unrecoverable storage failure; M3 cannot claim zero-data-loss availability. Service restart changes PagerBinding incarnation; old supply/fail replies cannot target a new instance. Transparent process continuation is NOT guaranteed for M1 or later Pager-backed mappings.

Fuchsia uses this kind of pager-backed mechanism and describes detach/dirty page concerns [S4,S5]. This is precedent, not a UniOS ready-to-run subsystem.

## 8. Usercopy, IPC ReceiveEscrow and Pager recursion

A user pointer may be valid at initial inspection but unmap, revoke or point at new contents before copying. Kernel copyin of message descriptors/cap arrays requires bounded snapshot/revalidation at commit. Kernel cannot block in user Pager while holding global IPC/page-table/allocator locks or unbounded IRQ-disabled critical sections.

- **Nonfaulting fast path:** bounded, fault-safe usercopy under permitted locks, never uncontrolled recursive Pager call. Fail/defer appropriately.
- **Faultable slow path:** explicit Thread/Pager context, recursion budget and pre-reserved queue/fault metadata; no lock inversion or self-paging critical path.
- **ReceiveEscrow:** user buffer disclosure itself is irreversible even if cap transfer/dequeue later fails. Require exclusive authorized Claim or explicitly named disclosure commit before exposing confidential bytes (RFC-003).
- **Lost IPC syscall return:** committed sender MOVE/receiver install cannot be reported as NOT_COMMITTED after copyout failure. Protected receipt/reconcile or fail-closed result is still P0 (RFC-003/009).
- **Pager self-fault:** Pager's code/stack/IPC descriptors/handler cannot require same Pager to page itself in during recovery; if impossible, fail safely instead of cycling.

Two authorized receiver Tasks may both be permitted to read ordinary data, but a *single-message delivery contract* cannot falsely promise exclusivity after copying to A, aborting A's receive, then delivering the same message to B. Revocation also cannot make A unlearn copied bytes. The exact Claim → disclosure → cap-install boundary requires formal study.

## 9. File cache: identity, clean eviction and dirty writeback

**File-backed mapping is not file durability.** Backing identity must include immutable object/snapshot version, offset, length and enough content integrity information to reject stale or substituted bytes. Pager may control file contents and belongs to file asset TCB-S; verified executable image/anti-rollback remains RFC-017.

| Page | Reclaim? | Requirement |
|---|---|---|
| clean immutable reproducible | conditional yes | authorized unchanged backing version/hash |
| clean but mutable file | conditional | snapshot/truncation/version invalidation |
| dirty file-backed | **no**, until valid writeback/commit | retain dirty data or explicit failure |
| live anonymous private | **no**, without verified swap/reconstruction | never silently lose application memory |
| explicit discardable cache | only as contract says | app agrees data may disappear/rebuild |
| DMA-pinned/shared writable | not while unsafe references remain | R2-F/R2-O and actual device fence |

**Placement alternatives:** Cache-A user Pager picks clean eviction with kernel safety checks, Cache-B kernel tracks generic pager-backed page age/LRU and evicts reclaimable clean frames, Cache-C mostly user-managed frame allocator. Fuchsia kernel clean-page reclaim is genuine precedent [S6]; no blanket "all cache in userspace" claim.

**Writeback deferred:** dirty generation, concurrent writer isolation, writeback begin/snapshot, actual data+metadata durability, success/failure acknowledgement, fsync/flush semantics, crash replay, truncate/hole-punch and Pager death must be jointly designed with RFC-008. An OOM fallback cannot pin arbitrary dirty data forever. Failed/destroyed Pager may make unsaved data unavailable or lost; model application-visible data-loss state, read-only fallback, supervised shutdown, and durable recovery boundaries rather than an impossible unconditional preservation guarantee. Merely receiving a Pager ACK cannot prove stable storage or file-system crash consistency. Current Fuchsia pager-detach/writeback descriptions differentiate dirty retention [S5,S7]. No writeable mmap general compatibility promise now.

## 10. Memory pressure and OOM safe fallback

**Different errors:** VA exhaustion, sponsor quota, global RAM pressure, kernel metadata shortage, non-evictable dirty/pinned pressure, Pager unavailable, unsafe DMA quarantine, overcommit-at-fault. **Reserve VA is not commit RAM, and commit RAM is not a permanent resident/pinned guarantee.** A successful virtual reservation may not commit RAM; overcommit would allow later faults to fail. Error messages/SDK must distinguish.

**Candidate staged response:**

~~~text
NORMAL -> WARNING (advisory shrink and safe clean cache eviction)
 -> CRITICAL (throttle optional work, conservative reclaim)
 -> EMERGENCY (bounded fail-fast, wired supervisor if runnable)
 -> UNRECOVERABLE (scoped terminate or controlled fail-safe)
 -> RECOVERY (verify actual free frames/metadata, not assumed)
~~~

User-space Memory Manager/Supervisor should choose discretionary OOM victim when functioning; kernel must protect hard budgets, rescue resources, and a deterministic minimum safe response if that daemon itself is blocked. Distinguish **memory isolation survives** from **service recovers** and **unsaved data persists**. Outcome classes include denied allocation, app failure/termination, degraded service, controlled shutdown, or unrecoverable halt. Normal recovery may be claimed only after actually reusable frames, metadata reserve and backing/Pager paths recover. **Not** a magical global kill permission, guarantee that exactly one kill frees RAM, or guarantee no data is lost in unrecoverable faults. An app may ignore pressure signals; enforcement still uses quotas and frame references.

If Pager died, a file fault is BACKING_UNAVAILABLE rather than GLOBAL_OOM. Killing a Task cannot instantly reclaim externally referenced Shared MemoryObject, dirty file pages, DMA-pinned frames, quarantine, outstanding IPC fault requests or sponsor debt. No unlimited low-memory reclaim recursion; resource reserves and watchdog/error completion must survive. Swap, compressed RAM and anonymous disk backing are not M0/M1 guarantees.

## 11. Boot/Pager/driver dependency graph

**Wired boot cut proposal:**

~~~text
UEFI loader and memory map (RFC-006/017)
 -> Kernel MMU/PMM, reserved frames, kernel metadata and IRQ state
 -> RootAuthority + minimal Supervisor/recovery handler (wired)
 -> minimal storage control and completion/IRQ (wired)
 -> Pager code, stack, queue, tokens, page tables (wired)
 -> optional read-only file-backed pages
 -> regular services, apps, compat personality
~~~

A startup order alone does not prove runtime liveness. Analyze wait-for edges: App → Pager → StorageDriver → IRQ is possible only if all essential driver/IRQ/Pager paths are resident and queue credits reserved. Pager → VFS → Pager, kernel usercopy → Pager → the IPC lock held by usercopy, or memory allocator reclaim → pager → allocator are forbidden cycles. Critical operations must be possible with normal app allocations denied. A user-space Pager with an unpaged executable but pageable stack/IPC buffers is **not** wired enough.

RFC-006 defines service DAG and restart/recovery authorization. RFC-005 covers Thread priority/donation/deadline. Do not implicitly assume high priority or arbitrary scheduling context donation will solve a missing page cycle.

## 12. DMA pins, IOMMU and safe physical reclamation

Normal handle references, CPU mappings and **long-term DMA pin** are different lifetime/charge classes. A device could write RAM after its driver process has crashed; handle closure is not device quiescence. A safe handoff requires actual device group isolation, pin range/charge, IOVA mapping, driver/device quiesce or reset, IOTLB/interrupt remapping completion, unmap/unpin, CPU TLB fence and scrub.

- DMA authority is separately scoped DeviceGroup/IOMMUDomain+MemoryObject; arbitrary raw PFN never suffices.
- IOMMU group, PCIe ACS, peer-to-peer DMA and MSI/remapping capability are hardware-specific. QEMU does not prove Reference PC safety.
- R1 CPU mapping cutoff is weaker than R2-F family DMA quiescence, which is weaker than R2-O across all independent roots/pins.
- Failure to stop DMA leads to **Quarantined**, not frame reuse or a dishonest completed-revoke indicator.
- Linux's separate pin_user_pages/longterm pin guidance documents why reference counts cannot represent this safely [S9].

## 13. Revocation and guest executables

A MappingRecord must preserve grant-lineage identity after issuer handle close. After revoke barrier B, new fault replies and MAP operations through descendant caps must fail. R1 completion requires TLB/shootdown and no valid CPU mapping under revoked grant. Independent root grants to the same MemoryObject may stay live, preventing object-wide R2-O despite family A being quiesced.

Linux mmap/mprotect/brk/fork, Windows VirtualAlloc/sections and Darwin Mach VM semantics are user-space personality obligations, not automatic kernel syscalls. RFC-001 G-R restricted guest cannot map supervisor pages, interpret guest root as native MAP authority or forge kernel-internal FaultWaiter/continuation authority. W^X including alias/JIT exception and verified executable PagerSupply needs RFC-009/015/016/017 contracts. No memory subsystem design alone proves absence of microarchitectural side channels or firmware compromise.

## 14. Failure and concurrency matrix

| Fault/race | Unsafe result | Required safe disposition |
|---|---|---|
| invalid/overflow map descriptor | arbitrary PFN or kernel page exposed | no map committed |
| map vs B revoke | revoked grant maps new page | epoch recheck and commit order |
| stale fault reply after VA reuse | wrong backing installed | MappingGen+PagerGen mismatch reject |
| UNMAP without completed TLB fence | old core accesses recycled frame | delay free until fence |
| two COW writers | parent corruption/double charge | single-winner commit and retry |
| zero page first write OOM | writes to global shared zero frame | bounded fail or private frame |
| Pager crash mid-fault | infinite blocked Thread | fail/timeout/rebind under new gen |
| Pager stack faults on Pager | deadlock | wired cut, fail closed |
| usercopy fault during receive | partial cap install or untracked byte leakage | exclusive Claim+copyout protocol |
| sender no longer owns mapped cap | frame prematurely freed | mapping ref retained |
| dirty file page reclaimed | silent user data loss | retain dirty or explicit failure |
| image backing changed mid-fault | wrong executable contents | hash/version/snapshot validate |
| OOM victim's shared frames alive | falsely reports freed RAM | charge and ref conservation |
| DMA pin and Task death | device writes reused memory | fence or quarantine |
| guest mprotect privilege injection | native TaskControl/escalation | capability scope enforcement |
| W+X aliased mappings | executable bytes writable through alias | object-level alias rule |

## 15. Security and correctness invariants (VM-01..VM-32)

| ID | Proposed invariant — NOT verified |
|---|---|
| VM-01 | No raw PFN/VA/PID/foreign handle becomes mapping authority. |
| VM-02 | MAP/PROTECT rights = valid Task/AS/MemoryObject scope and current lineage. |
| VM-03 | Every mapping retains generation and revoke provenance. |
| VM-04 | No map after B; R1 requires completed CPU access cutoff and TLB synchronization. |
| VM-05 | Cross-domain reuse needs scrub and all CPU/DMA/object dependencies safe (R2-O). |
| VM-06 | No unauthorized map of kernel/supervisor/another Task private memory. |
| VM-07 | W^X and executable alias exceptions are correctly enforced for supported hardware. |
| VM-08 | VA reserve, physical commit, map, pin and persistence states cannot be conflated. |
| VM-09 | Newly exposed anonymous frames are zeroed/isolated, zero-page writes COW privately. |
| VM-10 | Concurrent COW/revoke cannot corrupt shared source or double-charge frame. |
| VM-11 | Per-Thread FaultWaiter binds Task/AS/Mapping/grant/Pager generations; one PagerRequest may correspond to several waiters. |
| VM-12 | PagerBinding/PageSupply cannot increase mapping rights, supply arbitrary foreign PFN or resume a foreign Thread; each waiter is revalidated by the kernel. |
| VM-13 | Dead/detached/restarted Pager cannot authorize stale or indefinitely stalled faults. |
| VM-14 | Critical boot/Pager/Supervisor/driver IPC path cannot fault into its own dependency cycle. |
| VM-15 | Kernel usercopy does not invoke unbounded Pager RPC under forbidden locks. |
| VM-16 | Receive bytes disclosure is bound to exclusive authorized Claim/irreversible publication. |
| VM-17 | Frames, mappings, dirty/pinned pages and fault metadata remain charged across death. |
| VM-18 | Untrusted Task cannot consume protected Pager/boot recovery reserve. |
| VM-19 | Nonreproducible anonymous/dirty/pinned data not silently evicted. |
| VM-20 | PagerSupply and dirty-clear alone are not durable file write/fsync proof. |
| VM-21 | OOM has bounded refusal/termination/fail-safe, not infinite privileged allocation. |
| VM-22 | Closing a MemoryObject handle cannot prematurely free external mapping/DMA refs. |
| VM-23 | R2-F/R2-O await required hardware fences or explicitly remain quarantined. |
| VM-24 | SMP PCID/TLB/VA reuse and stale faults do not resurrect old map authority. |
| VM-25 | Guest Linux/Windows/Darwin VM credentials do not mint native memory grants. |
| VM-26 | Boot/recovery wait-for graph has no mandatory Pager/self-storage/IPC cycle. |
| VM-27 | PageSupply and per-Thread fault continuation are distinct; every waiter has independent rights and lifetime validation. |
| VM-28 | Pager detached, Pager authority destroyed and file write durably committed are distinct dirty-data retention/availability states. |
| VM-29 | VA reserve, RAM commitment, resident frame and DMA pin have independent guarantees and charges. |
| VM-30 | Clean eviction and refault require source identity/version, integrity and explicit backing-availability failure semantics. |
| VM-31 | OOM isolation, normal service recovery and unsaved-data preservation are not presented as equivalent guarantees. |
| VM-32 | M0 boot/recovery path can run without file Pager and has a finite accounted stack/page-table/IPC/IRQ/error reserve. |

All 32 are proof/testing obligations with explicit CPU and trust assumptions; **none are claimed proven or implemented**.

## 16. Design alternatives, evidence gates and measurements

| Dimension | VM-A split | VM-B user VM | VM-C kernel VMO |
|---|---|---|---|
| anonymous allocation | minimal kernel candidate | mostly external | kernel-managed |
| read-only file supply | user Pager | user manager/Pager | kernel VMO + user Pager |
| reclaim | split TBD | user-driven and kernel safe release | kernel aging/clean eviction |
| OOM | kernel safety fallback + user policy | external manager dependency | kernel reclaim + service policy |
| core TCB | medium hypothesis | smaller kernel, broader trusted user infra | larger kernel mechanisms |
| proof debt | fault/IPC/revoke bridges | boot/cap management | dirty/reclaim allocator logic |
| speed | unmeasured | unmeasured | unmeasured |

**First evaluate correctness:** mapping/provenance, fault/restart, remote shootdown, clean vs dirty, Pager self-deadlock, DMA and OOM. Then compare equivalent workloads/security with p50/p95/p99 after separate implementation approval. Do not compare a fast weaker implementation to a stronger one and declare performance winner. RFC-001's IPC/performance targets are hypotheses, not achieved metrics.

## 17. Proposed model checking / test plan (no tests run)

- Finite Tasks A/B, two AS generations, one shared MemoryObject with independent grant roots, one frame, Pager P1/P2, one PagerRequest and independent FaultWaiters, one COW writer, one DMA pin and a finite ResourceBudget.
- Explore MAP vs revoke B, UNMAP+VA reuse vs PagerSupply, two writers+OOM, R1 while CPU stale translation, R2-F one root while independent root B and DMA remain, Pager death/restart with queued replies.
- Model copyin/out vs IPC ReceiveEscrow Claim, user page fault while kernel lock is held, critical Pager/storage wait-for cycles and resource reserve exhaustion.
- Prove/check conservation of frames/charges, no data exposure on cross-domain reuse, no dirty eviction, termination/failure semantics under finite memory, no capability amplification by guest.
- Prepare TLA+/PlusCal/Alloy style model and review counterexamples **as future design work**. This document contains no executed model or verified theorem.
- Future experimental authorization would cover QEMU x86_64 + OVMF demand-zero/COW/fault/revoke stress, then physical PC IOMMU/IRQ/device fence separately; capture SMP and mitigation configurations, tail latency and failure paths.

## 18. Open issues O4-01…O4-29

| ID | Priority | Question | Related |
|---|---|---|---|
| O4-01 | P0 | VM-A vs VM-B/VM-C final placement of anonymous/COW/reclaim? | 001/004 |
| O4-02 | P0 | Scoped PagerRequest/cookie identity, replay and kernel-internal FaultWaiter authorization? | 002/004/009 |
| O4-03 | P0 | Pager/boot/driver wired dependency and resource reserve? | 003/004/006 |
| O4-04 | P0 | Usercopy nonfaulting/slow path vs kernel lock order? | 003/004/009 |
| O4-05 | P0 | ReceiveEscrow Claim/disclosure vs page faults? | 003/004 |
| O4-06 | P0 | R1 TLB/MappingGen/CPU revoke race? | 002/004 |
| O4-07 | P0 | R2-O independent roots and DMA pin safety? | 002/004/007 |
| O4-08 | P0 | Bounded OOM kernel fallback if user coordinator cannot run? | 004/006 |
| O4-09 | P0 | Frame/COW/pin/dirty shared sponsor charge ledger? | 002/004 |
| O4-10 | P0 | Verified immutable backing version/hash/truncate? | 004/008/017 |
| O4-11 | P0 | Writable page writeback/fsync/crash consistency? | 004/008 |
| O4-12 | P0 | Boot firmware reserved RAM map and cross-domain scrub? | 004/006 |
| O4-13 | P0 | Physical IOMMU group, DMA pin/quiesce/reset? | 004/007 |
| O4-14 | P1 | Anonymous overcommit and no-swap default? | 004/009 |
| O4-15 | P1 | Kernel vs user clean-page eviction and cache policy? | 004/008 |
| O4-16 | P1 | Fork/COW/mremap guest and multi-Thread cloning? | 004/010 |
| O4-17 | P1 | W^X aliases, JIT, verified code page source? | 004/015/017 |
| O4-18 | P1 | CPU PCID/KPTI/SMEP/SMAP and page size requirements? | 004/015 |
| O4-19 | P1 | Coalesced faults, waiter fairness and cancellation? | 003/004/005 |
| O4-20 | P1 | Memory pressure thresholds and reclaim livelock? | 004/005 |
| O4-21 | P1 | Resize/hole-punch/invalidating shared file page? | 004/008 |
| O4-22 | P2 | Future swap/compress/NUMA/hugepage and ABI? | 004/018 |
| O4-23 | P0 | Coalesced PagerRequest vs per-Thread FaultWaiter and scoped PageSupply authority? | 002/003/004/009 |
| O4-24 | P0 | Dirty data lifetime when Pager detaches vs object destroyed, storage fails or OOM? | 004/006/008 |
| O4-25 | P0 | M0 file-Pager-free boot/teardown and worst-case wired pages/queue/IRQ reserve accounting? | 003/004/006 |
| O4-26 | P0 | VA reservation vs physical commit vs resident/pinned promise and overcommit errors? | 004/009 |
| O4-27 | P1 | Immutable clean page evicted then Pager unavailable or backing version changes? | 004/008/017 |
| O4-28 | P1 | PagerSupply multiwaiter races with B revoke, UNMAP and Task death? | 002/003/004 |
| O4-29 | P1 | OOM terminal data-loss reporting/read-only volume/shutdown decision boundary? | 004/006/008/015 |

## 19. Proposed decisions VM-D01…VM-D18 (not Accepted)

- **VM-D01:** VM-A is conditional research baseline; VM-B/VM-C alternatives remain active.
- **VM-D02:** Explicit AddressSpace/MemoryObject/MappingRecord/Frame/PagerBinding/PagerRequest/kernel FaultWaiter/Budget separation.
- **VM-D03:** Kernel owns authorization/page tables/TLB/frame zeroing, Pager cannot arbitrarily map foreign frames.
- **VM-D04 [revised]:** Research stage M0 wired boot + explicit anonymous/shared independent of file Pager, M1 readonly Pager, M2 COW/clean eviction, M3 writable paging/writeback; no stage implemented.
- **VM-D05 [revised]:** PagerBinding-scoped PageSupply and kernel-private per-Thread FaultWaiter/resumption are different authorities; coalesced faults require independent checks.
- **VM-D06:** Boot, Pager, storage, recovery IPC code/stack/tables are wired and resource-reserved.
- **VM-D07:** Kernel usercopy must not page back into forbidden lock cycle; ReceiveEscrow Claim models disclosure.
- **VM-D08:** R1 CPU cutoff, R2-F family drain, R2-O whole-frame reuse distinguished with DMA fences.
- **VM-D09:** Cross-owner zero/scrub and charged COW correctness before optimization.
- **VM-D10:** Clean reproducible eviction only, writable dirty mapping needs RFC-008 durability design.
- **VM-D11:** User-space policy OOM with kernel bounded hard quota and minimal reserved fallback.
- **VM-D12:** DMA pins are separate from normal refs, require real IOMMU hardware group fences.
- **VM-D13:** Guest POSIX/Windows/Darwin virtual memory semantics stay outside native Ring 0.
- **VM-D14:** No security/performance/ABI/recovery approval before RFC cross-review and proof evidence.
- **VM-D15 [new]:** Kernel owns waiter state and Thread wakeup; Pager only supplies/fails scoped pages/ranges, even with coalesced requests.
- **VM-D16 [new]:** Do not claim Pager detach/death/OOM preserves all dirty data without a durable recovery contract.
- **VM-D17 [new]:** VA reserve, charged physical commit, and pinned residency have distinct guarantees and failure cases.
- **VM-D18 [new]:** Prove a bounded M0 pager-independent wired recovery cut and OOM safety before higher-level VM availability claims.

Status remains **Proposed Specification**, no modification of D1–D8 Accepted Direction or ADR-002.

## 20. RFC joint review / exit gates

R1 design exit gate **not passed**. Need: MappingRecord/FaultWaiter/PagerBinding rights and shootdown model, ReceiveEscrow plus usercopy proof against Pager deadlock, wired boot/recovery graph with bounded memory/queue reserve, fail-safe OOM under dead Supervisor, clean vs dirty page durability boundaries, DMA group and pin/hardware reset assumptions, competing VM-A/B/C trust/performance comparison.

**Next recommended work:** RFC-006 Boot Trust Chain, Service Startup & Recovery jointly with RFC-003+004. RFC-005 Scheduling, RFC-007 physical DMA and RFC-008 durability remain important dependencies.

## 21. Official primary technical references

- **[S1] seL4 Mapping:** https://docs.sel4.systems/Tutorials/mapping.html
- **[S2] Fuchsia Address Spaces:** https://fuchsia.dev/fuchsia-src/concepts/memory/address_spaces
- **[S3] Fuchsia Virtual Memory Object:** https://fuchsia.dev/fuchsia-src/reference/kernel_objects/vm_object
- **[S4] Fuchsia Pager:** https://fuchsia.dev/fuchsia-src/reference/kernel_objects/pager
- **[S5] Fuchsia pager detach:** https://fuchsia.dev/reference/syscalls/pager_detach_vmo
- **[S6] Fuchsia memory reclamation:** https://fuchsia.dev/fuchsia-src/concepts/memory/memory_reclamation
- **[S7] Fuchsia RFC-0226 writeback:** https://fuchsia.dev/fuchsia-src/contribute/governance/rfcs/0226_zircon_pager_writeback
- **[S8] Linux userfaultfd:** https://docs.kernel.org/admin-guide/mm/userfaultfd.html
- **[S9] Linux page pins:** https://docs.kernel.org/core-api/pin_user_pages.html
- **[S10] Linux OOM policy:** https://docs.kernel.org/admin-guide/sysctl/vm.html

**Research limitation:** Third-party official documentation supports external-system comparison only. UniOS is not implemented or formally verified; no external code imported or licensed from these projects.

## 22. v0.2 심층 재검토 기록 및 반례 (2026-10-24)

**방법:** RFC-001/002/003 상호 정합성, 공식 seL4/Fuchsia/Linux 아키텍처 및 writeback/detach/OOM 자료를 비교하고 보안·수명 반례를 분석했다. 실제 모델 검사·구현·벤치마크·독립 감사가 아니다.

| 위험 | v0.1에 남은 모순 | v0.2 개선 | 다음 검증 |
|---|---|---|---|
| Page supply vs Thread continuation | 모든 Pager 공급을 Thread별 one-shot FaultToken으로 취급하면 coalesced fault와 결합할 때 Pager 권한/TBC 확대 | Kernel FaultWaiter와 scoped PagerRequest/Binding 분리 | N waiters/one page, revoke/kill/restart |
| Dirty data after Pager death | detach와 Pager 객체 파괴를 같은 영속성 계약으로 생각할 수 있음 | detached, destroyed, writeback pending, data lost를 별도 상태로 분리 | OOM vs dirty pinned data, storage failure |
| VM-P0 overcomplexity | read-only Pager, anonymous demand-zero, COW와 reclaim을 초기 필수 요소처럼 묶음 | M0 Pager 없는 resident 익명/공유 기반 → M1 readonly → M2 COW/reclaim → M3 writeback 단계 제안 | TCB/boot DAG, later performance |
| Reservation semantics | Reserved VA, promised resident RAM, actual mapped frame 충돌 | 독립된 reserve/commit/present/pin 계약 | quota conservation, overcommit OOM |
| Clean page availability | 재생성 가능한 clean backing을 항상 다시 가져올 수 있다고 가정 | content identity/version plus explicit Pager unavailable outcome | executable refault and source churn |
| OOM promise | 커널이 안전하게 거절해도 운영 복구/dirty persistence는 실패 가능 | safety, availability and data integrity 분리 | worst-case supervisor dead, hardware DMA quarantine |
| Code integrity and W^X | 주소별 RWX 검사가 실제 물리/객체 alias를 놓칠 수 있음 | per-object/alias policy와 JIT 전용 승인 요구 유지 | cross-AS alias and COW mapping |

### 22.1 P0 resolution order

1. Pager 없이 독립적으로 실행되는 **M0 wired resource cut**와 필요한 page-table, IRQ, fault, IPC, stack의 worst-case 최대 charge 경계를 모델링.
2. Kernel-private **FaultWaiter**와 PagerBinding PageSupply·FailRange·generation을 분리하고 coalesced waiter 및 fail/refault 상태기계를 검토.
3. RFC-003 ReceiveEscrow data disclosure와 VM usercopy re-entrancy 및 revoke B, R1 CPU shootdown을 공동 모델링.
4. Pager detach, object destruction, dirty writeback crash, OOM 및 data-loss reporting을 RFC-008과 통합해 설명.
5. VM-A/B/C를 **동등한 보안·정상/실패 동작** 조건으로 비교하고 메커니즘을 최소화한다. 속도 측정은 별도 구현 승인 후.

### 22.2 결론과 효력

VM-A는 여전히 유력하지만 최선으로 확정되지 않았다. M0~M3는 설계 검토의 순서이지 승인된 구현 단계나 기능 일정이 아니다. RFC-001/002/003과 Architecture v0.2의 상위 방향을 변경하지 않았으며, 새 VM-D15~18도 **Proposed**이다. ABI/기술 수치와 데이터 무손실 보장, 실물 하드웨어 시험 결과를 주장하지 않는다.

**Version history:** v0.1 registered 2026-10-24, preserved in Git history; v0.2 re-reviewed 2026-10-24 and corrected pager authority, dirty-data retention, staged feasibility and OOM outcomes.

---

**Document history:** 2026-10-24 v0.1 original research draft retained in Git; 2026-10-24 v0.2 critical review of Pager authorization, data-loss limits and staged recovery. No code, builds or benchmarks.

## 23. 2026-10-25 CAPIPC receipt admission, usercopy and Pager boundary — Proposed

**Cross-RFC research reference:** [CAPIPC Cross-RFC Safety Case v0.1](../reviews/capipc-cross-rfc-safety-case-v0.1.md). A(q) may reserve immutable protected record before K-S/K-I but cannot depend on faultable usercopy while holding locks required by Pager/IPC/M0. Page faults, Task/Mapping/PagerBinding generation changes and buffer aliases can intervene before copyout; first possibly visible user byte V is not undone by failed K-I or later receipt GC. A consumed q frontier is not permission to free FaultWaiters, escrow, pinned pages or sponsor debt. Recursion, lock order and bounded M0 fault/cleanup resources remain unproved P0; no virtual-memory algorithm or ABI is accepted.

## 24. 2026-10-25 CAPIPC formal-preparation usercopy/Pager boundary — Proposed

[CAPIPC Formal Model Preparation v0.1](../models/capipc-formal-model-preparation-v0.1.md) lists future guard/action obligations for copyin snapshot, partial usercopy, possibly observable V, ReceiveClaim, Pager FaultWaiter epoch revalidation and M0-safe nonrecursive lock/credit use. Kernel receipt admission/GC cannot imply a faulted copyout was invisible or free pinned/queued/physical debt. The proposed small model only **abstracts** Pager/hardware transitions; no real MMU/IOMMU TLB safety, low-memory feasibility, formal checker execution or implementation is established. RFC-004 remains Proposed.

## 25. 2026-10-25 Receiver exposure, usercopy aliases and K-I ordering — Proposed

[CAPIPC Receive Exposure, K-I & Handle Publication Ordering v0.1](../models/capipc-receive-exposure-ki-handle-publication-v0.1.md) refines RFC-004's untrusted user-memory boundary: a valid virtual output address may fault/unmap/repoint or alias other writable backing under concurrent Threads; potentially observable Vm/Vp/Vt must be marked before any write/map and cannot be rolled back by failed native cap K-I. No faultable Pager copyout may hold incompatible global IPC/MM/cleanup/IRQ lock; reserve protected R[q], cap slots and quota before irreversible exposure, but pre-reservation alone does not prove atomic output. This paper does not validate MMU/TLB behavior, paging performance, M0 feasibility, formal invariants or any kernel source.

## 26. 2026-10-25 Unified IPC copyout and Pager liveness handoff — Proposed

**Reference:** [CAPIPC Unified SEND–RECEIVE Semantic Contract v0.1](../contracts/capipc-unified-send-receive-semantic-contract-v0.1.md). K-S and K-I candidate protected cuts do not make Vm/Vp/Vt user memory stores atomic; aliases, concurrent remap and Pager faults can leave partial possible disclosure. K-C holds a kernel-custodied message until whole K-I or terminal; after relevant Vp, failed K-I cannot trigger foreign automatic requeue. Receipt, queue, claim and fault budgets, M0 reserve and lock/wait cycle proof are independent OPEN constraints. No VM code, formal checker or ABI approved.

## 27. 2026-10-25 CAPIPC P0 Pager and M0 feasibility gate — Proposed

[CAPIPC P0 Conflict Resolution & ADR Readiness Matrix v0.1](../reviews/capipc-p0-conflict-resolution-adr-readiness-v0.1.md) identifies faultable usercopy versus protected K-S/K-I cuts, Vm/Vp/Vt exposure, pinned/aliased output memory and Pager wait-for cycles as a coupled P0 feasibility gate (draft ADR-CAPIPC-C/E). No global IPC/VM/Claim/Revocation lock may be held across a faultable path needed by Pager/recovery; Q/W/queue/Claim/Token/lineage/pin debt and M0 CPU/frame/IRQ/cleanup must be independently accounted and bounded. This is a *candidate invariant, not proven resource or lock implementation*; no VM changes, tests, accepted ADR or ABI.

## 28. 2026-10-26 CAPIPC guarded publication/Pager race — Proposed

[CAPIPC Joint A/C/E Semantic-Cut Resolution v0.1](../models/capipc-joint-semantic-cut-resolution-a-c-e-v0.1.md) isolates a critical information-safety/liveness conflict: Vp/Vt authorization may be checked, then a faultable usercopy blocks on Pager, while the receiving Endpoint E-M or issuer audience PolicyEpoch changes; an unfenced resumed store after the cut may leak bytes even if K-I later denies all grants. A bounded in-flight publisher/epoch protocol with protected scope and termination or restricted resident/register-only outputs are the **unproved comparators**. No IPC/cap/VM lock that Pager needs may remain held across faultable copy. Precharged FaultWaiter/Claim/Receipt/Token/queue credit and independent M0 CPU/IPC/IRQ/frame/cleanup are required proof targets. No actual MMU implementation, checker evidence or numeric budget accepted.

## 29. 2026-10-26 Publication barrier and prepared no-fault usercopy feasibility — Proposed

**Study link:** [CAPIPC Publication Barrier & Pager-Safe Copyout Feasibility v0.1](../models/capipc-publication-barrier-pager-safe-copyout-feasibility-v0.1.md). Validating a writable user VA, pre-faulting or pinning its backing **does not alone** prevent remap/COW/other-Task writable aliases, ensure correct Task/MappingGeneration or authorize private-data publication. The P1 *paper comparator* requires actual prepared bounded non-faulting slices, issuer/Endpoint epoch PermitStart vs P-DENY serialization, and P-DRAIN evidence before claiming no further old-permit stores. An unbounded faultable permit cannot guarantee M0/owner-handoff completion; no global Pager-required lock may remain across usercopy. P0 register-only/P2 controlled mailbox are alternate scopes, not accepted VM designs or tests.

## 30. 2026-10-26 Destination VA versus MemoryObject and M-DRAIN — Proposed

[CAPIPC Destination Memory & Publication Barrier Cross-RFC Decision Table v0.1](../reviews/capipc-destination-memory-publication-cross-rfc-decision-table-v0.1.md) adds a precise RFC-003/004 ownership fork: a physical page pin can keep backing alive without preserving the user's **current VA→frame mapping**, correct COW/hugepage/TLB identity or private output audience. Exact current-VA V mode needs MappingGeneration-bound writes and VM **M-DENY/M-DRAIN** ordered against issuer P-DENY/P-DRAIN; O mode promises delivery only to a typed MemoryObject/version/range, not arbitrary VA. Small R returns avoid pageable output but still require ReturnRelease. No faultable usercopy under global Pager-needed locks; independent M0/claim/pin/tombstone budgets remain P0. No accepted VM algorithm, proof, numeric ABI or source code.

## 31. 2026-10-26 Paired VM M-DENY/M-DRAIN and issuer P-DENY/P-DRAIN — Proposed

[CAPIPC Paired Mapping–Publication Cut & Invalidation Ordering v0.1](../models/capipc-paired-mapping-publication-invalidation-ordering-v0.1.md) requires DestinationBinding (actual Task/AS/MappingGeneration and backing/share class) M to be jointly current with issuer/Endpoint audience P at a protected PublishStart. Physical page pinning alone is not stable *current VA* semantics, alias-private delivery or permission; COW, remap, TLB and pinned lifetime are different. M-DENY prevents new old-binding permits; M-DRAIN requires no old publisher may store before binding retired; P-DENY/P-DRAIN are separate issuer information fences. No faultable global Pager lock inside publisher slice; actual CPU/IRQ bound and M0 reserves remain OPEN. No actual VM algorithm, TLB proof, source or stable ABI accepted.

## 32. 2026-10-26 Minimal L1 no-Pager fixed-mapping diagnostic envelope — Proposed

[UniOS Minimal Boot, Exception & Memory MVP Contract v0.1](../contracts/unios-minimal-boot-exception-memory-mvp-contract-v0.1.md) deliberately excludes general VM/Pager/COW/MemoryObject and frame reclaim while researching a fixed **privileged only, W^X, guarded main/DF-IST stack** mapping needed for first QEMU/OVMF boot diagnostics. EFI GetMemoryMap descriptor size/version and reserved runtime/ACPI/MMIO/boot image/page-table extents must be validated; pinned or loader-owned ranges cannot be implicitly reclaimed. Exact loader CR3/page-table handoff, virtual entry mapping, stack transition and alias guard remain L1 P0 OPEN, not proven or implemented. This restricted scope **does not close** RFC-004 native Pager/OOM/CAPIPC and hardware R1/R2-F issues.

## 33. 2026-10-26 L1 fixed high-half CR3/bridge and early page ownership — Proposed

[L1 UEFI Handoff & Kernel Entry Microcontract v0.1](../contracts/unios-l1-uefi-handoff-kernel-entry-microcontract-v0.1.md) researches an ELF64 embedded kernel loaded into preallocated physical frames and mapped at one fixed high-half virtual base via a new supervisor-only CR3. A **dual-access mapped transition stub and temporary stack**, kernel RX/no writable alias, rodata RO+NX, data/BootContext/stack NX, guard/IST and deliberately no generic PMM are the first L1 candidates. Actual old UEFI linear→physical mapping, CR3 switch/entry PC/RSP, descriptor residency and exact ELF linker/relo subset remain OPEN; merely having EFI long mode or `x86_64-unknown-none` output does not certify mapping coherence. Pager, COW, full VM and R1/R2-F stay outside first scope. No implementation.

## 34. 2026-10-26 Lab high-half CR3 bridge, W^X and L1 Go readiness — Proposed

[L1 Fixed-Link ELF, CR3 Bridge & Entry-State Closure v0.1](../contracts/unios-l1-fixed-link-elf-cr3-bridge-entry-state-closure-v0.1.md) specifies development-only `0xffffffff80000000` ELF VA and fixed 4-level PML4/4KiB NX/WP mappings, distinct bridge/kstack/DF IST, no global low direct map and no writable alias to kernel RX frames. The old EFI bridge *linear* address must be audited to its true physical backing and remapped identically in new CR3 **at runtime after future user Go**; firmware identity mapping is **not universally assumed**. [L1 Go/No-Go](../reviews/unios-l1-implementation-go-no-go-assessment-v0.1.md) recommends only **L1 experimental implementation can be authorized**, not a functioning VM subsystem; Pager/COW/MemoryObject, Task AddressSpaces and CAPIPC receive publication remain OPEN/NO-GO.

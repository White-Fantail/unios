# UniOS RFC-004 — Virtual Memory, Pager, Cache & OOM Contract

> **Status:** Proposed — Specification v0.1 (미승인·미검증)
> **Date:** 2026-10-24 NZDT
> **Baselines:** [Architecture v0.2](../architecture/unios-architecture-v0.2.md), [RFC-001 v0.2](rfc-001-kernel-boundary-and-tcb.md), [RFC-002 v0.2](rfc-002-object-capability-lifetime-delegation-revocation.md), [RFC-003 v0.2](rfc-003-ipc-wire-protocol-and-synchronization.md)
> **Scope:** AddressSpace, MemoryObject, frames, mapping, page fault/user Pager, sharing/COW, copyin/out, dirty/clean caches, OOM, DMA pins, revocation, boot dependency.
> **Restriction:** Research/design only. This does not authorize kernel, SDK or service implementation, ABI freeze, verification, hardware support or performance claims.

## 0. 요약 — 조건부 연구 결론

**VM-A를 우선 연구:** 커널은 메모리 보호의 최소 집행(MMU·page table·TLB·physical frame의 소유권·zeroing·MappingRecord·FaultToken·pin·비상 자원)을 담당한다. 사용자 공간 Pager/VFS는 파일 backing 식별, 콘텐츠 공급, 파일 캐시, 재요청, 실행 파일 검증, writeback 및 일반 OOM 희생 프로세스 정책을 담당하는 방향을 검토한다. 익명 메모리 demand-zero와 COW의 위치, generic clean eviction은 아직 결론이 없다.

- **VM-A: Minimal kernel enforcement + anonymous mechanism + user file Pager.** 현재 우선 연구 후보. 실행 가능성이 곧 최선의 증거는 아니다.
- **VM-B: seL4-like user-space VM manager.** user-space가 frame/resource와 fault를 광범위하게 통제하고 kernel은 hardware mapping primitive를 제공한다. 낮은 정책 침투 가능성 vs 복잡한 자원 관리·부팅.
- **VM-C: Zircon-like kernel VMO and reclaim.** kernel anonymous demand, VMO, generic reclaim/page aging 역할 증가. 데스크톱 가용성 및 구현 용이성 가능성 vs TCB 확대.

**VM-P0 초기 범위:** 안전하게 resident한 부팅·복구 core, anonymous private, 제한된 explicit shared MemoryObject, 선택적 verified read-only pager-backed file page. Writable file-backed mmap, swap/compression, transparent POSIX fork, hugepage, broad dedup, aggressive JIT, writable executable aliases는 별도의 결정/검증 없이 구현 목표가 아니다.

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

Threats: malicious/buggy Pager/driver/user app, wrong or stale page contents, forged FaultToken, recycled VA frame mappings, SMP/TLB races, memory disclosure across Tasks, kernel usercopy fault recursion, queue-induced Pager death, OOM budget laundering, dirty-page data loss, DMA use-after-free and compatibility guest privilege escalation.

## 3. Object taxonomy and authority matrix

| Type / metadata | Scope and kernel role | Proposed rights |
|---|---|---|
| Task/AddressSpace | protection domain, page table root, VA range generation | MAP_CONTROL, UNMAP, PROTECT, QUERY for scoped Task |
| MemoryObject | backing identity/size, frames and anonymous/file/shared type | MAP_READ/WRITE/EXECUTE, PIN_IF_AUTHORIZED, QUERY, controlled DERIVE |
| MappingRecord | internal (AS generation, range, backing offset, rights, grant lineage) | not a global mutable user handle; source of revoke dependencies |
| Frame record | physical ownership, page-state refs, charge, zero/dirty/pin | allocated through typed resource authority, raw PFN not credential |
| PagerBinding | named pager instance backing a precise MemoryObject/range | SUPPLY_PAGE, FAIL_FAULT, DETACH, OBSERVE under that scope |
| FaultToken | one-shot resume/fail for exactly one fault generation | kernel-minted specific CONTINUE_ONCE or FAIL_ONCE |
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
6. VA reuse creates new MappingRecord generation; old FaultToken or COW completion cannot silently target the newly mapped backing object.

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

**FaultToken candidate:** faulting Task/Thread generation, AS/MappingRecord generation, MemoryObject backing version+offset, access rights, origin grant/epoch, PagerBinding identity/generation and one-shot resolution authority.

**State model:**

~~~text
CPU Fault -> Classify Current AS Mapping and Rights
 invalid/protection/revoked -> scoped exception/fail
 anonymous -> reserve + zero/supply + map OR bounded OOM fail
 file missing -> FaultPending + authorized Pager request
    -> PagerSupply | PagerFail | PagerDetached | Dead | Timeout
    -> Recheck TaskGen, ASGen, MappingGen, BackingVersion, PagerGen, B
    -> Install+ResumeOnce | CancelStale | FailThread
    -> Release fault/wait/queue/metadata charges safely
~~~

PagerSupply grants contents only for the MemoryObject/range to which the service is bound, not arbitrary Task mapping authority or raw physical PFN control. Already queued fault does **not** justify skipping new rights/epoch rechecks when page is installed. A single page may satisfy coalesced faults, but every waiter has its own Task/Mapping generation, lifecycle, cancellation and sponsor obligations.

**Pager detach/failure:** no new request into detached binding. Inflight faults terminate or transfer through a separately authorized rebind path; never hang forever simply because service disappeared. Already resident clean content can stay mapped where safe; dirty contents cannot be silently deleted. Service restart changes PagerBinding incarnation; old supply/fail replies cannot target a new instance. Transparent process continuation is NOT guaranteed in VM-P0.

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

**Writeback deferred:** dirty generation, concurrent writer isolation, writeback begin/snapshot, actual data+metadata durability, success/failure acknowledgement, fsync/flush semantics, crash replay, truncate/hole-punch and Pager death must be jointly designed with RFC-008. Merely receiving a Pager ACK cannot prove stable storage or file-system crash consistency. Current Fuchsia pager-detach/writeback descriptions differentiate dirty retention [S5,S7]. No writeable mmap general compatibility promise now.

## 10. Memory pressure and OOM safe fallback

**Different errors:** VA exhaustion, sponsor quota, global RAM pressure, kernel metadata shortage, non-evictable dirty/pinned pressure, Pager unavailable, unsafe DMA quarantine, overcommit-at-fault. A successful virtual reservation may not commit RAM; overcommit would allow later faults to fail. Error messages/SDK must distinguish.

**Candidate staged response:**

~~~text
NORMAL -> WARNING (advisory shrink and safe clean cache eviction)
 -> CRITICAL (throttle optional work, conservative reclaim)
 -> EMERGENCY (bounded fail-fast, wired supervisor if runnable)
 -> UNRECOVERABLE (scoped terminate or controlled fail-safe)
 -> RECOVERY (verify actual free frames/metadata, not assumed)
~~~

User-space Memory Manager/Supervisor should choose discretionary OOM victim when functioning; kernel must protect hard budgets, rescue resources, and a deterministic minimum safe response if that daemon itself is blocked. **Not** a magical global kill permission, guarantee that exactly one kill frees RAM, or guarantee no data is lost in unrecoverable faults. An app may ignore pressure signals; enforcement still uses quotas and frame references.

If Pager died, a file fault is BACKING_UNAVAILABLE rather than GLOBAL_OOM. Killing a Task cannot instantly reclaim externally referenced Shared MemoryObject, dirty file pages, DMA-pinned frames, quarantine, outstanding IPC fault requests or sponsor debt. No unlimited low-memory reclaim recursion; resource reserves and watchdog/error completion must survive. Swap, compressed RAM and anonymous disk backing are not VM-P0 promises.

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

Linux mmap/mprotect/brk/fork, Windows VirtualAlloc/sections and Darwin Mach VM semantics are user-space personality obligations, not automatic kernel syscalls. RFC-001 G-R restricted guest cannot map supervisor pages, interpret guest root as native MAP authority or forge FaultToken. W^X including alias/JIT exception and verified executable PagerSupply needs RFC-009/015/016/017 contracts. No memory subsystem design alone proves absence of microarchitectural side channels or firmware compromise.

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

## 15. Security and correctness invariants (VM-01..VM-26)

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
| VM-11 | One-shot FaultToken binds Task/AS/Mapping/Pager generations. |
| VM-12 | PagerSupply cannot increase map rights or supply arbitrary foreign PFN. |
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

All 26 are proof/testing obligations with explicit CPU and trust assumptions; **none are claimed proven or implemented**.

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

- Finite Tasks A/B, two AS generations, one shared MemoryObject with independent grant roots, one frame, Pager P1/P2 and FaultToken, one COW writer, one DMA pin and a finite ResourceBudget.
- Explore MAP vs revoke B, UNMAP+VA reuse vs PagerSupply, two writers+OOM, R1 while CPU stale translation, R2-F one root while independent root B and DMA remain, Pager death/restart with queued replies.
- Model copyin/out vs IPC ReceiveEscrow Claim, user page fault while kernel lock is held, critical Pager/storage wait-for cycles and resource reserve exhaustion.
- Prove/check conservation of frames/charges, no data exposure on cross-domain reuse, no dirty eviction, termination/failure semantics under finite memory, no capability amplification by guest.
- Prepare TLA+/PlusCal/Alloy style model and review counterexamples **as future design work**. This document contains no executed model or verified theorem.
- Future experimental authorization would cover QEMU x86_64 + OVMF demand-zero/COW/fault/revoke stress, then physical PC IOMMU/IRQ/device fence separately; capture SMP and mitigation configurations, tail latency and failure paths.

## 18. Open issues O4-01…O4-22

| ID | Priority | Question | Related |
|---|---|---|---|
| O4-01 | P0 | VM-A vs VM-B/VM-C final placement of anonymous/COW/reclaim? | 001/004 |
| O4-02 | P0 | FaultToken identity, replay and PagerBinding scope? | 002/004/009 |
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

## 19. Proposed decisions VM-D01…VM-D14 (not Accepted)

- **VM-D01:** VM-A is conditional research baseline; VM-B/VM-C alternatives remain active.
- **VM-D02:** Explicit AddressSpace/MemoryObject/MappingRecord/Frame/PagerBinding/FaultToken/Budget separation.
- **VM-D03:** Kernel owns authorization/page tables/TLB/frame zeroing, Pager cannot arbitrarily map foreign frames.
- **VM-D04:** VM-P0 research covers anonymous/shared and verified read-only file paging before dirty writeback/swap/JIT.
- **VM-D05:** FaultToken is one-shot, scoped to original Task/AS/Mapping/Pager generation.
- **VM-D06:** Boot, Pager, storage, recovery IPC code/stack/tables are wired and resource-reserved.
- **VM-D07:** Kernel usercopy must not page back into forbidden lock cycle; ReceiveEscrow Claim models disclosure.
- **VM-D08:** R1 CPU cutoff, R2-F family drain, R2-O whole-frame reuse distinguished with DMA fences.
- **VM-D09:** Cross-owner zero/scrub and charged COW correctness before optimization.
- **VM-D10:** Clean reproducible eviction only, writable dirty mapping needs RFC-008 durability design.
- **VM-D11:** User-space policy OOM with kernel bounded hard quota and minimal reserved fallback.
- **VM-D12:** DMA pins are separate from normal refs, require real IOMMU hardware group fences.
- **VM-D13:** Guest POSIX/Windows/Darwin virtual memory semantics stay outside native Ring 0.
- **VM-D14:** No security/performance/ABI/recovery approval before RFC cross-review and proof evidence.

Status remains **Proposed Specification**, no modification of D1–D8 Accepted Direction or ADR-002.

## 20. RFC joint review / exit gates

R1 design exit gate **not passed**. Need: MappingRecord/FaultToken rights and shootdown model, ReceiveEscrow plus usercopy proof against Pager deadlock, wired boot/recovery graph with bounded memory/queue reserve, fail-safe OOM under dead Supervisor, clean vs dirty page durability boundaries, DMA group and pin/hardware reset assumptions, competing VM-A/B/C trust/performance comparison.

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

---

**Document history:** 2026-10-24 — RFC-004 v0.1 researched and drafted for design review. No code, build or benchmark.

# UniOS RFC-005 — Scheduling, Preemption & CPU Budget

> **Status:** Proposed — Specification v0.2 (adversarial re-review; not Accepted, implemented, benchmarked, or formally verified)
> **Date:** 2026-10-24 (Pacific/Auckland)
> **Architecture:** [UniOS v0.2](../architecture/unios-architecture-v0.2.md)
> **Cross-RFC contracts:** [RFC-001 Kernel/TCB](rfc-001-kernel-boundary-and-tcb.md), [RFC-002 Capabilities](rfc-002-object-capability-lifetime-delegation-revocation.md), [RFC-003 IPC](rfc-003-ipc-wire-protocol-and-synchronization.md), [RFC-004 VM/Pager](rfc-004-virtual-memory-pager-cache-and-oom.md), [RFC-006 Boot/Recovery](rfc-006-boot-trust-chain-service-startup-and-recovery.md).
> **Out of scope:** kernel code, running scheduler, selected numeric timer period/slice/priority, native ABI freeze, hard realtime guarantee, benchmark or proof.

## 0. Executive recommendation: SCHED-A as a conditional research baseline

**SCHED-A:** per-logical-CPU preemptive, kernel-enforced **weighted-fair general scheduling** + explicit Task/ResourceGroup CPU runtime charging, separate **weight vs maximum cap vs guaranteed reservation** accounting + a separately bounded **M0 essential recovery reservation/class** with time, IRQ and memory resources. Local runqueues and SMP work distribution are candidates; no final algorithm or queue data structure is adopted.

The intent is a practical first x86_64 desktop OS, not an arbitrary hard-realtime system. Prefer a **small trustworthy scheduler enforcement mechanism** over adding a complex all-powerful user scheduler or assuming highest-priority threads will make progress. Scheduler policy/admission is user-space only to the extent that kernel still enforces CPU usage, protected recovery resource eligibility and safe preemption.

**Important not-yet-adopted choices:**
- SCHED-B: kernel hybrid weighted fair + EDF/CBS-style admitted deadline scheduling for a bounded subsystem (audio/input/video or drivers in future).
- SCHED-C: seL4 MCS-like explicit SchedulingContext/replenishment/passive servers with request-scoped budget transfer, subject to huge capability/lifetime and async-IPC complexity.
- SCHED-D: user-space configurable policy/plug-in scheduler with a permanently safe kernel fallback, considered after M0 correctness. This need not entail a general eBPF-like runtime.
- **M0 recovery CPU time is not free, infinite, or magic.** A critical reservation must have finite CPU/IRQ runnability resources and an isolation plan, and recovery cannot work if the CPU/clock/interrupt mechanism itself fails.
- **No automatic budget/priority donation on Channel send.** RFC-003 proposes bounded asynchronous Channel E1 endpoints plus a scoped optional ReplyToken; there is no kernel-defined synchronous server execution owner in ordinary async send. Passive server donation from seL4 does not directly fit without a new explicit request token and lifecycle.

**v0.2 핵심 정정:** Child **maximum CPU caps may be overcommitted**; this is distinct from minimum guaranteed **reservations that require admission and cannot be double-sold**. Group charge is CPU execution-time summed across running cores, not wall-clock duration. Kernel/IRQ work is neither an unlimited free pool nor always attributable to a specific untrusted thread. Runnable M0 with booked budget is not guaranteed dispatch if kernel nonpreemptible latency, timer/interrupt or dependency chains lack proved bounds. No numerical reservation or timing guarantee is approved.

**Core distinctions:** Priority / scheduling weight is not a guaranteed runtime. A wall-clock deadline is not a CPU budget. Permission to change scheduling policy is not permission to mint CPU time. A blocked Thread may have remaining budget but no runnable work. Donating *priority* is not transferring executable *budget*. A timer firing is not proof the recipient has executed.

**The design can fail:** a strict elevated recovery class with no enforced bounded charge can starve all normal users; an unrestricted donated budget can let an attacker monopolize a Pager; refusing any service CPU while its own resource budget is depleted can deadlock recovery. This RFC treats each as a P0 gate.

## 1. Motivation, threats and acceptance philosophy

### 1.1 Functional and safety targets — Proposed, unproven

- Run normal GUI/user applications, server processes, Pager, VFS, drivers and sandboxed Linux personalities with sensible relative fairness and no one-app universal monopolization.
- Preempt a CPU-bound Thread and admit a bounded critical scheduling class capable of error delivery/teardown under ordinary quota exhaustion.
- Make every CPU-time charge and scheduling privilege auditable across creation, context switch, kernel entry, IPC, blocking, dying and restart.
- Provide monotonic deadline and timeout semantics to RFC-003, independent of wall clock changes and with suspend meaning explicitly documented.
- Integrate wakeups, IRQ scheduling, priority inversion, CPU hotplug/migration and VM page faults without creating new kernel deadlocks or unauthorized work amplification.
- Preserve the trust boundary: untrusted guest UID0, app-supplied priority and user-space service discovery name do not authorize privileged CPU class or M0 reservation use.
- Allow a Task/service to be throttled without making its existing caps magically invalid or reclaiming shared page/DMA resource prematurely.

### 1.2 Adversarial scenarios

Malicious app creates thousands of runnable Threads, floods IRQ or IPC Events, spins without syscalls, yields to game latency classification, changes affinity to concentrated CPU, deliberately causes page faults to make Pager run for it, loops unbounded syscall/exception entry to hide CPU cost, requests deadline policy with infeasible utilization, holds high-priority resource locks, spoofs a critical service name, overuses cancellation/restart to replenish budget, sends nested calls across servers to amplify donation, and mixes guest restricted mode with normal-mode supervisor transitions.

Compromised service/driver may falsely claim health/priority, fail to acknowledge IRQ, crash while a borrowed budget remains, or hold a lock needed by RecoveryAuthority. Damaged CPU timer, malicious firmware or unsupported IOMMU hardware is a different layer of trust and may prevent all scheduler liveness guarantees.

### 1.3 What this RFC must NOT promise

- **No hard scheduling response-time bounds** without worst-case execution and blocking/IRQ/timer drift proofs for a specified hardware profile.
- **No guaranteed fairness latency** under unlimited runnable creation, severe overload or unaccounted IRQ/firmware interference.
- **No exact p95/p99 scheduler or IPC latency**, percent CPU reservation or timer accuracy before tests.
- **No universal POSIX SCHED_FIFO, RT/FUTEX PI, Windows scheduler policy, Mach thread QoS/turnstile support** from defining a native Thread object.
- **No proof that priority inheritance fixes arbitrary wait cycles** or a file Pager that depends on its own unavailable code/stack.
- **No assumption that a user-space scheduler can guarantee CPU dispatch** if privileged kernel scheduling itself is broken.

## 2. Primary-source architecture comparison

| Official source | Verified existing-system fact | UniOS use and limitation |
|---|---|---|
| seL4 MCS tutorial [S1] | SchedulingContext provides upper runtime bounds, replenishment chunks and passive-server donation in synchronous IPC | **Upper cap is not guaranteed dispatch; finite replenishment entries may lose usable time**; async Channel requires independent protocol and accounting |
| seL4 Scheduling API [S2] | scheduling context objects can bind to Thread or Notification; rights control scheduling authority | scheduling control is typed and separate from mere Thread handle |
| Fuchsia Zircon Scheduler Overview [S3] | per-CPU scheduler has Fair and Deadline disciplines, IPIs coordinate cores | SCHED-A/B study; Fuchsia product behavior is not a UniOS guarantee |
| Fuchsia Fair Scheduler [S4] | general-purpose Threads receive weighted CPU proportion across competition | weighted fair baseline, not a fixed maximum wait guarantee |
| Linux EEVDF [S5] | eligible tasks with positive/zero lag and virtual deadlines balance fairness and short-slice responsiveness | fair policy evolution matters; do not freeze CFS or clone EEVDF as an assumed simple primitive |
| Linux SCHED_DEADLINE [S6] | EDF/CBS runtime/deadline/period plus admission controls and CPU affinity/domain restrictions | RT feasibility is conditional on affinity, overhead, blocking, capacity and hardware, not just Σ(runtime/period) ≤ CPUs |
| Linux PREEMPT_RT [S7] | most IRQs threaded; rtmutex PI helps many lock paths, but core scheduler/entry remain critical exceptions | blocking/nonpreemptible kernel sections, not priority alone, dominate worst-case latency |
| Linux RT-mutex design [S8] | owner-linked PI and nested chain propagation required to prevent unbounded priority inversion | user mutex PI/IPC donation/CPU transfer must not be conflated |
| Linux timekeeping [S9,S10] | clocksources and clockevents distinct; monotonic timeline can stop during suspend; boottime vs monotonic differ | expose exact clock domain/deadline/timer meaning; no wallclock deadline assumption |
| Linux sched_ext [S11] | optional programmable scheduler reverts on failures/stalled runnable tasks | fallback can be designed but cannot rescue a failed core scheduler/timer/IRQ |
| Linux cgroup v2 CPU controller [S12] | cpu.weight, cpu.max, cpu.max.burst distinguish proportional sharing, limit and burst; generic distribution model permits overcommitted limits but not exclusive allocations | **Correct SCHED-A's cap/reservation ambiguity:** admission of protected minimum and validation of maximum are separate |
| Linux capacity-aware scheduling [S13] | CPU capacity and IRQ/DVFS effects differ from logical CPU count | do not assume every logical CPU represents identical schedulable work or a fixed timing bound |

**External systems are comparisons, not copied code, accepted algorithms or validation results.** Historical Linux CFS descriptions must not be quoted as the current sole general Linux scheduling discipline in 2026. Newer Fuchsia scheduler documentation includes Deadline scheduling; old notes describing it as future-only are stale.

## 3. Kernel, user-space policy and TCB boundary

**Kernel enforcement should own:** Thread run state and dispatch, per-CPU runqueue correctness, preemption and context switching, scheduler clock/IRQ interface, enforcement/charging of authorized CPU budgets, permit checks for policy/priority/affinity changes, wakeup/timeout semantics, cross-core IPIs, bounded critical sections and schedulable M0 recovery reservation.

**User-space service policy may own:** default weight/interactive profile requests, app sandbox quotas within a signed ResourceBudget envelope, power/thermal hints, user consent to boost specific media/desktop services, per-service resource admission and reporting. These policies are **suggestions constrained by kernel rights and physical availability**, not a way to schedule kernel threads directly from arbitrary user memory.

The actual clocksource and local timer interrupt hardware interface may require privileged kernel code and carefully limited CPU/IRQ/TSC configuration. Device drivers and GUI frameworks remain outside kernel per RFC-001. A trusted CPU policy service that is compromised can affect its allowed quotas but not mint new reserve/bypass runqueue admission.

### 3.1 Asset-specific trust

- TCB-K: scheduling queues/state, accounting, timer interrupt/reschedule IPIs, Thread/Task identity, policy-enforcement hooks and their data structures.
- TCB-B: M0 bootstrap admission config, protected recovery scheduler resources, independent post-ExitBootServices timer/watchdog assumptions.
- TCB-S: any ServiceSupervisor/ResourceIssuer/PolicyPortal allowed to allocate/alter other services' weights/budgets. Limit their grants and distinguish broad policy influence from CPU-enforcement rights.
- Hardware assumptions: actual monotonic clocksource/TSC/APIC/HPET, firmware power state, CPU online set and hypervisor behavior; QEMU cannot establish physical timer/delay guarantees.

## 4. Terms, typed objects and authority

| Candidate typed object/record | Responsibility | Permission and ownership |
|---|---|---|
| Thread (RFC-002) | execution context, block/wake/kill, Task and restricted guest state | ThreadRun/Stop/Observe limited to owning Task or authorized Supervisor |
| ResourceGroup / CpuBudgetGroup | hierarchical accounting and throttling envelope over one/many Task Threads | CreateSubbudget, Charge, Observe, ConfigureScoped; CPU group sponsor not equal to Thread handle owner |
| SchedulingPolicyGrant | range-limited authority to select weight/class/affinity/deadline profile for named Task/group | CONFIGURE_POLICY within parent authorized bound, not CREATE_CPU |
| SchedulingContext (optional SCHED-C) | explicit kernel-accounted budget token attached to a runnable or passive server | MINT only by limited root + CPU share reservation; BIND/UNBIND/TRANSFER narrowly scoped; not in SCHED-A v0 |
| SchedulerAdmissionRecord | internal proof of accepted group demands and resource sums | kernel-managed record; no user forged admission |
| Timer/DeadlineWait (candidate) | monotonic ClockID, expiry, slack and generation | WAIT / CANCEL for a scoped Thread/WaitSet; cannot wake foreign Task by numeric ID |
| RecoveryCpuReserve | protected finite CPU service eligibility and IRQ/cleanup budget | kernel-fixed trusted M0 grant; untrusted Task cannot inherit by ServiceID name |
| DonationLease (future, not default) | temporally and identity-scoped transfer/loan of chargeable caller budget to a named receiver | optional explicit cap, finite chain/depth, revocation and return state; MUST NOT mint extra time |

**Separate edges:** authority to configure priority, account paying for actual CPU cycles, Thread held scheduling entitlement and actual runnable/wait relationships. These four graphs change differently across IPC MOVE, Task death, revocation and sponsor adoption.

A ResourceGroup may own many Threads. A user creating 100 Threads inside the same group must not multiply its aggregate **minimum reservation**, or effective shared CPU cap, by 100. **Overcommittable child hard caps are constraints, not reservations** and their configured sum need not fit parent capacity; aggregate *actual* execution is still parent-capped. Likewise two aliases to one SchedulingContext must not represent twice the original CPU runtime. Effective budget rights must be attenuated and ancestor/global admission enforced. Runnable Thread may be charged to a group different from process creator only through explicit authorized service attribution, not hidden donation.

## 5. SCHED-A general fair scheduling candidate

### 5.1 Weighted fair vs strict realtime

One possible initial model: each logical CPU holds a scheduler runqueue with weighted-fair eligible Threads (or groups with nested fair scheduling) and a separate bounded M0 emergency class. A runnable Thread consumes CPU of its authorized ResourceGroup. Group hierarchy prevents a malicious app from generating many Threads to gain a larger system share.

- A **weight** controls relative share among eligible competing entities, not minimum service within a fixed wallclock interval.
- A **hard group cap** is a maximum amount of charged CPU execution permitted in a defined period (or sliding window if chosen). It throttles after effective parent+child limit exhaustion; its numerical value can exceed currently feasible dedicated CPU capacity, and child caps may sum above parent cap, as a **maximum** is not an allocation. Configured burst/carryover is distinct and must be bounded. Other groups may use free eligible capacity according to fairness and M0 reservation.
- **Min service/CPU reservation** for M0 is separately admitted against effective capacity of specified CPUs after bounded system/IRQ overhead and competing hard reservations. **Reserved CPU is an execution eligibility/service floor under assumptions, not immediate dispatch or a service-completion guarantee.** Cap the M0 class's aggregate excess as well to avoid normal desktop starvation.
- Runqueue fairness does not imply no-starvation if runnable count is unbounded or threads can select deliberately hostile affinity. A bounded admission and eligible-set rule are necessary to make any starvation claim.
- Migration and work stealing must preserve group accounting and bounded fairness debt. One Thread never executes simultaneously on two cores simply because of concurrent wakeups or a stale runqueue entry.
- Kernel cleanups should happen in preemptible bounded chunks where possible, not as huge unaccounted raw critical sections that defeat user weights.

**M0 special case:** an emergency class may preempt ordinary work to execute a finite critical recovery operation. Admission and accounting must bound its operation/time, even for a privileged server bug. If critical work indefinitely loops, recover via watchdog/terminal failure; no claim ordinary apps retain service when M0 consumes all CPU.

### 5.2 Comparison with weighted EEVDF-like policy

A virtual-runtime+virtual-deadline discipline can improve wakeup behavior for interactive short tasks while preserving long-term fairness. But complexity arises in sleeper credit, migration, nested groups, burst starvation and attacker-triggered excessive wake/sleep. The initial scheduler policy is **an architectural family**, not EEVDF code or a fixed virtual-deadline formula. Microbenchmark and adversarial desktop workload comparison after separate implementation authorization decide details.

### 5.3 Alternative baseline simpler round-robin

A bounded round-robin with per-group caps could be easier for an M0 bootstrap experiment than full fair queues. That may be appropriate as a *development milestone hypothesis*, not proof it meets developer-desktop responsiveness. The design should keep an internal scheduler interface that can evolve without exposing virtual-runtime implementation to KABI callers.

## 6. CPU budget hierarchy and accounting conservation

**v0.2 ledger split:** A parent/group holds four *different* scheduling values: (1) weight (relative fairness under contention), (2) maximum CPU consumption cap per declared window and burst, (3) guaranteed minimum execution reservation admitted against effective domain capacity, and (4) actual measured CPU runtime. Child **guaranteed reservations** cannot overdraw admitted parent/physical protected capacity. Child **maximum caps may be configured above parent cap in aggregate**—only the concurrently consumed real CPU time must satisfy effective ancestor constraints. A second Thread/handle creates neither another reservation nor extra physical execution time.

**Accounting example (logical, not a selected policy):** In one wall-clock millisecond, two cores each execute the same group for 1 ms. The group's aggregate CPU execution is **2 CPU-ms**, not 1 ms. Sum exclusive runtime events from every CPU; attribute each CPU interval once to running context and to each ancestor's *aggregate counters without counting those nested sums again as additional physical CPU*. Work-normalized CPU capacity is a separate metric; SMT and thermal effects prohibit interpreting 2 CPU-ms as a guaranteed amount of completed work.

**Candidate concepts:**
- Actual executed CPU time: attributed to current Thread/group for user-mode and attributable kernel synchronous syscall work, including guest-restricted vs normal supervisor transitions.
- Interrupt/softirq and cross-Task kernel maintenance: charge a separate bounded system overhead pool or attributed origin when safely knowable; **do not silently classify unbounded attacker-triggered interrupts as free**.
- Async server work for a request: normally charged to server ResourceGroup under SCHED-A; client sending an IPC message is not automatic budget transfer.
- CPU **maximum cap**: (CPU-time budget, charging/replenishment window, optional bounded burst, group ancestry) controls upper use. **Guaranteed reservation** separately requires admission of minimum service against available capacity and competing reservations on eligible CPUs. Do **not** reject overcommitted sums of configured maximum child caps solely as if each were a reserved allocation. Both are non-ABI concepts here.
- Thread weight and group cap are orthogonal. A low-weight group may burn its entire cap when alone; a high-weight group under a hard cap stops regardless of its nominal scheduling priority.

### 6.1 Accounting transitions

~~~text
BudgetEligible -> Runnable -> Running (charge execution)
  -> BlockedWait / FaultBlocked / Sleeping (no executing charge)
  -> RunnableOnWake (only if not throttled/terminated)
  -> Throttled (no budget remaining; deadline timer replenishes by policy)
  -> RunnableAfterRefill (only if still authorized and live)
  -> Dying -> Retired (charges reconciled; queued grants and DMA separate)
~~~

**Double-charge hazard:** a caller blocks on IPC, server handles work and returns a result. Charging both caller for elapsed waiting *wall time* and server for the same CPU runtime is not CPU-time accounting. A wall deadline still advances while blocked; per-Thread executed CPU budget does not. Likewise an async Pager serving multiple FaultWaiters should normally be charged once for each actual executed CPU interval, not once per waiter just because N threads were woken.

**Per-CPU charge commit:** elapsed running intervals need nonnegative monotonic deltas, no duplicate interval at context switch/IRQ, and atomic or formally safe batched debit at ancestor budgets. A per-CPU cached remaining quota without global reservation can overspend during concurrent execution. Kernel time charged to a Thread and a separate kernel-overhead *category* must not both become extra physical execution: a breakdown may be hierarchical but physical total is counted once.

**Charge migration:** when Task migrates across CPUs or from restricted guest to normal-mode supervisor in the *same Thread*, its current charge owner and eligibility must be checked without reset/refill. A distinct user-space broker or server runs under its own group unless a specifically authorized future lease is attached. Kill/restart cannot turn spent budget into a fresh unlimited one: child budgets and per-service restart policy reconcile against parent admission.

### 6.2 Capacity and admission are conditional

For **minimum reservations / future admitted deadline jobs** (not for maximum caps), demand-vs-capacity is a necessary sanity check. A sum of runtime/period shares less than some online CPU count is *not sufficient* for arbitrary affinity, SMT, deadlines, migration, blocking, IRQ overhead or capacity changes. Double-counting parent and child reservation as independent jobs is also invalid. CPU reservations in overlapping domains need explicit per-domain feasibility and online-core re-admission. CPU affinity, migration restrictions, nonpreemptible kernel/IRQ intervals, shared locks, capacity/thermal heterogeneity and blocking matter. Real-time deadline admission needs documented WCET/execution, relative deadline and period with feasibility proof for selected SMP model (Linux EDF/CBS documentation [S6]).

M0 reserved capacity on a single available core may reduce normal-work share; admission must explicitly account it and avoid claiming 100% of CPU is simultaneously reserved for other deadlines. If hardware provides fewer online cores after an outage, revalidate admission or safely degrade/fail; a bound proven for four CPUs is invalid after three go offline.

## 7. Thread state machine, blocking and single wake authority

A Thread may be CREATED, RUNNABLE, RUNNING, BLOCKED_IPC, BLOCKED_FAULT, BLOCKED_TIMER, THROTTLED, SUSPENDED, STOPPING, TERMINATED or RETIRED. Separate **runnable eligibility** from **physical running** and **still-has-budget**. A Thread that sleeps and times out may still be throttled; a newly supplied page may resolve fault but fail to run while its group is exhausted. **Recommended decomposition:** orthogonal wait/stop state (Running/Ready/IPCBlocked/FaultBlocked/TimerBlocked/Stopped/Dead), quota eligibility (Eligible/Throttled) and CPU ownership (none/one CPU). The flat state list is shorthand, not one exclusive enum: a FaultBlocked Thread may simultaneously have depleted budget and an expired deadline without becoming runnable.

- A wakeup must validate Thread/Task generation and current state, and atomically queue at most once. Multiple IRQ, ReplyToken, PagerSupply and timeout events may race; wrong-generation wakeup is rejected.
- Cancelling a timeout or wait does not cancel work already committed remotely (RFC-003). CPU dispatch cannot transform a local expired wait into rollback of external effects.
- Stopping/killing Thread prevents later application-level execution in that identity; cleanup can still involve privileged kernel work and independent queued objects (RFC-002).
- A Thread that blocks on Pager FaultWaiter does not become runnable just because PagerSupply entered kernel; kernel revalidates its mapping/rights/generation first (RFC-004).
- A guest restricted/normal transition may modify which code executes, but cannot create a new Task/global scheduling right or reset its budget. The kernel must attribute elapsed CPU execution to the right sandbox and preserve preemption checks.
- A zombie Thread is not a runnable resource. Object/handle lifetime and scheduling life are distinct; closed Thread handle alone does not necessarily terminate running Thread.

### 7.1 State edges that must linearize

Running vs killed; timeout vs reply wake; blocked on IPC vs revoked Endpoint; fault resolution vs revoked mapping; timer callback vs CPU offline; Thread migration vs scheduler group change; budget throttle vs KernelCall completion; service restart vs old task reusing cached scheduling context. Each edge requires an owner, generation check and no duplicate runqueue insertion.

## 8. Preemption, kernel locks, syscall and IRQ constraints

**A scheduler cannot preempt CPU instructions while kernel holds an arbitrarily long nonpreemptible lock or interrupts are disabled.** Thus it is incorrect to claim a bounded M0 recovery dispatch solely because the M0 thread is high priority.

- Kernel runqueue/switch/IRQ entry require tiny carefully scoped nonpreemptible regions with measured/audited worst-case assumptions, not unlimited "fast" algorithms. **M0 reservation is conditional on this upper bound: no scheduler can dispatch user recovery code while the CPU executing the scheduler is deadlocked, IRQL/IRQ masks are stuck, or NMI/firmware code monopolizes cores.** External watchdog/reset is a separate optional containment mechanism, not recovery completion.
- Long capability revocation trees (RFC-002), Channel queue drains (RFC-003), VM reverse-map/TLB shootdown (RFC-004) and service teardown (RFC-006) should be segmented into bounded, preemptible pieces with explicit resume/journaled metadata. Do not claim zero-cost segmentation.
- Lock order: scheduler runqueue locks, ObjectCap table, VM metadata, IPC queues, timer wheel and device IRQ cannot be held across arbitrary Pager IPC, copyout that may page-fault or arbitrary user callback.
- A reschedule IPI and cross-core Thread wake must not double-run the same Thread. CPU-local state and memory-order barriers require a finite state/lock proof.
- IRQ storm must be bounded by timely acknowledge/mask, device authority and finite work per interrupt; otherwise even fairly admitted application groups can receive no CPU.
- Emergency kernel cleanup time must itself be accounted and bounded, including failed allocation paths when ResourceGroup quota is exceeded. **CPU charge policy must distinguish unavoidable basic syscall/IRQ cleanup from attacker-amplifiable deferred work**: a group at zero quota may be prevented from submitting *new* work, yet kernel must retain a bounded reserve for denial, IRQ ack, cap teardown and cancellation. Unlimited accounting exemption opens denial-of-service.

**Pending signal vs scheduling:** Signals, Linux futex waits and guest syscalls are user-personality semantics (RFC-010). Native KABI has only typed Thread/wait/clock/control primitives after design approval, not an automatic implementation of Linux pthread scheduler flags.

### 8.1 Interrupt-context policy

Separate short top-half/IRQ handling, deferred work, normal driver user-space Task and the essential M0 timer/wakeup. The kernel may keep minimal interrupt ack/mask and object delivery while user driver does the device logic. A user-space driver that fails to run must not hold an unbounded CPU-time loan just because IRQ arrived; IRQ/event floods are resource-budget and hardware-isolation problems as well as scheduling problems.

## 9. M0 recovery scheduling contract — joint RFC-004/006

**P0 safety target:** If normal apps, Pager and Supervisor are CPU-heavy, blocked or memory-starved, and the kernel scheduler, IRQ timer and M0 fully resident path remain functioning, a bounded M0 failure handler is still **eligible and has reserved execution capacity** for a defined critical operation. This does **not** guarantee recovery if CPU halted, timer failed, a kernel spinlock deadlocked, firmware/NMI monopolized CPU or storage/DMA failure cannot be repaired.

### 9.1 Separate three reservations

1. **CPU capacity reservation:** explicitly admitted limited M0 runnable budget/service share; cannot be consumed by normal ResourceGroups; do not equate "high priority" with reservation.
2. **Memory and IPC recovery reserve:** wired CPU context, kernel stack, page tables, Channel/Reply/fault/error slots, cleanup refs and bounded log buffers (RFC-004/006). A runnable Thread must not fault into unavailable user Pager.
3. **Timer/IRQ progress reservation:** a post-UEFI clocksource/timer interrupt and minimal IRQ delivery/resched logic independent of disabled EFI Boot Services watchdog (RFC-006). The timer may stop in suspend depending on selected domain.

### 9.2 Admission and fail-safe condition

Propose M0 max runnable Thread count, max concurrent critical work and upper bound on per-operation kernel nonpreemptible segment, IRQ noise and available online cores. For any finite service bound claim, all of these must be specified and measured/modelled. If assumptions fail, report Degraded/Fatal or hardware watchdog and fail-stop; do not falsely assert recovery succeeded.

**Progress-contract tuple:** an M0 claim requires (i) core online and admissible CPU capacity, (ii) independent timer/IRQ delivery, (iii) a bound on maximum interrupts/masked latency/nonpreemptible critical sections, (iv) fully wired error/queue/memory credits, (v) a ready M0 worker and eligible reserved CPU, (vi) bounded steps in essential teardown, and (vii) no unresolved dependency on a throttled Pager/driver or another unavailable server. Only when all hold may any finite scheduling-to-handler claim even be investigated. Nonrecoverable device or kernel failure yields fail-stop/degrade, never verified deadline success.

A compromised M0 service can itself saturate its reserved budget and stall ordinary work. A critical class must have finite total share and runtime/period or equivalent bounds, with denial of untrusted admission. If cleanup requires CPU beyond reserved capacity, it either runs as separately authorized normal charged work when safe or fails boundedly; no arbitrary borrowing of other users' paid CPU.

### 9.3 Dependencies by failure mode

| Failure | Proposed progress if assumptions hold | What is NOT guaranteed |
|---|---|---|
| CPU-bound sandbox flood | M0 admitted runnable share; app Thread/group caps throttle | unconstrained guaranteed p99 GUI latency |
| Pager fault storm | M0 code/stack/IPC resident; kernel independently fails/cancels faults | file bytes or guest state preserved |
| normal Supervisor crash | RecoveryAuthority observes and can request fresh scoped launch | M0 itself automatically recoverable |
| full ordinary IPC queues | bounded recovery/error channel/credit independent | queued app-side request rolls back |
| kernel nonpreemptible deadlock | kernel may fail-stop/watchdog only | that M0 user Thread can run |
| failed timer or hardware IRQ | no scheduling deadline guarantee | hard real-time response |
| DMA driver cannot fence | isolate/quarantine device group and perhaps restart unrelated services | reclaim of pinned frame or device |
| reboot after repeated essential failure | persistent retry/rescue rules from RFC-006 | BootEpoch-only counter preventing reboot loops |

## 10. IPC scheduling, priority inversion and possible donation

RFC-003 IPC-A is an **asynchronous bounded Channel** with optional scoped ReplyToken; the kernel does not know which queued message belongs to what app-level service deadline or what remote side effect a user-space server has performed. Avoid silently converting every send into a synchronous scheduling donation.

### 10.1 Initial IPC candidate IPC-S0: server-owned CPU

Server Task uses its own admitted CPU ResourceGroup to dequeue bounded requests. Sender pays for its own send syscall and queue/buffer reservations, not the entire server CPU execution. A flooded low-trust client cannot automatically spend M0/Pager execution share simply by sending high-priority messages. Queue quota, producer credits and scheduling budget are separate admission gates.

**Dependency budget inversion case:** Client A owns CPU cap, sends IPC, waits. VFS Server B exhausted CPU quota while it must ask Pager C, and C requires storage IRQ deferred work D. A has remaining runtime, yet **none of B/C/D necessarily receives executable time**. Scheduling Client A higher cannot make the chain advance. For bootstrap-critical B/C/D, investigate independently admitted finite service capacity or conditional reserved error handling and enforce producer credits; no automatic transfer of caller budget via E1. Treat circular waits and scheduler throttling as separate necessary failure edges.

**Risk:** a client with ample CPU might wait on a server whose own group is throttled. This is a real liveness/latency limitation, not a justification for unaudited implicit donation. Investigate assigning adequate finite server group budget and admission, per-service request rate limits or explicit future donation only for proven paths.

### 10.2 Optional IPC-S1: scoped lease / limited priority inheritance

A future explicit donated budget lease could bind (client Task/group, server endpoint identity/generation, specific request nonce, authorized service scope, original charge owner, maximum time/priority, expiry, chain depth and return policy). It should never mean the server now owns the caller's capability to set arbitrary priorities or manage another Task.

- **No minted time:** donated runtime debits donor and any group ancestor's lawful reservation; simultaneous leases cannot spend same budget twice.
- **Terminal events:** ReplyToken consumed, deadline expiry, cancel, client/server exit, cap revoke, service restart and transport COMMITTED/UNKNOWN need independent lease terminal accounting; timeout alone cannot rollback completed external work.
- **No infinite chains:** server A blocking on B and B on C may transfer at most bounded authorized capacity across acyclic or cycle-detecting graph; ownership/priority restoration must not leak on failed reply.
- **Server fan-in:** one service processing many clients requires arbitration rather than one highest-priority attacker permanently overriding all other queued clients.
- **Passive server not free:** without own scheduling context, it can starve if no client lease arrives; crucial Pager, VFS bootstrap and M0 must not rely exclusively on arbitrary untrusted client donations.
- **Untrusted client cannot inflate M0:** critically reserved CPU time must not be loanable from general clients or converted to user-controlled donation. Deadline profile rights and donation admission separate.

### 10.3 Priority inheritance differs from CPU donation

In Linux RT-mutex a higher-priority Thread waiting on a lock can temporarily boost the **lock owner**; PI follows the known owner and nested chains [S7,S8]. That does not automatically transfer actual runtime/period quota, grant extra physical CPU, or resolve IPC messages with no kernel-visible lock owner. A FIFO Queue, multiple receivers and a faulting Pager do not necessarily have a single stable owner Thread to boost. Any PI implementation would require a typed owner/wait graph, bounded updates, cancellation and proof against spoofed dependencies.

**Lease warning from seL4 [S1]:** finite refill queue slots can cause forfeited usable budget after repeated blocks/preemption; a budget/period pair does not imply continuous service floor. UniOS future SchedulingContext must specify refill fragmentation, burst ceiling and donation chain reclamation rather than claim equivalent timing. An IPC ReplyToken returning late does not refund executed CPU time to client; only *unspent* eligible lease allowance can return under defined rules.

**P0 choice:** disable universal IPC donation/automatic PI. Keep SCHED-C opt-in explicit lease on hold until a formal compare with seL4 MCS and the chosen Channel ReplyToken contract. Desktop interactivity should first be approached with correct service budget, demand fairness and bounded queueing.


## 11. Deadline, timer, WaitSet and event ordering

A user-visible **deadline is an event wait/timeout condition**, not automatically an EDF admission request or a guarantee that a reply arrives in time. RFC-003 WaitSet W1 one-shot observer vs W2 persistent-level observer is not decided and must not be hidden in the clock API.

### 11.1 Time primitives research contract

- Clocksource: monotonic counter used to measure elapsed execution/wall time; check stable rate, counter wrap and per-core consistency.
- Clockevent: hardware local/global timer that raises an interrupt/request for scheduling and registered deadlines; different from the clocksource.
- DeadlineWait/Timer: scoped token with ClockID (suspend-sensitive or suspend-inclusive), target instant, optional slack/tolerance, generation and owner.
- Callback/completion: Woken/TimedOut/Cancelled/PeerDied/Replaced, not a universal exactly-once timer signal to application code.
- Wall clock (UTC/calendar) may jump due to synchronization, user policy or RTC; must NOT be the direct authority for monotonic timeout comparisons.
- A monotonic CPU scheduling clock is not the same as charged execution-time counter. A sleeping Thread's absolute deadline can expire while its executed CPU charge stays unchanged.

### 11.2 Race rules

1. Waiter registers before the readiness check linearization or uses an equivalent atomic lock/observer handshake, so ready signals are not lost between test and sleep.
2. If timeout and object readiness become true concurrently, define an atomic winner/status ordering; a woken Thread always rechecks the underlying condition and grant validity.
3. A timer cancellation may race with firing. It must retire/cancel a generation without waking a new Thread at the same numeric ID.
4. Deadline expiry while Thread is throttled records logical expiry/available status when scheduler can safely process it. It cannot promise actual user handler execution at the exact timestamp.
5. A server receiving a late reply after local client timeout must not turn expiration into remote-effect rollback. RFC-003 COMMITTED/UNKNOWN still applies.
6. Coalesced Pager fault waiters may share page content but cannot inherit each other's wait deadline or CPU eligibility.

### 11.3 Suspend, reset and time domains

Linux distinguishes CLOCK_MONOTONIC (does not count suspend) from boottime elapsed including suspend [S10]. UniOS should provide explicit clock-domain semantics for timeout, resource replenishment and lease expiration before implementing suspend/resume. Do not use wall clock to extend expired execution grants by changing date/time. Hibernation and migrating CPUs require reconstruction/validation of timer state and new boot or resume generation. If monotonic source is unstable on a given PC, disable affected guarantees or choose a validated fallback.

## 12. x86_64 timekeeping, preemption and CPU hardware

Candidate inputs for first x86_64 VM: invariant TSC when explicitly supported and calibrated, APIC/local timer/HPET or platform alternative if validated; firmware/ACPI per-CPU topology; reschedule IPIs and interrupt delivery. No chosen tick frequency, timer calibration, nanosecond precision, guaranteed TSC sync or fixed PCID/APIC mode is asserted.

- CPU frequency/power/thermal and virtualized clock offsets can distort naive cycle-count-to-nanosecond conversion. The clocksource needs calibration, overflow/wrap strategy, drift monitoring and suspend recovery.
- **Tickless idle** is a future power optimization only; it still requires correct nearest scheduler replenishment and deadline timer programming. A timer can fail to fire or be delayed by interrupt masking/hardware.
- Time-slice ending and a critical M0 deadline both need actual schedulable timer events. Firmware UEFI boot watchdog is **disabled after ExitBootServices** (RFC-006) and is not a permanent M0 availability mechanism.
- Kernel syscall accounting must be sensible at user→kernel→user boundaries without allowing a malicious app to hide repeated expensive kernel work. Traps/interrupt attribution require budget or explicit bounded system overhead.
- Counter overflow and rounding must be fail-safe: a huge user-provided period or negative duration cannot wrap into unbounded scheduling rights.
- Clock metadata needs a trust root and hardware assumption. Task-supplied timestamps are not verified CPU usage records.

**Separate hard assumptions:** scheduler can be correct with a stable clock but fail to meet a deadline because of long nonpreemptible MMU shootdown or device ISR. Temporal accuracy is an independent proof and hardware experiment.

## 13. SMP scheduling, CPU affinity and hotplug

### 13.1 Per-CPU runqueues and balance

Initial research direction: per-CPU ready queues, well-defined eligibility and CPU affinity scope, selected load balancing/work steal, inter-processor reschedule signals and a global or hierarchical group charge record.

- Runqueue state must satisfy a single-owner constraint: Thread is RUNNING on at most one CPU, RUNNABLE enqueued at most once, BLOCKED/THROTTLED not queued as runnable.
- Wake/migration changes queue ownership atomically with generation validation. A late IPI, timeout, CPU offline and Task stop must not resurrect a retired Thread.
- Task affinity requests are policy-constrained by valid currently online CPU set; guest affinity cannot override physical/critical CPU reservation. Invalid empty mask should return failure rather than trap a Thread in impossible runnable state.
- Per-CPU fair debt vs group global CPU budget must be reconciled; a malicious multi-CPU Thread farm cannot spend group quota independently on each CPU as if total quota were multiplied.
- On CPU offline/hotplug, move/reroute eligible Threads or explicitly fail the admission. **Pending deadline timers, local per-core budget debits and current running ownership must settle before CPU is removed. An admitted protected reserve may become infeasible; degrade/fail-stop rather than silently preserving its guarantee.** Pinned IRQ, M0 watchdog CPU and scheduler timer require more than simply migrating ordinary threads.
- QEMU virtual CPU oversubscription and host scheduling jitter invalidate hard timing claims even when guest kernel algorithm is conceptually correct.

### 13.2 CPU capacity profile and asymmetric hardware

D2 prioritizes x86_64 QEMU first plus actual Reference PC. CPU counts and simultaneous multi-threading capabilities are dynamic platform inputs. A later CPU could have heterogeneous capacity, SMT contention, variable thermals, NUMA and interrupt-affinity limits. Early policy should not promise strict utilization proof from "N logical CPUs" alone unless capacity assumptions are explicit.

### 13.3 Priority inversion across CPUs

Priority inheritance may need owner execution on another CPU, but a boosted priority that lacks runnable budget or whose owner is tied to an offline CPU cannot run. Dynamic affinity boosts or emergency CPU transfer need separate authority/feasibility proof. A blocked high-priority Thread waiting on unsafe DMA fence cannot be rescued by donation; hardware fence and resource dependency remain outstanding.

## 14. Realtime and media policy — deferred SCHED-B

SCHED-B studies combining general Fair with admitted Deadline/EDF/CBS-like jobs. Ordinary developer desktops may later need bounded audio processing, input event handling or video frame work, but a blanket hard realtime guarantee is **not justified**.

### 14.1 Admission variables

For each proposed deadline service i: execution runtime C_i, relative deadline D_i, minimum period T_i, online CPU/capacity domain, blocking bound B_i and affinity/device/IRQ dependencies. A simple feasibility criterion is C_i positive, deadline and period positive and C_i within D_i/T_i policy; global utilization under available CPU capacity is *necessary but not sufficient* in arbitrary SMP, lock, affinity and nonpreemptible conditions. Do not fix concrete percentage or milliseconds as proven.

- Critical M0 CPU reserve and kernel/IRQ overhead reduce admissible capacity. Ordinary desktop Fair work must retain a documented minimum eligibility where product policy requires it.
- Not all urgent tasks need kernel EDF: many soft deadlines can be handled by bounded fair groups and service-level queue priority.
- Never promote arbitrary app or fake service to Deadline by setting a priority number or passing a task name.
- In user space, a broken or malicious service can claim unrealistic WCET; the kernel can enforce caps but cannot guarantee useful work completed within the budget.
- If admission fails, return a specific denied/degraded condition; do not silently accept and assert hard deadline guarantees.

### 14.2 What is deferred

No unbounded SCHED_FIFO/SCHED_RR root privilege, scheduler-specific POSIX errno emulation, deadline task fork semantics, kernel priority queues in a user C ABI or priority donation across opaque async RPC. Realtime scheduling can be separately enabled after SMP/hardware/time and WCET gates.

## 15. Failure semantics and restart/revocation ownership

### 15.1 Expended budget is not revoked authority

Thread throttling stops CPU execution eligibility but does **not** close Task handles, invalidate MemoryObject mappings or cancel IPC queue content. RFC-002 R0/B/R1/R2-F/R2-O still governs those objects. Killing a Service Task ends that execution identity but may leave Pager/FaultWaiter, cap in-transit, CPU mapping and device DMA lifetimes.

A SchedulingPolicyGrant revoke must block **new policy changes** and remove/attenuate now-invalid inherited scheduling privileges through a defined barrier. It must not arbitrarily erase already spent CPU cycles, retroactively undo server actions or free other live objects. If policy grant scope disappears while Thread currently runs, kernel needs a safe rescheduling/requeue boundary and effective policy recomputation.

### 15.2 Service startup/restart budgeting

- New service identity is a fresh Task and SchedulerAdmission generation; no implicit copy of old high-priority group by Registry name alone.
- Parent ResourceGroup admission conserves total shares across restart. Simultaneously alive old zombie group and new service must not each spend full identical parent quota during overlap.
- Supervisor can request a new budget only through typed allowed grants and a compatible manifest (RFC-006). B/R1 teardown may proceed in separate reserved cleanup context even if Task CPU quota exhausted.
- M0 privileged recovery and DeviceGroup fences may require separately accounted kernel/IRQ time, especially when stop/kill is the action needed to release ordinary CPU and memory quotas.
- A race between optional DonationLease return and Service death cannot refund donor time twice or let a new incarnation inherit the old lease.

### 15.3 Distinguish status classes

Potential logical statuses, **not KABI error numbers**: CLASS_NOT_AUTHORIZED, INVALID_BUDGET, ADMISSION_DENIED, THROTTLED, DEADLINE_EXPIRED, WAIT_CANCELLED, THREAD_DIED, CPU_UNAVAILABLE, TIMER_UNAVAILABLE, SERVICE_RESTARTED, UNRECOVERABLE_KERNEL_DELAY, POLICY_REVOKED. They are not all synchronous syscall errors: a Thread can be throttled without one return code, and a timer may expire while it is blocked.

## 16. Failure and concurrency matrix

| Case | Forbidden result | Contract/potential response |
|---|---|---|
| App spawns 10k Threads | multi-Thread group mints 10k CPU quota | hierarchical charge and bounded admission |
| App repeatedly yields/wakes | negative fairness debt erased, others starve | scheduler eligibility preserves sleeper fairness state |
| User requests Deadline class without cap | high priority escalates from untrusted bytes | typed SchedulingPolicyGrant denial |
| Nested IPC calls with implicit donation | budget multiplied across services | no default donation; explicit lease only after proof |
| Caller timeout while server still works | server side-effect magically cancelled | wall deadline distinct from CPU and service commit |
| Two clients donate same budget object | server receives double CPU reservation | singular entitlement, conserved charge and caps |
| Server dies while using donated CPU | donor time never returned or minted twice | scoped lease lifecycle and generation |
| Passive Pager waits for client budget | critical page faults cannot progress | server-owned M0/Pager baseline or reserved credit |
| Kernel handles attacker syscall storm | unlimited uncharged privileged CPU | bounded or attributed kernel execution |
| IRQ flood from compromised device | recovery Thread never executes | bounded IRQ handling and mask/IRQ budget; hardware fallback |
| Long capability revoke with lock held | high-priority M0 cannot preempt | segmented, bounded critical sections and lock audit |
| IPC usercopy page fault under scheduler lock | Pager reentrancy deadlock | no faultable copy with forbidden scheduler locks |
| WaitSet ready and timeout race | missed wake or duplicate runnable Thread | atomic linearization and generation validation |
| Page supplied for 50 fault waiters | duplicate CPU charge or foreign wake | each waiter revalidated, actual runtime charged once per core |
| Guest restricted normal-mode exit | guest obtains fresh CPU budget | same Task/Thread group charge through transition |
| CPU offlined during M0 watchdog timer | timer lost and boot-good falsely reported | recalibrate/reroute or fail/degrade |
| TSC skew across CPU migration | deadline goes backward/infinite | validated time domain, monotonic normalization |
| Group quota exhausted before error path | OOM cleanup cannot run | protected finite cleanup and M0 reservation |
| M0 hangs under high priority | permanent starvation of ordinary desktop | finite M0 budget and fail-stop/watchdog |
| Driver DMA fence never returns | Thread boost releases hardware unsafely | quarantine DeviceGroup; CPU priority cannot replace fence |
| Old Task dies while new service launches | double counted/uncounted CPU share | restart admission and sponsor conservation |
| CLOCK_REALTIME moved back | timeout or lease never expires | monotonic domain used for elapsed deadlines |
| Suspend time not counted | lease/fault deadline semantics ambiguous | explicit suspend-sensitive vs inclusive ClockID |
| CPU capacity shrinks after admission | deadline guarantee incorrectly persists | re-admit/degrade and report changed assumptions |

## 17. Proposed invariants — SCHED-01…SCHED-45 (NOT proved)

| ID | Safety or bounded-liveness obligation |
|---|---|
| SCHED-01 | A Thread can execute on at most one CPU simultaneously and be on at most one runnable queue at a time. |
| SCHED-02 | Only authorized typed policy grants can raise class/weight/budget/affinity beyond a Task's inherited envelope. |
| SCHED-03 | Spawning additional Threads cannot multiply their ResourceGroup's aggregate CPU reservation. |
| SCHED-04 | **Guaranteed reservations** do not exceed admissible ancestor/global physical capacity; configured **maximum caps** may exceed parent capacity in aggregate while actual execution is parent-limited. |
| SCHED-05 | Actual executed CPU time and wall-clock wait/deadline time have separate accounting. |
| SCHED-06 | Untrusted repeated syscall/interrupt/guest transition cannot create free unbounded kernel CPU work. |
| SCHED-07 | Cap/refill/optional burst policy conserves **actual charged runtime across all cores**, with no alias/migration/restart extra consumption or silent guaranteed-reservation double sale. |
| SCHED-08 | A Thread with no eligible budget cannot gain new runnable CPU execution solely via wake signal, unless explicit protected M0 policy. |
| SCHED-09 | Priority/weight alone does not count as reserve/guaranteed deadline in any public contract. |
| SCHED-10 | M0 execution reservation is finite and cannot be spent/minted by normal app grants. |
| SCHED-11 | M0 liveness claims name online CPU, working timer, bounded kernel critical sections, IRQ progress and memory/IPC cut assumptions. |
| SCHED-12 | Failure of these assumptions results in explicit degraded/fail-stop policy, not a false success response. |
| SCHED-13 | Blocking on Wait/IPC/Pager does not charge unexecuted CPU time as actual Thread runtime. |
| SCHED-14 | IPC asynchronous SEND never automatically transfers SchedulingContext or high priority to arbitrary destination. |
| SCHED-15 | Any future DonationLease debits authorized donor and returns/settles bounded remaining time exactly once. |
| SCHED-16 | Donation across calls has finite chain/depth, scoped request generation, cycle and fan-in handling. |
| SCHED-17 | One client's high priority cannot mint additional M0/essential reserved runtime through untrusted Channel content. |
| SCHED-18 | Server CPU charging, queued IPC memory sponsorship and reply commit status remain distinct ledgers. |
| SCHED-19 | Waking/signal/timeout/cancel have a linearization that prevents lost wake and double dispatch. |
| SCHED-20 | PagerSupply causes kernel-per-waiter rights/generation check, not unconditional runnable state. |
| SCHED-21 | Terminated/revoked/old-generation Thread cannot be requeued or run under a fresh service incarnation. |
| SCHED-22 | CPU migration and online/offline transitions preserve runtime charges, caps and runnable identity. |
| SCHED-23 | Restricted guest→normal supervisor CPU execution doesn't reset native ResourceGroup budget. |
| SCHED-24 | Monotonic deadline/ClockID and wall clock adjustments cannot be conflated. |
| SCHED-25 | Suspend/inclusive timer semantics and lease expiry are explicit and generation-safe. |
| SCHED-26 | Kernel nonpreemptible regions and IRQ top halves are finite under stated environment or their liveness guarantee is explicitly unsupported. |
| SCHED-27 | No forbidden kernel lock is held across faultable usercopy, external Pager IPC or unbounded scheduler/cap teardown. |
| SCHED-28 | One user's workload/IRQ storm cannot consume emergency critical reserve without separate authorized admission. |
| SCHED-29 | Throttling/stopping a Thread doesn't imply cap/object R1/R2 teardown or data persistence. |
| SCHED-30 | Reaping/zombie CPU budgets and replacing a service do not duplicate live parent CPU reservation. |
| SCHED-31 | Device/IRQ completion and DMA fencing require hardware evidence; higher Thread priority is no substitute. |
| SCHED-32 | Scheduler failure cannot be reported as repaired merely because watchdog event was armed. |
| SCHED-33 | Realtime deadline admission includes CPU demand plus blocking/IRQ/affinity/online capacity conditions before any guarantee. |
| SCHED-34 | Fair scheduler promises long-term weighted eligibility, not arbitrary strict latency without bounded runnable demand. |
| SCHED-35 | Cross-CPU reschedule IPI, concurrent wake and hotplug cannot double-charge or run same Thread twice. |
| SCHED-36 | Scheduler policy, budget descriptor and ClockID are not accidentally fixed as a stable native or guest ABI. |
| SCHED-37 | Group scheduling weight, maximum cap, protected minimum reservation, and actual CPU runtime use four separate ledgers with explicit semantics. |
| SCHED-38 | Configured child cap overcommit is not incorrectly rejected as reservation overcommit; effective consumption still respects applicable ancestor caps. |
| SCHED-39 | Concurrent execution of one group on different CPU cores is charged as a **sum of CPU runtime**; no wall-time undercount and no double physical-charge across ancestor breakdowns. |
| SCHED-40 | Independent M0 dispatch requires finite limits on IRQ, kernel nonpreemptibility, timer/CPU availability and dependency path; reservation alone is not liveness. |
| SCHED-41 | Scheduler state decomposes Thread wait/stop, quota eligibility, and CPU ownership to avoid lost state during timeout, fault, throttling and wake races. |
| SCHED-42 | At exhausted ordinary quota, bounded deny/error/cleanup/IRQ-ack paths remain possible without unbounded exempt attacker CPU work. |
| SCHED-43 | Boot-essential Pager/IPC/storage CPU starvation is treated as a scoped admission/dependency problem; no assumed generic client-to-server budget transfer. |
| SCHED-44 | Protected CPU reservation, optional burst usage and fair borrowing of spare capacity remain distinct after CPU offline/affinity/SMT changes. |
| SCHED-45 | CPU-time replenishment records and future donation leases cannot be used to promise continuous service or refund already-executed time. |

All are proposed properties; specific hardware clock, memory-order, lock and fairness assumptions still require counterexample review, modeling and measurement. **No invariant is claimed proven, tested or implemented.**

## 18. Alternative study, likely costs and acceptance gates

| Approach | Pro | Cost and proof risk | What could reject it |
|---|---|---|---|
| **SCHED-A weighted fair + bounded M0 reserve** | relatively modest kernel enforcement, general desktop fairness, no automatic donation | group hierarchy, IRQ/kernel charging and recovery capacity bound | M0 starves/monopolizes or fairness cannot survive multi-core load |
| **SCHED-B fair + admitted EDF/CBS** | explicit time-sensitive media/driver jobs possible | WCET admission and IRQ/lock/hardware bounds; SMP complexity | no viable deadline validation for selected hardware |
| **SCHED-C seL4 MCS-like contexts/passive server** | explicit spendable budget and efficient synchronous RPC context semantics | UniOS async IPC and multi-client concurrent message/lifetime mismatch | correct bounded loan/return unreasonably complex or broad TCB |
| **SCHED-D user programmable policy with kernel failover** | workload adaptation without kernel policy rebuild | policy security/ABI, fallback switching races, code complexity | attacker-controlled policy causes deadlock or fallback loses recovery |
| **Bootstrap simple group RR** | smaller first-step model and easier invariants | interactivity and weighted fairness quality unknown | cannot meet desktop behavior without costly redesign |

**No accepted numeric choices:** timeslice, fair weight mapping, deadlines, IRQ limits, number of realtime classes, maximum donation depth, recovery reservation percentage, boost duration, and timer slack are left to quantitative study after hardware requirements. Do not turn invented example fractions into policy.

## 19. Proposed formal design work and future experiment program

### 19.1 Model checking before implementation

Model finite two-core and one-core profiles, 3 normal groups, RecoveryGroup, 2 concurrent server clients, one passive candidate, 1 PagerBinding+two FaultWaiters, bounded IPC/IRQ queue, donor budget token, one timer, one CPU online/offline transition, normal/restricted guest Task, a kernel critical lock and service generation.

Transitions: create/charge/refill/yield/block/wake/timeout/cancel, migrate, service crash/restart, cap revoke B, mapping R1, PagerSupply, interrupt storm, donation lease create/return/cancel and M0 watchdog expiry. Enumerate schedules where timer fires exactly when a waiter becomes ready and where all normal budget runs out while M0 must stop a dead Pager.

**Safety counterexamples:** same Thread RUNNING on 2 CPUs; double donation; boot reserve spent by a low-trust App; old generation wake; RESET of budget on syscall; overwritten timer generation; unsafe reallocation on driver death; priority change without cap.

**Liveness proof only under clear fairness/hardware assumptions:** admitted M0 work can reach a defined fail/cleanup terminal state if CPU, timer, IRQ and M0 wired resources are operational and maximum kernel nonpreemptible segments are bounded. Exclude indefinite corrupted hardware lock or firmware timer failures from a false bound.

Suggested abstract tools: TLA+/PlusCal/Alloy or equivalent, but none has been executed as part of RFC-005; proposing to model is not modeling.

### 19.2 Future performance and fault injection after separate Go

- Controlled QEMU single-core vs SMP preempt/wake latency, p50/p95/p99 and worst observed scheduling delay; distinguish host scheduler jitter.
- 1/10/100/10k Task Threads within and across 1/10/100 ResourceGroups, group cap conservation and fairness tail.
- syscall/IRQ storm accounting and bounded critical-section hold time during 1/100/10k cap revoke/VM shootdown/IPC queue cleanup.
- Two cores racing wake/timeout/cancel/service restart, Thread affinity/hotplug and PCID/TSC migration.
- Pager crash under memory pressure, supervisor M0 rescue runnable but normal IPC queue full, blocked IRQ driver and Quarantined DMA.
- Audio/visual frame workload with and without admitted deadline work to compare SCHED-A vs SCHED-B on equivalent security boundaries.
- Explicit optional donation experiments compare server-owned vs leased CPU budget for nested RPC, many-client fan-in and cancellation without making data durability claims.
- Reference PC TSC/HPET/APIC calibration, timer drift, suspend behavior and actual interrupt remapping/IOMMU hardware (separate RFC-007).
- Separate CPU counter attribution for native Task, kernel synchronous work and guest normal↔restricted transitions; no optimistic zero-cost kernel accounting.

Results must disclose physical/virtual hardware, CPU online count, IRQ load, power profile, clocksource, measured jitter and violated assumptions. **No such result presently exists for UniOS.**

## 20. Open issues O5-01…O5-39

| ID | Priority | Open design blocker | Related RFC |
|---|---|---|---|
| O5-01 | P0 | SCHED-A fair group policy + M0 reservation vs stronger contexts alternatives? | 001/005/006 |
| O5-02 | P0 | M0 reserve amount, runtime/period guarantee and total CPU capacity with offline core? | 004/005/006 |
| O5-03 | P0 | Kernel/IRQ/exception/spin busy work charging and source attacker budget? | 001/005/007 |
| O5-04 | P0 | Max nonpreemptible kernel/IRQ duration and bounded cap/IPC/VM cleanup? | 002/003/004/005 |
| O5-05 | P0 | CPU wake/block/throttle/stop single runqueue linearization under SMP? | 003/005/009 |
| O5-06 | P0 | Firmware watchdog disabled after ExitBootServices: verified OS clockevent/IRQ source? | 005/006 |
| O5-07 | P0 | Circular Pager/IPC/recovery wait-for graph including CPU/queue credits? | 003/004/005/006 |
| O5-08 | P0 | Fairness/CPU budgets under malicious Thread amplification and guest syscalls? | 005/009/010 |
| O5-09 | P0 | M0 itself running forever: finite cap and fail-stop after reserve consumption? | 005/006 |
| O5-10 | P0 | Realistic hardware clocksource, time unit, overflow and per-core skew correctness? | 005/007/009 |
| O5-11 | P0 | Scheduler policy/CPU entitlement cap rights and boot manifest admission trust? | 002/005/006 |
| O5-12 | P0 | Budget exhaustion while executing kernel cleanup/IRQ ack or holding lock? | 002/005/006 |
| O5-13 | P0 | Future request-specific donation vs default server-owned, ambiguous remote effects? | 003/005 |
| O5-14 | P1 | Fair algorithm exact virtual time/sleeper lag, work stealing and fairness tail? | 005/015 |
| O5-15 | P1 | Nested ResourceGroup weights vs caps, burst carryover and ancestor conservation? | 002/005 |
| O5-16 | P1 | WaitSet W1/W2 and monotonic timeout/suspend/ClockID contract? | 003/005/009 |
| O5-17 | P1 | Priority inversion with user-server IPC/futex ownership and capped PI chains? | 003/005/010 |
| O5-18 | P1 | Deadline SCHED-B WCET, D/T and blocking/IRQ admission on SMP? | 005/007 |
| O5-19 | P1 | CPU affinity, hyperthreading, hotplug and heterogeneous thermal capacities? | 005/007 |
| O5-20 | P1 | Guest normal↔restricted same Thread CPU charge and scheduler-state identity? | 001/005/010 |
| O5-21 | P1 | CPU charge during asynchronous server Pager work and fault coalescing? | 003/004/005 |
| O5-22 | P1 | Thread/task restart budget reclaim, zombie objects and sponsor transfer? | 002/005/006 |
| O5-23 | P1 | Timer cancellation/reply timeout cross-CPU ordering and stale wake prevention? | 003/005 |
| O5-24 | P1 | IRQ/device storm mitigation, mask/ack, budget and driver crash? | 005/007 |
| O5-25 | P1 | Stable Native KABI clock/time/group interfaces and version evolution? | 005/009/018 |
| O5-26 | P1 | M0/normal policy-group fairness under congestion and power management? | 005/006 |
| O5-27 | P1 | Policy grant revoke: active boost/class reset vs group throttling and B/R1? | 002/005 |
| O5-28 | P2 | Scheduling plug-in/failover, real-time media improvements after design gates? | 005/015 |
| O5-29 | P2 | Energy/thermal QoS, advanced NUMA and big.LITTLE equivalent future platforms? | 005/007/013 |
| O5-30 | P0 | Precise weight/cap/guaranteed-minimum/resident CPU runtime ledger, hierarchy and numeric units? | 002/005/009 |
| O5-31 | P0 | Child max cap overcommit vs non-overcommittable reserved-min admission incl. burst/spare borrowing? | 005/006 |
| O5-32 | P0 | SMP per-core runtime delta and ancestor atomic charge: simultaneous concurrency without overspend? | 001/005 |
| O5-33 | P0 | M0 finite service floor vs IRQ/NMI/locks/timer failures and independent watchdog fail-stop? | 005/006/007 |
| O5-34 | P0 | Kernel/IRQ basic error/cleanup at zero quota vs attacker-amplified uncharged work? | 002/003/005 |
| O5-35 | P0 | Pager/IPC/driver budget inversion with queued work + fault worker while clients retain CPU? | 003/004/005/006 |
| O5-36 | P0 | Orthogonal wait/wake/fault/throttle/stop/CPU ownership state machine and wake generation? | 003/004/005 |
| O5-37 | P1 | CPU hotplug/SMT/thermal capacity changes after protected CPU admission, re-admission policy? | 005/007 |
| O5-38 | P1 | Replenishment chunk fragmentation and priority/wall deadline versus actual guaranteed execution? | 003/005 |
| O5-39 | P1 | CPU reserve budget propagation and validation after recovered service restart and R1/R2-F overlap? | 002/005/006/007 |

## 21. Decision proposals SCHED-D01…SCHED-D23 — NONE Accepted

- **SCHED-D01:** SCHED-A preemptive hierarchical weighted fair scheduling plus finite separately controlled M0 recovery service reservation is first research baseline.
- **SCHED-D02:** SCHED-B admitted EDF/CBS, SCHED-C MCS-like contexts, and SCHED-D pluggable policy remain comparison alternatives.
- **SCHED-D03:** Native kernel enforces actual CPU dispatch, typed policy grants, scheduling/group accounting and M0 eligibility; user service policy is bounded.
- **SCHED-D04 [revised]:** Creating extra Threads cannot mint minimum reservation or evade effective cap. Concurrent CPU runtime is aggregated across cores; maximum child caps can be configured over parent capacity, whereas guaranteed minimum reservations cannot be double-committed.
- **SCHED-D05 [revised]:** Weight, hard cap, protected minimum reservation, actual executed CPU-time, wall deadline and privilege to configure these are separate contracts; do not sell a maximum cap as a minimum guarantee.
- **SCHED-D06:** Async Channel uses server-owned budget by default; no automatic priority or SchedulingContext donation for IPC SEND/Reply.
- **SCHED-D07:** Explicit leased budget donation, passive servers and PI are deferred pending scope, fan-in, multi-hop cycles, timeout, return/death proofs.
- **SCHED-D08:** Kernel blocking/IRQ/fault/critical cleanup paths must be bounded and correctly charged or protected by a limited emergency envelope.
- **SCHED-D09:** Thread state transitions/wake/stop/throttle and CPU migration require one-owner runqueue and generation invariants.
- **SCHED-D10:** Monotonic clock and timer domain distinguish wall time, charged CPU runtime, suspend-sensitive vs inclusive deadline.
- **SCHED-D11:** No post-UEFI watchdog promise based on disabled EFI Boot Services; timer/IRQ/hardware clocks are explicit feasibility gates.
- **SCHED-D12:** M0 runnable reservation, IPC/memory wired cut and IRQ/timer progress are separate co-required recovery conditions.
- **SCHED-D13:** Real-time deadline claims require explicit WCET/blocking/IRQ/affinity admission and selected hardware validation; no general hard RT claim.
- **SCHED-D14:** A throttled/exited Thread does not imply cap or MemoryObject reclaim; RFC-002 B/R1/R2-F/R2-O stay separate.
- **SCHED-D15:** Restricted guest/normal supervisor execution retains scoped Task budget and cannot mint priority via guest ABI.
- **SCHED-D16:** Do not freeze numeric policy layout, timeslice, ClockID or realtime ABI before RFC-009/018 and hardware profile.
- **SCHED-D17:** Correctness, fairness and recovery bounds must be modeled and tested before any accepted SCHED policy or implementation authorization.
- **SCHED-D18 [new]:** SCHED-A accounting has independent weight, overcommittable maximum child caps, admission-controlled minimum guarantees and physical executed-time charge; all still Proposed.
- **SCHED-D19 [new]:** SMP aggregate CPU execution is sum of concurrent per-core runtime; ancestor accounting is inclusive but never physically double-counted.
- **SCHED-D20 [new]:** M0 CPU guarantee is conditional on bounded nonpreemptible/IRQ/timer plus wired IPC/memory and dependency graph; failure falls back to explicitly allowed safe-stop, not an invented deadline.
- **SCHED-D21 [new]:** Blocked/fault/throttled/wake/CPU-ownership are orthogonal lifecycle dimensions and need model-checked generation/linearization rules.
- **SCHED-D22 [new]:** At exhausted ordinary budget, bounded kernel error/cleanup/IRQ paths remain charge-protected without unlimited exempt work; server-owned Pager time requires independent admission and producer caps.
- **SCHED-D23 [new]:** CPU offline/capacity changes and seL4-like replenishment fragmentation are explicit feasibility questions, not grounds to claim a constant guaranteed service floor.

**Decision status:** Proposed only. None changes architecture D1–D8 or converts RFC-001/002/003/004/006 Proposed research decisions into accepted technical spec.

## 22. Adjacent RFC handoff and key contradictions

| RFC | Required shared test/contract |
|---|---|
| RFC-001 | kernel Thread/context switch, IRQ/timer KABI vs user policy/TCB split; restricted guest preemption |
| RFC-002 | ResourceGroup sponsor and time budget hierarchy, typed scheduling grant, bounded preemptible revocation fan-out |
| RFC-003 | async E1 and ReplyToken, WaitSet clock deadlines, shared queue fairness, no implicit IPC donation |
| RFC-004 | coalesced Pager FaultWaiters, usercopy slow path, clean refill and OOM critical CPU/memory reserve |
| RFC-006 | watchdog independence from UEFI, M0 RA runnable admission, finite restart vs RA own death and fail-stop |
| RFC-007 | IRQ storm/mask, DMA active driver, DeviceGroup quarantine, CPU/timer physical platform |
| RFC-008 | CPU not a storage durability promise; blocked writeback vs shutdown deadlines |
| RFC-009 | native clock/Thread/Scheduler grant call semantics, usercopy validity and ABI evolution |
| RFC-010 | Linux sched/affinity/futex/guest syscall, no guest priority escalation, G-R same-thread accounting |
| RFC-013 | graphical interactive workloads, frame deadlines and power/latency preference hypotheses |
| RFC-015 | security resource-exhaustion fuzz, fair-share/starvation and scheduling model proof goals |
| RFC-016 | user/admin consent to privileged scheduling and app policy grant lifecycle |
| RFC-018 | versioned clock IDs/units, group policy, native vs guest priority/errno and time type safety |

**P0 integration cluster:** (003+005) IPC/Reply deadline and CPU budget; (003+004+005+006) runtime Pager/Recovery wait-for CPU/IPC/IRQ resource cut; (002+005) sponsor and scheduling caps; (005+007) real IRQ/timer hardware; (001+005+010) guest restricted execution charging. These are not independently solved by naming a Priority field.

## 23. Design exit gates and next research

**R1 Scheduling design gate FAIL/OPEN** (v0.2 adds explicit cap-versus-reservation and physical run accounting blockers), until:
1. A complete Thread/block/throttle/wake/stop transition model with exactly-once CPU run ownership for 1 and multiple cores, including service restart and generation change.
2. A concrete typed group/priority/clock policy authority graph and proof of no CPU entitlement duplication through cap copy, Thread creation, migration, IPC and service restart.
3. A finite M0 runtime/IRQ/memory/IPC recovery cut with no self-Pager/CPU wait cycle, independently operable after ExitBootServices and with clear fail-stop conditions when kernel/timer broken.
4. A justified accounting policy for kernel syscall work, deferred IRQ/driver CPU, Pager Server work and coalesced faults that limits attacker-amplifiable uncharged CPU cost.
5. Timeout/wakeup and WaitSet race contract jointly with RFC-003, without promising remote cancellation or exact delivery timestamp.
6. Competing SCHED-A/B/C/D comparison on equivalent workloads and security/availability assumptions, with no invented latency targets.
7. Real x86_64 timer/CPU/IRQ feature gate and clock profile description from selected VM/Reference PC (research specification now, tests after Go).

**Recommended next technical research:** RFC-007 User-space Drivers, DMA & IOMMU, since IRQ control and hardware fence are necessary to make scheduling and the M0 recovery claim realistic. RFC-008 Storage and RFC-017 Verified Boot/Update resolve related reboot, boot-good and dirty-data facts. Re-review RFC-005 before declaring it Accepted.

## 24. Official first-party technical references (checked 2026-10-24)

- **[S1] seL4 MCS scheduling contexts, passive servers and timeout:** https://docs.sel4.systems/Tutorials/mcs.html
- **[S2] seL4 API SchedulingContext / MCS:** https://docs.sel4.systems/projects/sel4/api-doc.html
- **[S3] Fuchsia Zircon scheduler overview:** https://fuchsia.dev/fuchsia-src/concepts/kernel/kernel_scheduling
- **[S4] Fuchsia Zircon weighted fair scheduler:** https://fuchsia.dev/fuchsia-src/concepts/kernel/fair_scheduler
- **[S5] Linux EEVDF scheduler:** https://docs.kernel.org/scheduler/sched-eevdf.html
- **[S6] Linux SCHED_DEADLINE and CBS/EDF:** https://docs.kernel.org/scheduler/sched-deadline.html
- **[S7] Linux PREEMPT_RT theory:** https://docs.kernel.org/core-api/real-time/theory.html
- **[S8] Linux RT-mutex priority inheritance:** https://docs.kernel.org/locking/rt-mutex-design.html
- **[S9] Linux clocksources and clockevents:** https://docs.kernel.org/timers/timekeeping.html
- **[S10] Linux ktime get/boottime/real:** https://docs.kernel.org/core-api/timekeeping.html
- **[S11] Linux sched_ext programmable scheduler and fallback:** https://docs.kernel.org/scheduler/sched-ext.html
- **[S12] Linux cgroup v2 hierarchy, CPU weight/cap/burst and distribution models:** https://docs.kernel.org/admin-guide/cgroup-v2.html
- **[S13] Linux capacity-aware scheduling, CPU IRQ loss/thermal DVFS:** https://docs.kernel.org/scheduler/sched-capacity.html

## 25. v0.2 Adversarial re-review — counterexamples and design gates (2026-10-24)

This is a documentation-only technical criticism of RFC-005 v0.1, cross-checked with RFC-001/002/003/004/006 and official seL4/Fuchsia/Linux kernel documentation [S1–S13]. **No code, models, static analysis, proof, scheduling simulation, interrupts, benchmarks or kernel tests were executed**. An algorithm family remaining plausible is not a finding of optimality.

### 25.1 Corrected conceptual mistakes and threats

| Finding | v0.1 ambiguity / adversarial example | v0.2 correction | Evidence needed |
|---|---|---|---|
| Maximum CPU cap vs guaranteed reservation | Two child groups each have a maximum 80% CPU limit but share a parent 100% group. Rejecting their config because 80+80>100 incorrectly treats independent *maximum ceilings* as simultaneously guaranteed reservations. | Four ledgers: fair weight, overcommittable maximum cap, non-overcommittable protected minimum and actual runtime. Effective ancestor limit is enforced at execution, not by summing every configured maximum. cgroup v2 distinguishes weights, limits and allocations [S12]. | explicit bound/window accounting model and sibling contention work-conservation |
| CPU runtime vs elapsed walltime | On two cores, group runs 1 ms on each simultaneously. If global group is charged only 1 ms walltime, it consumes **2 CPU-ms** while charged for one. | Sum exclusive per-core running intervals; charge correct group and ancestors without physically double-counting inclusive counters. CPU-ms still not normalized completed work on heterogenous or SMT hardware. | SMP migration/IRQ/cycle-vs-time counters, atomic global cap debit |
| Reservation does not schedule blocked code | RecoveryAuthority has booked runtime but its worker waits on pageable stack, an empty channel credit, nonrunning Pager or a kernel lock held with interrupts masked. | Bound **CPU+IRQ+timer+nonpreemptible critical sections+wired memory/IPC+dependency graph** jointly. A scheduling reservation alone proves neither readiness nor response deadline. | simultaneous worst-case M0 cut and fail-stop assumptions RFC-003/004/005/006 |
| Missing server budget causes distributed starvation | Client with remaining budget sends to throttled VFS; VFS needs Pager; Pager needs storage driver. No IPC peer necessarily runs though client is healthy. | Server-owned budget is still the default. Admit finite essential service progress and bounded producer queue/error credits; SCHED-C lease not automatically enabled. | wait-for edges with budget-exhausted servers, coalesced fault and ordinary queue full |
| Kernel work exemption is exploitable | Process at zero CPU quota issues repeated expensive syscalls, faults and IRQ-generating requests; unlimited privileged cleanup execution bypasses group limit. | Define attribution for synchronous kernel work and bounded exceptional denial/cleanup/IRQ charge. No universal free work and no impossible claim all external IRQ is attributable. | adversarial syscall/IRQ storm and kernel lock latency |
| SMP entitlement races | Each core locally sees last 1 CPU-ms and admits simultaneous 1 CPU-ms, allowing parent cap overspend before cached state reconciles. | Use atomic or reserved chunk model with bounded aggregate overshoot and explicit enforcement; no refill on migration/context transition. An unacceptable overshoot is a failing proof gate. | concurrent charge/refill/refund/CPU-offline race state model |
| Thread state orthogonality | FaultBlocked Thread's quota expires and timer fires; one flat enum may drop fault wait or requeue illegally. | Represent thread wait/stop state, budget eligibility and one-CPU running ownership as orthogonal predicates with atomic wake/ready generations. | timeout vs PagerSupply vs kill and CPU hotplug counterexamples |
| Replenishment is not a continuous entitlement | Sporadic scheduling context with finite replenishment entries can forfeit usable runtime when queue fills, even if configured C/T seems sufficient. | Distinguish maximum window enforcement vs lower service guarantee. seL4 MCS [S1] is temporal-isolation precedent, not proof of UniOS M0 response service. | refill fragmentation, burst, affinity and repeated block testing |
| Source and capacity drift | "N CPUs" treated as N equal execution capacities; thermal/SMT/hotplug/IRQ load can defeat admission. | Capacity-aware *eligible online domain* and reduced-capacity re-admission; no hard bound from logical-core count. [S13] | Reference PC clock/IRQ/SMT/thermal profile |
| Unconditional highest-priority recovery | M0 thread boosted to absolute priority but deadlock holds scheduler raw lock, timer IRQ absent, or M0 loops forever. | Bounded M0 reservation and self-budget, explicit kernel/hardware failure branch, watchdog not credited as successful recovery; no fixed deadline claim. PREEMPT_RT [S7] still has low-level exceptions. | kernel critical-section WCET evidence and fail-stop path |

### 25.2 Proposed ledger/state decomposition

| Ledger | What changes it | What it can and cannot promise |
|---|---|---|
| Fair scheduling weight | typed scheduling-policy grant | relative CPU share among competing runnable/eligible groups, *not* a fixed minimum |
| Hard maximum cap | configured window/burst and actual charged per-core runtime | no more than permitted execution when ancestor/global policy enforces it, *not* a service reservation; child ceilings may sum above parent |
| Protected minimum reservation | trusted capacity domain admission and independent M0 grant | candidate amount of service if runnable and all hardware/blocking prerequisites hold; not an unconditional completion-time SLA |
| Executed CPU-time counter | every actual CPU execution interval and eligible kernel charge | resource usage sum across CPUs, not wall-clock delay or units of completed instructions |

A task's wait condition, scheduling eligibility and CPU running ownership are **orthogonal**; a forced-kill transition must revoke executable eligibility and invalidate subsequent wake by generation even if a concurrent page supply logically completes. The actual budget sponsor may differ from Task owner only under explicit future authorization; changing a Task name, entering restricted guest code or restarting a service does not reset accounting rights.

### 25.3 Bounded M0 proof checklist

To say "M0 can progress" the RFC-003+004+005+006 evidence must bound or explicitly rule out: CPU online capacity; a correct monotonic clocksource; post-UEFI clockevent; timer/IRQ delivery; IRQ storm and critical-section nonpreemption; scheduler runqueue wake; a charged M0 group; wired code/stack/page tables; IPC/reply/fault/error queue credits; lower-layer driver/storage dependency; allocator and cap/VM cleanup cost; and failed root/normal Supervisor states. If any indispensable dependency has no path, outcome is degraded or fail-stop, not silently recovered.

### 25.4 Prioritized additional design tests (future only)

1. A 2-core model with 2 siblings at maximum caps whose **sum exceeds parent** but with separately admitted minimum reservations; verify actual parent CPU debit and productive reuse of unused capacity.
2. 2 CPU cores concurrently running the same group, same global last quota, quota replenish during migration, then Task death/restart; prove no unauthorized overspend.
3. Blocked Pager client, throttled VFS/Pager workers, saturated ordinary channels and independent wired M0 error credits; prove no scheduler-only "high priority" deadlock.
4. Timer expires while waiter is FaultBlocked and Throttled; PageSupply, revoke B, Task kill and CPU migration race; prove one CPU owner and no stale resume.
5. Failure of post-ExitBootServices clockevent or indefinite nonpreemptible lock: distinguish inability to run M0, optional watchdog reset, and actual recovered state without inventing a bounded dispatch.
6. Untrusted zero-budget app generating syscalls/IRQs/errors: prove restricted forced-kernel overhead plus bounded safe cleanup/denial, or name denied hardware profile.
7. Process/service restart with independent capability ancestor and CPU group billing: no inherited undocumented reserved CPU rights and no duplicated reservation during overlapping old/new processes.
8. Contrast SCHED-A and SCHED-C on **equivalent async IPC** reply/timeout/queue semantics and total cost, not only seL4 synchronous fast-path numbers.

### 25.5 Revised status and constraints

**SCHED-A remains the first conditional research hypothesis** because weighted fair + bounded critical reservation is an understandable desktop-oriented architecture. It has **not** been shown faster, simpler in LOC, more secure or more available than SCHED-B/C/D, and guaranteed minimum service remains blocked on an explicit admission/proof contract. There is no selected algorithm, numeric slice, cap, minimum, time unit, clocksource, ABI or production hardware target beyond the architecture's x86_64-first direction.

D1–D8 are accepted at top-level design only. All SCHED-D01…D23 remain **Proposed**. RFC-005 v0.2 must not be confused with authorization to implement, or with passing any formal/physical design gate.

**Evidence and legal limitation:** These are cited as technical references for other independently implemented systems, not copied source code or proof of UniOS safety/performance. Verify future hardware and source versions before selecting a production policy. No third-party code imported.

---

**Document history:** 2026-10-24 v0.1 original design retained in Git; 2026-10-24 v0.2 adversarial review corrected CPU cap/reservation overcommit, SMP accounting, M0 progress conditions, server budget inversion and lifecycle modeling. No scheduler, driver, kernel code, benchmarks or formal proofs.

## 2026-10-25 CAPIPC KR-M2 concurrency and M0 progress boundary — Proposed

[CAPIPC Cross-RFC Safety Case v0.1](../reviews/capipc-cross-rfc-safety-case-v0.1.md) adds design-only admission-liveness and bounded-cost dependencies. Task-global C/B can reject future q beyond W when a lower q remains unadmitted; scheduler fairness alone does not fill a missing request or authorize SKIP. Kernel protected A/X/K-S/K-I critical sections, query/GC work and sponsor-charged cleanup need finite non-starving CPU/IRQ/lock bounds and independent M0 capacity. No runtime scheduling measurements, fairness proof, policy selection or implementation has been performed.

## 2026-10-26 CAPIPC joint A/C/E and M0 publication-progress study — Proposed

**Non-normative link:** [CAPIPC Joint A/C/E Semantic-Cut Resolution v0.1](../models/capipc-joint-semantic-cut-resolution-a-c-e-v0.1.md). Admission A(q), protected whole-grant K-S/K-I and Pager-faultable Vm/Vp/Vt must have bounded CPU/IPC/IRQ/lock/Claim/sponsor demands without borrowing M0 recovery credits. A finite KR-M2 W cannot guarantee progress through a permanent low-q hole. A publication-in-flight fence during Endpoint/issuer audience handoff creates a new conditional liveness question if the old publisher is pager-stalled: fairness alone cannot release a held publication reference. Bounded publication chunks, alternative narrowed receive ABI and honest fail-stop remain paper-only research, not scheduling implementation, model proof, benchmark or approved policy.

## 2026-10-26 Publication quiescence versus scheduler and M0 fairness — Proposed

[CAPIPC Publication Barrier & Pager-Safe Copyout Feasibility v0.1](../models/capipc-publication-barrier-pager-safe-copyout-feasibility-v0.1.md) distinguishes issuer-scoped P-DENY (new old-authority publication permits blocked) from P-DRAIN (all old publishers fenced). If a publication permit may Pager-fault indefinitely, scheduler fairness alone cannot bound handoff drain. A claimed bounded non-faulting slice must specify CPU/preemption/IRQ critical length, max active publisher count and independent M0 CPU/frame/IPC/cleanup reserves. Register-only small IPC remains a narrower fallback. This is only a conditional progress target, with no runtime numbers, admitted RT policy, model checks or code.

## 2026-10-26 CAPIPC paired mapping/publication and bounded drain — Proposed

[CAPIPC Destination Memory & Publication Barrier Cross-RFC Decision Table v0.1](../reviews/capipc-destination-memory-publication-cross-rfc-decision-table-v0.1.md) distinguishes VM mapping M-DENY/M-DRAIN from issuer P-DENY/P-DRAIN during bounded IPC publication. A nonfaulting finite-byte writer may still be preempted or lose IRQ/CPU eligibility indefinitely; strong handoff and mapping reuse cannot report DRAIN without actual Finish/Fence, bounded scheduler/CPU/IRQ/critical-work assumptions and M0 resident credits. Register ReturnRelease is a smaller comparator, not a proven real-time return path. All OPEN/Proposed; no chosen timeslice, IRQ bound, model or kernel implementation.

## 2026-10-26 Paired publication permit: preemption and drain progress — Proposed

[CAPIPC Paired Mapping–Publication Cut & Invalidation Ordering v0.1](../models/capipc-paired-mapping-publication-invalidation-ordering-v0.1.md) distinguishes finite byte count/no-Pager work from **bounded actual CPU/IRQ/preemption completion**. A joined (M,P) active writer may remain preempted while M-DRAIN/P-DRAIN is pending; scheduler fairness alone cannot promise strong finite quiescence without health/admission assumptions. A naive two-ticket M/P wait can deadlock under opposite lock ordering. Small register-only ReturnRelease removes output user VA but still needs issuer/Task-generation cut. Wired independent M0 CPU/IRQ/IPC/cleanup reserves remain a Proposed proof target; no timings, real-time class, test or code accepted.

## 2026-10-26 L1 single-BSP no-scheduler CPU exception scope — Proposed

[UniOS Minimal Boot, Exception & Memory MVP Contract v0.1](../contracts/unios-minimal-boot-exception-memory-mvp-contract-v0.1.md) scopes first proposed QEMU boot code to one BSP and interrupts masked, no scheduler, Thread preemption, SMP, CPU donation, native Task/IRQ service or user mode. It still requires defined exception CPU state, GDT/IDT/TSS/DF IST, halt/fail-stop and CPU feature detection; NMI/unrecoverable triple fault may not produce a reliable serial trace. Later RFC-005 scheduler/M0 budgets are **not** implied validated by this initial scope. L1 implementation approval is currently **NO-GO**; design-only.

## 2026-10-26 L1 IF-masked CPU/exception entry and ABIs — Proposed

The [L1 UEFI Handoff & Kernel Entry Microcontract v0.1](../contracts/unios-l1-uefi-handoff-kernel-entry-microcontract-v0.1.md) separates UEFI firmware calling convention from bare-metal System V kernel `extern "C"`, with explicit kernel RSP alignment, no red zone, DF cleared, IF masked, valid GDT/IDT/TSS and double-fault IST through an early CR3 switch. Even with IF=0, NMI/synchronous exceptions may invalidate an unsupported interim handler; no universal serial trace is promised. BSP-only, no scheduler/timer/SMT/CPU budget or SMP implemented. H-O03 exact entry/exception context is still an L1 safety design blocker.

# UniOS RFC-005 — Scheduling, Preemption & CPU Budget

> **Status:** Proposed — Specification v0.1 (research draft; not Accepted, implemented, benchmarked, or formally verified)
> **Date:** 2026-10-24 (Pacific/Auckland)
> **Architecture:** [UniOS v0.2](../architecture/unios-architecture-v0.2.md)
> **Cross-RFC contracts:** [RFC-001 Kernel/TCB](rfc-001-kernel-boundary-and-tcb.md), [RFC-002 Capabilities](rfc-002-object-capability-lifetime-delegation-revocation.md), [RFC-003 IPC](rfc-003-ipc-wire-protocol-and-synchronization.md), [RFC-004 VM/Pager](rfc-004-virtual-memory-pager-cache-and-oom.md), [RFC-006 Boot/Recovery](rfc-006-boot-trust-chain-service-startup-and-recovery.md).
> **Out of scope:** kernel code, running scheduler, selected numeric timer period/slice/priority, native ABI freeze, hard realtime guarantee, benchmark or proof.

## 0. Executive recommendation: SCHED-A as a conditional research baseline

**SCHED-A:** per-logical-CPU preemptive, kernel-enforced **weighted-fair general scheduling** + explicit Task/ResourceGroup CPU-budget charging and admission + a separately bounded **M0 essential recovery reservation/class** with time and IRQ resources. Local runqueues and SMP work distribution are candidates; no final algorithm or queue data structure is adopted.

The intent is a practical first x86_64 desktop OS, not an arbitrary hard-realtime system. Prefer a **small trustworthy scheduler enforcement mechanism** over adding a complex all-powerful user scheduler or assuming highest-priority threads will make progress. Scheduler policy/admission is user-space only to the extent that kernel still enforces CPU usage, protected recovery resource eligibility and safe preemption.

**Important not-yet-adopted choices:**
- SCHED-B: kernel hybrid weighted fair + EDF/CBS-style admitted deadline scheduling for a bounded subsystem (audio/input/video or drivers in future).
- SCHED-C: seL4 MCS-like explicit SchedulingContext/replenishment/passive servers with request-scoped budget transfer, subject to huge capability/lifetime and async-IPC complexity.
- SCHED-D: user-space configurable policy/plug-in scheduler with a permanently safe kernel fallback, considered after M0 correctness. This need not entail a general eBPF-like runtime.
- **M0 recovery CPU time is not free, infinite, or magic.** A critical reservation must have finite CPU/IRQ runnability resources and an isolation plan, and recovery cannot work if the CPU/clock/interrupt mechanism itself fails.
- **No automatic budget/priority donation on Channel send.** RFC-003 proposes bounded asynchronous Channel E1 endpoints plus a scoped optional ReplyToken; there is no kernel-defined synchronous server execution owner in ordinary async send. Passive server donation from seL4 does not directly fit without a new explicit request token and lifecycle.

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
| seL4 MCS tutorial [S1] | SchedulingContext has budget/period and controlled replenishments; passive servers can execute using a caller's scheduling context on seL4_Call/ReplyRecv | SCHED-C comparison, but async Channel sends/queued jobs need a **different explicit handoff contract** |
| seL4 Scheduling API [S2] | scheduling context objects can bind to Thread or Notification; rights control scheduling authority | scheduling control is typed and separate from mere Thread handle |
| Fuchsia Zircon Scheduler Overview [S3] | per-CPU scheduler has Fair and Deadline disciplines, IPIs coordinate cores | SCHED-A/B study; Fuchsia product behavior is not a UniOS guarantee |
| Fuchsia Fair Scheduler [S4] | general-purpose Threads receive weighted CPU proportion across competition | weighted fair baseline, not a fixed maximum wait guarantee |
| Linux EEVDF [S5] | eligible tasks with positive/zero lag and virtual deadlines balance fairness and short-slice responsiveness | fair policy evolution matters; do not freeze CFS or clone EEVDF as an assumed simple primitive |
| Linux SCHED_DEADLINE [S6] | EDF plus Constant Bandwidth Server, runtime/deadline/period and admission controls | SCHED-B needs admission and WCET/blocking assumptions to claim deadline behavior |
| Linux PREEMPT_RT [S7] | most IRQs threaded; rtmutex PI helps many lock paths, but core scheduler/entry remain critical exceptions | blocking/nonpreemptible kernel sections, not priority alone, dominate worst-case latency |
| Linux RT-mutex design [S8] | owner-linked PI and nested chain propagation required to prevent unbounded priority inversion | user mutex PI/IPC donation/CPU transfer must not be conflated |
| Linux timekeeping [S9,S10] | clocksources and clockevents distinct; monotonic timeline can stop during suspend; boottime vs monotonic differ | expose exact clock domain/deadline/timer meaning; no wallclock deadline assumption |
| Linux sched_ext [S11] | optional programmable scheduling class with kernel-controlled fallback on errors/stalls | consider only after stable safe kernel scheduler; do not adopt code or require dynamic BPF in kernel |

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

A ResourceGroup may own many Threads. A user creating 100 Threads inside the same group must not multiply its aggregate CPU reservation by 100. Likewise two aliases to one SchedulingContext must not represent twice the original CPU runtime. Effective budget rights must be attenuated and ancestor/global admission enforced. Runnable Thread may be charged to a group different from process creator only through explicit authorized service attribution, not hidden donation.

## 5. SCHED-A general fair scheduling candidate

### 5.1 Weighted fair vs strict realtime

One possible initial model: each logical CPU holds a scheduler runqueue with weighted-fair eligible Threads (or groups with nested fair scheduling) and a separate bounded M0 emergency class. A runnable Thread consumes CPU of its authorized ResourceGroup. Group hierarchy prevents a malicious app from generating many Threads to gain a larger system share.

- A **weight** controls relative share among eligible competing entities, not minimum service within a fixed wallclock interval.
- A **hard group cap** limits chargeable CPU use in a rolling/accounting period and throttles on exhaustion until a defined replenishment. A hard cap is *not* a reserved lower bound: other groups may still use otherwise idle CPU according to admission rules.
- **Min service/CPU reservation** for M0 is separate from a mere cap; scheduler must ensure a reserved capacity band when M0 is runnable while limiting its ability to monopolize all cores.
- Runqueue fairness does not imply no-starvation if runnable count is unbounded or threads can select deliberately hostile affinity. A bounded admission and eligible-set rule are necessary to make any starvation claim.
- Migration and work stealing must preserve group accounting and bounded fairness debt. One Thread never executes simultaneously on two cores simply because of concurrent wakeups or a stale runqueue entry.
- Kernel cleanups should happen in preemptible bounded chunks where possible, not as huge unaccounted raw critical sections that defeat user weights.

**M0 special case:** an emergency class may preempt ordinary work to execute a finite critical recovery operation. Admission and accounting must bound its operation/time, even for a privileged server bug. If critical work indefinitely loops, recover via watchdog/terminal failure; no claim ordinary apps retain service when M0 consumes all CPU.

### 5.2 Comparison with weighted EEVDF-like policy

A virtual-runtime+virtual-deadline discipline can improve wakeup behavior for interactive short tasks while preserving long-term fairness. But complexity arises in sleeper credit, migration, nested groups, burst starvation and attacker-triggered excessive wake/sleep. The initial scheduler policy is **an architectural family**, not EEVDF code or a fixed virtual-deadline formula. Microbenchmark and adversarial desktop workload comparison after separate implementation authorization decide details.

### 5.3 Alternative baseline simpler round-robin

A bounded round-robin with per-group caps could be easier for an M0 bootstrap experiment than full fair queues. That may be appropriate as a *development milestone hypothesis*, not proof it meets developer-desktop responsiveness. The design should keep an internal scheduler interface that can evolve without exposing virtual-runtime implementation to KABI callers.

## 6. CPU budget hierarchy and accounting conservation

Let parent budget B have a finite amount of CPU execution entitlement over an explicitly named accounting window. A child reservation cannot escape parent/global admitted capacity, and merely creating a second handle or Thread cannot mint a new entitlement. Avoid hidden extra CPU donation/refill from kernel entry, IRQ or restart.

**Candidate concepts:**
- Actual executed CPU time: attributed to current Thread/group for user-mode and attributable kernel synchronous syscall work, including guest-restricted vs normal supervisor transitions.
- Interrupt/softirq and cross-Task kernel maintenance: charge a separate bounded system overhead pool or attributed origin when safely knowable; **do not silently classify unbounded attacker-triggered interrupts as free**.
- Async server work for a request: normally charged to server ResourceGroup under SCHED-A; client sending an IPC message is not automatic budget transfer.
- CPU quota: (budget, replenishment period, max outstanding carryover, group ancestry), not public numeric ABI; no admission if group reserved shares exceed a hardware-aware feasible bound.
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

**Charge migration:** when Task migrates across CPUs or from restricted guest to normal-mode supervisor in the *same Thread*, its current charge owner and eligibility must be checked without reset/refill. A distinct user-space broker or server runs under its own group unless a specifically authorized future lease is attached. Kill/restart cannot turn spent budget into a fresh unlimited one: child budgets and per-service restart policy reconcile against parent admission.

### 6.2 Capacity and admission are conditional

For a set of periodic budgets, an arithmetic sum of runtime/period shares less than number of available cores is a *necessary capacity sanity check* for many models, **not sufficient for all deadlines**. CPU affinity, migration restrictions, nonpreemptible kernel/IRQ intervals, shared locks, capacity/thermal heterogeneity and blocking matter. Real-time deadline admission needs documented WCET/execution, relative deadline and period with feasibility proof for selected SMP model (Linux EDF/CBS documentation [S6]).

M0 reserved capacity on a single available core may reduce normal-work share; admission must explicitly account it and avoid claiming 100% of CPU is simultaneously reserved for other deadlines. If hardware provides fewer online cores after an outage, revalidate admission or safely degrade/fail; a bound proven for four CPUs is invalid after three go offline.

## 7. Thread state machine, blocking and single wake authority

A Thread may be CREATED, RUNNABLE, RUNNING, BLOCKED_IPC, BLOCKED_FAULT, BLOCKED_TIMER, THROTTLED, SUSPENDED, STOPPING, TERMINATED or RETIRED. Separate **runnable eligibility** from **physical running** and **still-has-budget**. A Thread that sleeps and times out may still be throttled; a newly supplied page may resolve fault but fail to run while its group is exhausted.

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

- Kernel runqueue/switch/IRQ entry require tiny carefully scoped nonpreemptible regions with measured/audited worst-case assumptions, not unlimited "fast" algorithms.
- Long capability revocation trees (RFC-002), Channel queue drains (RFC-003), VM reverse-map/TLB shootdown (RFC-004) and service teardown (RFC-006) should be segmented into bounded, preemptible pieces with explicit resume/journaled metadata. Do not claim zero-cost segmentation.
- Lock order: scheduler runqueue locks, ObjectCap table, VM metadata, IPC queues, timer wheel and device IRQ cannot be held across arbitrary Pager IPC, copyout that may page-fault or arbitrary user callback.
- A reschedule IPI and cross-core Thread wake must not double-run the same Thread. CPU-local state and memory-order barriers require a finite state/lock proof.
- IRQ storm must be bounded by timely acknowledge/mask, device authority and finite work per interrupt; otherwise even fairly admitted application groups can receive no CPU.
- Emergency kernel cleanup time must itself be accounted and bounded, including failed allocation paths when ResourceGroup quota is exceeded.

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

**P0 choice:** disable universal IPC donation/automatic PI. Keep SCHED-C opt-in explicit lease on hold until a formal compare with seL4 MCS and the chosen Channel ReplyToken contract. Desktop interactivity should first be approached with correct service budget, demand fairness and bounded queueing.


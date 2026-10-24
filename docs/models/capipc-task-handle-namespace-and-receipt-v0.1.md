# UniOS CAPIPC — Task Handle Namespace, Stale-Token Safety & Kernel Receive Receipts v0.1

> **Status:** Proposed Joint Design Study — 2026-10-25 NZDT; **NOT Accepted, implemented, formally proven, simulated, measured or ABI-stabilized.**
> **Primary owner RFCs:** [RFC-002](../rfcs/rfc-002-object-capability-lifetime-delegation-revocation.md) public Handle/anti-ABA/sponsor and [RFC-009](../rfcs/rfc-009-native-abi-and-system-call-contract.md) caller entry/result, jointly [RFC-003](../rfcs/rfc-003-ipc-wire-protocol-and-synchronization.md) E1 MOVE/receive commit and [RFC-018](../rfcs/rfc-018-versioned-idl-abi-evolution-and-compatibility-contracts.md) SABI version/effect.
> **Preceding reviews:** [HPA v0.2](capipc-handle-publication-activation-v0.2.md) (B+ general T1 proof leader, C optional), [HPA v0.1](capipc-handle-publication-activation-v0.1.md), [Unified Receive State Model v0.2](capipc-send-receive-revoke-state-model-v0.2.md).
> **Boundary co-dependencies:** RFC-004 VM/Pager, RFC-005 scheduling, RFC-006 service recovery/wired M0, RFC-007 DMA fence, RFC-008 crash durability, RFC-010 Linux guest, RFC-016 consent.
> **Activity constraint:** **Design & research only.** No code, runnable state model, tests, simulator, syscall numbers, C struct layout, handle bit widths, SDK, deployment, or compatibility/security verification.

## 0. Executive recommendation: two different namespaces, two different proof problems

**HN-B (public slot + explicitly nonwrapping generation, fail-closed exhaustion) is the leading *structural* candidate for native Task Handle identity**, with **HN-A (monotonic never-reissued public token)** as a simple reference candidate and HN-C (collision-checked unpredictable identifiers) as a probabilistic comparator. The current source RFC-002/009 no-stale-rebind requirement is stronger than the documented Fuchsia behavior where closed handle numbers can be reused; this is a conscious **trade-off, not a correctness fact that all kernels require** [S1]. No numeric widths, counts or bit positions have been decided.

**KR-A (Task-bound, preidentified operation nonce + kernel-owned protected completion record) is the leading *receipt design* candidate for both HPA-B+ and HPA-C.** A nonce/slot identity must already be known to its caller **before** the irreversible SEND K-S or receive K-I; an identifier generated only in an output whose result was lost cannot support assured recovery. A request nonce is a **correlation key, not authority**. Actual query rights derive from kernel TaskGeneration and an applicable operation/Endpoint grant, not caller-provided bytes. Receipt records need bounded quotas, explicit expiry semantics and privacy controls; neither a receipt nor a status query proves remote SABI effects or file durability.

**Hard new finding:** A caller-supplied preknown nonce is not by itself sufficient for *deduplicating forever*. If a kernel forgets a completed record and later treats the same nonce as fresh, it can re-execute an old operation even though the caller believes the key gives exactly-once semantics. Either maintain nonreuse within a bounded Task/receipt namespace (eventual fail-closed exhaustion) or explicitly limit the dedupe horizon and return EXPIRED/UNKNOWN rather than an invented NOT_COMMITTED. Task termination prevents normal native Task receipt query, but may not undo queued grants, outstanding DMA or externally applied effects.

**Another hard distinction:** A stale token accidentally reauthorizing a distinct object in the *same Task* is an integrity/use-after-close problem. Guessing a possible future token in that Task is not by itself unauthorized cross-Task access because Task-wide native cap rights are already that Task's ambient boundary. If different modules/Threads within one Task require separate security principals, they need actual Task/compartment separation or issuer-authorized per-operation capabilities; opaque/random tokens are not a substitute. Likewise native public Handles and guest Linux FD numbers deliberately have different reuse semantics.

**Provisional architecture:** native public HandleNamespace and native KernelReceiptNamespace are **logically distinct**; physical data structures may later share bounded implementation machinery, but public CapHandle values are not operation IDs, and neither maps to Linux guest FD, SABI RequestId, service incarnation or file transaction key.

## 1. Primary precedents and truth-in-advertising

| Source | Documented reference behavior | Design lesson, not automatic UniOS guarantee |
|---|---|---|
| [S1] Fuchsia Zircon Handles | Handles are process-local; Fuchsia **may reuse integer values of closed Handles**; stale/racy use is a programming hazard | UniOS lifetime public-token nonreuse is deliberately stronger; do not falsely cite Fuchsia as using the same anti-ABA guarantee |
| [S2] Fuchsia Channel read | message bytes/handles have independent output buffers and actual length/count; a message is logically read in full | cannot infer atomic arbitrary user byte/Handle-ID stores from API-level whole-message claim |
| [S3] Fuchsia Channel object | unread message carries transferred Handle refs and destruction may close them | token slot released is not necessarily end of object/queue/hardware lifetime |
| [S4] Linux close(2) | FD may be freed and reused early even if later close reports error; retry can close unrelated new FD | native ABA goals must not silently be projected onto Linux guest FD ABI |
| [S5] Linux epoll(7) | FD and OpenFileDescription are not same; events can continue under duplicated OFD after close of one number | guest FD/OFD/registration generation separate from native cap and receipt identities |
| [S6] Linux io_uring(7) | caller supplied user_data links a submission to its later CQE; ordering of completion is not necessarily submission order | preknown correlation is feasible precedent, but CQE correlation is not a secure native capability or durable remote effect proof |
| [S7] seL4 IPC tutorial | explicit CSpace lookup, cap transfer and badge semantics with different synchronous endpoint model | CSpace names are not SABI caller human identity; seL4 contract not E1 drop-in |
| [S8] Linux kernel usercopy | user-memory copying may sleep/fault and leave partial output; invalid output can occur after protected internal commit | protect actual commit separately from unreliable observer copyout |
| [S9] Fuchsia zx_channel_read_etc | actual kernel handle type/rights returned for untrusted peer verification | a sender-authored IDL ordinal doesn't establish native cap rights |
| [S10] RFC-002/003/009/018 source contracts | Task-local public nonreuse, MOVE-only E1, UNKNOWN after commit and separated SABI effects | local Proposed targets, not proven implementation facts |

## 2. Identity taxonomy — NEVER alias these names

| Identity | Authority domain | Reuse/lifecycle |
|---|---|---|
| TaskGeneration | kernel isolation domain per admitted Task instance | new Task generation on death/relaunch; raw Task display ID insufficient |
| Native PublicHandle | Task-local integer lookup for exact kernel grant | may not rebind an **old externally valid or potentially published value** to unrelated grant during same Task lifetime under present Proposed goal |
| HandleSlot | kernel-internal table/storage location | may reuse if public token+generation/revocation guarantees still hold |
| GrantID/lineage | kernel right, object reference, issuer/controller/sponsor | can outlive Task Handle, queue or receipt |
| EndpointObject/EndpointGeneration | one exclusive E1 half and owner transfer state | two Channel halves distinct, even if logical service same |
| KernelInvocationId | kernel transient in-flight admitted work reference | not necessarily public; unrelated to user ticket or Handle integer |
| Preknown OperationNonce | caller-chosen / bootstrap-preallocated correlation under actual Task and scoped grant | untrusted key; cannot mint rights or guarantee indefinite dedupe |
| Protected ReceiptSlot | kernel-owned record indexed by Task/nonce/operation plus generation and admission binding | quota, tombstone and garbage-collection policy; distinct from cap Handle namespace |
| ReceiptResult | actual K-S/K-I/K-U transition evidence at kernel scope | may be pending, terminal or expired; **not** remote effect receipt |
| ReplyToken | one-shot kernel RPC reply right | caller/endpoint/request generation; not general receipt query authority |
| ServiceRequestId | SABI user-space correlation/idempotency proposal | independent authenticated backend ledger required for durable dedupe |
| Guest FD / OFD / PID | Linux personality emulation namespaces | FD numeric reuse is part of guest semantics; OFD/registration/proc generation separate |
| FileDurableReceipt | scoped storage/media persistence evidence | cannot be inferred from kernel IPC receipt |
| SecurityFloor/BootGood | authenticated boot trust state | totally separate from operation nonce or cap number |

A Task can copy a public token integer to any other Task as bytes. That alone does **not** transfer native authority: the receiving Task's kernel cap table lookup must independently contain a grant and rights. A same-valued integer in two Tasks is *not* the same kernel grant. Hence no broad promise that "a token from dead Task can never be accepted in another Task" based only on the raw bits; the correct isolation boundary is the live calling Task identity and actual table. Cross-Task replay of privileged receipt data is forbidden by authenticating the Task-generation/specific grant, not by relying on globally unique string IDs.

## 3. Native public Handle namespace candidates — finite and enforceable

| Family | Public identity concept | Advantage | Security/progress limits |
|---|---|---|---|
| **HN-A** monotonic per-Task token allocator | public values never repeat within Task | simple freshness argument; decouples internal slot reuse | sparse table/index, whole-Task lifetime identifier exhaustion; Task longevity and adversarial create/close rate |
| **HN-B** table slot + externally included generation, no wrap reuse | token identifies slot and version, compare full bits against live slot | reuses internal slots efficiently while rejecting old externally valid values; bounds per-slot state | finite generation may wrap: retire slot/fail-closed; token structure/bit width must be studied, not assumed 64-bit |
| **HN-C** randomly chosen collision-checked opaque values | sample unique in current/recent namespace and validate against active table | less predictable token values, potentially simpler output encoding | active collision check doesn't prove never-used value after old tombstone GC; unpredictability is not authorization within Task and probabilistic safety ≠ deterministic no-rebind |
| **HN-D** reuse raw closed numbers + SDK hygiene | smaller namespace and table; reference behavior in Fuchsia | simpler/fast reference alternative | violates current strict native no-stale-rebind target unless security objective is explicitly relaxed in new ADR |
| **HN-E** immutable public ticket object per grant as separately derived cap | identity/lifetime explicit | rights transfer/expiry can be explicit | risks recursion: needing a handle to identify a handle; consumes more TCB, slots, queue quotas |

**Research leader:** HN-B, *conditional* on a mathematically checked bit/slot/generation budget. Never assume infinite generations. On generation wrap, retire slot or fail closed; do not silently roll public bits to an old token. HN-A is equally legitimate as simpler baseline if a sufficiently large finite namespace and sparse table are feasible. HN-C alone cannot prove strict lifetime nonreuse without persistent issued-value history, effectively recovering the same budget/tombstone question.

### 3.1 Three visibility classes for candidate token reservation

- **NeverIssuedNeverExposed:** an internally provisional token reserved only in kernel memory, provably never returned, mapped, made valid or externally communicated. Internal reservation may be reclaimed after a whole-system proof; it cannot have been previously valid.
- **MaybeExposedButInactive:** kernel attempted to write token ID to user memory, including a partial copy or alias. Treat as publicly known even if the store faulted; the exact public value must never later identify an unrelated Grant in the current Task lifetime. Reserve/retire its identity and charge bookkeeping as needed.
- **IssuedActiveOrRetired:** public value was live for some grant, then closed/moved/revoked. Stale copies might exist indefinitely in the same Task; never reassign exact value to another grant while that Task is alive.

Caveat: code in the same Task can deliberately guess future numbers; if separate principals within a Task are in the threat model, these three classes **do not create** a privilege boundary between them. A token ID is a lookup key, not a secret bearer transferable to a different Task.

### 3.2 Public value nonreuse is not object nonreuse

A kernel may reuse memory holding retired HandleSlot and even object allocator addresses if grant lineage/independent roots/refs and hardware R2-F/R2-O prove it is safe. The no-rebind property applies to externally observed **Task-local token bits** and the authority that those bits could unlock, not a perpetual ban on memory-slot recycling. A native Handle close can invalidate a token while the same underlying object remains referenced by other grants or queued IPC.

### 3.3 Fork/exec and Task-generation policy is a separate choice

Native Task creation, rights issuance, clone semantics and AddressSpace reset must distinguish **Task lifetime** from user-space runtime library lifetime. The current native nonreuse requirement is scoped to one kernel Task-generation, not implicitly to an entire family tree of guest Linux processes. A Linux guest FD is intentionally allowed to reuse numeric values, with guest FD/OFD/epoll generations maintained separately. Do not hard-code POSIX fork/exec rules into native Handle namespace merely because a future Linux personality will emulate them.


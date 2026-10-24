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


## 4. Kernel receipt design candidates: known-before-commit is necessary, NOT sufficient

| Family | Construction | Advantage | Hidden failure and trust boundary |
|---|---|---|---|
| **KR-A1: Task bootstrapped receipt slots** | Task receives a bounded indexed receipt namespace as part of already-authorized Task admission; slot and expected generation are known in advance | no fresh ID created only in a lost K-I/K-S return | finite slot count; concurrent Threads competing for same slot; capacity and slot-generation rollover |
| **KR-A2: client-selected nonce per admitted invocation** | Task supplies chosen nonce+expected slot epoch as **input** before kernel cut; kernel binds it to exact operation fingerprint | caller already knows the correlation key even when result unavailable | nonce reuse, malicious guessing inside same Task, dedup after tombstone GC, parameter mismatch; no inherent authorization |
| **KR-B: bounded Task receipt enumeration** | query all eligible unresolved entries through live Task/Endpoint grant without knowing newly minted ID | rescues an operation whose fresh ID was lost | metadata privacy, large list DoS and lost pagination cursor; must not mint capabilities from list |
| **KR-C: shared mapped completion page/ring** | kernel writes protected per-Task completion slot with ordering barrier, client scans mapping | fewer query calls possible and precedent in io_uring CQ [S6] | user remap/alias, overwrite, producer/consumer memory ordering, page fault and ring overflow; record cannot rely solely on writable user memory |
| **KR-D: only an unambiguous register return** | architecture-specific single-return classification | avoids multiword copyout for narrow synchronous syscall | Thread death/context interruption after irreversible cut still loses observer knowledge; cannot claim user always sees result |
| **KR-E: no protected status, fail-closed UNKNOWN** | disallow blind automatic retries; use caller/session teardown if necessary | very small kernel TCB | reduced availability, source cap ownership reconciliation may require Task kill or forced quarantine; cannot guarantee graceful app continuation |

**Preferred combination for first research:** a finite **preidentified receipt slot with caller-specified expected epoch + per-invocation unique nonce** under KR-A1/A2; a narrowly scoped KR-B fallback only if needed. Use KR-C as an optimization comparison, not sole security authority. Choosing a nonce/slot pair at call entry permits a kernel to reserve receipt storage **before the irreversible K-S or K-I cut**; it does NOT guarantee the caller has received the final status, nor that the record can be kept for the entire lifetime without resource exhaustion.

### 4.1 What counts as actual authority to query?

**Proposed query rule:** require (i) real calling TaskGeneration, (ii) live scoped receipt-query right derived from a Task admission/Endpoint authority, (iii) exact operation kind and nonce/slot generation binding, and (iv) appropriate access to sensitive result fields such as destination Endpoint, cap type/rights, public token vector or bytes-count projection. The nonce is **not a bearer security capability**. A guessed nonce from unrelated Task or post-restart ServiceID must not reveal a previously protected receipt. A trusted Supervisor may need recovery visibility after client Task death, but only via **explicit, constrained supervisory grant**; this is not an ordinary query bypass.

**Different authorization moments:** the caller's cap may have been MOVEd or revoked after the original commit. Receipt query cannot simply demand possession of the *same moved handle* that no longer exists, or it would make correct SEND reconciliation impossible. Instead, preissue a separate narrow receipt-read authority under the Task's admission grant, or prove an equivalent safe method. That authority should not permit a new SEND, reactivation of revoked grant, or read of unrelated Tasks' message content.

### 4.2 Operation fingerprint and duplicate requests

An operation fingerprint is an internal binding to the actual committed kernel operation, not a hashing scheme chosen here. It must include *at least*: TaskGeneration; operation class K-S/K-I/K-U; unique receipt namespace slot+expected generation; immutable request invocation identity; relevant Endpoint object/generation; exact cap grants/lineage/rights (as kernel authenticated metadata) and caller/admission or claim generation; current access policy where necessary. Do **not** trust a hash of user-provided handle integers as evidence the current kernel cap grants match.

Under concurrent same nonce:
- A second invocation with the same still-live slot/nonce **MUST NOT start another irreversible operation**. It may see ALREADY_PENDING/COMMITTED status if authorized, or be rejected as mismatch. Deduplicating a mutation by same text bytes alone is unsafe.
- After a final receipt is acknowledged, a stale caller may still retain the old nonce/slot ID. Exact identity reuse as a *different* operation must either be forbidden until Task end or explicitly versioned so old published keys cannot query/influence a new entry.
- If receipt evidence is GC'd, the kernel may no longer truthfully know the original result. **Expired evidence is not proof of NOT_COMMITTED.** A Task/receipt namespace may eventually fail-closed rather than reassign the same externally published receipt reference to unrelated work.
- A user-supplied nonce is not proof of uniqueness; kernel must enforce uniqueness (or reject duplicate operation identities) at admission using a finite per-Task ledger.

### 4.3 Receipt slot bootstrap without circular identity loss

If a "register receipt slot" operation *returns* a newly allocated slot ID only through output that can itself be lost, the scheme merely moves the original problem earlier. Candidates:
1. fixed Task-admission receipt slots discoverable by a **known** Task-local ordinal and TaskGeneration, without any fallible allocation response used to identify them;
2. Task supplies a known candidate slot index/expected generation at a registration call, which atomically reserves it or reports explicit conflict, and status query later uses that same known identifier;
3. small bounded secure enumeration under the caller's independent receipt authority, with no cross-Task data or secret Endpoint metadata to unauthorized readers.

The selected scheme must describe the first bootstrap cut at Task creation and after Task restart, budget and quota protection, and what happens if both request call and initial slot allocation result are unavailable. If the normal caller Task no longer exists, a Task-scoped receipt may be unrecoverable to ordinary callers; that is an honest limitation, not silent remote rollback.

## 5. Kernel receipt record state, truth scope and lifetime

### 5.1 Historical actual state versus observer knowledge

| Actual kernel record phase | Meaning | Valid caller-visible claim |
|---|---|---|
| R0 PreallocatedEmpty | known Task-generation/slot exists; no admitted request for this identity | not yet admitted *for this exact slot epoch*, not blanket “no effect anywhere” |
| R1 ReservedPending | capacity and immutable request fingerprint reserved, but irreversible commit not yet happened | PENDING; may later COMMIT, so NOT_COMMITTED_FINAL is false |
| R2 FinalNotCommitted | operation definitely terminated before K-S/K-I and cannot commit later | definitive NOT_COMMITTED for this exact kernel operation; sender kept source only when no prior MOVE |
| R3 CommittedSend | K-S actually appended message and MOVEd complete cap vector | COMMITTED_KS: original source tokens gone; receiver/application effect unknown |
| R4 CommittedReceive | K-I actually installed vector and retired message in a single kernel cut | COMMITTED_KI: receiver may own caps; output/Thread observation unknown |
| R5 CommittedRelease | optional K-U enabled specific T2 escrowed caps after trusted validation | COMMITTED_KU: no promise remote effect |
| R6 TerminalReceive | claim/message closed/quarantined without K-I, maybe disclosed payload/metadata | TERMINAL with exposed scope and sponsor debt; does not reissue original sender handles |
| R7 RetiredEvidence | no longer retaining full receipt after authorized bounded GC | EXPIRED/UNKNOWN if query cannot prove outcome; never silently treat as final no-commit |
| R8 TaskDead/KernelRestarted | Task authority gone, kernel receipt volatile, or restart destroyed history | ordinary Task query unavailable / UNKNOWN to outside observer; service and physical effects may survive |

**Observer state:** a caller sees COMMITTED/NOT_COMMITTED only after a protected query or trustworthy return; if the return was lost, it may know merely UNKNOWN. A status read of **PENDING** at one instant does not freeze future kernel progress or prove the operation can be retried. The kernel's own protected record ought to know its current cut **while it exists**; “UNKNOWN” is typically the observer's incomplete knowledge, or a retained-record failure after destruction/restart, not a kernel instruction to flip committed grants back.

### 5.2 Exactly where record and commit must agree

For a protected receipt to reconcile K-S/K-I, the kernel must serialize the irrevocable state transition with recording its actual phase. Reserving a record only **after** commit risks leaving a fully transferred vector with no discoverable truth when allocation fails. A design therefore pre-reserves charged metadata *before* commit, and the commit cut logically changes both kernel resource custody and its corresponding protected result state; a query must see a coherent version of that transition. No pageable write/copyout is allowed to be the sole authoritative record of it. No concrete mutex, memory barrier or data layout is selected; SMP read/write ordering and failure behavior remain proof gates.

**Strong safety subtlety:** a query racing a commit may read PENDING immediately before that commit. The caller must not interpret PENDING as **NOT_COMMITTED_FINAL** and resend. It may await a final protected phase with a bounded timeout. A final result must never later be replaced with an opposite final result for the same operation identity.

### 5.3 Acknowledge, retire, tombstone, and expiry

One cannot retain unbounded per-Task history without DoS risk. Candidate life:
- After actual transition finishes, keep the protected final record until a scoped recipient acknowledges it or a named bounded retention deadline expires. ACK is an **observation/GC signal**, NOT rollback, cannot change committed cap custody, and must be serialized against query.
- Evict big payload/secret per-field data separately from a smaller immutable final-state tombstone; do not leak content through stale queries. Tombstones and their IDs require quotas, sponsor accounting and a strategy for Task lifetime.
- **On expiry of the only evidence, return EXPIRED_EVIDENCE/UNKNOWN**, unless an independent, still-protected commit ledger definitively resolves the result. Do NOT return clean NOT_COMMITTED because a record is absent.
- If a receipt identity can be reused for a new operation after GC, its **public generation must differ and not wrap to an old visible value**, and the kernel must prevent stale queries/duplicate retries with old generation. Otherwise strict once-per-operation reconciliation cannot be promised across unbounded time.
- If caller explicitly ACKs/forgets the receipt, it loses its own recovery evidence; code should not claim an old operation can be queried forever.
- Task death can release its receipt namespace only after outstanding kernel queue/claims/DMAs have independent custodians and sponsor accounting. It cannot erase an already-applied external effect.

### 5.4 Receive receipts do NOT mint a second cap set

Under HPA-B+ receive, once K-I commits the receiver Task owns G and its pre-reserved public tokens resolve to those grants. A query may reveal the **historically committed token identities** and current retained result, but must not install a new vector, duplicate each cap, or resurrect caps that another Thread already closed, MOVEd or revoked. Historical COMMITTED_KI and **currently invokable** are separate facts. Optional extra current-cap query needs a separate rights/generation check on each token. Returning “committed” for a token closed by B-Thread2 remains historically true.

After HPA-C Stage but before AcceptCommit, the claim's receipt can show staged/inactive; any token values reported remain inert. A distinct AcceptCommit result receipt has its own irreversible K-I/K-U cut, and its output can also be lost. Whether Stage and Commit can share one protected result slot with a monotone phase sequence or require two slots remains an explicit P0 research choice.

## 6. Red-team identity/ABA and receipt races (NR-01…NR-26) — paper only

| ID | Adversarial interleaving | Forbidden false result |
|---|---|---|
| NR-01 | closed public token h reissued for unrelated grant in same live Task | stale h acquires new authority |
| NR-02 | internal slot reused with public generation never advanced | old external bits accepted as new cap |
| NR-03 | public generation hits maximum and silently wraps | stale h eventually valid again |
| NR-04 | token randomly assigned but collisions checked only against active entries | long-retired value may recur despite deterministic no-rebind promise |
| NR-05 | T1 Receive token only *partly* written before copyout fault; reserved identity recycled | B's saved old bits later act as unrelated grant |
| NR-06 | unused internal token reservation proven no publication, but allocator burns all such reservations under malicious failures | catastrophic exhaustion despite potential safe reclamation |
| NR-07 | Task A token bits passed to Task B that lacks the grant | copied number wrongly treated as cross-Task MOVE |
| NR-08 | privileged module incorrectly treats same-Task token secrecy as subprincipal security | guessing token crosses boundary that doesn't actually exist |
| NR-09 | guest Linux close(7) followed by open reuses fd=7 while native cap ID also considered “FD” | host receipt/epoll mistakenly bound to new FD/OFD |
| NR-10 | Task client specifies nonce N for irreversible SEND; return lost | record discoverable by preknown Task-scoped N, no source rollback |
| NR-11 | Task registers receipt and loses *new* slot ID returned from registration | status falsely advertised queryable |
| NR-12 | two Threads submit different mutations with identical nonce/slot | both accepted or one overwrites other's committed result |
| NR-13 | after ACK/tombstone GC same nonce accepted for new Send, stale Thread retries old Send | old op unexpectedly runs again |
| NR-14 | protected query reads PENDING and caller treats it as final NOT_COMMITTED | second cap MOVE or double external side effect |
| NR-15 | K-I succeeds, receiver's result copyout faults, other Thread closes installed handles | receipt claims currently usable grants just from historical COMMITTED |
| NR-16 | receipt includes Endpoint and rights metadata, queried from different Task by guessed nonce | confidential cross-Task disclosure |
| NR-17 | TaskGeneration reused/display TaskID same, old claimant query accepted | old Task receipt grants new Task access |
| NR-18 | SEND moved the original caller's only Handle, query erroneously requires same moved Handle | committed result permanently impossible to reconcile |
| NR-19 | kernel commits K-S before allocating record, allocation OOM afterward | committed no-receipt state falsely called NOT_COMMITTED |
| NR-20 | record final written but user status overwritten by another Thread | return buffer contents mistaken for trusted kernel receipt |
| NR-21 | Task ACKs committed receipt then loses ACK result, re-ACKs | ACK may retire only this record, not another recycled slot |
| NR-22 | old receipt expires; query returns NOT_COMMITTED based on absence | false no-effect / wrong cap ownership |
| NR-23 | Task dies after SEND; Supervisor reads every Task's raw receipt list via ServiceID name | ungranted cross-principal observation |
| NR-24 | kernel restarts and loses volatile K-S receipt, filesystem write already applied | receipt reconstructed as no write, duplicate non-idempotent effect |
| NR-25 | receipt collection exhausts memory, queue and M0 metadata | critical RecoveryAuthority starved by malicious outstanding operations |
| NR-26 | Linux guest epoll old FD/OFD registration maps to same numeric native Handle due reuse | FD generation/confidentiality and event identity corrupted |


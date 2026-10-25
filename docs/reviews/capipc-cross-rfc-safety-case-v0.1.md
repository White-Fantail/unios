# UniOS — CAPIPC Cross-RFC Safety Case v0.1

> **Status:** Proposed, Independent Design-Only Safety Case / Open Findings — 2026-10-25 NZDT. **Not Accepted, implemented, formally proved, model-checked, fuzzed, benchmarked, hardware-tested, security-certified, or ABI-stabilized.**
> **Main claim under review:** A bounded Task-scoped KR-M2 receipt admission fence can coexist with E1 transactional Capability MOVE, exposure-aware RECEIVE, revocation, VM/Pager, M0 recovery, native ABI and external effect truth — **not yet established**.
> **Base models:** [Ordered vs Sliding-Window Receipt Admission v0.1](../models/capipc-ordered-vs-sliding-window-receipt-admission-v0.1.md), [Finite Handle Namespace, Receipt Ledger & GC v0.1](../models/capipc-finite-handle-namespace-receipt-ledger-gc-v0.1.md), [Task Handle Namespace & Kernel Receive Receipts v0.1](../models/capipc-task-handle-namespace-and-receipt-v0.1.md), [Handle Publication & Activation v0.2](../models/capipc-handle-publication-activation-v0.2.md), [Capability · IPC · SABI Integration v0.2](../contracts/capability-ipc-sabi-integration-contract-v0.2.md), [Cross-RFC Consistency Audit v0.1](cross-rfc-architecture-consistency-audit-v0.1.md).
> **Direct RFC owners:** [RFC-001](../rfcs/rfc-001-kernel-boundary-and-tcb.md) kernel TCB, [RFC-002](../rfcs/rfc-002-object-capability-lifetime-delegation-revocation.md) Capability, [RFC-003](../rfcs/rfc-003-ipc-wire-protocol-and-synchronization.md) IPC, [RFC-004](../rfcs/rfc-004-virtual-memory-pager-cache-and-oom.md) VM/Pager, [RFC-005](../rfcs/rfc-005-scheduling-preemption-and-cpu-budget.md) CPU/progress, [RFC-006](../rfcs/rfc-006-boot-trust-chain-service-startup-and-recovery.md) restart/M0, [RFC-009](../rfcs/rfc-009-native-abi-and-system-call-contract.md) KABI. Adjacent RFC-[007](../rfcs/rfc-007-user-space-drivers-dma-and-iommu.md)/[008](../rfcs/rfc-008-storage-filesystems-and-crash-durability.md)/[010](../rfcs/rfc-010-linux-personality-and-binary-compatibility.md)/[018](../rfcs/rfc-018-versioned-idl-abi-evolution-and-compatibility-contracts.md).
> **Safety-case method:** Claim–Argument–Evidence–Defeater. A cited Proposed RFC is **design intent**, not independent proof. No guarantees inferred from other kernels' implementations.

## 0. Executive verdict — CONDITIONAL ALIGNMENT, NO-GO for adoption

**No new direct text-level contradiction has been confirmed** among the inspected current RFC-002/003/004/005/006/009 descriptions of Task identity, transactional MOVE, exposure-aware receive, Pager faults and M0 recovery. This does **not** mean KR-M2 is proven compatible. The existing corrected RFC-003 E1-DERIVE inconsistency remains a **historical, editorially corrected** item, not an active newly found conflict.

**Six safety-critical ambiguities / proof gaps newly sharpened in this joint review:**

1. **Admission is not transfer authorization forever.** KR-M2 A(q) can bind immutable input/fingerprint, but after Endpoint owner MOVE, grant revocation barrier B, changed pager mapping or user-buffer mutation, a later S(q)/I(q)/U(q) must obey a documented commit-time authority and admission-reference ordering. A stale A(q) must not act as blanket permission to commit.
2. **Receive disclosure is not K-I completion.** A receiver's q may be consumed and receipt PENDING while exclusive ReceiveClaim is active; first possible user byte V is a separate irreversible cut. SKIP must not retrospectively cancel that q, and loss of receive result cannot justify foreign requeue after V.
3. **High-water frontier means consumed identity, not final result.** KR-M2 C may pass q while R[q] remains pending. Any cross-RFC component that frees pending R[q], grants, claims, FaultWaiters or queues just because q≤C breaks ownership/evidence.
4. **Task death invalidates ordinary receipt identity, not committed effects.** Recovery must reconcile old queue/claim/DMA/storage with independent custodians and incarnation-bound rights. A new Task with same display name/q may not query the old record or assume external work was undone.
5. **Resource dependency can deadlock the safety mechanism.** Bounded window W alone does not bound outstanding receipts Q, FaultWaiters, queued caps, cleanup or SMP wait-for cycles. Receipt admission/query/SKIP cannot consume protected M0 reserve or block holding locks that the Pager/recovery path needs.
6. **A caller's q is not service idempotency.** KR-M2 prevents one Task-local kernel operation identity from being admitted twice; a semantically identical SABI request can use a new q, a new Task or new ServiceIncarnation. Durable/external exactly-once remains outside this contract.

**Research decision:** retain **KR-M2 as the candidate for multi-Thread *proof exploration only***, KR-M1 as simpler baseline, and HPA-B+ as the current T1 research reference. Do **not** select KR-M2 as an Accepted kernel feature or update an ABI. If the required bounded liveness + no-replay + M0 proof fails, retreat to KR-M1, a scoped receipt lane, or fail-closed UNKNOWN rather than relaxing critical invariants silently.

## 1. Assumptions, evidence tiers and precise claim boundary

### 1.1 Trust and authority assumptions (not yet discharged)

- **T0 kernel** can authenticate current TaskGeneration, current CapGrant/Endpoint generation, revocation scope and sponsor; it can reserve protected bounded receipt state and enact an atomic admission/commit cut on SMP. Whether that is feasibly implementable within the RFC-001 minimal TCB is **OPEN**.
- **Untrusted caller** controls q, syscall input/outputs and possibly concurrent same-Task Threads; raw q, nonce, PID, ServiceID, guest UID and SABI RequestId do not grant native authority.
- **Protected Task receipt namespace** is born with actual TaskGeneration. It has a narrow, independently scoped query/abandonment grant that can survive authorized source MOVE while the Task is live; post-death inspection is *not* ambient.
- **All buffers** can fault, alias or change after inspection; copying to user memory can be partially visible. The kernel must not hold inappropriate global locks across faultable usercopy or Pager requests.
- **M0** code, relevant stacks/IPC/queues/metadata/IRQ/cleanup budget must remain actually resident and independently admitted, not just high priority.
- **E1** one effective Endpoint owner applies; same-Task Threads may share within the Task boundary; cross-Task transfer is MOVE, not implicit DERIVE.
- **Kernel-lifetime** receipt availability is volatile across reboot; physical DMA and media effects can persist; no hardware/software component is presumed fail-free.

### 1.2 Classification rules

| Class | Meaning | Evidence status |
|---|---|---|
| ALIGN | Current Proposed text gives mutually consistent intended responsibilities | document comparison only, NOT proof |
| AMBIGUOUS | Two or more outcomes could follow because a combined linearization or owner isn't fixed | unresolved P0 contract |
| GAP | Shared invariant/proof/finite bound missing even though prose agrees | unresolved P0 verification |
| CONFLICT | Two live contracts directly require mutually incompatible action | none newly established in this review |
| EXTERNAL | Depends on hardware, media, service policy or guest compatibility | separate validation |
| HISTORICAL | Prior confirmed text contradiction since edited | do not misreport as active bug |

**Evidence classes:** E-D = inspected Git main design text; E-P = official technical precedent; E-X = execution/model/test trace; E-H = hardware measurement/fence. This study supplies **E-D and selected E-P only**, not E-X or E-H. Every top-level assurance claim remains **OPEN**.

## 2. Claims–arguments–evidence–defeaters matrix

| Claim ID | Desired safety claim | Cross-RFC argument / required owner | Evidence now | Blocking defeater / status |
|---|---|---|---|---|
| SC-C01 | Each q admits at most one native irreversible operation in one T/R | RFC-002 Task+grant; RFC-003 A/X; RFC-009 protected C/B and R[q] | E-D: sliding-window model §5 | SMP double A, ring-ABA; **GAP/P0** |
| SC-C02 | Receipt query/skip cannot mint or widen a capability | RFC-002 query/abandonment grant, actual TaskGeneration and issuer | E-D: RFC-002 §§7,21–23 | guessing q, holder/source confusion; **GAP/P0** |
| SC-C03 | Admission fence survives record GC without false final result | RFC-003 C/B replay fence separate from R[q] phase | E-D: KR-M2 state model §5.2 | C slide frees PENDING R[q], stale q becomes fresh; **GAP/P0** |
| SC-C04 | A, Endpoint MOVE, revoke B and K-S authorize one MOVE only | RFC-002 B/lineage + E1; RFC-003 commit; RFC-009 revalidation | E-D: RFC-002 §§9–11; RFC-003 §5 | A prior to B used as irrevocable right after B; **AMBIGUOUS/P0** |
| SC-C05 | Receive's earliest possible V prevents foreign redisclosure | RFC-003 ReceiveClaim/terminal; RFC-004 usercopy; HPA-B+ | E-D: integration §4 / RFC-004 §8 | fault after first byte, erroneous requeue; **GAP/P0** |
| SC-C06 | K-I installs complete cap vector once without granting twice | RFC-002 CapGrant; RFC-003/009 install/receipt | E-D: integration §5 and RFC-009 §5.3 | status requery reissues cap after close; **GAP/P0** |
| SC-C07 | Unknown or revoked SABI resource cannot silently become active authority | RFC-018 method authorization + HPA-B+/T2 trust choice | E-D: integration §5 / RFC-018 §4 | CAP-BEARER promises unavailable T2 gate; **AMBIGUOUS/P0** |
| SC-C08 | Usercopy/page fault never deadlocks Pager/IPC/M0 | RFC-004 FaultWaiter/locks, RFC-005 runnable budget, RFC-006 M0 | E-D: RFC-004 §§7–11, RFC-006 §7 | global IPC lock held over Pager recursion; **GAP/P0** |
| SC-C09 | R[q], C/B, claim, queue, sponsor remain coherent on OOM/kill | RFC-002 sponsor, RFC-003 quotas, RFC-004 OOM, RFC-006 cleanup | E-D: RFC-002 §6; RFC-003 §9; RFC-006 §§7,9 | capacity allocated *after* commit or premature refund; **GAP/P0** |
| SC-C10 | Task and Service restart cannot transplant old rights/receipts | RFC-006 new incarnation/registry; RFC-002 generation; RFC-009 KABI | E-D: RFC-006 §§9,14 | same display ID or q allows stale query/reply; **GAP/P0** |
| SC-C11 | Receipt/Task expiry never reports no remote or durable effect | RFC-008 scoped storage; RFC-018 service ledger; RFC-009 error truth | E-D: RFC-008 §5; RFC-009 §16 | mapping expired to NOT_COMMITTED/EINTR success; **GAP/P0** |
| SC-C12 | DMA/pinned physical memory never freed by receipt GC | RFC-002 R2-F/R2-O; RFC-007 physical fence; RFC-006 quarantine | E-D: RFC-002 §10; RFC-007 §7 | Task died and sponsor refunded while bus mastering; **EXTERNAL/P0** |
| SC-C13 | E1 Endpoint service/client concurrency preserves actual provenance | RFC-003 distinct E1 Channels, RFC-006 registry, RFC-018 issuer | E-D: earlier audit CX-02–04 editorial clarifications | Task receipt lane assumed a new Endpoint owner; **AMBIGUOUS/P0** |
| SC-C14 | Guest FD/errno/PID and native q/CapHandle cannot alias | RFC-009 KABI, RFC-010 GABI, RFC-018 versions | E-D: RFC-009 §§13,16,26–28 | guest retry closes/replays different native op; **GAP/P0** |
| SC-C15 | No-pending-capability loss when SKIP races admitted operation | RFC-003 protected A/X order, RFC-002 ownership | E-D: KR-M2 §6.2 | SKIP incorrectly cancels after A or after V; **GAP/P0** |
| SC-C16 | No unbounded wait under documented admission/fairness assumptions | RFC-005 CPU groups, RFC-006 M0, RFC-003 backpressure | E-D: KR-M2 missing-hole theorem | permanent q gap fills finite W; **LIMITED/NO universal liveness** |
| SC-C17 | Optional supervisor recovery stays narrow after Task death | RFC-006 scoped RecoveryAuthority, RFC-002 query grants | E-D: Task restart rules only | unrestricted receipt enumeration leakage; **AMBIGUOUS/P1** |
| SC-C18 | Kernel IPC receipt does not become storage/boot durability receipt | RFC-008 storage, RFC-006 boot, RFC-018 effects | E-D: explicit separate ledgers | confusion of COMMITTED_KS with FS_DURABLE/BootGood; **ALIGN boundary, unproved** |

**Interpretation:** Nothing marked ALIGN/EXTERNAL/GAP is an implementation test. A source RFC's statement that a property “MUST” hold is an unproved obligation, not an executable witness.

## 3. Shared state inventory and authoritative custodians

| Entity/ledger | Authority and lifetime | Must survive independently from |
|---|---|---|
| TaskGeneration T | protected kernel task instance; reset only upon actual distinct Task admission | human-readable PID/name, SDK runtime restart |
| Native CapHandle token h | T-local authenticated grant lookup; current public no-stale-rebind proposal | internal HandleSlot recycling, guest FD |
| CapGrant G, lineage anchor and sponsor | kernel, until all descendant/queued effects safely settled | closing original h, R[q] GC |
| Endpoint E/EndpointGeneration | kernel E1 exclusive owning Task; bounded invocation refs | service registry name and caller's q |
| Window frontier C and consumed-bit B | kernel T/R for whole live T, not retained-result lifetime | ACK, GC of final R[q], state query timeout |
| Protected R[q] + immutable fingerprint F | kernel at A(q) before S/I/U; bounded Q and sponsor | lost register return, caller usercopy buffer |
| Claim(M,B)/exposure flag V | kernel authorized receiver/Endpoint incarnation; terminal custody | q admission, record GC, Pager fault |
| Queue record M and moved G vector | kernel channel queue after K-S; recipient after K-I | sender Task death or status timeout |
| Revocation B/R1/R2-F/R2-O | kernel authority; CPU and hardware fence evidence | local close, SKIP, Task receipt expiry |
| FaultWaiter/MappingGen/PagerBinding | kernel per Thread, address mapping and Pager incarnation | page supply response, Task HWM |
| M0 reserve and CPU credit | protected kernel/RecoveryAuthority budget | ordinary Task receipt, bitmap, claim or OOM |
| ServiceInstance/policy epoch | trusted issuer/supervisor/service; authenticated grants | Task q, raw ServiceID, old ReplyToken |
| SABI RequestId/ledger | service-authorized protocol, possible persistent idempotency | native q, Endpoint FIFO |
| File DurableReceipt | storage + validated media protocol with scope | kernel K-S/K-I completion |
| Physical DMA pins/fence/quarantine | actual DeviceGroup/hardware/sponsor evidence | driver Task, closed Handle, q GC |

### 3.1 C is *consumed* frontier, not completed frontier

**Mandatory clarification for all RFC owners:** A(11) and A(12) may make C=12 even when R[11] is still PENDING and R[12] is final. C's monotonicity proves **no readmission**, not **operation completed, bytes published, source CapGrant freed, caller notified, hardware quiesced, or remote service durable**. No cleanup or exit check may use only q≤C.

### 3.2 q binding: no accidental two-operation identity

KR-M2's q refers to **one exact native invocation class and fingerprint** within a protected T/R namespace. K-S in sender Task A and K-I in receiver Task B are normally **different independently admitted calls and different q namespaces**, even when handling the same message. The message ID/ReceiveClaim links their effects, not an accidental identical numeric q. Optional K-U is a separately authorized operation/receipt identity if independently irreversible; cannot silently reuse already-consumed q as a new admission.

An invocation *may progress through multiple internal preparatory steps*; this is not authorization to apply unrelated subsequent irreversible operations under one consumed q. A post-receive SABI effect is not covered by sender/receiver receipt idempotency automatically.

## 4. Cross-boundary operation timelines and counterexamples (paper only)

### 4.1 SEND normal / lost-result path

```text
A(TaskGen) supplies q, immutable-input candidate, scoped E1 SEND
  -> bounded copyin/snapshot + validate current h/lineage/E/G/policy
  -> reserve R[q], queue slots and sponsor before A(q)
  -> A(q): mark protected consumed B[q]; create PENDING(q,F)
  -> revalidate E owner, current grants/revoke B, queue capacity
  -> K-S: atomically append full message + MOVE all G; R[q]=COMMITTED_KS
  -> result copyout/register path may be lost
  -> query scoped R[q] [or EXPIRED_EVIDENCE/UNKNOWN after final GC]
```

**Question left open:** if revocation B/Endpoint MOVE interposes between A(q) and K-S, either a defined **bounded admitted-before-barrier** continuation that does not authorize *new* post-B actions, or **FINAL_NOT_COMMITTED(q)** with all original sender grants retained *if no K-S*. The cross-RFC ordering must be one consistent serialization cut; A(q) alone is not sufficient authority. There is no blanket rollback after K-S.

### 4.2 RECEIVE claim, exposure and result

```text
B(TaskGen) admits its own q_B and receives exclusive Claim(M,B,Egen)
  -> protected R[q_B]=PENDING, reserve B public token IDs + cap slots
  -> validate mapped user output/claimed recipient and allocation
  -> V: conservatively mark potentially exposed before first user byte write
  -> possible Pager fault / partial usercopy
  -> K-I: one full cap-vector install + queue retirement, R[q_B]=COMMITTED_KI
  -> output result may be lost; query historic record (not a cap mint)
```

If a fault occurs after V but before K-I, **never requeue M to a different Task/ServiceIncarnation** by using X(q_B), q result expiry or “no successful receive” as evidence. Instead preserve authorized original receiver continuation if demonstrably safe, or terminal settle/quarantine with honest possible disclosure, quotas and independent grant custody. The exact ordering of cap token publication against K-I and HPA-B+ is still an open joint proof.

**Alternative K-I-before-V** also has a safety cost: a newly installed usable cap may be used before user-space SABI method-role checks. HPA-C escrow is optional only when an independently trusted deferred activation requirement warrants its extra TCB. Neither variant is proved here.

### 4.3 SKIP and concurrent input preflight

```text
Thread A1 selects q=31, copies mutable descriptors, then stalls on Pager
Thread A2 requests authorized SKIP(31)
  X(31) wins protected cut, q=31 consumed, no native mutation
Thread A1 resumes and attempts A(31)
  => fails as consumed, cannot SEND even if old preflight looked valid
```

If A(31) won first, X(31) cannot erase PENDING, cannot remove queued G and cannot label an uncertain result “no cut”. A SKIP acknowledgment that itself gets lost must be reconciled as an **identity-consumption question**; not by allocating a fresh q and assuming old business intent was canceled.

### 4.4 Concurrent revoke/Endpoint MOVE race

```text
A(q) admitted under Egen=5 and Grant g
Revoker publishes B, or E owner MOVEs to another Task
Old Thread later enters K-S with stale preflight
```

Fail closed unless commit-time grants/current owner and the explicitly allowed pre-B in-flight reference are coherently ordered. If S commits before B, preserve actual committed custody and track quiescence; if B wins, no newly unauthorized S may commit. Endpoint public owner and bounded old in-flight reference cannot be conflated. Pending receipt must final settle or remain honestly PENDING while quiescing, never be dropped by GC.

### 4.5 Task crash and service restart

```text
A q=40 COMMITTED_KS, kernel result was lost
B receives data, performs an external effect, reply not delivered
B Task dies; Supervisor starts B2 with same ServiceID but new generation
A discovers B2, tries new q=41 with same SABI request
```

Task-local q replay fence cannot prevent duplicate remote effect across B→B2. Only a scoped service backend idempotency transaction (durably recorded *when required*) can authorize replay. New Registry Endpoint must not inherit old ReplyToken, cap grants or receipt-query authority by name. Even a reliable kernel COMMITTED_KS proves only SEND queue/move, not B's execution.

### 4.6 Receipt GC versus pending physical/VM debt

```text
q≤C, R[q] finalized; ACK and GC complete
Another kernel-owned queue grant / FaultWaiter / pinned DMA / dirty media log remains
Quota reaper mistakes final receipt GC for system-wide completion
```

Unsafe: sponsor refund, memory reuse, or false R2-F/R2-O/DurableReceipt. Keep independent custody, M0-safe iterative cleanup and true hardware/storage proof. Receipt B/C and R[q] are not the only ledgers.

## 5. Attack/failure schedule matrix (SC-X01…SC-X24) — NOT EXECUTED

| ID | Adversarial cross-RFC interleaving | Unsafe outcome to exclude | Lead |
|---|---|---|---|
| SC-X01 | two CPUs A(q) simultaneously | double kernel admission | 003/009 |
| SC-X02 | X(q) races A(q) after faultable preflight | SKIP cancels admitted/committed cap operation | 003/004 |
| SC-X03 | C slides over R[q] PENDING | pending receipt and mutation lost while still commit-capable | 003/009 |
| SC-X04 | GC last final evidence and retry old q | stale q admitted as new | 003/009 |
| SC-X05 | map user q to ring position only by modulo W | ABA after window reuse | 002/009 |
| SC-X06 | q>C+W accepted by integer wrap | replay/unauthorized future admission | 002/009 |
| SC-X07 | A(q) before B revocation, S(q) after | post-barrier unauthorized grant transfer | 002/003 |
| SC-X08 | E1 owner MOVEs between preflight and SEND commit | old/new Task both effectively send | 002/003 |
| SC-X09 | source handle reused or stale in cap vector | unrelated grant accidentally moved | 002/009 |
| SC-X10 | A(q) publishes prior to receipt allocation success | no protected truth after subsequent commit | 003/004 |
| SC-X11 | SEND succeeds then copyout result faults | NOT_COMMITTED reported despite MOVEd sources | 003/009 |
| SC-X12 | ReceiveClaim V then partial Pager fault | new service sees requeued secret | 003/004 |
| SC-X13 | K-I installs grants before SABI role validation | untrusted method receives usable cap while policy claimed otherwise | 003/018 |
| SC-X14 | K-I receipt requery after second Thread closes grant | cap duplicated/resurrected | 002/003 |
| SC-X15 | old PagerBinding supplies page after Task generation changed | stale page installed / waiter awakened | 004/006 |
| SC-X16 | global IPC lock held while usercopy waits on Pager's same IPC | deadlock despite free CPU | 003/004/006 |
| SC-X17 | W bitmap small but R[q] pending continues after C | Q/M0 exhaustion via unbounded pending | 003/005/006 |
| SC-X18 | permanent missing low q then arbitrary high q submitted | promised nonblocking progress violates finite W | 003/005 |
| SC-X19 | Task dead while old queue/claim/sponsor survives | premature cap/metadata free | 002/006 |
| SC-X20 | new ServiceIncarnation gets old q/ReplyToken by name | authority/metadata leak | 006/009/018 |
| SC-X21 | backend effect applied before service died; new Task sends fresh q | external action duplicated | 008/018 |
| SC-X22 | receipt GC while DMA still bus-mastering pinned frame | cross-domain physical memory corruption | 002/007 |
| SC-X23 | native EXPIRED/UNKNOWN mapped to Linux clean EINTR/no effect | unsafe guest retry/FD alias | 009/010 |
| SC-X24 | malicious guessed q from another Task calls frontier query | cap rights/Endpoint metadata exposed | 002/009 |

## 6. Explicit cross-RFC safety obligations (SC-I01…SC-I24) — UNPROVED

| ID | Required predicate |
|---|---|
| SC-I01 | Actual TaskGeneration and scoped grant, not q or ServiceID, authorize receipt operations. |
| SC-I02 | For each T/R/q, at most one A(q) and one immutable matching fingerprint F is admitted. |
| SC-I03 | X(q) may consume only never-admitted q; its outcome cannot erase a prior A/S/I/U. |
| SC-I04 | C increases only across contiguous q consumed by A or X, with nonwrapping public sequence identity. |
| SC-I05 | q≤C and consumed in-window B positions are never eligible for a different future A, even after R[q] GC. |
| SC-I06 | A(q) and protected PENDING R[q] installation share a fault-safe admission cut after reserved capacity. |
| SC-I07 | No pending R[q] may be GCed while its kernel work can still commit or requires authoritative settlement. |
| SC-I08 | K-S commits full byte+grant queue append, source-token removal and protected actual phase coherently. |
| SC-I09 | Concurrent revoke B and Endpoint owner transfer are ordered with in-flight accepted operations; no new unauthorized post-B/owner-move commit. |
| SC-I10 | F is based on kernel-authenticated CapGrant/Endpoint/Task and stable copied inputs, not raw user bytes alone. |
| SC-I11 | A ReceiveClaim/Task/Egen is unique and authorized; V possible exposure is marked no later than first user write. |
| SC-I12 | After V, same message cannot be requeued to unrelated recipient or new ServiceIncarnation, even after copyout failure or q GC. |
| SC-I13 | K-I installs complete transferred cap vector once or not at all; historic query cannot reinstall or resurrect grants. |
| SC-I14 | CAP-BEARER vs CAP-ESCROW/T2 promise is explicit; unknown IDL resource never weakens rights/revocation checks. |
| SC-I15 | Usercopy/page fault cannot hold inappropriate kernel global locks or create Pager/recovery wait cycles. |
| SC-I16 | Task/Mapping/PagerBinding/Grant generations are revalidated before completing fault/wake or resuming work. |
| SC-I17 | Receipt, bitmap/frontier, queue, claim, lineage, FaultWaiter and sponsor charges are bounded and separately tracked. |
| SC-I18 | M0 CPU/IPC/frame/cleanup reserves cannot be depleted by ordinary receipt/query/SKIP flood; failure is bounded. |
| SC-I19 | Task death/svc restart does not clone rights, merge old ServiceIncarnation or assert prior noncommitted remote effect. |
| SC-I20 | Receipt/ACK/GC cannot directly refund live queue, in-flight reply, CPU mapping or DMA/quarantined physical debt. |
| SC-I21 | Guest FD/OFD/PID and native Task/q/Handle identities never alias by raw numeric equality; status maps truthfully. |
| SC-I22 | K-S/K-I/K-U receipts do not imply SABI effect success, media durability, filesystem namespace persistence or BootGood. |
| SC-I23 | Snapshot query may race transition but returns consistent state or safe PENDING/UNKNOWN; never misleading definitive no-cut. |
| SC-I24 | Liveness claims explicitly name fairness/resource availability/permanent-hole conditions; no universal progress under finite W. |

## 7. Architectural options for receipt ownership and scope

| Option | Benefit | New burden / acceptability |
|---|---|---|
| **T-global C/B/Q** (current KR-M2 research baseline) | one TaskGeneration, simple scope, small number of ledgers | contention; a missing q or authorized SKIP by one Thread can block/suppress others |
| **Scoped per-lane C/B/Q** (new comparator) | independent progress by caller group/Endpoint operation lane | lane issuer/rights, Sponsor, nonwrapping LaneGeneration, old-lane ABA, cross-lane atomic MOVE and revocation proof; **not accepted** |
| **KR-M1 global monotonic H** | fewest admission states | preempted/hung next-q bottleneck |
| **Protected nonce/slot KR-A** | unordered requests | lifetime nonreuse fence, GC and slot/wrap complexity |
| **No reconciliation / fail-closed UNKNOWN** | least receipt state | app availability and user-visible uncertainty, still needs safe cap custody cleanup |

**Scope warning:** Making a lane secret or guessing-resistant is not a security principal. Every lane must be created by an actual scoped grant; per-lane q cannot replace Endpoint/CapGrant rights. Deleting/reusing a lane ID while an old q may survive introduces another ABA surface. The preliminary preference is **compare Task-global and scoped-lane KR-M2 on paper**, not invent a kernel per-Thread policy manager.

## 8. Dependencies, priority and acceptance-blocking questions (SC-O01…SC-O18)

| ID | Priority | Open proof/decision | Owners |
|---|---|---|---|
| SC-O01 | P0 | Can kernel atomically reserve A(q) + PENDING record under realistic SMP without pager-dependent allocation? | 003/004/009 |
| SC-O02 | P0 | Can commit-time rights/revoke/Endpoint owner checks coexist with previously admitted A and source MOVE without unbounded blocked refs? | 002/003 |
| SC-O03 | P0 | Choose coherent RECEIVE V/K-I/HPA-B+ copyout/public-token exposure order and protected receipt status | 002/003/004 |
| SC-O04 | P0 | Define X(q) and A(q) race, result-loss of SKIP, user-space abandonment authority/data-loss semantics | 002/003/009 |
| SC-O05 | P0 | C/B bit reuse and GC proof with pending R[q] and nonwrapping T/R/q identity | 003/009 |
| SC-O06 | P0 | Prove Q/W/claim/fault/lineage queues and sponsor charges cannot eat M0 reserves | 002/004/005/006 |
| SC-O07 | P0 | Model wait-for cycles and safe Pager/IPC/usercopy lock ordering incl. timeout cleanup | 003/004/005/006 |
| SC-O08 | P0 | Define Task death/status rights/supervisor postmortem visibility and privacy ceiling | 002/006/009 |
| SC-O09 | P0 | Precisely document which service effect/receipt status may map to native COMMITTED vs UNKNOWN | 008/009/018 |
| SC-O10 | P0 | Link GrantLineage revocation B/R1 and DMA R2-F/R2-O to queue/receipt retirement | 002/003/007 |
| SC-O11 | P0 | Prevent stale ServiceIncarnation/ReplyToken/policy epoch being promoted via receipt query | 003/006/018 |
| SC-O12 | P0 | Bound “q reused with new business intent” via service-specific idempotency; kernel never claims global exactly-once | 008/009/018 |
| SC-O13 | P0 | Define source Task/receiver Task independent receipt namespaces, message relation and K-U identity | 002/003/009 |
| SC-O14 | P1 | Compare Task-wide and per-lane C/B: gap liveness, lane-generation retirement, isolation/TCB cost | 002/003/005/009 |
| SC-O15 | P1 | Establish quantitative upper bounds for per Task Q/W and finite q/public-token turnover without inventing ABI fields | 002/005/009 |
| SC-O16 | P1 | Define future bounded model state domain and proof artifacts to audit each claim/defeater | 003/004/006 |
| SC-O17 | P1 | Guest Linux fd/OFD/EINTR/restart translation of EXPIRED versus genuine precommit rejection | 009/010 |
| SC-O18 | P1 | Audit durable storage and boot Good/floor terminology to avoid kernel receipt conflation | 006/008/017/018 |

### 8.1 Suggested proof order (design-stage only)

1. **Native identity/receipt layer:** independently model TaskGeneration, q, C/B, A/X, R[q], final GC and duplicate clients; show SC-I01–07/23–24.
2. **Join with capability/Endpoint:** add two source grants, E1 owner MOVE, revoke B, one K-S, confirm ownership and result truth.
3. **Join RECEIVE/Pager:** add ReceiveClaim V/K-I, usercopy fault and Task/Mapping/PagerGeneration; disallow foreign requeue.
4. **Join resource/recovery:** bounded Q/W, sponsor/lineage/queue, two CPUs, M0 reserves and Task death/service restart; show absence of cleanup/dedup/liveness lies.
5. **Boundary to effects:** explicitly treat backend durable effect, Linux guest FD and hardware R2-F as **separately evidenced abstractions**. Kernel-only model cannot establish hardware durability or platform compatibility.

No step above has been run. Selection of TLA+/PlusCal/Alloy/Ivy/model-checking tooling and executable models requires a **separate explicit authorization**; no hidden proof claim is made.

## 9. Decision proposals (SC-D01…SC-D10) — all Proposed, none Accepted

- **SC-D01:** Keep KR-M2 *research leader for multi-Thread workloads*, KR-M1 reference and KR-A nonce/slot alternative; refuse ABI adoption now.
- **SC-D02:** Define C as contiguous **identity consumed** frontier, not completion, notification, cleanup or durability.
- **SC-D03:** Require protected A(q)/X(q) linearization and protected R[q] before irreversible S/I/U; no same-q second admission.
- **SC-D04:** Separate K-S sender Task q, K-I receiver Task q and optional K-U identity even on the same message.
- **SC-D05:** Recheck actual grant, E1 owner and revocation ordering at final irreversible cut or define a provably bounded pre-barrier invocation exception.
- **SC-D06:** For receive, regard V as independent irreversible exposure; no foreign requeue after V, no SKIP of admitted q and no cap reissue through receipt query.
- **SC-D07:** Treat M0 reserve, Pager and faultable usercopy lock/wait graph as admission-level safety constraints, not afterthoughts.
- **SC-D08:** Require separate receipt/queue/claim/lineage/DMA/ServiceId/effect/durability ledgers; GC and Task restart never merge their lifetimes.
- **SC-D09:** Evaluate scoped receipt lanes only if measurable multi-Thread liveness/contended admission demands it; never confuse them with separate security principals.
- **SC-D10:** Maintain cross-RFC OPEN/NO-GO status until independently executed proof artifacts + hardware/effect validation where applicable exist.

## 10. Go/No-Go and artifacts demanded before any future acceptance

**Gate SC-R1: OPEN / NOT PASSED.** All relevant P0 items remain unresolved; paper agreement is insufficient. At minimum demand:

- One mechanically checkable identity+receipt invariant model producing reproducible traces for A/X/C/B/R and result loss.
- A concrete E1/Capability joint commit linearization argument with revoked lineage/Endpoint MOVE and no duplicate cap grant.
- A ReceiveClaim V/K-I/usercopy/VM fault case showing at-most-one-recipient exposure-or-terminal settlement under the explicit trust model.
- Finite upper bounds and fault-injection evidence for Q/W/M0, plus Pager/storage/IRQ wait-cycle analysis.
- A documented Trust/Endpoint/ServiceIncarnation policy explaining scoped receipt query, SKIP, old Task supervision and new Task regrant.
- A falsifiable ABI/error table not equating UNKNOWN/EXPIRED to a precommit error or treating kernel IPC receipt as durable/remote effect evidence.
- For hardware claims, actual IOMMU/DeviceGroup/fence validation; for durability, selected device/file-system protocol and real power-cut/flush evidence.
- Product language explicitly states the receipt's limited **Task-generation and kernel-lifetime scope**, never universal/external exactly-once.

**Current conclusion:** KR-M2 remains plausible but unproved, with no new proven live textual contradiction; several combination assumptions are **not yet defined strongly enough to verify**. Any later accepted ADR must independently show why it dominates KR-M1 in feasibility/security/TCB cost. No development schedule, implementation, tests, measurements or kernel ABI is approved here.

## 11. Official technical references and methods

- [S1] Linux Kernel `copy_to_user()` may fault/sleep and return uncopied length: https://docs.kernel.org/kernel-hacking/hacking.html
- [S2] Fuchsia `zx_channel_read`: whole message at API level, bytes output before handles, documented overlapping buffers: https://fuchsia.dev/reference/syscalls/channel_read
- [S3] Fuchsia Zircon Handle API permits closed integer Handle reuse: https://fuchsia.dev/fuchsia-src/concepts/kernel/handles
- [S4] IETF RFC 6479 anti-replay bitmap (analogy, not kernel cap-transfer proof): https://www.rfc-editor.org/rfc/rfc6479.html
- [S5] IETF RFC 9000 §12.3 packet number uniqueness (analogy): https://www.rfc-editor.org/rfc/rfc9000.html
- [S6] Linux epoll FD/OFD distinction (guest semantics): https://man7.org/linux/man-pages/man7/epoll.7.html
- [S7] RFC ownership, cross-audit, HPA and receipt papers linked in header.

**Evidence limits:** direct GitHub main design-doc inspection and first-party public technical manuals only. No semantic equivalence to external kernel behavior is claimed; counterexample schedules are reasoned, **not executed**. Detailed numerical bounds remain intentionally unset.

---

**History:** 2026-10-25 first cross-RFC Proposed safety case for CAPIPC KR-M2, linking RFC-001/002/003/004/005/006/009 and 007/008/010/018: **18 scoped claims, 24 unexecuted cross-boundary attack schedules, 24 unproved obligations, 18 open blockers, 10 Proposed decisions.** No OS code or stable ABI.

## 12. Follow-up formal-model preparation — 2026-10-25, Proposed only

The follow-on [CAPIPC Formal Model Preparation v0.1](../models/capipc-formal-model-preparation-v0.1.md) maps this review's SC-C/SC-I safety case to future **four-stage small finite models**: KR-M2 protected A/X/C/Used/R, E1 SEND/MOVE and revoke B, receive V/K-I/public token exposure, and resource/Pager/M0/Task restart. It records 21 natural-language action contracts, 28 *unproved* invariants, 8 conditional liveness goals and 20 paper-only counterexample seeds. Proposed tooling is TLA+/TLC first, with independent Apalache/Alloy research. This is a **preparation only** pointer: no checker/spec source, run, proof, ABI freeze, accepted ADR or actual UniOS implementation. The parent safety-case **NO-GO** remains unchanged.

## 13. Follow-up FM-O01: SEND versus revoke/Endpoint owner ordering — Proposed

The independent [CAPIPC Commit, Revocation Barrier & E1 Endpoint Ownership Ordering v0.1](../models/capipc-commit-revocation-ordering-v0.1.md) deepens SC-C04/SC-I08–10 and SC-O02. Under conditional CO-S, K-S before the relevant B/E1 ownership-cut commits whole grant vector; B/E-M before K-S precludes a fresh old-owner transfer and requires honest precommit settlement. A(q) by itself is only receipt-identity admission. CO-E-C cancel/settle-before-effective-owner-handoff is a research candidate; queued Endpoint InTransit may have no public Task owner until K-I. **New wording precision:** SEND `NOT_COMMITTED` preserves original source entries against consumption *by that SEND*, but cannot undo independent CLOSE, MOVE or revoke. Both designs remain **unproved, NOT Accepted**; original review's Open/NO-GO finding remains.

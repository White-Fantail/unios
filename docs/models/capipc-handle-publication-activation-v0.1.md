# UniOS CAPIPC — Handle Publication, Receive Commit & Capability Activation v0.1

> **Status:** Proposed Independent Design Study — 2026-10-25 NZDT. **NOT Accepted, implemented, executable, tested, modeled in TLA+/Alloy, formally verified, ABI-frozen or benchmarked.**
> **Context:** [CAPIPC Unified SEND–RECEIVE–REVOKE State Model v0.2](capipc-send-receive-revoke-state-model-v0.2.md), [CAPIPC integration contract v0.2](../contracts/capability-ipc-sabi-integration-contract-v0.2.md), [RFC-002](../rfcs/rfc-002-object-capability-lifetime-delegation-revocation.md), [RFC-003](../rfcs/rfc-003-ipc-wire-protocol-and-synchronization.md), [RFC-009](../rfcs/rfc-009-native-abi-and-system-call-contract.md), [RFC-018](../rfcs/rfc-018-versioned-idl-abi-evolution-and-compatibility-contracts.md).
> **Dependencies:** RFC-004 usercopy/Pager/VM, RFC-005 CPU, RFC-006 wired M0/service restart, RFC-007 DMA R2-F, RFC-008 effect/durability, RFC-010 Linux guest and RFC-016 authenticated consent.
> **Scope:** Research and documentation only. No new system calls, C structs, handle widths, binary wire encoding, code, runnable model or test. This is an adversarial ordering study, not a tested implementation recipe.

## 0. Executive conclusion and proof limit

**Preferred next research candidate: HPA-C — two-phase receiver claim with kernel-private grant escrow, pre-reserved never-reused public token values, staged user-visible outputs and an explicit authenticated all-or-none activation/receive-commit.** Keep HPA-A (activate/install-first), HPA-B (token-first single syscall) and HPA-R (tiny register-only/capless path) as meaningful lower-cost countermodels. HPA-C is not necessarily the fastest or globally optimal and is NOT selected for final UniOS ABI.

The strongest defensible claim is **no native rights usable before an explicit kernel activation cut** and **no partial activation of a moved cap vector** under a correctly scoped claim. It does NOT guarantee that the initiating userspace Thread observed return SUCCESS, that all buffer bytes are immutably retained by that Thread, that no user-visible payload leaked on failure, or that remote effects were not applied. Kernel result and user observation are different events.

**Important naming:** HPA-C's activation event **is RFC-003's candidate K-I receive commit** (whole-vector installation + queue/claim retirement, all-or-none) for T1 bearer-cap receive; it does not add a second hidden success/atomicity layer. Before K-I, the entire moved grant vector remains in **kernel-only, noninvokable claim escrow**, even if numeric tokens have been copied to user memory. In optional RCV-T2, K-I can install a *separately modeled inert receiver escrow/lease* and a later independently authorized **K-U** decides native invokability. Whether this T2 form still satisfies the RFC-003 wording of “cap installation” requires its own type distinction and remains OPEN.

**Crucial impossibility boundary (informal, NOT a theorem):** kernel cannot atomically make a series of user-owned, concurrently readable and possibly pageable memory stores, an internal capability-table transition and the caller's subjective knowledge of success into a single observable event using an ordinary copyout interface. Even a register-return result can be lost if a Thread terminates after the kernel commits. Therefore the design must distinguish **kernel actual state**, **user memory visibility**, **rights invokability** and **caller knowledge** and allow an honest UNKNOWN with scoped reconciliation.

## 1. Precise terms and independent cut points

| Name | Meaning | What it must not imply |
|---|---|---|
| E1 Endpoint | distinct sender/receiver kernel endpoint halves, one public owning Task per half | the two halves share one owner, or a human identity |
| K-S | prior sender queue append + complete cap-vector MOVE, source handles irrevocably invalidated | user received message; receiver installed handles |
| K-C | exclusive claimed receiver TaskGeneration+EndpointGeneration+audience; reserved bounded escrow | bytes published, cap installed or release permitted |
| K-Vm / K-Vp | possibility of confidential metadata/payload exposure to claimed Task, marked before corresponding publication | actual number of bytes read, ability to retract exposure |
| K-HR | reserve never-reused destination public token identities in receiver Task namespace, **inert** until K-I | user-owned copies of numbers already authorize cap grants |
| K-HP | user-memory publication of one or more reserved numeric token values (possibly partial) | kernel cap-table entries installed or entire array seen |
| K-STAGE | kernel records successful staged output-copy attempt for a specifically bound claim and immutable descriptor identity | reader saw the values, or memory cannot subsequently change |
| K-I | T1 all-or-none receiver **live** cap table install + receive queue/claim retire at one kernel cut | initiating Thread received a success return; external effect absent |
| K-U | optional T2 activation of already independently validated inert grants | an untrusted B asserted IDL validation correctly |
| R-B | capability family revocation barrier published | prior admitted work/CPU mapping/DMA/storage write undone |
| K-Q | Task-generation scoped query of preidentified claim/receive result record | status alone proves remote service effect or storage durability |
| K-T | terminal settlement if claimed payload/secret metadata possibly exposed and cannot complete | no actor observed bytes; device R2-F done |

**Distinguish K-HP from K-I:** public token bits are lookup keys, NOT native rights in themselves. Until a live CapGrant occupies the kernel Task namespace slot for that token and passes all rights/lineage/activation checks, using those bits must fail closed. A token that might have appeared in user memory cannot ever later refer to an **unrelated** native grant in the same Task lifetime even if the first receive failed.

**Distinguish K-I from K-U:** HPA-C's explicit AcceptCommit authorizes ordinary T1 **bearer receipt**. It is NOT a trusted method-verification proof. Optional T2 continues to hold cap rights inert until an independently trusted authority issues a scoped release and K-U validates live rights/policy/revoke at its own cut.

## 2. Candidate comparison — do not equate UX with proof

| Family | Order (logical only) | Strength | Mandatory drawback |
|---|---|---|---|
| **HPA-A — live install first** | K-C → K-I live install → user token array/output copy | simple kernel grant movement, fewer explicit steps | installed orphan rights and partial token output; concurrent other Task Thread might use published number while primary caller sees error |
| **HPA-B — one-call publish first** | K-C → reserve numbers → user copyout → K-I activation before syscall return | caps not active on failed copyout; closer to low-overhead T1 | some reserved numbers may leak, result can still be lost after K-I; must account for token tombstones, claimant changes and output buffer races |
| **HPA-C — explicit two-phase receipt (preferred to study)** | K-C + reserve token IDs → output stage (K-V, K-HP) → K-STAGE → separately scoped explicit K-I activation → protected K-Q | cleanly separates partial output failure from live rights; preidentified result allows reconciliation when activation return is lost | extra kernel state, operation/round-trip, queue hold and timeout; malicious claimant DoS; no guarantee Task Thread read bytes |
| **HPA-R — tiny register return** | limited bytes/no cap or very few independently specified cap identifiers returned via registers | bypasses bulk copyout for restricted path | ABI/architecture-specific; result or context still may be lost; not generic resource-bearing SABI |
| **HPA-T2 — verified inert escrow** | stage and reserve as C, then separately trusted verifier decides K-U | can limit native rights until independently checked method policy | trusted verifier/issuer expands TCB and may deadlock or fail; K-V disclosure still irreversible |

**Why HPA-C instead of immediate HPA-B default?** HPA-B could be viable for general IPC if the entire failure graph is provable and protected reconciliation works. HPA-C provides a *more explicit* activation cut for formal reasoning, but a real OS may prefer a carefully proven single-call fast path because Claim/Commit IPC overhead and resource pressure are significant. Do not make performance/latency assumptions without benchmarks.

## 3. Proposed HPA-C custody and execution-order contract

### 3.1 Publicly visible concepts, NOT syscall API declarations

- **Preidentified ClaimReference**: a unique, nonrecycled opaque Task-generation scoped claim or a previously authorized preallocated Task-local slot. It must exist or be discoverable **before a result that could be lost**, not be created only in a lost return register. A raw user-supplied number is never itself a grant.
- **StagedOutput**: snapshot of bounded payload, metadata and reserved token values, tied to one message, recipient Task+Endpoint generation, issuer-defined confidential audience and claim generation. Userspace may see only a prefix after a copy fault.
- **AcceptCommit**: an explicit invocation from a Task genuinely authorized to accept that ClaimReference; after live rights/current issuer checks it performs a single kernel **K-I** (for T1); repeated commits cannot install a second copy or double-dequeue the message.
- **QueryReceipt**: privileged Task-scoped observation of actual claim and K-I result, with anti-ABA, privacy and bounded retained metadata. A QueryReceipt can recover what the kernel committed, **not** whether a user Thread read the buffer or whether an external backend completed.
- **Terminalize/Cancel**: if K-Vm/K-Vp already potentially disclosed secrets, the same message may not be silently returned to a different audience; settle/close/quarantine grants, preserve debt and a bounded outcome record. Before exposure requeue is conditional on current issuer/endpoint rights.

None of these names fixes the count of KABI system calls. They could be separate operations, a single call with preallocated Task record and an explicit second admission, or other interfaces preserving the *observable contract*. They are not function signatures or protocol ordinals.

### 3.2 Finite state path under normal success

~~~text
Sender A: native caps G moved at K-S, original public handles invalid
  |
Kernel queue: Message M owns G and bytes, bounded sender-sponsored debt
  |
Receiver B: validates RECEIVE + authorized audience + EndpointGeneration
  |
K-C: exclusive ClaimReference (already known or safely discoverable)
  |
K-HR: reserve distinct B Task-local public token values, not invokable
  |
K-Vm / K-Vp: mark potential exposure before each sensitive publication
  |
K-HP: copy user payload, reserved token values, authorized metadata
       (may fail partly; reserved tokens remain INERT)
  |
K-STAGE: kernel notes complete copy ATTEMPT against claim
         (NOT proof user Thread actually observed or retained data)
  |
Receiver B: explicit AcceptCommit of live authorized ClaimReference
  |
K-I: atomically recheck Endpoint/Task/lineage/revoke/issuer limits
     + install ALL G into B Task cap table + retire queued claim
     + mark receiver record Committed (possible lost result)
  |
K-Q: scoped protected status query/reconcile if return/report lost
  |
SABI: B may validate method; actual bearer caps are already usable.
Optional T2: inert receipt + independently trusted validation -> K-U.
~~~

**At K-C and K-HR** kernel queue still owns the moved grants (with an exclusive claimed/escrow custody state) and B owns NO active native grants from this message. Claim identity lives in a Task-generation scoped namespace; the data vector in user memory is not a capability. No arbitrary SABI parser is part of Ring-0 enforcement. A client may concurrently modify its own result buffer after K-STAGE; this is a receiver responsibility, not permission to use invalid or stale token numbers.

### 3.3 Edge-case table: errors and fixed custody

| Cut of failure | Custody and token state | Exposure and reuse policy | Truthful observer report |
|---|---|---|---|
| Before K-C | kernel queue still owns grants; no reserved token | no bytes/authorized metadata yet disclosed | no claim / possibly WOULD_BLOCK |
| After K-HR, before any K-V | grant vector held by claim; reserved token values inert | if claim aborts, **burn/retire** reserved token identities; requeue only under still valid rights | claim aborted before exposure |
| K-HP partial user copyout | claim still owns all G, some token bytes may be visible but inert | **never reuse** any potentially published token values; payload exposure state governs queue terminal vs permitted same-B continuation | PARTIAL_OUTPUT_POSSIBLE, no active grant yet |
| K-STAGE succeeded but return lost | claim holds G, reserved tokens inert | QueryReceipt by authenticated B/slot; no blind new receiver | STAGED or UNKNOWN to initiating Thread |
| K-STAGE succeeded, B no longer authorized due revoke/Endpoint MOVE | no K-I, escrow remains until bounded settlement | terminal if prior exposure; requeue only if proven no exposure | DENIED_OR_TERMINAL (not installed) |
| K-I live install committed; result lost | B owns all installed live grants; queue retired; reserved token values now authoritative | previously copied numbers may be usable by another B Thread; status query needed | COMMITTED actual / observer UNKNOWN until reconciled |
| B dies after K-I but before query | Task cap table + outstanding invocation refs cleaned by controlled death | no source rollback; effect and disclosure may survive | receiver dead; sender effect unknown |
| T2 verification succeeds but R-B occurs before K-U | cap grants remain inert or settle; no new use | cannot activate based on stale approval | REVOKED/REVALIDATE |
| Kernel queue/claim expires after K-V | terminal settlement, all grants accounted/closed/quarantined | no automatic delivery to new C/B2 | TERMINAL; cannot prove zero-byte disclosure |
| DeviceCap dropped after terminal | pin/IOVA/IRQ sponsor debt may survive | no automatic DMA fence or object-wide reuse | R2-F pending/quarantined |

### 3.4 Multiple Threads and the impossible "only the initiating Thread knows" promise

If B's Task has multiple Threads sharing user memory and Task-local public handles, another B Thread may read copied token values as soon as K-HP occurs and may invoke them **after** K-I even before the original syscall's return is observed. This is *not* unauthorized cross-Task grant escalation: the issued cap is Task-scoped. If a service needs one Thread/one principal consent binding, Task-wide bearer semantics are insufficient; it needs independently enforced per-call or narrower authority, not a false promise about the timing of the return instruction.

User memory may be unmapped or overwritten immediately after K-STAGE, so an explicit AcceptCommit does not prove the receiver retained the bytes. The kernel only proves that an authorized Task explicitly requested activation while the correct protected claim was live.

## 4. A/B comparison of commit rights, receiver exposure and policy

### 4.1 B1 publish-first then K-I and B2 install-first

HPA-B is *not* automatically insecure because public token values appear before K-I: these values have no native authority without installed live table entries. Its main obligations are nonreuse, commit-result recovery and endpoint/revoke races. HPA-A is *not* automatically insecure because a Task intentionally authorized by the sender receives real native rights at K-I: it is unsafe only if the service contract falsely claims the rights remain inaccessible until method validation or that failed output cannot leave usable rights. The choice must follow the actual threat/security claims, not a universal preference for more IPC rounds.

### 4.2 T1 bearer grants and T2 authenticated resource admission

- **T1 receiver trust:** before K-S the sender/issuer must have attenuated G to exactly the authority B may possess **even if method decode fails**. Its RequestId/IDL method cannot restore revoked rights or add missing native cap rights.
- **T2 special promise:** HPA-C activation may prepare a *noninvokable* installed/escrow structure rather than live bearer grants; activation of native rights is deferred to **K-U** after a genuinely independent trusted verifier endorses current actual grant set + method resource roles + issuer policy epoch + specific claim/recipient + generation. A compromised B's self-reported accepted flag is invalid.
- **Revocation during validation:** if K-U loses race with revoke B, no new invokability. If it wins and a downstream invocation is admitted before B, R1 and DMA R2-F and remote effect obligations remain independent.
- **Status:** both T1 K-I and T2 K-U may succeed while their return values are lost; preidentified, scoped query or honest UNKNOWN still required.

### 4.3 Metadata, publication and Audience

K-Vm must account for confidential length/count/type/rights/status metadata when issuer classifies it as private. K-Vp precedes any possibly observable payload store/map. K-HP may reveal numeric token values and authorized native cap metadata even if K-I is denied; such publication must be charged to the same recipient audience and possibly terminalize redelivery if metadata is secret. Count/size errors are not automatically safe public oracles. Under Endpoint MOVE, Task death, policy floor change or ServiceIncarnation transition, revalidate the claim's intended audience before each new disclosure and at K-I.


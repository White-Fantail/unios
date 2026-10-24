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


## 5. Finite custody state and guarded transitions — paper model only

This is **human-readable contract research, not runnable pseudocode**. B is the legitimate receiving Task, C a different Task, B2 a later service incarnation, R a grant controller. A Channel contains two **distinct E1 Endpoint halves**. Only the intended receiver Task-generation may accept a claimed message. A message has at most two MOVEd grants and one bounded queue slot.

| State | Actual custodian | User token bits | Native cap usability |
|---|---|---|---|
| HS-00 Queued | kernel Channel queue | none | no new B grants |
| HS-01 Claimed | kernel exclusive receiver claim escrow | none | inert |
| HS-02 Reserved | same claim, charged fixed recipient token IDs | token values not yet published | inert |
| HS-03 Exposed | claim; first sensitive store/map may have happened | none/some/all copied | inert |
| HS-04 Staged | claim; complete output-copy attempt recorded | all copied *when attempted*, can later mutate | inert |
| HS-05 CommittedT1 | B Task owns complete vector; queue retired | reserved IDs resolve to native B grants | all-or-none live if not revoked |
| HS-06 InertT2 | receiver-specific escrow, issuer validated | reserved IDs can be visible | inert until independent K-U |
| HS-07 ReleasedT2 | B live vector after trusted release | same appropriate public IDs | current scoped grants live |
| HS-08 Terminal | all grants closed, pending cleanup, or charged quarantine | old IDs never reused for unrelated grants | none newly activated |
| HS-09 ObserverUnknown | **epistemic condition only**, not another actual kernel ownership state | any | query or fail closed |

Expose and terminal flags are orthogonal to custody state: a message can be terminalized after some data was observable but before any cap becomes active.

| Step | Mandatory guard / event | Authoritative transition |
|---|---|---|
| HP-01 | valid current ReceiveHalf owner and issuer-approved audience; protected slot preidentified | establish K-C exclusive claim |
| HP-02 | Task public token capacity + escrow memory/queue credits available | K-HR reserve distinct nonrecycled inactive destination token IDs |
| HP-03 | authorization for confidential size/type/rights data, same Task/Endpoint epoch | K-Vm mark possible metadata visibility prior to disclosure |
| HP-04 | immutable bounded bytes staging and valid audience, before first payload/map store | K-Vp mark possible byte disclosure |
| HP-05 | claimed receiver output ranges/alias policy and M0-safe usercopy | K-HP may copy some or all tokens; token bits **do not activate rights** |
| HP-06 | entire staged output copy attempt completed; claim still current | mark K-STAGE; cannot assert caller Thread read or retained output |
| HP-07 | explicit receiver AcceptCommit on genuine scoped claim; owner/rights/lineage/revoke and quota current | **K-I** installs entire native T1 grant vector AND retires queue/claim atomically, or none |
| HP-08 | optional T2 independent trusted release on exact cap vector and latest PolicyEpoch | K-U activates only approved inert grants after live B recheck |
| HP-09 | authenticated preidentified Task scoped claim-slot result query | K-Q returns actual kernel K-I/terminal truth, not external effect truth |
| HP-10 | claim abort before any secret metadata/payload exposure, original eligibility still valid | optional bounded requeue; burn token reservations |
| HP-11 | exposure possible or receiver death/owner changed after exposure | K-T terminal settlement, **never** deliver same message to C/B2 automatically |
| HP-12 | B revoke wins before K-I/K-U | no new native authority from revoked lineage; escrow settles |
| HP-13 | K-I wins before B, then controller publishes revoke | existing admitted work handled by R1/R2-F and effect receipt separately |
| HP-14 | stale claim or resource cleanup cannot finish, device fence pending | sponsor-charged quarantine/bounded M0 cleanup, no false R2-O |

K-I and R-B must have one defined linearization order. K-I versus terminalization and two concurrent AcceptCommit calls must also have a **single winner**. Claim reservation and output preflight cannot be assumed to pin mutable user mappings forever; however usercopy must never run under a global kernel lock which can deadlock the Pager. K-STAGE does not imply the initiating Thread read any value: a malicious Task can AcceptCommit without looking, and another Thread can copy the token array.

### 5.1 Receive result and lost-return contract

A ClaimReference must be *known before the vulnerable return* (a preauthorized Task-local slot) or recoverable through authorized bounded enumeration. A fresh ClaimID only supplied in a result whose delivery can fail is **not** sufficient. Once K-I commits, losing the AcceptCommit return cannot undo grants. K-Q can truthfully answer whether K-I happened for that scoped claim, but cannot prove the original Thread observed success, nor that a remote write was durable. Query must enforce Task generation and ClaimReference anti-ABA, message secrecy, bounded retained receipts and expiration. An expired ticket must never be recycled into an unrelated operation.

### 5.2 Public token exhaustion and denial of service

A malicious receiver can repeatedly stage a message, cause a fault after just one token-number store, abort, and attempt again. Strict never-reuse of exposed token identities can consume a finite public-token namespace without transferring a single live cap. **Fail-closed token exhaustion is a safety result**, but usable desktop availability requires proof of admission budget, expiration/terminalization, aggressive prevention of unnecessary publication and a sufficiently large Task token space. Never silently weaken anti-ABA to rescue performance.

## 6. Counterexample schedules (HA-01…HA-20) — none executed

| ID | Adversarial schedule | Forbidden result |
|---|---|---|
| HA-01 | B copies token #1, faults on token #2 before K-I | #1 later used to authorize unrelated grant |
| HA-02 | Another B Thread tries published token before K-I | unauthorized native use from inert identifier |
| HA-03 | K-STAGE completes; B mutates/unmaps output before AcceptCommit | proof that initiating Thread retained values |
| HA-04 | K-I commits then return report lost | false no-install result / source rollback |
| HA-05 | New ClaimID generated only in lost return path | claim magically discoverable |
| HA-06 | Two B Threads simultaneously AcceptCommit one claim | duplicated cap vector or double-dequeue |
| HA-07 | B holds queue claim forever without accepting | unbounded queue/Pager/M0 starvation |
| HA-08 | ReceiveHalf moved B→C during stage before K-I | stale B receives active caps without new authorization |
| HA-09 | B saw payload prefix; claim terminated; B2 started under same ServiceID | automatic confidential message replay to B2 |
| HA-10 | Controller B barrier revokes G2 just before K-I on [G1,G2] | half-vector install or newly usable revoked G2 |
| HA-11 | Controller B barrier after K-I but before new invocation | old Task allowed new revoked use merely due earlier receive |
| HA-12 | T2 verifier authorizes claim X, attacker reuses permit for Y | wrong vector or wrong Endpoint rights release |
| HA-13 | T2 verifier accepts P0, new issuer floor P1 before K-U | stale permit activates revoked/forbidden right |
| HA-14 | guest reads message size without RECEIVE rights | metadata confidentiality breach via error/status |
| HA-15 | byte/count/token buffers alias same backing pages | output corruption despite distinct virtual address ranges |
| HA-16 | B Task killed, token bits copied, Task ID reused by B2 | token/claim/receipt valid in unrelated Task generation |
| HA-17 | A sender dies after K-S while B stages | moved cap falsely returned to A; transit charge lost |
| HA-18 | repeated aborted claims drain public handle token namespace | reuse vulnerable stale token rather than fail closed |
| HA-19 | terminated claim closes DeviceCap while DMA pin still active | R2-F/R2-O falsely declared / frame prematurely recycled |
| HA-20 | guest errno after K-I result lost implies no rights/effect | unauthorized blind retry or hidden native grants |

## 7. Safety invariants and research decisions

### 7.1 Twenty-four unproved properties (HA-I01…HA-I24)

| ID | Unproved property |
|---|---|
| HA-I01 | Each E1 half has one effective owner; SendHalf and ReceiveHalf distinct. |
| HA-I02 | K-S source MOVE remains committed even on later receive/activation errors. |
| HA-I03 | Reserved Task-public token bits are never reused for an unrelated grant after possible publication. |
| HA-I04 | An inactive published token number cannot invoke any native cap before authorized K-I/K-U. |
| HA-I05 | K-I installs the entire moved grant vector in B or none and retires claim/queue in one cut. |
| HA-I06 | Terminalization and K-I cannot both commit for one message/claim. |
| HA-I07 | Concurrent AcceptCommit attempts cannot duplicate grants or create second receiver. |
| HA-I08 | K-I must revalidate current Task/Endpoint generations, cap revocation and grant rights. |
| HA-I09 | R-B revocation and K-I/K-U each have a definable total order and no post-B new authority. |
| HA-I10 | Lost K-I result does not falsely signal no installation; protected query or Unknown. |
| HA-I11 | A query ticket is preidentified or securely enumerable; lost newly minted ID alone is not sufficient. |
| HA-I12 | Claim/status query cannot access another Task's results or fresh Task via ABA. |
| HA-I13 | K-STAGE is copy-attempt completion, not a guarantee of user observation/retention. |
| HA-I14 | Sender and receiver KABI states are not equivalent to external SABI effect completion. |
| HA-I15 | K-Vm/K-Vp guard sensitive metadata and payload exposures before publication. |
| HA-I16 | After exposure to B, same message is not silently requeued to different C/B2 audience. |
| HA-I17 | Current issuer-approved audience is pinned/rechecked on Endpoint MOVE and service restart. |
| HA-I18 | T1 bearer rights are limited to grants B is allowed even if schema invalid. |
| HA-I19 | T2 K-U requires independent trusted verifier of live kernel grant identity and current policy. |
| HA-I20 | Inert escrow and T1 staging are not equivalent to IDL method authorization. |
| HA-I21 | Unreported live rights, terminal tombstones and unfenced DMA remain sponsor charged. |
| HA-I22 | Handle close/terminal receive never proves device R2-F or storage effect rollback. |
| HA-I23 | Critical M0 CPU/IPC/memory reserve not exhaustible by staged or abandoned claims. |
| HA-I24 | Every property remains unproved and does not freeze code, a syscall or public ABI. |

### 7.2 Open research gates (HA-O01…HA-O14)

| ID | Priority | Question |
|---|---|---|
| HA-O01 | P0 | Can reserved token values be permanently nonreused and sponsor charged under Task exhaustion? |
| HA-O02 | P0 | Is K-I all-or-none linearizable with R-B, owner MOVE and claimant death? |
| HA-O03 | P0 | Is a preidentified scoped ClaimReference recoverable even when its initial reply is lost? |
| HA-O04 | P0 | What explicit actor may AcceptCommit Task-scoped rights, and do identity-sensitive methods need narrower issuer consent? |
| HA-O05 | P0 | How can copyout preflight/alias/Pager fault avoid global lock and partial token leaks? |
| HA-O06 | P0 | Are staged queue slots and critical M0 costs bounded against malicious claimant stalling? |
| HA-O07 | P0 | Does K-STAGE status semantics remain truthful under concurrent user-memory mutation? |
| HA-O08 | P0 | How are caps, escrow and queue charge/accounting retained across sender/receiver death? |
| HA-O09 | P0 | What is the exact separate trusted T2 verifier/issuer model at K-U vs B revoke? |
| HA-O10 | P0 | Can terminalize-on-exposure avoid foreign redelivery without promising lossless availability? |
| HA-O11 | P0 | Can HPA-C justify extra TCB and IPC relative to HPA-B without measured latency/memory budget? |
| HA-O12 | P0 | How can signed/current issuer policy bind the Task+Endpoint audience without kernel SABI parser? |
| HA-O13 | P1 | Which register-only fast path and batching alternatives preserve the core contract? |
| HA-O14 | P1 | How should guest Linux fd/errno results report completed K-I with a lost userspace result? |

### 7.3 Proposed decisions (HA-D01…HA-D12) — NONE Accepted

- **HA-D01:** Keep E1/typed native Capability with full-vector MOVE as proposed baseline.
- **HA-D02:** Study HPA-C as the first explicit activation cut model; not a chosen future shipping API.
- **HA-D03:** Preserve HPA-A/B/R as serious candidates and compare TCB/performance without made-up measurements.
- **HA-D04:** Reserve destination token numbers as inert before publication and never recycle possibly published ones.
- **HA-D05:** Keep G in kernel escrow during HPA-C stage; source sender never regains G after K-S.
- **HA-D06:** Treat T1 AcceptCommit's K-I as **the** RFC-003 atomic receive install/dequeue cut.
- **HA-D07:** Use an authenticated preidentified Task claim/slot or safely scoped enumeration for lost return.
- **HA-D08:** Do not call K-STAGE proof that any user Thread observed the payload.
- **HA-D09:** Enforce monotonic K-Vm/K-Vp potential exposure and no automatic foreign requeue.
- **HA-D10:** Distinguish T1 bearer acceptance from optional T2 trusted K-U authorization.
- **HA-D11:** Keep terminal, orphan, token and DeviceGroup debt sponsored to true cleanup and fencing.
- **HA-D12:** Keep all decisions Proposed, with no code, executable model, benchmark, syscall signature or ABI freeze.


## 8. Source-RFC alignment and compatibility caveats

| Existing Proposed document | Compatibility finding | Action to carry into next design review |
|---|---|---|
| RFC-002 §7/§9 | Public Task Handle anti-ABA and true MOVE ownership are aligned with inert reserved token identities | prove burned tokens on failed stage, distinct lineage/current sponsor, cap invoke after K-I |
| RFC-003 §§5–6/§23 | all-or-none cap install + queue retirement and ReceiveClaim remain coherent IF activation K-I is the sole live install cut | clarify HPA-C staging/claim may exist before K-I; explicit successful receive or terminal failure; do not requeue after exposure |
| RFC-003 §13/§14 | BUFFER_INVALID/UNKNOWN and IPC-06/09/28 do not fully describe staged token publication | identify additional logical, not numeric, states TOKEN_RESERVED, PARTIAL_PUBLISHED, STAGED, COMMIT_UNKNOWN, RECEIPT_QUERY and TERMINAL |
| RFC-004 Pager/VM | public buffer validity at preflight does not make future usercopy atomic; shared aliases and page faults possible | bounded staging, no global lock across usercopy and conditional admission/terminalization |
| RFC-005/006 | two-phase staging holds claims longer and adds quota/verifier critical path | finite CPU, IPC, timer, M0 reserve and fail-stop model |
| RFC-007 | closure of DeviceCap after terminalized receive cannot imply IOMMU DMA quiescence | preserve R2-F fence/quarantine and sponsor charge separate from token close |
| RFC-008 | K-I/QueryReceipt reports kernel grant commit, not FS/remote effect | separate durable backend journal, IO completion and scoped File/Directory receipt |
| RFC-009 §§5–7/§25 | ABI copyout and status lost after irreversible transition are known P0 issues | do not prematurely choose numeric KABI, syscall layout or register result semantics |
| RFC-010 | guest Linux FD/errno/signal may lack exact native pending claim state | independently scope guest profile or refuse unsupported mapping rather than fake success/no-effect |
| RFC-018 | protocol role/issuer PolicyEpoch checks remain in user space | T1 cap grant already authorized on bearer receive; T2 independent verifier, never generic evolving IDL parser in kernel |

**Important contract preservation:** under HPA-C for T1, K-I MUST atomically install the complete cap vector **and retire the queue claim/message** to honor RFC-003's all-or-none receive contract. Intermediate staging is kernel custody, not a completed Receive. On post-exposure terminalization an explicit *separate* discard event removes the message with caps closed/quarantined, not a half-successful K-I. A stale ticket or partial user buffer is not an independent Native Cap grant.

## 9. Audit exit, performance feasibility and research roadmap

**HPA-R1: OPEN / NOT PASSED.** A design-paper preference is not mathematical proof, a running kernel or a supported ABI.

1. Prove the never-recycled public token namespace under Task lifetime, numeric exhaustion and duplicate slots; must be feasible on bounded x86_64 profile or refuse grant requests safely.
2. Choose precise K-I ordering with Endpoint MOVE, revocation B, claimant death, double AcceptCommit and current issuer audience, including release of precommitted reserved quota.
3. Demonstrate preidentified ClaimReference discovery if *initial claim result itself is lost*, not just final AcceptCommit return; define permission to query and tombstone lifetime.
4. Model byte/metadata/token exposure and usercopy faults with unmap, remap, aliases, concurrent Threads and Pager; no hidden global lock or statement of atomic multi-store.
5. Prove one queue record/one custodian at all times: sender K-S committed before receiver Claim; token stage cannot move actual native grant; K-I installs all caps or none; K-T terminalizes separately.
6. Define T1's maximum authorized bearer rights independent of future SABI decoder and T2's separately trusted verifier/release cut, including current policy and revoke B.
7. Prove finite budgets for K-C, staged output, burned token reservations, pending receipts and orphaned G, plus M0 resident IRQ/IPC/CPU cleanup under malicious repeated aborts.
8. Distinguish K-I success from initiating Thread observing results and from actual file/network effect, then demonstrate truthful guest Linux fallback/refusal.
9. Compare T1 HPA-B one-call candidate against HPA-C two-phase on **named** microkernel TCB cost, CPU round trips, queue head-of-line blocking and copyout failure frequency. There are no measurements yet; don't assert HPA-C is automatically best.
10. Obtain separate review of updated RFC-002/003/009/018 Proposed contracts and resolve new status/receipt definitions without freezing ABI numbers.
11. Any executable finite model, simulator, QEMU harness or kernel implementation requires *new explicit approval* beyond this research document.

**Prioritized next step:** independently adversarially review this HPA-C proposal, especially **lost initial ClaimReference, public token exhaustion, same-Task malicious concurrency, and staged queue starvation**. If HPA-C's TCB and liveness cost cannot be kept within the independent M0 reserve, a simpler HPA-B with honest ambiguity and protected Task-level reconciliation may be superior for the desktop IPC baseline.

## 10. Technical references and evidence limits

- **[S1] Fuchsia Channel kernel object:** https://fuchsia.dev/fuchsia-src/reference/kernel_objects/channel — different established atomic message/handle transfer design and nested Channel references.
- **[S2] Fuchsia zx_channel_read:** https://fuchsia.dev/reference/syscalls/channel_read — byte output before handles; buffer overlap; size/count results.
- **[S3] Fuchsia zx_channel_read_etc:** https://fuchsia.dev/reference/syscalls/channel_read_etc — kernel-sourced object type/rights in receiver output.
- **[S4] Fuchsia zx_channel_write:** https://fuchsia.dev/reference/syscalls/channel_write — basic write consumes handles even on failure, unlike UniOS proposed true precommit NOT_COMMITTED.
- **[S5] Fuchsia Zircon handles:** https://fuchsia.dev/fuchsia-src/concepts/kernel/handles — Task-local handle values, in-transit ownership and object lifetime.
- **[S6] Linux kernel User Access documentation:** https://docs.kernel.org/kernel-hacking/hacking.html — copy_to_user/copy_from_user may leave uncopied data and can sleep; ordinary multi-value usercopy not an all-or-none transaction.
- **[S7] Linux kernel Memory Management API:** https://docs.kernel.org/core-api/mm-api.html — user access can fault/sleep under allowed context.
- **[S8] seL4 IPC tutorial:** https://docs.sel4.systems/Tutorials/ipc — explicit receiver CSpace path and IPC buffers as contrasting architecture, not E1 semantics.
- **[S9] Fuchsia FIDL wire format:** https://fuchsia.dev/fuchsia-src/reference/fidl/language/wire-format — serialized handles are represented separately from actual kernel handle vector.
- **[S10] Fuchsia zx_channel_read_etc rights/type validation advice:** https://fuchsia.dev/reference/syscalls/channel_read_etc — untrusted sender resource claims cannot replace actual cap metadata.

**Reference interpretation limits:** These links document *other* kernels and APIs. Neither Linux usercopy semantics nor seL4 and Fuchsia can prove a native UniOS receive cut. Some Fuchsia semantics explicitly differ from the UniOS design and must not be silently imported. The proposed two-phase HPA-C protocol, resource bounds, exact kernel code/locking behavior and real latency have **not** been modeled or measured.

---

**Document history:** 2026-10-25 Proposed v0.1 specialized research on exact Handle Publication versus Capability Activation order, following [CAPIPC state model v0.2](capipc-send-receive-revoke-state-model-v0.2.md). HPA-C preferred for *proof research*, with HPA-A/B/R maintained as countermodels. Twenty (20) unexecuted adversarial schedules, twenty-four (24) unproved safety invariants, fourteen (14) open research blockers, twelve (12) Proposed decisions, and fourteen (14) guarded transition candidates. No OS/source code, executable model, formal proof, ABI stabilization or performance test.

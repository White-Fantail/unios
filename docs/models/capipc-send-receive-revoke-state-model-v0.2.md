# UniOS — CAPIPC SEND–RECEIVE–REVOKE Unified State Model v0.2

> **Status:** Proposed Adversarial Re-Review (2026-10-25 NZDT), **NOT Accepted, proven, implemented, tested, benchmarked or stable ABI**.
> **Prior version (preserved):** [Unified State Model v0.1](capipc-send-receive-revoke-state-model-v0.1.md).
> **Required cross-reference:** [CAPIPC v0.2](../contracts/capability-ipc-sabi-integration-contract-v0.2.md), [RFC-002](../rfcs/rfc-002-object-capability-lifetime-delegation-revocation.md), [RFC-003](../rfcs/rfc-003-ipc-wire-protocol-and-synchronization.md), [RFC-009](../rfcs/rfc-009-native-abi-and-system-call-contract.md), [RFC-018](../rfcs/rfc-018-versioned-idl-abi-evolution-and-compatibility-contracts.md).
> **Other dependencies:** RFC-004 Pager/VM, RFC-005 scheduling, RFC-006 M0 recovery, RFC-007 DMA, RFC-008 durability, RFC-010 guest semantics and RFC-016 consent.
> **Scope:** Human-readable design research ONLY. No OS code, runnable pseudocode, TLA+, Alloy, tooling, testing, binary ABI/wire-format freeze or deployment. The following are unproved safety obligations, not verified features.

## 0. Executive outcome: retain T1 as CONDITIONAL research candidate, narrow its security claims

**Recommendation:** RCV-T1 (exclusive receive claim, conservative potential-exposure marker, fail-closed post-exposure cross-recipient redelivery ban) remains the less TCB-heavy general IPC research baseline. RCV-T2 (inert capability escrow until independently trusted authorization) is a **composable optional security profile**, not a magic safer alternative for every message. Neither solves arbitrary user memory copyout atomicity. A confidential payload may be visible to an authorized Task before full handle installation regardless of T2.

**Critical v0.2 changes from v0.1:**

1. **Handle-table installation, user-visible token publication and ability to invoke are three distinct events.** A whole-vector atomic kernel install does not guarantee the entire user handle array was written. Partial handle-number copyout, orphaned installed rights, delayed invokability and public-token ABA require separate proposed state.
2. **Message metadata can disclose information even without payload bytes.** Size, count, type, rights, presence/absence and timing of optional branches may be sensitive. Distinguish metadata-visibility (K-Vm) from possible payload exposure (K-Vp), or explicitly declare certain metadata nonsecret under issuer policy.
3. **K-Vp must dominate EVERY exposure path, not only ordinary memcpy.** Page mapping, shared buffer alias, handle metadata output, speculative publication assumption, delegated shared MemoryObject and DMA are separate routes. A kernel flag before a particular copyout instruction does not cover other uncontrolled data paths automatically.
4. **The receiver's claim and Endpoint ownership can race.** Binding to Task+Endpoint generation and authorized ServiceIncarnation is necessary, but not sufficient to pin a confidentiality audience when Endpoint moves or issuer policy changes between claim and exposure. Introduce a fixed, issuer-authorized audience label at claim/admission, with defined revalidation/abort.
5. **RCV-T2's independent verifier must bind a *kernel-authenticated* live cap vector and current revocation/policy cut at the actual activation linearization point K-U.** A signature/digest over sender-authored metadata cannot prove the live cap contents; a prior validation receipt cannot override later revocation B.
6. **Status retrieval after sender loss is an additional privacy channel.** A Task-scoped ticket or bounded enumeration could leak message size, destination, rights or lifetime to an actor who did not own the original SEND; scope result authority separately and fail closed on Task generation reuse.
7. **Terminal settlement is not guaranteed resource reclamation.** Exposed message records, outstanding user-visible rights, orphan installed caps, independent roots and DeviceGroup fences each have different lifetime and sponsor owners. Message terminal state must carry debt/quarantine information or be garbage-collected only after those obligations have independent records.
8. **Do not conflate the security objective with reliable delivery.** At most one authorized kernel recipient-generation for automatic delivery is a deliberate, narrow constraint that can trade off availability. A new *application-level* retry after exposure requires fresh issuer authorization and new message/effect identity; the kernel may not silently requeue the old record.

**Decision posture:** The proposed architecture is still compatible with the original RFC-003 E1 exclusive Endpoint/Channel, but RFC-003 §6 currently lacks an explicit metadata/payload visibility cut, the public Handle publication state, terminal-receive outcomes and T2 release authority. These are **contract gaps rather than proofs of an implementation defect**.

## 1. Evidence from reference systems (not imported semantics)

| Primary evidence | What the reference actually says | Cautious UniOS implication |
|---|---|---|
| Fuchsia Channel kernel object [S1] | nonduplicable endpoint handle; defined ordering across ownership transfer and queue grant ownership | separate endpoint ownership from admitted invocation references; formalize concurrency under E1 rather than assume it |
| Fuchsia zx_channel_read [S2] | message read in full, bytes buffer written before handle buffer, overlapping buffers may overwrite bytes; BUFFER_TOO_SMALL can report needed bytes and handles | two output buffers and size/count reports have different visibility/failure concerns; Fuchsia semantics do not imply atomic UniOS usercopy |
| Fuchsia zx_channel_read_etc [S3] | returns live object type/rights for transferred handles and allows size-oriented error outcomes | no user IDL type label can replace kernel-authenticated cap role validation; metadata disclosure is a separate API policy |
| Fuchsia Channel abstract two-phase transfer [S4] | queue transfer and receiver handle install are separate kernel cuts; channel handles may be discarded on errors according to Fuchsia policy | preserve UniOS's separately proposed NOT_COMMITTED sender retention rather than borrowing Fuchsia error contract |
| Fuchsia handle lifetime [S5] | handles are Task-local and in-transit handles keep referenced objects alive | a dropped userspace result need not destroy queued or installed authority; public handle numbers are not security roots |
| seL4 IPC [S6] | shared badged endpoint caps and synchronous rendezvous with reply objects | distinct ownership and liveness family, not evidence a shared E1 endpoint is already safe |
| Fuchsia ChannelWrite [S7] | source handle disposition on failure differs from UniOS's proposed E1 precommit retention | avoid confusing proof state with imported syscall semantics |

**Evidence limitation:** none of these documents demonstrate a working UniOS kernel, that T1/T2 is realizable under the present target TCB, or that a given partial copyout state is recoverable. In particular, real Fuchsia Channel read behavior is only a source for alternate designs and adversarial test ideas.

## 2. Revised state space: must separate bytes, tokens, cap authority and effects

### 2.1 Explicit Endpoint halves and independent identity dimensions

One Channel contains **two different E1 Endpoint objects** (SendHalf and ReceiveHalf). Each has at most one effective owning public Task handle at a time; their Task owners may be different. The model must not encode a single global endpointOwner variable as if both halves were the same object. A logically named ServiceID, an authorized ServiceIncarnation, the receiver's kernel TaskGeneration and an issuer-defined confidential-data audience are separate values.

**Actors:** A sender Task; B initial authorized receiving Task (two untrusted Threads); C prospective successor or different Task; B2 new service incarnation; issuer I, controller R, kernel K, Pager P, optional independent T2 verifier Vt and DMA group D. Choose a queue capacity one, one message with at most two grants, two independent root references to one backing object, one protected claim and one bounded ticket for future finite analysis.

### 2.2 State and authority axes

| Variable (abstract only) | Candidate finite values | Must remain distinct from |
|---|---|---|
| sendHalfOwner / receiveHalfOwner | TaskGeneration + EndpointGeneration / none | ServiceID string; the *other* half |
| sendActual / sendObserved | NotCommitted or Committed / Unknown or certified result | remote service effect and caller return register |
| queueCustody | source / pending queue / claim escrow / receiver / closed / quarantine | object-wide refcount and sponsor |
| claimAudience | one issuer-eligible TaskGeneration+ServiceIncarnation+allowed confidentiality scope | merely current Endpoint handle or Task numeric PID |
| claimEpoch | EndpointGeneration, TaskGeneration, revocation generation, issuer PolicyEpoch | raw user-supplied feature number |
| payloadExposure | NotPossible / PossiblyPublished (monotonic) | actual byte-read count |
| metadataExposure | None / authorized metadata may be observed | payload visibility or method effect |
| tokenPublication | None / Partial / Complete / Unknown | actual cap install and cap invokability |
| capInstall | None / AllInstalled / InertEscrow / Settled | writable userspace handle array |
| capInvokability | InTransit / Inert / Enabled / Revoked / Closed / Quarantined | whether token is known to B |
| terminalDisposition | None / SameAudienceContinuation / TerminalDrop / ChargedQuarantine | message was actually read or service effect absent |
| resultReceipt | FreshPreidentified / Observed / Lost / Reconciled / Expired | backend effect receipt |
| externalEffect | NotApplied / Applied / Unknown / DurableScoped | kernel RECEIVE result |
| sponsorDebt | finite charged queue/claim/handle/pin/metadata/terminal debt | holding Task lifetime |
| revokeBarrier | Before / After current action admission | CPU R1, DMA R2-F or all-ref R2-O |

**Model guard rule:** a saved Task ID, cap field ordinal, Endpoint owner pointer or signed schema version by itself is insufficient. All cuts which install or enable *new* native rights must check the live kernel state and scoped revocation, and issuer/service policy must independently authorize identity-sensitive actions.

## 3. Independent trust properties for RCV-T1 and RCV-T2

### 3.1 T1 receive grants are bearer authority, not SABI admission

In T1, once the kernel enables receiver cap handles, B may exercise whatever rights the sender/issuer intentionally conferred, even if B's SABI decoder later returns BAD_SCHEMA. Therefore T1 is only suitable when that bearer authority **already reflects the maximum permitted access to B**. A privileged DeviceGroup capability cannot be sent to an untrusted plugin while promising the IDL parser will block its use before validation.

### 3.2 T2 verifier and escrow add a new trusted authority

T2 can potentially ensure that an *untrusted* B does not gain native rights until a trusted verifier has approved specific method/resource roles. It must bind approval to:
- the kernel's actual live grant set, with type, rights, scope, lineage, sponsor and revocation epoch;
- exact queued message / claim generation and receiver Task/Endpoint generation;
- authorized ServiceIncarnation, SABI method/required-feature contract and issuer policy epoch;
- one-shot, non-transferable release right with scoped effect/destination and bounded expiry;
- **re-check of actual kernel rights and revocation B at K-U** (the activation point), not merely at validation time.

An untrusted B cannot supply a boolean, hash, guessed SessionID or replayed verifier response to unlock escrow. A malicious or compromised independent verifier is within T2's newly expanded TCB; it can release authority wrongly unless constrained by an issuer-generated limited release grant. The kernel still does **not** parse full evolving SABI data to determine whether a message is allowed.

### 3.3 T2 does not fix exposure, availability or exact-once

If B is permitted to read plaintext payload during validation, then B may leak that payload even while the associated grant is inert. If a verifier fails after payload visibility, no automatic foreign requeue remains the proposed confidentiality rule. If a verifier stalls while an escrow holds scarce caps, liveness/resource accounting becomes more complex; M0 recovery cannot depend on that verifier. T2 doesn't make an earlier external file write reversible.

**Interim judgment:** compare T1 and T2 against *specific* privileged service threats, not general performance marketing. No measured CPU/memory/latency exists and no implementation is authorized.


## 4. Revised receive cuts: Metadata, Payload, TokenPublication, Install, Activation

### 4.1 Proposed temporal relation, NOT a kernel execution recipe

~~~text
Issuer grants scoped Endpoint half and eligible audience
    |
K-S: sender bytes and moved caps committed to kernel queue
    |
K-C: exclusive receive claim for eligible TaskGeneration/EndpointGeneration
    | (kernel can still reject undersized buffers or unauthorized claims)
    +-- K-Vm: optional metadata release (if issuer allows it)
    |
Private receiver staging / cap-slot reservation / current B/policy recheck
    |
K-Vp: conservatively mark payload POSSIBLY EXPOSED before first store/map
    |
User-visible payload copy may fault or reveal only a prefix
    |
    +-- separate token publication / cap install order MUST be specified:
    |      token written but grant not yet installed?
    |      grant installed but token not completely written?
    |      caller loses receive result after either of those?
    |
K-I: all or none receiver cap grants installed + queue retired (T1)
    |
T2 only: inert escrow -- trusted role check -- K-U with LIVE revalidation
    |
S-A: service independently authorizes any external method effect

At any cut: Revoke B, Endpoint MOVE, Task death, Service restart,
             user page fault, caller timeout, device DMA fence failure.
~~~

**No hard-coded B1/B2 order yet.** A single “atomic handle installation” operation says nothing about atomically copying N public handle numbers into a concurrently readable, pageable user array. A kernel must reserve recipient slots and clearly define the point when previously returned or visible token bytes begin to authorize operations. That can require additional indirection/tombstones or an escrow-like state even in otherwise T1 receive. Merely reserving memory and not holding a global lock across faults is necessary, not sufficient.

### 4.2 Token publication counterexamples

| Candidate order | Failure cut | Safety/liveness issue still to settle |
|---|---|---|
| **Token-first:** write public handle number array, then K-I installs active caps | first 1 of 2 tokens visible, later usercopy faults | token bytes were disclosed but installation failed; **never** recycle reserved bits to a different grant while stale B copies can exist |
| Token-first then installation succeeds before result observed | B's other Thread already copied token bytes | rights can become usable while primary receiver thinks call failed; generation/issuer policy must define exposure as grant |
| **Install-first:** K-I installs all active grants, then token array copyout | array copy faults after installing caps | B has orphaned active rights with no reliable returned handles; cleanup must account for any concurrent use |
| Install-first then close on copyout error | B may already have discovered/invoked a token through concurrently visible bytes | handle close cannot undo previously admitted operations or external effects |
| **Inert install:** kernel reserves noninvokable token slots, copies outputs, then activates | activation changes all-or-none, copyout/report might still be lost | safe activation and result discoverability need explicit Task generation and protected preidentified claim |
| T2 trusted release after inert install | policy revokes between verifier checks and K-U | K-U must validate actual live lineage/current policy and not accept stale validation |
| Shared user-memory aliases for bytes, handles and counts | VA ranges appear separate but same backing pages | can corrupt earlier output; physical backing alias and concurrency analysis required |

**New audit finding:** CAPIPC v0.1/v0.2 and RFC-003 emphasize whole-vector K-I but have not yet selected a credible **public-token publication + invocation activation** contract. The prior v0.1 model's B1 K-V→copyout→K-I was a *good research simplification*, not a proven safe ABI recipe. A separate finite model of TokenPublication/CapInvokability is mandatory before choosing B1 or B2.

### 4.3 Metadata observation can be a distinct privacy leak

Fuchsia zx_channel_read with a too-small buffer may return required byte and handle counts [S2]; zx_channel_read_etc may reveal live object types and rights [S3]. That is a documented behavior in Fuchsia; UniOS must decide its *own* policy.

- **K-Vm:** disclose only a fixed, issuer-authorized metadata projection (for example, exact byte length and cap count, or merely within-quota indicator), after current RECEIVE and eligible audience authorization. If counts/types are confidential, metadata counts as an exposure to the recipient and must be included in its irreversibility and requeue policy.
- **K-Vp:** mark potential sensitive payload visibility before any payload byte, mapped page or equivalent receiver-accessible data is published. A kernel flag tied only to memcpy is insufficient for explicit shared-buffer transfer or alias publication.
- **Size error before claim:** possible only when authorized size projection is allowed. BUFFER_TOO_SMALL must not allow a non-holder, stale Endpoint owner or foreign Task to probe the unread queue's confidential structure.
- **After a public metadata result:** whether the queued message can be requeued to a *different eligible Task* depends on whether that metadata was classified private and which audience obtained it. This requires an explicitly named policy, rather than falsely claiming no disclosure merely because K-Vp=false.
- Timing and presence side channels may remain outside the defined guarantee. Do not claim “no information leakage” merely from blocking byte copyout.

**Default research recommendation:** if sensitive service packets could disclose secret structure, treat unauthorized metadata disclosure as a K-V barrier for the chosen confidentiality scope, or expose only a coarse/constant preclaim status. Separating K-Vm and K-Vp is necessary for honest proof; whether both are needed in the final KABI is a later decision.

### 4.4 Binding an audience through Endpoint MOVE and policy changes

Suppose B acquires K-C, then transfers its receiving Endpoint to C before K-Vp. If kernel checks only "current Endpoint exists" and then writes bytes into B, the original claim may no longer have an authorized holder. If it instead writes into C without new issuer authorization, it changes the disclosed audience during an earlier claim. If B has already K-Vp, simply reassigning its claim to C duplicates potential disclosure.

**Proposed rule:** claim pins ReceiverTaskGeneration, EndpointGeneration and an **issuer-policy audience**; before first metadata/payload publication validate that claim remains authorized under current Endpoint owner and revocation B/policy cut. If ownership or audience changes, unexposed claim may abort/revalidate under a newly authorized claim; exposed claim terminalizes rather than silently migrates. A claim must not be transferred just by copying an SABI token. An approved same-Task Thread may process claim only if Thread-to-Task semantics, session ownership and sponsor are defined.

After an exposed claim is terminalized, a *new independent* authorized application request may legitimately send equivalent data, but this is not an IPC rollback/requeue. It requires new issuer approval, new SEND record and explicit remote effect/dedupe analysis. A literal “no one else may ever see same bytes” policy would be stronger than possible kernel guarantees and is **not** claimed.

## 5. Adversarial re-review matrix (AR-01…AR-20)

| ID | Precise interleaving | Unsafe claim / correction |
|---|---|---|
| AR-01 | B owns RECEIVE, queries exact cap count; C newly owns Endpoint | old B's size query did not disclose any confidential metadata |
| AR-02 | C lacks RECEIVE but can query unread payload length before K-C | size probe harmless / metadata necessarily public |
| AR-03 | B K-C then Endpoint MOVEs to C before K-Vp | claim silently follows owner or publishes to stale B without policy |
| AR-04 | B K-Vp and sees prefix, Endpoint MOVEs to C, claim canceled | same message requeued to new C without disclosure record |
| AR-05 | B buffers have nonoverlapping VAs but writable aliases of same page | pointer nonoverlap proves safe contents |
| AR-06 | B copies one of two raw destination handle numbers, then second copy faults before K-I | reserved token may be recycled as unrelated live authority |
| AR-07 | B cap vector installed but token report faults | installed orphan rights assumed never usable/charged |
| AR-08 | B's second Thread invokes published token after K-I but before first caller hears SUCCESS | receiver result loss means grant was never activated |
| AR-09 | B self-declares SABI verification successful to T2 kernel gate | independent trusted verifier existed |
| AR-10 | trusted verifier signs old grant set; Controller publishes B; B later asks K-U | old permit activates revoked grant |
| AR-11 | trusted verifier authorizes message X, B uses result for queued message Y | release token not bound to exact cap/message generation |
| AR-12 | K-Vp before first byte attempt, first store faults with zero bytes copied | allow message drop for conservative safety rather than guarantee availability |
| AR-13 | user copyout publishes handle metadata before marking exposure | K-Vp protecting payload automatically covers all sensitive metadata |
| AR-14 | Receiver B crashes after partial publication, Supervisor restarts B2 under same ServiceID | name equality authorizes same confidential claim replay |
| AR-15 | actor reuses old SEND ticket after Task generation recycled | stale status query may reveal new Task's cap/Endpoint metadata |
| AR-16 | B reads buffer and triggers external write before final K-I; later claim fails | no remote effect because receive returned error |
| AR-17 | a terminal Message holds unknown DeviceCap and unresolved DMA write | terminal-drop means R2-F and sponsor refund finished |
| AR-18 | T2 verifier blocks on the very full Channel whose receive claim it is checking | no deadlock because cap escrow is inert |
| AR-19 | client cancels after SEND_COMMITTED, service effect already happened, reply lost | cancellation proves no-effect / safe non-idempotent retry |
| AR-20 | multiple sender-derived grants refer to same backing Object; family revoke affects only one root | same Object implies all roots/children revoked, safe frame reuse |

## 6. New review invariants (AR-I01…AR-I24) — unproved

| ID | Proposed safety obligation |
|---|---|
| AR-I01 | Queue ownership/atomic K-S retains precisely the moved grant set or none; return loss never rewinds it. |
| AR-I02 | Claim eligibility and ServiceIncarnation audience are kernel/issuer-authorized and bound to actual receiving Endpoint half. |
| AR-I03 | One E1 Endpoint half has at most one effective public Task owner; SendHalf and ReceiveHalf are distinct. |
| AR-I04 | Source cap revocation B and Endpoint ownership MOVE have defined linearization before any new use. |
| AR-I05 | K-Vm metadata publication has an issuer-authorized confidentiality scope separate from payload K-Vp. |
| AR-I06 | No metadata probe by unprivileged or stale endpoint owners can leak protected unread message structure. |
| AR-I07 | K-Vp is set before the earliest possibly receiver-visible payload event on every allowed transport path. |
| AR-I08 | Potential exposure history is monotonic; copyout failure never resets it to hidden. |
| AR-I09 | Same message cannot be automatically redelivered to a different audience after relevant sensitive metadata/payload disclosure. |
| AR-I10 | A claim pinned to Task/Endpoint generation cannot silently survive an owner or issuer audience change. |
| AR-I11 | Atomic whole-vector K-I and user-visible token publication/activation are independently specified, not implied equivalent. |
| AR-I12 | Reserved/public token bits visible on failed installation are never reassigned to unrelated grants while stale copies can be used. |
| AR-I13 | Installed but unreported capabilities remain charged and have defined bounded cleanup or protected reconciliation. |
| AR-I14 | Exposing handle numbers doesn't itself mint or multiply cap grants; only a kernel-owned slot maps tokens to authority. |
| AR-I15 | T1 receivers have at most the native bearer cap authority intentionally granted before SABI parse. |
| AR-I16 | T2 release requires independent verifier/issuer authority and exact kernel-authenticated grant/message/destination binding. |
| AR-I17 | K-U rechecks actual revoke B/current authorization policy and one-shot release validity at activation. |
| AR-I18 | Unknown cap roles or protocol variants do not silently install active rights in security profiles requiring pre-use validation. |
| AR-I19 | Metadata, bytes, cap publication, cap invokability, method effect and result observation never collapse to a single success signal. |
| AR-I20 | Receive terminalization blocks unauthorized redelivery but doesn't claim prior user disclosure or backend effects have vanished. |
| AR-I21 | Queue/claim/escrow/terminal/handle/ticket/DMA debt remain sponsor-accounted until separately settled. |
| AR-I22 | M0 protected reserve does not depend on pageable T2 verifier or malicious claim progress. |
| AR-I23 | Reconciliation result access is scoped to an authenticated Task generation and cannot reveal unrelated messages after token reuse. |
| AR-I24 | All invariants are Proposed hypotheses without execution, proof, wire, ABI or implementation authorization. |



## 7. Outstanding research decisions (AR-O01…AR-O16)

| ID | Priority | Decision not settled by this audit | Owner |
|---|---|---|---|
| AR-O01 | P0 | Does RCV-T1 use token-first, install-first, or explicitly inert publication+activation, with true post-fault cleanup? | RFC-002/003/009 |
| AR-O02 | P0 | How are partially published public token bits tombstoned under Task lifetime and cap slot exhaustion? | RFC-002/009 |
| AR-O03 | P0 | How can installed but unreported usable caps be enumerated/revoked without exposing other Task's rights or undoing admitted use? | RFC-002/003/009 |
| AR-O04 | P0 | Does an exact type/rights/count size query reveal sensitive metadata before K-Vp, and which caller audience is permitted? | RFC-003/009/018 |
| AR-O05 | P0 | What qualifies as first possible payload exposure for shared VM, remap, DMA/aliases or data-only fastpath? | RFC-003/004/007 |
| AR-O06 | P0 | How is K-C audience pinned and rechecked on receive Endpoint MOVE, policy change, B revoke and service restart? | RFC-002/003/006/018 |
| AR-O07 | P0 | May a post-K-V exposed message continue with same Task but different Thread/session, and under which bounded policy? | RFC-003/006/018 |
| AR-O08 | P0 | How do we guarantee K-Vm/K-Vp exposure history and terminal debt are recorded before risky publication without holding pageable locks? | RFC-003/004/005 |
| AR-O09 | P0 | Does T2 trusted verifier have sufficient independent authority to reject malicious B, without requiring full IDL parser in kernel? | RFC-001/003/018 |
| AR-O10 | P0 | Is K-U release single-use and atomic with live revoke/Task generation/policy epoch and exact actual cap set? | RFC-002/003/018 |
| AR-O11 | P0 | How are post-T1/T2 grant invocation, premature use and remote external effects reconciled on abort or restart? | RFC-003/008/009/018 |
| AR-O12 | P0 | Can finite M0 recovery and critical IPC remain live when terminal records, unclaimed handles, T2 verifier and DMA quarantine accumulate? | RFC-003/005/006/007 |
| AR-O13 | P0 | Does T1's at-most-one *automatic recipient generation* security objective meet app-level reliability/consent needs, or require a different documented tier? | RFC-003/016/018 |
| AR-O14 | P1 | What bounded abstract formal state/fairness covers Task reuse, full cap-vector, mapping alias and scheduler without assuming perfect timing? | RFC-002/003/009/015 |
| AR-O15 | P1 | How will Linux guest broker map receive partial publication/orphan native rights to errno without invented semantics? | RFC-009/010 |
| AR-O16 | P1 | Choose C ABI, KABI numeric/status representation, bounded handle vector format only after proof; no accidental freeze via documentation. | RFC-003/009/018 |

## 8. Proposed findings and decision register (AR-D01…AR-D12) — NONE Accepted

- **AR-D01:** Maintain RCV-T1 as first general receive proof target, not a verified feature.
- **AR-D02:** Treat T2 as T1 plus independently authorized inert-cap release for selected sensitive services, not a universal replacement.
- **AR-D03:** Receiver byte payload and sensitive metadata require separate prospective-visibility accounting or an equally conservative unified barrier.
- **AR-D04:** Whole-vector K-I cannot be equated with all-or-none user handle-array writes or the exact instant native rights become usable.
- **AR-D05:** Preserve public token nonreuse for failed/partially published token values, including unused reserved slots in same Task namespace.
- **AR-D06:** Do not approve B1 token-first/copy-before-K-I as a guaranteed secure fast path until invocation and usercopy race proof.
- **AR-D07:** Do not approve B2 install-first without an orphan-handle/recovery proof and sponsor accounting.
- **AR-D08:** Claims bind actual receiver Task+Endpoint generation and issuer confidentiality audience; invalidate/revalidate before publication on owner or policy change.
- **AR-D09:** After sensitive disclosure may have happened, do not automatically requeue the same committed message to another receiver generation; application retries require new authorization/identity.
- **AR-D10:** A T2 release proof comes from independently authorized verifier/issuer plus live kernel revocation/policy check at K-U, never a client asserted digest/flag.
- **AR-D11:** Terminal settlement, K-S result reconciliation, guest error and device fence are independent state/evidence ledgers; never combine as generic successful receive.
- **AR-D12:** All decisions stay Proposed. No runtime OS/IDL, executable formal model, tests, performance claim or stable ABI is authorized.

## 9. RFC-003 alignment register — exact semantic delta (no implicit acceptance)

| Current RFC-003 statement | v0.2 review finding | Proposed clarification |
|---|---|---|
| §5 one effective E1 Endpoint handle, opposite halves distinct | correct directional scope but concurrent receive claim across MOVE unclear | state SendHalf/ReceiveHalf owners and claim-generation revalidation |
| §6.1 preflight counts, reserve slots, atomic cap install/dequeue | good cap-vector goal, **not** complete user-visible token result contract | separate cap install, public handle-token copyout and invocation activation |
| §6.2 RCV-A exclusive ReceiveEscrow | promising, but exclusive claim doesn't prevent old B from reading partial bytes | conservative K-Vp and audience bound before ANY payload publication |
| §6.2 RCV-B copy-before-commit default rejected | still directionally correct | bounded T1 claim may copy before K-I only if exposure/terminal and token issues resolved |
| §13 BUFFER_INVALID and SIZE/COUNT errors | no metadata visibility classification | size/type/rights/count projection requires issuer-authorized K-Vm, confidentiality scope |
| §14 IPC-09/28 copyout/cross-task secrecy wording | can overstate complete lack of disclosure after failed receive | prohibit **automatic redelivery to different recipient** after potential exposure; don't claim B never saw bytes |
| §10 revoke B and in-transit cap | compatible P0 requirement | K-U must also recheck current lineage and issuer policy at activation |
| §9/10 queue/cleanup sponsor | compatible P0 requirement | terminal message, orphan installed cap and published token tombstone have different charges/expiry |
| §5.2 status ticket | currently unresolved, correctly Proposed | preserve ticket ID loss and privacy-safe Task-scoped reconciliation as P0 |

**Review posture:** This is a scope-tightening correction, not evidence RFC-003's architecture is intrinsically contradictory. The older table DERIVE inconsistency was corrected separately, and this new review doesn't reintroduce it.

## 10. Exit gates / order of further design work

**AR-R1: OPEN / NOT PASSED.** Do NOT accept RCV-T1, RCV-T2, kernel E1 RECEIVE ABI or security claim until:

1. A single proposed exact receive order covers *all four*: private bytes snapshot, metadata/payload publication, cap-vector table installation and public token activation, with fault/death/signal interleavings.
2. Token reuse/ABA after partial publication cannot grant unrelated objects; installed-but-unreported caps either remain safely gated or have authorized bounded cleanup/reconciliation.
3. Metadata probes are classified by secrecy, controlled by RECEIVE rights/audience and barred from stale owner/unauthorized Task.
4. Every possible data-bearing publication path includes a conservative irreversible K-V barrier, including shared alias/mapped buffers as applicable.
5. Task/Endpoint generation, ServiceIncarnation and issuer audience are revalidated across K-C, Endpoint MOVE and K-V; no user-named identity.
6. T2 verifier's own TCB and issuer authority are defined, and K-U revalidates live cap and policy/revoke under one documented cut.
7. Resource budgets/tombstones/terminal states/quarantine remain bounded even when recipients repeatedly fault, abandon claims or withhold verifier replies.
8. Future abstract model reviews safety and conditional liveness without silently assuming an infinitely reliable Pager, scheduler, storage or hardware fence.
9. Partial receive, unknown external effect and guest result mappings do not fabricate safe rollback/success semantics.
10. Re-read revised RFC-003/002/009/018 text and obtain independent approval of eventual ADRs; this v0.2 document is not that approval.

**Recommended next research step:** refine the **public Handle publication vs K-I/K-U invocation activation** finite countermodel to select a precise, minimal receive ABI. Compare token reservation/activation with fully protected receiver-side result records, then independently audit security/performance/TCB implications. Further executable state modeling or kernel code still requires an explicit future authorization separate from design research.

## 11. Technical references, provenance and limits

- **[S1] Fuchsia Channel reference (unique Endpoint owner and transfer ordering):** https://fuchsia.dev/fuchsia-src/reference/kernel_objects/channel
- **[S2] Fuchsia zx_channel_read (buffer ordering, overlap, size results):** https://fuchsia.dev/reference/syscalls/channel_read
- **[S3] Fuchsia zx_channel_read_etc (types, rights, counts):** https://fuchsia.dev/reference/syscalls/channel_read_etc
- **[S4] Fuchsia Channel older official source and handle transfer cuts:** https://fuchsia.googlesource.com/fuchsia/+/cc19d17b05c18cbc940da8ef6fa5bcc0d6464c1f/docs/reference/kernel_objects/channel.md
- **[S5] Fuchsia handle custody and Task-local meaning:** https://fuchsia.dev/fuchsia-src/concepts/kernel/handles
- **[S6] seL4 IPC tutorial (different shared badged endpoint family):** https://docs.sel4.systems/Tutorials/ipc
- **[S7] Fuchsia zx_channel_write (different failure handle semantics):** https://fuchsia.dev/reference/syscalls/channel_write

All references characterize other systems. They **do not** transfer a working algorithm or formal proof to UniOS, whose E1 precommit-failure cap retention is intentionally different from basic Zircon channel-write failure handling. Neither v0.1 nor v0.2 implemented, executed, simulated or formally verified a finite state model.

---

**History:** 2026-10-25 v0.2 independent adversarial re-review of [v0.1](capipc-send-receive-revoke-state-model-v0.1.md). Added capability install vs public token exposure/activation, confidentiality of preclaim metadata, receiver-audience and Endpoint MOVE races, exact trusted T2 verifier/PolicyEpoch release and terminal sponsor debt. 20 unexecuted cases, 24 additional unproved invariants, 16 open research issues, 12 Proposed decisions. All parent RFCs remain Proposed; design documentation only.

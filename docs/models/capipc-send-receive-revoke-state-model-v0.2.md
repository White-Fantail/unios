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

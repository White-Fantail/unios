# UniOS — CAPIPC SEND–RECEIVE–REVOKE Unified State Model v0.1

> **Status:** Proposed Research Model (2026-10-25 NZDT), **NOT Accepted, implemented, machine-checked, benchmarked or formally verified**.
> **Parent:** [CAPIPC v0.2](../contracts/capability-ipc-sabi-integration-contract-v0.2.md), historical [CAPIPC v0.1](../contracts/capability-ipc-sabi-integration-contract-v0.1.md).
> **Authority:** [RFC-002](../rfcs/rfc-002-object-capability-lifetime-delegation-revocation.md), [RFC-003](../rfcs/rfc-003-ipc-wire-protocol-and-synchronization.md), [RFC-009](../rfcs/rfc-009-native-abi-and-system-call-contract.md), [RFC-018](../rfcs/rfc-018-versioned-idl-abi-evolution-and-compatibility-contracts.md); dependencies RFC-004/005/006/007/008/010 and [Cross-RFC audit](../reviews/cross-rfc-architecture-consistency-audit-v0.1.md).
> **Scope restriction:** Human-readable *design and research only*. No TLA+, Alloy, executable model, kernel or userspace code, protocol binary encoding, SDK, tests, real hardware work or performance claim. Everything here is an unproved proposal.

## 0. Executive decision candidate — T1 baseline, T2 only for independently trusted release

**Provisional selection for next research: RCV-T1** (exclusive, bounded ReceiveClaim; mark potential exposure before any user-visible payload write; prohibit requeue to a different Task/service incarnation after potential exposure). **RCV-T2** (inert transferred capabilities until schema/policy admission) is a conditional extra security profile, not a standalone solution to exposure. Both retain the CAPIPC-A E1 exclusive endpoint and atomic SEND/MOVE candidate, but neither is proven safe.

Two separate threats:
1. **Duplicate disclosure:** after partial usercopy to B, read failure requeues to C; both B and C may see the same secret. Prevented by an irreversible exposure barrier, not by a cap-use lock.
2. **Premature native authority:** cap granted to B becomes usable before userspace IDL method validation. Prevented only by a genuinely enforced inert-cap escrow or trusted attenuating router; an untrusted B's self-declared "validated" is not authorization.

**Critical correction to simplistic T2:** a new kernel escrow-release API that accepts the same compromised B's unverified "validation complete" flag proves little. T2's release must be authorized by an independently trusted verifier or issuer-controlled, exact cap-vector/Endpoint/ServiceIncarnation/PolicyEpoch-bound grant. Otherwise T2 adds state, cost and TCB without the advertised safety.

**Scope of confidentiality:** at-most-one *queue delivery to distinct receiving Task/ServiceIncarnation* is not equivalent to at-most-one human observer, global secrecy or exactly-once external effects. An authorized B can itself forward bytes. The receiver audience/issuer policy must be explicit, including if any such redistribution is permitted.

**Direct predecessor:** CAPIPC v0.2's Exposed(M) state is a *conservative may-have-been-exposed* marker. This model refines its guards, not an assertion that any user Thread actually read a specific byte.

## 1. Trusted cuts, independent graphs and exact identities

| Cut | Meaning | Authoritative component | Does NOT establish |
|---|---|---|---|
| K-P | bounded byte+handle descriptor snapshot and temporary quota | kernel | source caps moved |
| K-S | full queue append and all-or-none cap MOVE, source handles invalidated | kernel | receiver saw data or service effect |
| K-C | one exclusive ReceiverTask/EndpointGeneration claim | kernel | no future partial disclosure |
| K-V | Exposed set **before** first attempted user-visible payload publication | kernel | bytes definitely read; cap installed |
| K-I | receiver cap-vector all-or-none install and queue removal | kernel | user bytes atomically published or SABI method valid |
| K-U | optional T2 trusted escrow release of usable caps | kernel plus independent verifier/issuer | sender-supplied flag was truthful |
| S-A | current authorized method admission, active PolicyEpoch | trusted SABI service/broker | effect applied/persisted |
| R-B | revocation barrier denies new scoped uses | kernel/authorized controller | admitted operations undone |
| R1 / R2-F / R2-O | CPU access cut / physical device fence / total roots reclaimed | kernel+hardware control | ordinary handle close enough |
| X-E / X-D / X-O | external effect / scoped persistent receipt / caller observed result | service+device+client | IPC COMMITTED equals all three |

Four graphs: (1) cap lineage and independent roots; (2) current custody and sponsor charge; (3) lifetime including queued caps, pin, Pager, IRQ/IOVA and pending external work; (4) information exposure to Task-generation and service incarnation. A cap holder, grant issuer, revocation controller, budget sponsor, current kernel Task and original human user **are not interchangeable**. IDL method/version and service-name strings cannot mint kernel rights.

## 2. Qualitative feasibility comparison — no fake performance figures

| Axis | RCV-T1: terminalize on exposure | RCV-T2: T1 plus inert-cap escrow |
|---|---|---|
| Primary property | prohibits foreign redelivery after possible exposure | stops usable native cap release until a **trusted** admission cut |
| Copyout risk | partial byte disclosure still possible, tracked | **identical** byte risk; T2 must preserve T1's exposure rule |
| Scope of actual cap authority | B becomes usable holder at K-I in bearer profile | kernel inert custody until authenticated K-U |
| Minimum trust | sender granted B the maximum native authority B may hold, even before decode | separate trusted verifier/issuer, or purpose-bound narrow policy proven by independent checker |
| Compromised B | can abuse rights transferred to it; MUST receive only attenuated grants | cannot invoke inert grants pre-K-U, but can leak plaintext payload bytes |
| Kernel trusted state | exclusive claim, staged bytes, exposure flag, cap install, terminal cleanup | all T1 state **plus** escrow identity, release grant/attestation, expiration, verifier binding |
| IDL decoder placement | userspace, schema outside Ring 0 | userspace trusted verifier/broker; still avoid full IDL parser in kernel |
| Extra CPU/IPC | bounded receive/copy, no required external verifier hop | verifier/release interactions, potential repeated usercopy/IPC |
| Extra resident memory | staging bytes, claim and queue-charge records | staging **plus** escrow record, cap role map, release/timeout charges |
| Service death | terminalize post-K-V, settle caps/sponsor | terminalize post-K-V; also revoke/invalidate outstanding release permits |
| Version/revoke race | policy check before any new effect; existing cap use under bearer grant | cap release must revalidate current issuer, lineage and policy epoch |
| Liveness | partial publication can sacrifice delivery to avoid double distribution | additional verifier unavailability can delay/deny release |
| Data-only methods | baseline relevance | no security gain from cap escrow when no cap exists |
| Special use | general-purpose native IPC after proof | privileged DeviceCap/SecretKeyCap/high-risk multi-principal router, conditional |
| Verdict | **first research candidate**; NOT approved | **optional research profile**, only with independent trust mechanism; NOT approved |

**Counterexamples:** A perfectly inert cap escrow does not stop B from reading the secret prefix already copied to its address space. Conversely, a perfect post-exposure terminalization rule does not stop B from invoking an unrestricted received FileCap before IDL decoding. RCV-T2 should be treated as *T1's visibility policy plus additional capability usability control*, not as a mutually exclusive competing confidentiality fix.

**Third option worth comparing:** a very small trusted user-space router obtains a tightly scoped cap, performs SABI resource checks, and issues attenuated caps to less-trusted workers. This avoids generic IDL parsing in Ring 0 but grows a service TCB and requires evidence of endpoint, budget, caller identity and policy epoch.

## 3. Abstract finite model — not machine-checker syntax

**Actors:** sender Task A; receiver Task B with two Threads; rival Task C; restarted service B2 (new Task/ServiceIncarnation); kernel K; controller R; optional trusted verifier Vt, Pager P and device D. Two grants to same object may be independently rooted; they do **not** count as duplication of the same MOVEd grant. One Channel queue capacity 1; one message M with sensitive byte payload and 0–2 moved grants; a bounded critical M0 reserve.

| State variable | Finite domain or relationship | Authority / observation |
|---|---|---|
| endpointOwner | A/B/C/B2/none + EndpointGeneration | kernel; E1 single effective owning Task |
| messagePhase | Absent/Prepared/Queued/Claimed/Exposed/Installed/InertEscrow/Terminal/Settled | kernel abstract bookkeeping |
| grantCustodian | sender/queue/claim/receiver/inert-escrow/closed/quarantine for **each** grant | kernel; distinct from underlying Object refs |
| sourceHandleValid | yes/no for each source Handle token | Task-local kernel table, anti-ABA |
| sendActual | NotCommitted/Committed | kernel truthful historical K-S |
| sendObserved | NotCommitted/Committed/Unknown | caller knowledge, not kernel rollback |
| claimant | none or TaskGeneration+EndpointGeneration+ServiceIncarnation | exclusive kernel claim with sponsor quota |
| exposed | false/true (monotonic) | conservative flag *before* attempted publication |
| exposureAudience | authorized Task-generation/ServiceIncarnation set | policy allows/records *possible* observation |
| copiedPrefix | 0/partial/full attempt | NOT evidence another Thread really read bytes |
| capUsability | transit/inert/usable/revoked/closed/quarantined | kernel granted rights and current B barrier |
| revokeEpoch | old/new for chosen lineage | kernel controller barrier; other independent roots unaffected |
| policyEpoch | P0/P1; SABI service S0/S1 | issuer/service; not raw packet-derived rights |
| ticket | preidentified/none/unobserved/reconciled | Task-scoped protected pending-record claim |
| charge | reserved/transit/claim/receiver/quarantined/released | sponsor ledger, may outlive Task |
| externalEffect | none/applied/unknown/durable-scoped | service/volume truth separate from IPC |
| hardwareFence | not-applicable/pending/proven/failed | actual DeviceGroup/IRQ/IOVA evidence |

**Assumption requirements:** simultaneous actions are interleavings of atomic **kernel cut points**, not whole syscalls. Copyout can fault after a prefix, or another Thread/alias can read it before failure. R-B, Endpoint MOVE, service restart, Pager fault, death, timeout and device-fence failure may occur between any two cuts. The model must represent status observation separately from actual state and distinguish current authoritative audience from an untrusted sender-supplied name.

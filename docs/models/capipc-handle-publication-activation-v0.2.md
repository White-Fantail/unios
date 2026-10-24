# UniOS CAPIPC — Handle Publication & Activation v0.2: Independent Adversarial Re-Review

> **Status:** Proposed Independent Adversarial Review, 2026-10-25 NZDT. **NOT Accepted; no executable state model, source implementation, proof, runtime test, performance result or frozen ABI.**
> **Historical v0.1 retained:** [Handle Publication, Receive Commit & Capability Activation v0.1](capipc-handle-publication-activation-v0.1.md).
> **Joint primary design contracts:** [RFC-002](../rfcs/rfc-002-object-capability-lifetime-delegation-revocation.md) Cap/lifetime; [RFC-003](../rfcs/rfc-003-ipc-wire-protocol-and-synchronization.md) E1 IPC; [RFC-009](../rfcs/rfc-009-native-abi-and-system-call-contract.md) native KABI/usercopy; [RFC-018](../rfcs/rfc-018-versioned-idl-abi-evolution-and-compatibility-contracts.md) SABI; [CAPIPC model v0.2](capipc-send-receive-revoke-state-model-v0.2.md).
> **Joint non-normative dependencies:** RFC-001 kernel boundary, RFC-004 Pager/VM, RFC-005 CPU, RFC-006 M0 recovery, RFC-007 DMA, RFC-008 filesystem durability, RFC-010 Linux guest, RFC-016 authorization.
> **Scope:** Design and documentary research only. No OS implementation, Linux compatibility work, numeric syscalls, frozen public token widths, IDL compiler, runnable pseudocode/TLA+/Alloy, hardware investigation, performance benchmark or formal-model execution. This document DOES NOT change the Accepted/Proposed standing of any previous RFC.

## 0. Executive verdict — REVISE the v0.1 preference, do not rubber-stamp HPA-C

**Independent finding:** HPA-C (two-phase Stage → explicit AcceptCommit → full-vector K-I) makes the activation cut *easier to describe* but, by itself, **does not create a new security principal or policy trust boundary** for an ordinary bearer T1 receive. An already-authorized, malicious or buggy receiving Task may invoke AcceptCommit without reading or verifying staged data. The entire Task shares its cap namespace, so an explicit second call cannot ensure only the invoking Thread receives/uses the grant or that a human consented.

HPA-C costs at least one distinct admission phase, protected claim/receipt state, extra transition and potentially an additional syscall/user–kernel round-trip. It holds queue custody, sponsor charges and token reservations for longer and creates additional cancellation, Task-death, fairness and availability states. **No benchmarks or exact resource bounds yet exist, so it is NOT justified as universal default IPC on security grounds alone.**

**Changed recommendation (research only):**
- **HPA-B+ as first research candidate for normal T1 bearer IPC:** single logical receive with pre-reserved inactive public token IDs, bounded staging, honest exposure marker, atomic K-I activation + queue retirement, and protected preidentified or safely discoverable result/status record. The + means that status loss and token nonreuse are explicitly required; it is not a claim original unqualified HPA-B is correct.
- **HPA-C as optional deferred-activation design study** when an independently authorized receiver policy truly needs the Task to prepare/validate a message *before* native bearer grants become usable, or staged asynchronous receive is required. If the verifier is the same untrusted recipient, C adds little isolation; if independent authority is needed, define true T2 verified escrow/K-U rather than call Task self-approval “secure.”
- **HPA-A retained as a high-risk simpler countermodel:** kernel installs live caps first and then tries to publish handle IDs; requires bounded installed-but-unreported grants, audit, cleanup, anti-ABA and status recovery. Do not assert it is categorically impossible; its proof burden is higher against lost output.

**No automatic adoption:** This re-review changes *research ordering* and identifies adversarial proof gaps. It **does not select a final receive syscall, decide handle widths, approve ABI semantics, or revise RFC-003 to an Accepted decision.** All proposed candidates remain OPEN.

### 0.1 What actually protects what

| Desired property | Minimum relevant mechanism | Misleading shortcut |
|---|---|---|
| A moved CapGrant exists at one custodian | kernel K-S/K-I ownership cuts and sponsor ledger | numeric token copyout is native cap MOVE |
| User does not get usable grant before kernel receive commit | inactive destination token reservation until one atomic K-I | second syscall automatically enforces new principal |
| Secret not silently redelivered to a different receiver after partial publication | K-Vm/K-Vp exposure history + authorized audience/terminal state | failed return means no bytes were observed |
| Policy-sensitive grant unusable before method verification | **independently trusted** verifier + bounded inert escrow + live K-U | untrusted B passes validated=true |
| Committed result can be discovered after result/ticket loss | preidentified Task-generation result slot or suitably scoped enumeration | ticket created only in lost return |
| Kernel does not recycle public token into unrelated authority | Task-local nonrecycling token identity / protected generation | internal slot generation while identical external bits reused |
| No effect replay after timeout | backend durable/request-specific reconciliation | Receive completed or service returned an errno |
| No M0 starvation | bounded queue claim/receipt work, critical CPU/Pager/IRQ reserve | correctness of an individual claim implies global progress |

## 1. Source evidence and careful boundaries

- **[S1] Fuchsia zx_channel_read:** https://fuchsia.dev/reference/syscalls/channel_read — whole-message receive contract, bytes buffer written before handle buffer, overlapping buffers overwrite, BUFFER_TOO_SMALL may return needed counts. These are Fuchsia-specific API semantics, **not proof** of atomic UniOS user memory publication.
- **[S2] Fuchsia zx_channel_read_etc:** https://fuchsia.dev/reference/syscalls/channel_read_etc — actual object type/rights are reported for cap transfer, not inferred from a sender-authored IDL role.
- **[S3] Fuchsia Zircon Kernel Concepts:** https://fuchsia.dev/fuchsia-src/concepts/kernel/concepts — process-local handle table, two-ended Channel, in-transit references, handle install on receive; a working comparator, not permission to copy implementation or proof.
- **[S4] Fuchsia zx_channel_write:** https://fuchsia.dev/reference/syscalls/channel_write — source handle semantics on write failure are not the UniOS E1 proposed NOT_COMMITTED-with-source-preserved contract.
- **[S5] Linux kernel user access:** https://docs.kernel.org/kernel-hacking/hacking.html — general usercopy may leave an uncopied suffix and sleep/fault; holding a spinlock/IRQ-disabled section across it is unsound.
- **[S6] seL4 official IPC tutorial:** https://docs.sel4.systems/Tutorials/ipc — explicit receiver CSpace destination selection, badges, small fastpath, and synchronous model unlike UniOS E1 queue.
- **[S7] Fuchsia Zircon Handles:** https://fuchsia.dev/fuchsia-src/concepts/kernel/handles — handle numbers scoped to a process, transit lifetime and installation; does not prescribe UniOS public-token ABA policy.

**Interpretation guard:** Fuchsia says Channel messages cannot be read partially at the logical message API. That does NOT let us assert an arbitrary UniOS usercopy implementation cannot fault after a prefix or publish a token before a subsequent fault. Conversely, Fuchsia is not evidence that it leaks bytes on failed reads; this paper describes **UniOS design countermodels**, not observed Zircon bugs.

## 2. Head-to-head candidate analysis: strongest claims versus added TCB

| Axis | HPA-B+ one logical Receive commit | HPA-C two-phase Stage/AcceptCommit | HPA-A live install-first |
|---|---|---|---|
| Receive grant to authorized Task | one full-vector K-I | one full-vector K-I at second phase | initial K-I before output |
| Native rights prior to K-I | inert reserved tokens | inert reserved tokens | already live if installed |
| Public token copy failure before commit | burn exposed tokens, terminalize if confidential data visible | same | may have active, unreported caps |
| Caller return/status loss after K-I | needs protected result record or honest UNKNOWN | **same**; extra syscall does not resolve this | same |
| ClaimReference initial issue/report loss | can use preidentified status/result slot; no user-visible separate Stage ID required if single receive | must also resolve initial Claim ID loss, before second phase | needs output/status reconciliation |
| Multiple Tasks/Threads | Task-wide Handle namespace; another B Thread may invoke after K-I | SAME Task-wide rights after AcceptCommit, another B Thread can call Commit | SAME |
| Compromised receiver B | can use transferred bearer rights once K-I succeeds | can simply call AcceptCommit; no independent isolation | can use rights as soon as installed |
| Independent schema/consent proof | not provided; T2 requires trusted verifier | not provided unless T2 trusted verifier added | not provided |
| Partially disclosed secret redelivery | no foreign automatic requeue after K-V | same | same rule needed |
| Token nonreuse/exhaustion | one set of reservations per attempted Receive, including burned values | reservations can be held longer and burned on repeated stage abort | unreported live grants plus tokens need reconciliation |
| Kernel escrow lifetime | bounded intra-operation, receipt/tombstone retained | cross-syscall queue claim, expiry and cross-restart cleanup | may create orphan caps in Task until cleanup |
| Extra kernel transitions | minimal atomic K-I after copyout | Stage state and later explicit AcceptCommit | simpler commit, hard failed output |
| Latency/throughput | **potentially** fewer syscalls and less head-of-line time; **not measured** | **potentially** more round-trips and held queue credits; **not measured** | lower syscall count possible but hard error semantics |
| Kernel TCB | staging, token IDs, revalidation, K-I and protected result | B+ **plus** persistent claim/ref validation/cancellation/GC | grants can become invokable before return |
| Preferred research slot | **general IPC first** after P0 proof | optional deferred acceptance / trusted T2 studies | explicit countermodel, not silently rejected |

**Important correction to v0.1:** v0.1 treated HPA-C's extra explicit Task AcceptCommit as a potentially stronger security baseline. That is too general. A malicious B may call it without reading the message, and even an honest B's successful Stage attempt doesn't guarantee the user buffer will still contain the original data when Commit occurs. Unless the second phase is bound to *independent trusted admission* or an application-specific intentional acceptance requirement, it may add complexity without improving native cap isolation relative to B+.

### 2.1 T1 and T2 are orthogonal, again

**T1 bearer cap delivery:** The sender/issuer MUST attenuate grants such that B is permitted to hold/use them **even if** its IDL method decoder fails after receive. HPA-B+ can enforce native rights unavailable until K-I; HPA-C additionally lets B delay K-I but cannot secure method semantics from B itself.

**T2 trusted method admission:** Even HPA-C cannot claim cap unusable until legitimate method validation unless a separate trusted verifier/issuer checks **immutable kernel-authenticated grant identity**, allowed resource role and current policy, then issues a one-shot scoped release authorizing K-U. A B-provided digest or accepted flag is no substitute. T2 also needs K-V barriers, sponsor/quarantine, fresh session epochs and honest result loss.

## 3. Independent counterexample classes that undermine universal HPA-C

### 3.1 Claim ID bootstrap and lost initial result

If Stage reserves ClaimID X, returns X only in a userspace output, and that return is lost, B cannot safely invoke AcceptCommit(X) or query status by X. A fresh result ticket in a failed return path is useless. HPA-C needs a **preidentified Task-scoped claim/result slot** or privacy-safe bounded enumeration keyed by an independently existing authority. If such a slot already exists, it may also be used by **HPA-B+** to recover single-call K-I results; this eliminates a supposed exclusive advantage of C.

The preidentified slot itself must not silently become a privilege. It needs bounded concurrency, Task-generation binding, Endpoint/method-grant scope, no reuse of stale public slot IDs into unrelated claims and clear issuer/sponsor ownership after Task death.

### 3.2 Public Handle-token exhaustion, with an important nuance

It is unsafe to reuse any token value that *might have been published* to B for an unrelated native grant during that Task lifetime. However **not every internal token reservation necessarily needs permanent burning**: a reservation proven never to have been visible and never to have represented an active unrelated grant may be reclaimable under an explicitly validated namespace rule. Whether a deliberately guessed future value in the **same Task** constitutes a security concern is different from stale authenticated handle reuse; a public handle is not a secret and Task ownership is already the authority boundary.

For possibly published values, fail-closed exhaustion is correct but can become a practical denial of service. A malicious B can Stage, fault after one token byte/array entry, abort and repeat. C's long escrow/timeout may amplify queue and kernel metadata pressure. Candidate mitigations (all unproved): reserve public nonrecycling values on a bounded per-Task issuance budget, retire only potentially observed values, cap failed claims/specific receiver sessions, use large generation-encoded token space and kill/contain Tasks that exhaust their own budget. **No numerical 64-bit token commitment** is made.

### 3.3 Two Threads racing to AcceptCommit

A and B are different kernel Task owners; within B, both B-Thread1 and B-Thread2 may hold the same Task-scoped public claim identity. They simultaneously invoke AcceptCommit. Only one K-I may win; the loser must get an accurate already-committed/terminal status with no second cap-vector install. However, preventing a duplicate K-I **does not** guarantee that only the original initiating Thread can invoke the resulting grants after it wins. Task-wide grants remain Task-wide. Neither C nor B+ provides automatic human-consent or Thread-principal security.

### 3.4 Queue-head starvation, recovery and backpressure

In C, an exclusive Stage may hold queue custody across userspace scheduling and an arbitrary verifier/decode wait. A malicious receiver can indefinitely refuse AcceptCommit, even when another eligible reader is available, unless claim deadlines, cancellation, sponsor quotas and terminal settlement are imposed. After partial secret exposure, the message may need terminal drop, sacrificing delivery. In a one-message queue, a stalled head can block later SEND. The blocked receiver could itself await a reply or T2 verifier whose route depends on credits held by that very claim: a wait-for graph cycle. Independent M0 CPU/IRQ/IPC reserve must not be borrowed for routine claim staging.

B+ still has blocking usercopy/Pager and failed-result issues, but avoids *mandated* arbitrary application scheduling delay between stage and commit. That is a qualitative advantage, not a benchmarked latency guarantee.

### 3.5 A deeper TOCTOU: staged user bytes versus authenticated resource meaning

A compromised B may read/mutate the staged user-memory payload between Stage and AcceptCommit, or B's second Thread may alter it while an external trusted verifier checks a user-supplied field. A T2 verifier relying on mutable B bytes, sender-supplied handle-role metadata or a user-authored hash can approve **a different request than the one whose actual grants the kernel releases**.

If method authorization is promised, bind the verdict to an immutable kernel-owned message identity/content digest, the actual kernel-held cap vector (object identity, rights, scope, lineage), receiver generation, policy epoch and target method/role. Avoid placing a large evolving SABI parser in Ring 0: trusted userspace can validate a bounded sealed/snapshot representation, but its authentication and TOCTOU bridge into K-U remain open. For T1 B+ the message is already intended for the bearer B; the same user-space mutation is an application-integrity issue, not a proof that the kernel must decode SABI.

### 3.6 Effects before Commit and epistemic limits

Once Stage publishes plaintext to B, B can call another service and produce an external effect **before** it calls AcceptCommit. If the claim later terminalizes due to expiry, it is false to tell A "the request could not have been processed." Extra C phases therefore increase the window where **payload was observed but receive was never committed**. The kernel can prevent cap use, but cannot stop B from using other already held caps or network rights absent an explicit independent sandbox/effect gate. Do not promise exactly-once SABI delivery/effect merely because ReceiveClaim was exclusive.


## 4. HPA-B+ bounded candidate transition model

**Scope:** conceptual state machine for design proof, not executable notation or selected syscall sequence. Kernel and receiver Task are adversarially scheduled; actual cap custodianship and caller knowledge are separate.

| Phase | Cap custody / invokability | User-visible payload/tokens | Trust and fault cut |
|---|---|---|---|
| B0 Queued | kernel queue holds all moved grants after sender K-S | none B-side | sender source handles gone; result may be UNKNOWN |
| B1 ClaimBound | kernel-exclusive bounded claim for B TaskGeneration, receiving EndpointGeneration and issuer audience | no secret bytes yet | validates current RECEIVE + claim charge, prevents rival C |
| B2 Reserved | same claim plus destination inactive Handle IDs, cap slots and protected return status record | none/metadata as separately authorized | token IDs may fail closed on namespace exhaustion |
| B3 ExposurePossible | claim holds all grants, **no new cap usable** | some confidential metadata, payload or numeric IDs may be observable; K-Vm/K-Vp recorded before publication | usercopy may fault/alias; no foreign requeue if secret disclosure possible |
| B4 K-ICommitted | B Task cap table atomically owns full grant vector and queue/claim retired | prepared token IDs valid in B, user return still may be lost | live lineage/Endpoint/Task policy and R-B checked at one cut |
| B5 K-ITerminal | claim/message dropped or closed/quarantined, no new B authority | possibly observed bytes or inert IDs; leaked tokens burned | terminal cleanup may outlive Task, physical DMA pending |
| B6 ObservationUnknown | **not an actual kernel ownership state** | initiating Thread may not know B4 versus B5 | preidentified protected receipt query or honest UNKNOWN |

**HPA-B+ obligations:**
1. Provide a Task-scoped preidentified protected result slot or safe, bounded authenticated enumeration **before** an irreversible receive cut. If initial slot allocation result can also be lost, the discovery mechanism must not depend on that missing value.
2. Reserve complete cap table capacity and output staging *before* attempting sensitive user stores. A K-Vm/K-Vp potential-exposure marker must precede the first user-visible disclosure path. No global pageable lock can be held across copyout.
3. Public token IDs possibly copied into user memory remain INERT until K-I; after failed copyout any exposed IDs cannot later resolve to unrelated grants in the same Task. Internally reserved yet *proven never visible/previously live* IDs may be reclaimable under an explicit stronger proof.
4. K-I must have exactly one successful winner against Task death, receiver Endpoint MOVE, revocation R-B, competing receiver/claim abort, and queue close. All or none live grants installed plus exact queued-message retirement.
5. Upon K-I success, return/report can still be lost. Protected result record and cap ownership remain until accounted settlement; an initiating Thread's missing result is NOT proof no native grant exists.
6. After possible confidential exposure but failed K-I, prohibit automatic redelivery of *the same queue message* to a different receiver Task/ServiceIncarnation; settle caps/sponsor/quarantine and preserve possible application effect truth.
7. Do not assume a successful usercopy means B's Thread retained the payload or was the only Task Thread that observed/acted upon it. Policy-sensitive method authority remains a separate service concern.

**Why B+ is still not an accepted design:** An ordinary buffer copy is not guaranteed atomic or stable under alias/mapping changes; real performance, locked critical sections, CPU memory ordering, protected ticket storage, finite public token space and complete source-to-destination ownership proof are missing. B+ could fail these gates and require either C, a register-only narrower transport, or different ABI semantics.

## 5. Competing HPA-C deferred activation model — where it genuinely helps

The first C phase (Stage) is similar to B+ through inactive token publication, but **does not invoke K-I**. A separate, explicit Task AcceptCommit later invokes K-I for T1 bearer rights. If C is used for high-security method admission (T2), distinguish accepting a message from **trusted independent verifier release K-U**. A malicious B calling AcceptCommit doesn't demonstrate independent method authorization.

| Real C use case | Additional promise that may be meaningful | Tradeoff and hidden hazard |
|---|---|---|
| Application controls when Task-wide bearer rights go live | correct application can inspect staged bytes before acknowledging grant | a malicious/misconfigured B can always call Commit; more claim state |
| Asynchronous Task-level receipt admission / batching | B can deliberately defer activation while routing work | queue head held, claim expiration, fairness and claim-lane deadlock |
| Trusted broker decodes immutable message while cap stays inert | broker could apply issuer/service policy before forwarding to worker | broker/verifier is an expanded user-space TCB, need actual-grant attestation |
| Consent-sensitive cap | trusted consent authority can require K-U after verification | not equal to self-accepting Task; old consent/policy epoch invalid at release |
| Long-running interactive desktop IPC | staged work may be visible earlier | extra crossings and DoS surface may hurt responsiveness; **no measured cost** |

**Conditional C acceptance criterion:** document a required pre-activation action or independent trust boundary that B+ cannot satisfy, and bound claim duration/CPU/IPC/metadata budget without blocking M0. If C only separates staging for cosmetic neatness, B+ has a better complexity/availability hypothesis.

### 5.1 Two-phase verification does not prove immutable request meaning

With T2, any verifier receipt must be bound to immutable original bytes (or a proven authenticated digest under kernel/issuer witness), native actual CapGrant identity+rights+scope+lineage, message/claim generations, receiver Task/Endpoint identity, current issuer PolicyEpoch and allowed ResourceRole. A user-created checksum over a mutable alias is not such a witness. The kernel need not parse arbitrary evolving SABI schema, but a trusted verifier/service that does parse it belongs in the explicit policy TCB. Recheck B and policy at activation, even if an earlier trusted user-space verdict was valid.

### 5.2 No universally safe replacement for lost result

A C caller may lose the ClaimID return on Stage, the AcceptCommit return, or the trusted K-U return. Each needs scoped result/authorization independently. A B+ caller may lose only the single receive return, but protected result-discovery remains necessary. **Counting syscalls is not a proof of superior safety**, yet additional independently interruptible results enlarge the state space and verification burden.

## 6. Cross-RFC collision points and repair list

| Existing Proposed contract | Current v0.1 concern | Independent v0.2 correction proposed |
|---|---|---|
| RFC-002 Handle ABA | strict public nonreuse over same Task lifetime | distinguish definitely never exposed reservations from possibly published IDs; retain no unrelated reuse where stale public value can act |
| RFC-003 §6 ReceiveClaim | all-or-none cap vector install | doesn't require extra Task self-accepting phase; B+ and C both can preserve one K-I |
| RFC-003 §§23–24 | v0.1 describes C as a formal proof leader | keep that historical record; add v0.2 independent recommendation that **B+ leads general bearer proof research, C optional** |
| RFC-009 copyout/receipt | irreversible kernel transition and lost results | preidentified Task record is equally available to B+ or C, not a uniquely C benefit |
| RFC-018 SABI role and policy | unknown resource handling, T2 independent verifier | C untrusted Task acceptance does not enforce resource schema; T2 verifier must bind actual immutable request/grant |
| RFC-005/006 CPU/M0 | bounded ordinary and critical budget | C Stage can hold a queue while awaiting verifier/other IPC, potentially deadlocking circular service dependencies |
| RFC-007 DMA R2-F | DeviceCap close separate from hardware quiescence | terminalized receive must not refund DMA pin/IOVA without scoped physical fence |
| RFC-008 filesystem effects | backend already may have emitted writes | C may increase “visible but not received” window; no external no-effect guarantee |
| RFC-010 Linux guest | FD/errno differences | neither extra AcceptCommit nor result receipt converts native UNKNOWN into clean guest no-effect automatically |

**Compatibility with prior CAPIPC proposals:** E1 Endpoint owner uniqueness, MOVE-only source cap transfer, first-possible-exposure terminalization, cap-vector K-I, revocation barrier B, R1/R2-F/R2-O and no native cap from raw SABI numbers remain intact. This is a priority and trust-model refinement, not a newly discovered contradiction in those fundamentals.


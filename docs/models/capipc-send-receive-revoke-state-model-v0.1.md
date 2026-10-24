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

**Actors:** sender Task A; receiver Task B with two Threads; rival Task C; restarted service B2 (new Task/ServiceIncarnation); kernel K; controller R; optional trusted verifier Vt, Pager P and device D. Two grants to same object may be independently rooted; they do **not** count as duplication of the same MOVEd grant. One Channel pair with **separate sender and receiver E1 Endpoint objects** and queue capacity 1; one message M with sensitive byte payload and 0–2 moved grants; a bounded critical M0 reserve. Each half has its own owner and generation; the two halves are never treated as a single jointly owned Endpoint.

| State variable | Finite domain or relationship | Authority / observation |
|---|---|---|
| endpointOwners | SendHalf: A/C/none; ReceiveHalf: B/C/B2/none, each with its own EndpointGeneration | kernel; each distinct E1 Endpoint half has only one effective owning Task |
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


## 4. Guarded abstract transitions — the heart of the research model

These are **human-readable transitions**, not kernel opcodes, formal logic, executable pseudocode, or a claim that every step is currently implementable. A guard is necessary but still needs proof of atomic implementation if code is ever separately authorized.

| ID | Trigger and necessary precondition | One abstract state change | Possible exposed effect / reason to reject |
|---|---|---|---|
| TR-01 | Source A has live distinct transferable grants, owns E1 endpoint; safe bounded snapshot | prepare kernel-private immutable message bytes/handle-descriptor copy | A's later user-memory changes cannot mutate staged message; raw pointer validation alone insufficient |
| TR-02 | queue and sponsor reservations available; source still live | Prepared state with reversible reservations | no handles invalidated yet; failed reservation returns NOT_COMMITTED |
| TR-03 | revoke B, Endpoint MOVE and Task death have not invalidated A/rights; entire cap vector revalidated | **K-S** atomically queues one message, invalidates *all* source public handles and changes grant custody to transit | if even one source invalid, no partial MOVE; source slot duplication rejected |
| TR-04 | caller return status copyout lost after/around K-S | sendObserved becomes Unknown until separately authenticated record resolves sendActual | no speculative return of original handles; returned new ticket alone insufficient |
| TR-05 | eligible ReceiverTask B currently holds receive right, matches message audience and Endpoint generation; claim/slot quota | **K-C** creates exclusive B-bound ReceiveClaim | a rival C cannot simultaneously claim; neither gets bytes yet |
| TR-06 | B has not received user-visible bytes, output ranges and rights validated, cap slots and budgets reserved | private staged receiver buffer, still unexposed | must inspect pointer overflow/overlap and aliases; mapping can still change |
| TR-07 | exclusive current B claim, active issuer/policy audience; publication about to be attempted | **K-V** sets exposed=true and binds recipient-generation before any user-visible store | conservative: publication may fault without actually storing bytes |
| TR-08 | K-V already occurred and B output mapping remains admissible | attempt copyout, updating possible prefix or fault outcome | even partial prefix could be read/forwarded by B; cannot roll back |
| TR-09 | one valid exclusive claim, all cap slots reserved, rights/lineage rechecked; T1 bearer is allowed | **K-I** all-or-none cap-vector install in B plus retirement of message/claim | success does NOT imply B's buffer all-or-none or SABI method accepted |
| TR-10 | T2 used; trusted verifier Vt confirms exact cap vector/role/policy and current issuer proof, claim alive | **K-U** atomically releases specifically admitted inert grant(s) to B under bound identity/epoch | untrusted B self-attested boolean fails this guard |
| TR-11 | no K-V ever attempted; B claim aborted/died; queue still eligible and current audience authorizes future claimant | return owned caps to kernel queue and release exclusive claim, without restoring A | preserve ordering/budgets; if service instance changed re-evaluate audience |
| TR-12 | K-V occurred; claim abort, receiver death or copyout failure | mark terminal/settlement; no same-message automatic requeue to *different* recipient generation | bytes possibly exposed; release or quarantine caps and charge |
| TR-13 | still same verified B generation and service/policy scope, bounded attempt budget | continue same B-bound claim if explicitly allowed | do not use name-only equality or fresh B2 to reopen old claim |
| TR-14 | unknown SABI cap role after K-S and (for T1) K-I | authenticated receiver/escrow closes or quarantines mismatched grants | no retroactive sender restoration; no physical R2-F guarantee |
| TR-15 | authorized controller publishes B barrier for grant family | deny **new** grant use/derivation/admission under affected lineage | already-admitted work and independent roots need separate fences |
| TR-16 | an in-flight Endpoint ownership MOVE admitted | serialize with all earlier active SEND/RECEIVE invocation cuts or cancel uncommitted old ones, then install one new owner | cannot give old and new Task unrestricted concurrent ownership |
| TR-17 | source/receiver Task dies, Channel peer closes or Claim timeout | charge and cleanup according to current phase; terminalize if exposed | sender death is not rollback of K-S; receiver death doesn't erase disclosure |
| TR-18 | physical cap involved and R2-F unresolved after close/abort | retain quiescence/fence debt and quarantine pinned resources | no frame recycle or false DMA complete |
| TR-19 | backend effect happens after an admitted service method, then reply lost | externalEffect may be Applied while caller sees Unknown | no blind replay/non-effect errno solely from timeout |
| TR-20 | complete independent reconciliation record and authorized caller scope | update caller knowledge of actual kernel/remote result | query may confirm K-S but **not** independently infer X-E or scoped durability |
| TR-21 | resource settlement proven, remaining object roots and pins fenced | settle/release sponsor budget and reusable object when R2-O applicable | R0 handle close alone doesn't satisfy condition |

### 4.1 Send and revoke: same cut, different clocks

A preflight snapshot and queue reservation may succeed, then a controller revokes the grant family or another Thread moves the source Handle/Endpoint before commit. TR-03 requires revalidation of **actual live** rights and unique holder against revocation's own atomic publication cut. Either the transfer linearizes first (transit may later become revoked) or B linearizes first (a new prohibited transfer fails); no half-committed mixture. A cap that was valid in the queued record need not remain usable at eventual TR-09/K-U. The current grant lineage at each new-use cut is authoritative.

**Admitted-operation caveat:** some kernel invocation references may have begun before B; B does not automatically rewind their earlier changes. Give such references a bounded, named **admission generation** and distinct stop/fence policy. A device command submitted pre-B may still need R2-F; an external file write may remain EFFECT_UNKNOWN despite cap close.

### 4.2 Receive and revoke: visibility precedes knowledge

An observer can read B's writable output memory concurrently with kernel writing it. Therefore TR-07's conservative exposed=true must happen **before** the first attempted user-visible write. A later fault can stop TR-08 but not turn exposed back to false. If the recipient B had actual access authority to the data, this is not automatically a breach in isolation: the new prohibited action is **uncontrolled automatic redelivery of the same queued secret to a different audience** after B had possibly observed it.

**Important availability tradeoff:** the terminal rule can discard a message even if TR-07 was marked and the very first write faulted without copying a byte. This intentionally prefers a bounded single-audience disclosure guarantee over guaranteed delivery. Higher-level services must use their own authenticated new transaction/retry protocol if resending is needed. Such explicitly authorized new delivery is NOT the same message being silently requeued by the kernel and requires a new distinct identity/effect analysis.

### 4.3 Cap installation ordering and limits of pre-reservation

Compare two candidate orders:

| Order | Positive property | Bad path |
|---|---|---|
| B1: K-C → private reserve → K-V → bytes copyout → K-I | receiver shouldn't use cap before payload publication; partial byte faults become terminal/no caps | message may be lost after B sees bytes, before install; no atomic bytes+caps |
| B2: K-C → private reserve → K-I → K-V → bytes copyout | cap table transfer may be accomplished before pageable copyout | receiver can use cap when copyout later fails; no method-level validation |
| B3: K-C → private reserve → K-V → copyout → T2 inert escrow → trusted K-U | trusted authorization before usable grant | extra escrow state and trusted verifier, resource starvation and post-V terminal rule still needed |

**Recommendation:** study B1 as normal T1 proof target with preallocated destination cap slots and bounded staged usercopy, and B3 only for explicitly demanding resource/consent profiles. Pre-allocation may lower but does not eliminate errors due to Task death, mapping changes or pager issues. The kernel must not hold a global lock across pageable usercopy. The claimed whole-vector K-I is a **kernel table** atomicity goal, not a proof of atomic reads by other userspace Threads.

### 4.4 Terminal settlement must own the forgotten grant

When TR-12 terminalizes a message, it must not silently return cap ownership to A or leak an orphan to an uncharged queue. The receiver B may hold no installed cap (B1 before K-I), or may already hold usable grants (B2), or kernel escrow may hold inert grants (B3). Define a separate settlement branch for each. In all cases:
- the original source public handles invalidated at K-S stay invalid;
- the message cannot be silently delivered again to C/B2 after exposed;
- every pending cap/invocation/ReplyToken/pinned page carries a sponsor;
- DeviceGroup physical R2-F and independent-root R2-O are separate, possibly never completing;
- X-E remote work, FileSync and X-O caller knowledge are not inferred from queue terminalization.

## 5. Safety properties and desired counterexample outcomes

| ID | Candidate safety property (NONE proved) | Counterexample it must rule out |
|---|---|---|
| SM-01 | one effective E1 public Endpoint owner with transfer-order guarantee | Task A and C send concurrently after MOVE |
| SM-02 | one kernel custodian for each particular MOVEd Grant, with all-or-none K-S | both sender/queue own grant or partial vector lost |
| SM-03 | real NOT_COMMITTED preserves sender handles; ambiguous caller gets UNKNOWN | false rollback from lost return |
| SM-04 | public handle anti-ABA under reuse/exhaustion | stale Task integer becomes unrelated live authority |
| SM-05 | no byte/publication before correctly authenticated eligible receiver claim | C reads without RECEIVE or correct audience |
| SM-06 | exposed flag monotonic, marked no later than first attempted visible write | later copyout fault resets secrecy history |
| SM-07 | no automatic foreign/new-instance redelivery of same M after exposed | B reads prefix; C dequeues whole M |
| SM-08 | pre-exposure requeue only with current rights, audience and bounded claim age | revoked/new-owner Task reads old secret |
| SM-09 | all-or-none receiver kernel cap vector K-I, independent of byte-copy visibility | half installed cap vector |
| SM-10 | T1 bearer grants limited to rights B may hold even if schema decode fails | unexpected DeviceGroup right usable pre-validation |
| SM-11 | if T2 claims trusted method admission, K-U cannot be authorized by untrusted B alone | malicious B releases its own escrow with fake valid=true |
| SM-12 | revocation B prohibits newly admitted actions of affected lineage in queue/claim/installed table | queued revoked cap becomes newly invokable |
| SM-13 | distinct independent root not destroyed by a single family revoke | parent F revoke frees object held by G |
| SM-14 | no false R2-F on close/abort and no R2-O before all roots fenced | stale DMA corrupts recycled frame |
| SM-15 | every queue/claim/escrow/IRQ/pin cleanup and orphan has sponsor or quarantine charge | Task death frees quota while physical effect live |
| SM-16 | stuck malicious ReceiveClaim cannot exhaust protected wired M0 reserve | claim forever blocks recovery |
| SM-17 | queued Channel handle containment closes within bounded cleanup policy | self-send/cyclic queue unbounded recursion |
| SM-18 | method admission and policy-epoch binding never supersede actual kernel grant ceiling | caller forges new policy/ServiceID |
| SM-19 | timeout/ReplyToken loss doesn't prove service effect was absent | file write applied, caller blindly retries |
| SM-20 | no statement of full buffer atomicity, global confidentiality or exact-once without evidence | one visible prefix reported as no disclosure |
| SM-21 | kernel commit ticket correlation is recoverable or explicitly UNKNOWN if ID lost | false certainty from an unrecoverable ticket |
| SM-22 | all Proposed safety properties remain research-only and require future proof | paper matrix advertised as working OS |


## 6. Counterexample schedule catalogue (MX-01…MX-18)

An interleaving is a **future research test obligation**, not executed evidence.

| ID | Schedule | Expected design outcome |
|---|---|---|
| MX-01 | A snapshots source cap; R publishes revoke B; A attempts K-S | reject new action or prove K-S linearized before B, never both |
| MX-02 | A SEND vector includes same public Handle twice | all-or-none reject, no double ownership |
| MX-03 | A K-S commits, status and newly generated ticket ID lost | caller Unknown; no blind MOVE replay; preidentified query if adopted |
| MX-04 | A in-flight SEND; ownership of that E1 Endpoint transfers to C | one serialization/cancel rule; no concurrent unrestricted owner |
| MX-05 | Receiver B K-C; buffer count valid; other Thread unmaps next page | no global kernel lock held through fault; potential exposure tracked |
| MX-06 | B K-V then first write faults before a byte lands; B dies | still terminalized conservatively; delivery can be lost |
| MX-07 | B K-V; reads 16-byte prefix; K-I later fails; rival C claims | C cannot automatically receive same message |
| MX-08 | B K-V; service crashes; ServiceID reused by B2 | B2 not treated as identical authorized recipient generation |
| MX-09 | B K-V and K-I complete; SABI rejects unknown resource | moved cap settled/closed, B's observed bytes not erased |
| MX-10 | B receives live DeviceCap under T1; method malformed; B uses cap before decoder | only allowable if B intentionally granted bearer authority without needing SABI admission |
| MX-11 | B receives inert DeviceCap under T2; B forges valid=true to release it | release refused absent trusted issuer/verifier evidence |
| MX-12 | T2 trusted verifier unavailable while B holds claim and M0 waits on same channel | bounded denial/quarantine, no guaranteed progress from untrusted work |
| MX-13 | R publishes B after K-S but before K-I or K-U | no new usable descendant rights after barrier; old effect reconciled separately |
| MX-14 | B receives into overlapping byte/handle arrays via VA alias | reject or prove independent staged layout; no corrupted output field relied on |
| MX-15 | last Channel handle closes while queued nested Channel cap points back | no unbounded kernel recursion/stack/charge leak |
| MX-16 | A sponsor exits while terminalized message holds DMA pin | charge and quarantine continue until real R2-F |
| MX-17 | backend file mutation completes, B dies before response, A times out | effect Unknown from A, not definitely None |
| MX-18 | newly negotiated SABI P1 changes required consent while old P0 claim remains | live current issuer policy enforced before new effect or no work |

## 7. Resource budgets and liveness — assumptions must be explicit

### 7.1 Symbolic resource envelope, not measured values

| Resource | Symbolic bound to choose later | Consequence if it is exhausted |
|---|---|---|
| per-Task outstanding SEND records and preidentified ticket slots | Q_send | reject precommit or return protected Unknown; cannot consume M0 reserve |
| per-Channel queued bytes, caps and messages | Q_msg, B_msg, H_msg | fail precommit without destroying sender handles |
| concurrent ReceiveClaim count and maximum age | Q_claim, T_claim | refuse/abort/terminalize per exposure, no unbounded claims |
| staged kernel-private bytes and receiver cap slots | B_stage, H_slots | prior to K-V reject or reserve before publication |
| terminal settlement records and possible disclosure history | Q_term, T_term | bounded policy; cannot drop safety-critical outstanding debt |
| T2 inert escrow cap entries and authorized release tickets | Q_escrow, T_escrow | fail closed/revoke; protect trusted verifier from queue starvation |
| cap ancestry/revocation and parent-issuer metadata | Q_anc | deny new grants or retain charged immutable existing ancestry |
| queued Endpoint containment edges and cleanup work | Q_edges, K_cleanup | iterative bounded cleanup/quarantine, not recursive kernel stack |
| pinned DMA/IRQ/ATS and outliving sponsor charges | Q_dma, M_pin | quarantine/fail-stop; never free while fence incomplete |
| wired RecoveryAuthority CPU, stack, timer, Pager-independent IPC | M0_reserve | cannot be borrowed by untrusted protocol decode workload |

The bounds are **placeholders only**: not memory sizes, latencies or feasibility evidence. Resource accounting must assign every live item a sponsor or designated quarantined owner with protected reserved budget. Even terminal-message metadata cannot grow unbounded; propose bounded final receipt/tombstone retention with exact safety invariants across GC. Reclamation of a status record cannot make an old public ticket ID authorize a new unrelated result (anti-ABA).

### 7.2 What conditional liveness can reasonably mean

Under an operating scheduler, bounded kernel critical work, resident M0 reserve, healthy Pager and fair service, each eligible request either completes, safely rejects, enters terminal settlement or returns a declared Unknown within a separately specified finite budget. A malicious receiver may force many terminal drops, but cannot pin the entire kernel indefinitely *if* claim expiry, accounting and cleanup are correct.

**Not promised:** successful remote effect when service crashes; delivery after conservative K-V terminalization; hardware R2-F after an unresponsive device; recovery from corrupt firmware; true exactly-once external I/O across uncoordinated devices. A system can remain safe while it refuses work, quarantines hardware or enters fail-stop.

### 7.3 Trust/TCB and M0 availability comparison

**RCV-T1** requires protected claim, exposure, cleanup, bounded usercopy and correct cap install. This is already complicated; require small auditable kernel state and no arbitrary IDL parse in Ring 0.

**RCV-T2** also requires inert grants, release authorization, verifier liveness, policy-epoch binding and more persistent kernel custody. Extra IPC to a verifier could depend on the same congested Channel/Pager and deadlock recovery. Trusting an *untrusted client* to attest validation voids T2's special benefit; trusting a parser/verifier enlarges an explicit user-space TCB. A small high-risk service may find that trade defensible, but do not apply T2 to every data-only RPC without evidence.

## 8. Cross-RFC contract reconciliation register

| Document | Contract owned there | Proposed constraint from this research |
|---|---|---|
| RFC-002 | cap lineage, DERIVE rules by type, sponsor, Handle anti-ABA, B/R1/R2-F/R2-O | queued/claimed/inert grants remain lineage-checked, single-custody and charged; no source restoration after K-S |
| RFC-003 | E1 exclusive Endpoints, SEND/MOVE, ReceiverClaim, ReplyToken and critical queue credits | publish a separate K-V exposure guard, no foreign requeue after V, whole-vector K-I and Endpoint handoff order |
| RFC-004 | copyin/out, memory aliases, Pager faults, pinned user pages and immutable snapshots | no assumption that pointer preflight, VA nonoverlap or attempted usercopy makes bytes atomic |
| RFC-005 | scheduler, CPU/IRQ and critical budget / admission | ReceiveClaim/verifier/cleanup cannot indefinitely occupy entire critical reserve |
| RFC-006 | M0 wired path, service restart and authenticated current incarnation | B versus B2 Task/ServiceIncarnation identity and fail-stop, no old-same-name claim revival |
| RFC-007 | DeviceGroup, DMA/ATS/IRQ and true hardware R2-F | cap close/terminal drop does not prove stopped DMA or safe frame reuse |
| RFC-008 | storage durability, dirty generation, external-effect receipts | IPC status/receive abort does not mean VFS operation rolled back |
| RFC-009 | bounded KABI copyin/copyout, status loss, SDK ABI, guest/native entry | model SEND actual vs caller knowledge, no numeric opcode/status/struct freeze |
| RFC-010 | Linux guest FD/errno/futex/exec/signal semantics | no automatic mapping of partial exposure/unknown effect into safe no-effect errno |
| RFC-018 | SABI unknown-resource roles, issuer, ServiceIncarnation, PolicyEpoch | trusted verifier required for T2 promise, no kernel full IDL parser or user-forged resource release |
| CAPIPC v0.2 | K-S, K-C, K-V, K-I and terminal after exposure | this finite model is a proposal refinement, not superseding parent RFC decisions |

**Authority hierarchy:** Only architecture D1–D8 broad direction Accepted. CAPIPC v0.2 and all these RFCs remain Proposed; this document cannot silently settle their future ABI numeric layout or impose a universal kernel T2 parser. To change a source RFC, its own Proposed spec must be separately reviewed and updated.

## 9. Proposed decisions (MD-01…MD-12) — NONE Accepted

- **MD-01:** RCV-T1 is conditional first model target for general native IPC; no working implementation or approval claimed.
- **MD-02:** A monotonic conservative exposure barrier K-V before the first attempted user-visible publication is an essential proof obligation.
- **MD-03:** A message with exposure flag cannot be automatically requeued to a distinct receiver Task/ServiceIncarnation; terminalize or bound to same verified context.
- **MD-04:** A conservative terminal decision may drop a message without actual byte publication and sacrifices delivery availability for a specified single-audience property.
- **MD-05:** RCV-T2 is optional, composable with K-V/terminalization, and justified only if cap usability needs independent method-policy admission.
- **MD-06:** An untrusted receiver cannot attest its own T2 release using a boolean, IDL string, plain RequestID or unchecked hash; independent verifier/issuer authority required.
- **MD-07:** Every K-S/K-I/K-U/revoke/action admission has separately identified linearization and current live rights/epoch checks; no implied unified atomic usercopy.
- **MD-08:** Candidate receive normal order is protected staging and slot reservation, K-V, usercopy, then all-or-none K-I; alternative ordering and inert escrow remain P0 tests.
- **MD-09:** Lost SEND result needs preidentified protected query identity or fail-closed Unknown; a freshly returned lost ticket is not sufficient.
- **MD-10:** Queued Channel/cap cycles, receiver death and DMA failure require bounded charged cleanup/quarantine, not unlimited recursive kernel work.
- **MD-11:** A backend effect/FS receipt/guest errno is not inferred from kernel SEND/RECEIVE status; explicit reconciliation is method-specific.
- **MD-12:** All design/model content remains Proposed; no source code, executable model, formal result, performance number, ABI freeze or security guarantee is authorized.

## 10. Go/No-Go gates and research sequence

**MODEL-R1 approval: OPEN / NOT PASSED.**

1. Decide exact E1 endpoint ownership/in-flight invocation ordering, with sender and receiver endpoint halves **separately identified**, no shared-object assumption.
2. Specify preflight snapshot/overlap/alias, type/source uniqueness, multiple-grant all-or-none K-S, concurrent revocation and queue-full precommit behavior.
3. Provide a bounded trusted preidentified ticket/reconciliation policy for post-K-S lost status **or** explicitly fail closed as Unknown.
4. Prove exclusive claim and eligibility/audience validation before K-V, with no unapproved bytes published by count/status probes.
5. Define K-V conservative first-write point, immutable exposed state, possible prefix and no foreign redelivery under Task death/Endpoint transfer/service restart.
6. Reconcile B1 versus B2 K-I placement after/before copyout with all-or-none vector install and no claim of all-or-none user-visible buffer writes.
7. Choose T1 bearer-grant attenuation policy, or demonstrate T2 independently trusted exact-cap-vector release and bound issuer/PolicyEpoch; do not accept B self-attestation.
8. Model current revoke B and independent roots through queue/claim/K-I/K-U, with old admitted work separate from device R2-F and object R2-O.
9. Prove sponsor/queue/escrow/cleanup/tombstone/DMA pin conservation across death, cancellation, receiver manipulation and cyclic Channel containment.
10. Explicitly model already applied remote file/network effect and truthful status/guest translation despite lost reply.
11. Independently review state transition assumptions, security objectives and resource cost against RFC-002/003/009/018 before choosing any formal notation or implementation task.
12. If a future approval **explicitly** authorizes executable formalization, select a bounded TLA+/Alloy model, run its counterexamples and only then consider a separate kernel implementation decision. No such approval exists today.

**Next recommended design step:** independent re-review of this finite model's **K-V/R2-F/receive progress** details and a focused RFC-003 Proposed spec alignment. Do **not** label the model verified, do not freeze KABI call signatures, and do not claim at-most-once execution or guaranteed delivery.

## 11. Primary technical sources and scope limits

- **[S1] Fuchsia Channel object, unique endpoint ownership and transfer ordering:** https://fuchsia.dev/fuchsia-src/reference/kernel_objects/channel
- **[S2] Fuchsia zx_channel_write, repeated handles, self-channel transfer rejection and consume-on-failure:** https://fuchsia.dev/reference/syscalls/channel_write
- **[S3] Fuchsia zx_channel_read, whole-message semantics, byte-before-handle output, overlapping buffers and size errors:** https://fuchsia.dev/reference/syscalls/channel_read
- **[S4] Fuchsia zx_channel_read_etc, actual handle type/rights validation:** https://fuchsia.dev/reference/syscalls/channel_read_etc
- **[S5] Fuchsia FIDL language specification, strict/flexible and value/resource union:** https://fuchsia.dev/fuchsia-src/reference/fidl/language/language
- **[S6] Fuchsia Zircon Handles, Task-local numbers and in-transit ownership:** https://fuchsia.dev/fuchsia-src/concepts/kernel/handles
- **[S7] Fuchsia current kernel object lifetime and garbage collection concepts:** https://fuchsia.dev/fuchsia-src/concepts/kernel/concepts
- **[S8] seL4 IPC overview: synchronous badged endpoints and reply state as different design:** https://docs.sel4.systems/Tutorials/ipc
- **[S9] Fuchsia zx_channel_write_etc, different move/duplicate and error dispositions:** https://fuchsia.dev/reference/syscalls/channel_write_etc

**No guarantee transfers** from Fuchsia or seL4 to UniOS. Fuchsia basic zx_channel_write consumes source handles on failure, while UniOS E1's proposed true precommit NOT_COMMITTED retains handles; this is an intentional semantic difference. Fuchsia's read buffer overlap and invalid pointer documentation does not prove that arbitrary UniOS ReceiveClaim usercopy is all-or-none. This document did not license-audit or incorporate third-party source code and did not execute a proof/test.

---

**Document history:** 2026-10-25 proposed independent finite state model v0.1 synthesizing CAPIPC v0.2 and RFC-002/003/009/018. RCV-T1 chosen only as a *research leader*, RCV-T2 reserved as an independently trusted additional resource-admission profile. Twenty-one (21) proposed transition guards, twenty-two (22) candidate safety predicates, eighteen (18) unexecuted interleavings, twelve (12) Proposed decisions and twelve (12) unpassed Go/No-Go gates. No implementation, model checker, hardware test or validated compatibility.

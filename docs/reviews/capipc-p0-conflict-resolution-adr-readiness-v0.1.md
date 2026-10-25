# UniOS — CAPIPC P0 Conflict Resolution & ADR Readiness Matrix v0.1

> **Status:** Proposed, Design-Only Conflict Triage / ADR Preparation — 2026-10-25 NZDT. **NOT Accepted, implemented, formally proved, model-checked, benchmarked, hardware-validated or ABI-frozen.**
> **Purpose:** Audit the 18 OPEN blockers in the [Unified SEND–RECEIVE Semantic Contract v0.1](../contracts/capipc-unified-send-receive-semantic-contract-v0.1.md) and propose a finite, reviewable decision order and conditional ADR acceptance criteria. **A candidate preference is NOT a decision approval.**
> **Scope:** Research / architecture documentation only. No source code, runnable formal model, simulated kernel, test harness, syscalls, numeric Handle widths, SDK or implementation changes.
> **Baseline status:** The eight architecture directions in [Architecture v0.2](../architecture/unios-architecture-v0.2.md) are accepted **at direction level**, while CAPIPC RFC-002/003/004/005/006/009/018, CO-S/CO-E-C, HPA-B+, KR-M2 and this matrix remain Proposed.
> **Primary sources:** [Unified SEND–RECEIVE Contract](../contracts/capipc-unified-send-receive-semantic-contract-v0.1.md); [CO Commit/Revocation](../models/capipc-commit-revocation-ordering-v0.1.md); [RP Receive Exposure/K-I](../models/capipc-receive-exposure-ki-handle-publication-v0.1.md); [Formal Model Preparation](../models/capipc-formal-model-preparation-v0.1.md); [Cross-RFC Safety Case](capipc-cross-rfc-safety-case-v0.1.md).
> **Owners:** [RFC-001](../rfcs/rfc-001-kernel-boundary-and-tcb.md) TCB, [RFC-002](../rfcs/rfc-002-object-capability-lifetime-delegation-revocation.md) Capability, [RFC-003](../rfcs/rfc-003-ipc-wire-protocol-and-synchronization.md) IPC, [RFC-004](../rfcs/rfc-004-virtual-memory-pager-cache-and-oom.md) VM/Pager, [RFC-005](../rfcs/rfc-005-scheduling-preemption-and-cpu-budget.md) scheduling, [RFC-006](../rfcs/rfc-006-boot-trust-chain-service-startup-and-recovery.md) M0, [RFC-007](../rfcs/rfc-007-user-space-drivers-dma-and-iommu.md) DMA, [RFC-008](../rfcs/rfc-008-storage-filesystems-and-crash-durability.md) durability, [RFC-009](../rfcs/rfc-009-native-abi-and-system-call-contract.md) KABI, [RFC-010](../rfcs/rfc-010-linux-personality-and-binary-compatibility.md) Linux boundary, [RFC-018](../rfcs/rfc-018-versioned-idl-abi-evolution-and-compatibility-contracts.md) SABI.

## 0. Verdict and explicit limits

**Present verdict: documentary alignment, P0 NOT RESOLVED; ADR NOT READY FOR ACCEPTANCE.** The joint RFC design points toward a plausible first E1/T1 protocol, but it still lacks a proven bounded atomic K-S / K-I procedure, a fixed user output publication micro-order, a measured or bounded M0/Pager dependency graph, and recoverable finite receipt/token budgets.

**The most valuable near-term simplification is to define a narrow verification target instead of silently approving a universal design:**

- **SEND proof candidate:** CO-S = revoke deny barrier B and actual K-S serialize; A(q) is only admission identity, not perpetual authorization. CO-E-C = old uncommitted invocations are canceled/settled before effective E1 owner MOVE. Atomic whole Grant vector or no SEND.
- **RECEIVE proof candidate:** normal **T1 bearer** HPA-B+ family with exclusive audience-bound Claim, conservative Vm/Vp/Vt visibility, pre-reserved inert public tokens, all-or-none K-I and a protected receiver receipt. Exact token/payload/K-I micro-order remains UN-O07; **the candidate family does not establish feasibility**.
- **Receipt candidate:** KR-M2 Task-generation scoped C/Used replay fence + bounded R[q] for multi-Thread paper work; KR-M1 strict sequential baseline retained, no promise of unbounded progress under missing q.
- **Recovery candidate:** message custody, exposed information, Task receipt, Sponsor/Lineage, M0 and physical R1/R2-F/R2-O all have separate owner/lifetime. A sender COMMITTED_KS cannot be undone by a failed recipient K-I.
- **Scope reduction:** optional **T2 method-before-native-use** is a separately authorized trusted-verifier track; unresolved T2 verifier machinery must not block a clearly labeled ordinary T1 baseline that explicitly promises no such gate. DMA hardware guarantees require physical evidence and cannot be inferred from paper IPC safety.

**Red line:** do not accept an ADR merely because its English requirements seem consistent. A concrete, independently audited transition/refinement argument and later separately authorized verification artifacts are required for safety-critical acceptance; there is no runnable model or implemented UniOS kernel today.

## 1. How conflicts and readiness are classified

| Class | Exact meaning | Consequence |
|---|---|---|
| **TX-CONFLICT** | Two concurrently current normative design requirements cannot both be true for the same state/inputs | explicit RFC revision necessary |
| **CUT-AMBIGUITY** | Prose leaves incompatible orderings/authorization outcomes possible | select precise candidate transition before any formal model |
| **PROOF-GAP** | Desired contract is coherent on paper but no sound bounded/SMP/GC evidence exists | no Accepted ADR/ABI |
| **LIVENESS-GAP** | Finite resources, death, missing q or Pager cycle can block progress | bound/block/terminal fallback; never promise universal progress |
| **POLICY-CHOICE** | Different product trust/availability promises; not inherently a kernel bug | split profiles, name authority issuer |
| **EXTERNAL-EVIDENCE** | Hardware/VM/media/guest assumptions cannot be proven by native IPC model alone | separate tests/hardware evidence before product guarantee |
| **WORDING-DRIFT** | Earlier broader phrase remains in a base RFC even though proposed addendum narrows it | normalize at future RFC consolidated revision; do not pretend root RFC is already rewritten |

**Current classification:** no newly **proven live, directly contradictory** RFC requirements found during this review. This does **not** mean a safe implementation exists: most blocking findings are **CUT-AMBIGUITY and PROOF-GAP**, and some involve inevitable liveness trade-offs. The earlier E1 DERIVE rights-table contradiction was reportedly editorially corrected in previous reviewed documents; the current matrix does not recertify unrelated RFC sections.

### 1.1 Decision gate language

- **D0 — study recorded:** comparison and paper race scenarios exist. Many CAPIPC areas satisfy D0.
- **D1 — semantics reviewable:** exact protected cuts/guards, principal, status, terminal cases, allowed resource limits and failure fallback documented; alternatives explicitly rejected/conditional. Several rows still fail D1.
- **D2 — future formal acceptance candidate:** non-vacuous finite concurrency model/checker evidence plus independent abstraction audit, with remaining assumptions declared; **not performed**.
- **D3 — later implementation-ready ADR:** D2 plus concrete SMP/memory/lock, quota, ABI and code-refinement plan approved by separate decision; **not performed**.
- **D4 — production assurance:** implementation tests, hardware/IOMMU/FS power-loss and adversarial benchmark evidence matched to claims; **not performed**.

**A document labeled “ready to D1 review” does NOT mean Accepted.** D2/D3/D4 require new separately authorized activities. No current text grants automatic permission to perform them.

## 2. Scored priority method — heuristic, not measurement

To avoid subjective blanket P0 rank, use **4 design factors** for each workstream:

- **S — safety impact (1–5):** potential duplicate native grant/unauthorized access or disclosure (5) versus local availability (1).
- **C — coupling/blockage (1–5):** number/depth of dependent cuts and RFC owners.
- **F — feasibility unknown (1–5):** number of unresolved consistency/concurrency/resource mechanisms, not percentage of failure.
- **V — validation load (1–5):** future abstract-state + implementation/hardware verification burden.

**Triage score = 3S + 2C + F + V.** This is an **editorial rank**, not a security risk probability, measured effort in person-days, or justification to weaken any invariant. Resource and M0 risks may be high despite smaller S than unauthorized cap minting.

| Workstream | S | C | F | V | Heuristic | Readiness | First evidence needed |
|---|---:|---:|---:|---:|---:|---|---|
| **TR-01 K-S/B/E-M atomic ownership** | 5 | 5 | 5 | 5 | **35** | D0 only | exact competing-cut winner/loser relation + no-half-send proof |
| **TR-02 K-I + Vm/Vp/Vt + revoked vector** | 5 | 5 | 5 | 5 | **35** | D0 only | full-vector K-I/terminal race and first-publication contract |
| **TR-03 M0/Pager/wait-for and all-accounting** | 5 | 5 | 5 | 5 | **35** | D0 only | bounded resource conservation + acyclic critical wait graph |
| **TR-04 receipt q/GC/public token ABA** | 5 | 4 | 4 | 4 | **31** | D0 only | q replay fence, exact token nonrebind and lost-return statuses |
| **TR-05 claim audience and metadata secrecy** | 5 | 4 | 4 | 4 | **31** | D0 only | audience/policy cut plus confidential Vm/Vp no-foreign requeue |
| **TR-06 cleanup, endpoint cycles, physical revoke** | 5 | 5 | 5 | 5 | **35** | D0 only | lineage/custody graph, bounded terminal/quarantine and R1/R2-F separation |
| **TR-07 T2 trusted method activation (optional profile)** | 5 | 4 | 5 | 5 | **33** | D0 only for T2 | independent verifier authority, immutable original provenance, K-U |
| **TR-08 service effects/guest/compatibility** | 4 | 4 | 4 | 5 | **29** | D0 only | explicit kernel-vs-effect-vs-durability boundary and non-retry mapping |
| **TR-09 runtime alternatives, fairness, future refinement** | 4 | 4 | 4 | 5 | **29** | D0 only | finite gap/credit tradeoffs, measurable profiles, refinement map |

**Use:** TR-01/02/03 form a **joint critical chain**, not strictly serial milestones: a syntactically neat K-I model can still deadlock Pager; queue cleanup cannot be judged without the revoked-vector terminal path. TR-07 does **not** become universal baseline P0 merely because optional high-security T2 has P0 requirements within its own scope.

## 3. All 18 Unified-contract blockers mapped to proposed decision packages

**All rows remain OPEN.** Owner references point to Proposed research responsibility, **not** accepted code ownership.

| Unified ID | Dominant class | Decision package / next paper artifact | D1 exit test |
|---|---|---|---|
| **UN-O01** | CUT-AMBIGUITY + PROOF-GAP | **ADR-CAPIPC-A**: K-S, multi-Grant MOVE, B, Close and E-M shared order | no possible split vector/double owner; no faultable commit lock |
| **UN-O02** | CUT-AMBIGUITY + LIVENESS-GAP | **ADR-CAPIPC-A**: Endpoint removal→InTransit→recipient effective owner / CO-E-C | no old-owner K-S after owner cut; bounded cancellation or honest stall |
| **UN-O03** | PROOF-GAP + LIVENESS-GAP | **ADR-CAPIPC-E**: queue/Endpoint cycles/terminal cleanup | bounded iterative cycle handling or explicit excluded graph in M0 |
| **UN-O04** | CUT-AMBIGUITY + LIVENESS-GAP | **ADR-CAPIPC-D**: preknown sender/receiver q, protected R, KR-M1 vs KR-M2 | no lost bootstrap, replay after final GC, or infinite-unclaimed q progress promise |
| **UN-O05** | CUT-AMBIGUITY + POLICY-CHOICE | **ADR-CAPIPC-B**: Task/Egen/issuer Audience at K-C | no claim carryover after Endpoint move or ServiceIncarnation change |
| **UN-O06** | POLICY-CHOICE + PROOF-GAP | **ADR-CAPIPC-B**: metadata projection and safe requeue rule | confidential sizes/type cannot escape via buffer-too-small or status |
| **UN-O07** | CUT-AMBIGUITY + PROOF-GAP | **ADR-CAPIPC-C**: Vm/Vp/Vt vs K-I, token-first/install-first/register-only | no active-before-authorized, no missed publication/ABA, no global Pager lock |
| **UN-O08** | CUT-AMBIGUITY | **ADR-CAPIPC-C**: B on one mandatory queued grant between K-S/K-I | either exactly N real current grants installed or none, never N−1 falsely final |
| **UN-O09** | WORDING-DRIFT + PROOF-GAP | **ADR-CAPIPC-C/D**: NO_KI × possible exposure × observer UNKNOWN | no “no data/no effect” inference from native no-install or lost result |
| **UN-O10** | PROOF-GAP | **ADR-CAPIPC-C**: K-I race with Task death/Claim/queue close | one winning full-vector install+queue retirement+R_R status, or safe terminal |
| **UN-O11** | PROOF-GAP + LIVENESS-GAP | **ADR-CAPIPC-D**: token no-rebind and caller result loss | finite namespace exhaustion fails closed, never remints a committed Grant |
| **UN-O12** | PROOF-GAP + LIVENESS-GAP | **ADR-CAPIPC-E**: all sponsor/queue/claim/token/receipt/pin budgets vs M0 | finite charged upper bound, never borrow essential recovery credit |
| **UN-O13** | PROOF-GAP + LIVENESS-GAP | **ADR-CAPIPC-E**: Pager faults, lock wait graph and cancellation | no forbidden cycle or unbounded faultable critical section |
| **UN-O14** | POLICY-CHOICE + PROOF-GAP | **ADR-CAPIPC-F**: optional independent T2 verifier | required ONLY before claiming method-gated native grants |
| **UN-O15** | EXTERNAL-EVIDENCE | **ADR-CAPIPC-E / RFC-007**: R1 CPU / R2-F DMA physical fences | no “quiesced” report or frame reclamation without scoped evidence |
| **UN-O16** | EXTERNAL-EVIDENCE + WORDING-DRIFT | **ADR-CAPIPC-G**: guest errno/FD and SABI backend effect/durability | native COMMITTED/UNKNOWN never silently becomes external exactly-once |
| **UN-O17** | POLICY-CHOICE + LIVENESS-GAP | **ADR-CAPIPC-D/C**: KR-M1/2/lanes, HPA B+/C/register-only comparator | equal-assumption complexity/fairness/capacity comparison, no fake benchmark |
| **UN-O18** | PROOF-GAP | **ADR-CAPIPC-H**: formal/implementation refinement gate | artifact/spec provenance and negative controls; no claiming model=kernel proof |

### 3.1 Cross-RFC point of highest semantic risk

- **B before K-S** forbids a newly committed SEND by the scoped lineage; if K-S won first, its historical whole vector has moved even if the receiver will not get usable rights later.
- **B between K-S and K-I** is *not* the same race. First normal T1 candidate denies a full native K-I if any mandatory queued grant would be newly installed usable after its relevant B, and terminal/close/quarantines according to honest V/custody accounting. It does not silently install N−1, reverse K-S, or claim no disclosure.
- **K-I before B** installs exactly N actual grants at its cut, after which B may restrict future new native uses; earlier admitted CPU/DMA work remains an R1/R2-F concern.
- These three cases require one consistent **Grant lineage epoch and Claim/message custody model**, not separate “SEND succeeded” and “RECEIVE succeeded” booleans.

## 4. ADR decision packages A–H (drafts, not actual Accepted ADRs)

### ADR-CAPIPC-A — Sender transfer and E1 owner handoff

**Question:** How does one K-S atomically MOVE a vector while relevant B, competing Close/MOVE, Endpoint E-M and Task death can race?

| Option | Merits | Critical cost / rejection criterion |
|---|---|---|
| **A1 CO-S + CO-E-C** (research preference) | simpler post-B denial and unique old/new E1 owner cut | cancel all pending precommit work before handoff; bounded non-faulting commit/lock dependency must be proved |
| A2 permit scoped admitted-reference post-B/owner transfer | in-flight availability | requires explicit pre-barrier authority, no concurrent public owner, FIFO and bounded refs; **not implicit from A(q)** |
| A3 giant global lock across copyin/usercopy/commit | superficially simple serial ordering | **reject** as default: faultable Pager access and cross-service waits create deadlock/M0 risk |
| A4 restore source grant after already committed SEND on peer error | retries seem easy | **reject**: double custody and external effect ambiguity |

**Proposed D1 scope:** one message, 2–3 grant IDs, one grant revoked, one competing source Close and E1 MOVE. Define cut order and source-Handle “preserved by THIS send” wording. **No final lock/RCU design chosen.** For a moved Endpoint as payload, distinguish owner removal at K-S from new Task ownership at K-I, with a legal ownerless InTransit interval.

### ADR-CAPIPC-B — ReceiveClaim issuer audience and metadata barrier

**Question:** Which exact Task/Endpoint audience can see which counts/types/bytes?

| Option | Merits | Critical cost / condition |
|---|---|---|
| **B1 audience-bound K-C + explicit Vm projection** (preference) | conservative secrecy and no stale Claim migration | metadata classification and current policy/owner revalidation required |
| B2 all sizes/counts universally public | easy size probing | **reject for secret schemas**; issuer-approved public classes may permit it |
| B3 exposure tracked only at K-I | simple API outcome | **reject**: Vm/Vp may occur earlier and cannot be rolled back |
| B4 move Claim to new Task after V | apparent recovery | **reject** for possibly confidential disclosure to old B |
| B5 no metadata publication at all | strongest simplest secrecy | may require supported fixed-size/reg-only initial ABI; evaluate workload |

**Proposed D1 scope:** Task B vs B2 with same registry name, one Endpoint Egen move, confidential/public metadata projections; distinguish safe “same authorized audience” continuation from distinct Task delivery. Kernel needn't know arbitrary SABI schemas, but the projection/policy attestation owner must be explicit.

### ADR-CAPIPC-C — K-I, revocation-in-transit and public token publication

**Question:** How to install all N grants once, atomically retire m, publish public token IDs and tell the truth after faults?

| Option | Merits | Critical cost / condition |
|---|---|---|
| **C1 HPA-B+ inert-token-first** (first proof target only) | no live transferred grants before one whole-vector K-I | user output partial visibility, token nonrebind, exact K-Vt/Vp-to-K-I order and status loss unsolved |
| C2 K-I first, output tokens later | smaller logical commit | active unreported orphan grants, other Thread invoking, close cannot undo already-used effects |
| C3 HPA-C Stage→Task AcceptCommit | allows intentional Task-level delay | self-accept is NOT independent policy; greater claim/timeout/receipt burden |
| C4 register-only/narrow message profile | avoids some pageable bulk-usercopy ambiguity | cannot automatically meet arbitrary N-grant/full-size IPC; would shrink product ABI scope |
| C5 auto-redeliver after Vm/Vp | availability | **reject default**: cross-audience possible disclosure cannot be undone |
| C6 install N−1 surviving grants after a mandatory member was revoked | apparent partial progress | **reject** for the current full-vector contract; alternative typed placeholder is a new explicit contract requiring separate proof |

**Proposed first coherent outcome:** if one mandatory Grant becomes newly unauthorized before K-I, **no full K-I**; terminal/same-authorized-audience path with truthful possible disclosure, never sender rollback. A historical COMMITTED_KI does not mean the initiating Thread saw its token array, or any public token remains usable after independent close/revoke.

**Hard D1 blocker:** exact K-Vt and K-Vp publication order, slot allocation/faultable copyout, K-I protected phase/receipt and coherent SMP token invokability without holding a Pager-required global lock. Do not label C1 “resolved” before selecting and defending that ordering.

### ADR-CAPIPC-D — Receipt identity and public-token nonrebind

**Question:** Prevent duplicate K-S/K-I when completion is lost, without storing unlimited historical receipts.

| Option | Benefit | Proof/availability burden |
|---|---|---|
| **D1 KR-M2 bounded C/Used/R** (multi-Thread research leader) | admits limited out-of-order q while retaining Task-local replay fence after R GC | missing q eventually saturates W, explicit SKIP can discard never-admitted intent, SMP bitmap/tag and Q budgets |
| D2 KR-M1 strict HWM | smaller kernel admission TCB | missing next q stalls every later admit |
| D3 unordered nonce/slot history | no ordering requirement | exact lifetime nonreuse, slot/generation wrap, GC proof |
| D4 per-lane receipt window | availability isolation | lane authority/lifetime/ABA and cross-lane ownership proof |
| D5 recreate grant/replay SEND on expired receipt | surface convenience | **reject**: makes historical result loss into duplicate native effect |
| D6 public token numerical recycle with only hidden internal generation | cheap | **reject** under current Task-public no-stale-rebind requirement |

**D1 clarification:** q is correlation, not bearer authority; sender qS and receiver qR are distinct. Final R evidence may expire while q remains replay-fenced for live TaskGeneration, returning EXPIRED/UNKNOWN rather than falsely NOT_COMMITTED. Finite namespace exhaustion must fail closed, not silently wrap. No public Handle bit width or W/Q number chosen.

### ADR-CAPIPC-E — Bounded custody/cleanup, Pager and independent M0

**Question:** Can the security-preserving receipt/Claim/terminal path operate under faults, cycles, starvation and DMA that won't fence?

| Option | Benefit | Safety/feasibility consequence |
|---|---|---|
| **E1 precharge mandatory cuts + dedicated M0 reserve + bounded iterative cleanup** (research preference) | explicit quota-fail-before-cut and isolated recovery | exact worst-case charge accounting, cooperative bounded critical sections, wait-for acyclicity and quarantine budget must be proved |
| E2 rely on normal Task/Page cache/IPC pools for Pager/recovery | less reserved memory | **reject** for M0 recovery guarantee: Pager could require the already exhausted pool or same blocked IPC |
| E3 recursive destructor for queue/Endpoint cycles | short source | **reject** as unrestricted kernel cleanup: cycles/stack unbounded |
| E4 free DMA pins after native receipt ACK/Task death | immediate memory reuse | **reject**: lacks physical R2-F fence |
| E5 permanently quarantine everything unsafe | fail-closed integrity | bounded DeviceGroup/physical resource exhaustion; honest fail-stop required |

**New explicit M0 accounting requirement:** for any native operation, charge receipt R, pending queue bytes/grants, claim, token reservations including burned IDs, lineage anchor, fault waiter, sponsor and DMA separately. A closed Task/expired q cannot erase those debts. M0 has its own wired CPU/stack/IPC/IRQ/frame/cleanup reserve that ordinary admission **cannot borrow**. If even M0 reserve is exhausted, specify an honest fail-stop/degraded outcome rather than false recovery success.

**Pager wait-for path:** copyin/copyout can fault. No operation may hold a global queue/cap/VM/revoke lock across usercopy whose fault resolution needs a service blocked behind that same lock or exhausted credit. This is a **future enforceable graph property**, not an assertion that current code has a safe lock ordering.

### ADR-CAPIPC-F — Optional T2 verifier

**Decision:** baseline T1 bearer grants may be usable by *any authorized receiver Task Thread* after K-I even before its untrusted method decoder accepts the message. T2 is an **optional stronger contract**, not silently required for every IPC.

A true T2 promise must identify an **independently authorized issuer/verifier** and bind approval to immutable message bytes / actual kernel GrantID, type, rights, lineage, receiver TaskGeneration, Endpoint/ServiceIncarnation, method/resource role and current PolicyEpoch, with live B at separate K-U. B's own “validated=true” or HPA-C self-accept is **not** independent authorization. If no trusted role verifier is available, do **not** market pre-method-gated native rights; remain T1 or refuse feature.

### ADR-CAPIPC-G — Application effect and guest translation boundary

**Decision:** native K-S means only source→queue, native K-I only whole CapGrant vector installation+dequeue. A service may apply a write after Vp before K-I via rights it already has. ReplyToken, SABI RequestId, filesystem durable receipt and hardware fence have distinct issuers. Linux FD/OFD reusable numeric identity and guest EINTR/restart rules require per-operation translation; native EXPIRED/UNKNOWN cannot become clean “no effect, safe retry”.

**Scope:** this is an *honest marketing/error semantics contract*, not an attempt to make universal application exactly-once a kernel property. File-system durability and physical DMA need their own RFC-008/007 validation.

### ADR-CAPIPC-H — Future proof evidence and negative controls

Require two independent analyses before any Accepted CAPIPC ADR: (a) claim/guard-to-RFC traceability; (b) a separately authorized finite-state concurrency model with non-vacuous success paths, intentionally injected broken rules and reproducible counterexamples. A finite no-counterexample outcome is not proof for arbitrary workloads, and formal proof of an abstract model is not implementation refinement or physical hardware evidence.

**Candidate falsifiers:** stale post-B K-S; two queue custodians for same Grant; owner overlap on Endpoint handoff; partial N−1 K-I reported full; TokenID becomes different Grant after first Vt; cross-recipient redelivery after Vp; caller result lost then second K-I; GC deletes pending receipt; Pager global-lock cycle; M0 reserve overdraw; post-Task death stale receipt authority.

## 5. Proposed ADR register — READINESS ONLY, NONE APPROVED

| Draft ID | Topic / dependency | Proposed preference | State today | Evidence for D1 review |
|---|---|---|---|---|
| **ADR-CAPIPC-A** | K-S/B/E-M, Endpoint InTransit | CO-S + CO-E-C | **OPEN D0** | strict cut table incl multi-g and cancellation |
| **ADR-CAPIPC-B** | Claim issuer/metadata/audience | authenticated K-C + Vm projection | **OPEN D0** | audience classification, E-M race and terminal policy |
| **ADR-CAPIPC-C** | K-I/Vt/Vp/token | HPA-B+ T1 *family* (micro-order not chosen) | **OPEN D0** | atomic N-vector/receipt versus copyout fault proof-plan |
| **ADR-CAPIPC-D** | receipt identities/token no-rebind | KR-M2 leader, KR-M1 baseline | **OPEN D0** | finite q/W/Q, SKIP/lost ID and GC truth |
| **ADR-CAPIPC-E** | Pager/M0/resource/revoke cleanup | bounded credits and actual fences | **OPEN D0** | resource ceilings, wait-for graph, Task death/quarantine |
| **ADR-CAPIPC-F** | independently verified T2 K-U | optional T2 only when required | **CONDITIONAL D0** | separate policy issuer and immutable provenance |
| **ADR-CAPIPC-G** | remote effect/guest/durable meanings | conservative native-only truth | **OPEN D0** | explicit cross-layer error/effect table |
| **ADR-CAPIPC-H** | formal/refinement acceptance gate | stage checks, negative controls | **OPEN D0** | tool/assumption/trace/proof classification contract |

**ADR numbering is local placeholder nomenclature, not a registered global ADR sequence.** Do not create eight Accepted ADR files, freeze ABI or alter existing eight architecture directions on the strength of this matrix.

### 5.1 Conditional decisions we can meaningfully recommend today without pretending proof

1. **Cut separation:** record A(q), K-S, B/E-M, K-C, Vm/Vp/Vt, K-I, K-U and backend effect as **distinct semantic events**. Definitional agreement is not proof of realizable concurrent atomicity.
2. **Failure truth:** COMMITTED_KS, NO_KI, POSSIBLE_DISCLOSURE and UNKNOWN remain independent. A failed SEND does not undo a *different Thread's* successful Close/Revoke.
3. **Scope:** only default T1 first; separately justify T2 with trusted issuer. Avoid forcing extra Trusted Computing Base onto the general desktop's hot path without evidence.
4. **Conservatism:** when source→queue MOVE committed, never restore source grants; when data maybe exposed, never automatically requeue to a different receiver; when physical fence unproven, quarantine rather than reclaim.
5. **Governance:** before choosing numeric ABI, require D1 semantic-cut selection, then separately approved D2 proof, then D3 implementation decision.

None of these statements declares an already-enforced UniOS kernel property.

## 6. Cross-RFC edge cases requiring a single owner

| Case ID | Conflict edge | Single responsible contract and failure statement |
|---|---|---|
| **CR-01** | B races K-S after receipt A | RFC-002/003: strict applicable B vs whole-vector K-S cut; A alone not permanent authority |
| **CR-02** | Endpoint own HANDLE included in message elsewhere | RFC-002/003: owner removal at original K-S, ownerless queue custody, new owner only at K-I |
| **CR-03** | B wins between K-S/K-I on one mandatory Grant | RFC-002/003: no silent N−1 K-I; terminal/claim disposition with full charge |
| **CR-04** | Confidential Vm reveals size then buffer error | RFC-003/018: issuer projection, no unauthorized metadata oracle |
| **CR-05** | Vp before K-I then Task/Endpoint change | RFC-003/004: possible disclosure, no old claim migration or foreign requeue |
| **CR-06** | Vt public ID before K-I, TokenID eventually recycled | RFC-002/009: inert pre-K-I and external token nonrebind |
| **CR-07** | K-I succeeds but caller result copyout lost | RFC-003/009: protected committed receipt; no new CapGrant on Query |
| **CR-08** | KR-M2 C passes pending q | RFC-003/009: replay fence is not final completion/GC permission |
| **CR-09** | Pager fault under queue/Claim lock | RFC-004/005/006: no wait-for cycle; M0 independently admitted |
| **CR-10** | Task dies with queued Endpoint cycle or DMA | RFC-002/006/007: bounded iterative cleanup / hardware fence or quarantine |
| **CR-11** | Server performs file write after Vp but K-I terminal | RFC-008/018: backend effect is separate and may be irreversible |
| **CR-12** | Linux guest retry after native UNKNOWN | RFC-009/010: per-op semantic translation and external request ledger |

**No architecture owner is allowed to “solve” a cross-RFC edge by silently weakening the other owner's promise.** If source-contract meaning changes, create a Proposed addendum and explicit other-owner acknowledgement; accepted ADR comes only after independent evidence.

## 7. Paper-only adversarial acceptance schedules (P0-X01…P0-X24) — NOT EXECUTED

| ID | Proposed race/schedule | Counterexample target |
|---|---|---|
| P0-X01 | A(qS) then B then K-S | unauthorized post-B send |
| P0-X02 | K-S then B then lost return | false sender restoration/NOT_COMMITTED |
| P0-X03 | K-S entire [g1,g2] vs separate concurrent Close(g2) | half-vector custody |
| P0-X04 | competing K-S on same g but different q | duplicate grant mover |
| P0-X05 | E-M wins before old owner K-S | two public owner paths |
| P0-X06 | E1 as payload, old owner→InTransit→new owner K-I | overlapping effective owner / vanished queue |
| P0-X07 | B revokes queued g2 between K-S and K-I | silent N−1 success or false sender rollback |
| P0-X08 | K-C before receipt identity known but initial return lost | impossible reconciliation after possible V |
| P0-X09 | Vm secret size leaks on BUFFER_TOO_SMALL | metadata oracle |
| P0-X10 | K-C owner E-M before Vp | recipient audience confusion |
| P0-X11 | Vp copied prefix, second output page fault, another Task claims m | foreign secret redistribution |
| P0-X12 | Vt first token copied, fault on second, same numeric ID later used | external Handle ABA |
| P0-X13 | Vt copied to B Thread2, K-I commits, Thread1 unaware | wrong thread-exclusive guarantee |
| P0-X14 | K-I wins before terminal close in another CPU | two owners / broken receipt phase |
| P0-X15 | K-I committed then token closed before receipt query | cap resurrected by historical query |
| P0-X16 | final GC while R[q] Pending | missing postcommit evidence / replay |
| P0-X17 | permanent q hole, finite W saturates | unsound unconditional liveness |
| P0-X18 | SKIP loses to A but response lost | incorrect claim old A was canceled |
| P0-X19 | Q/W receipt admission and active Claim fill ordinary credits | M0 starvation |
| P0-X20 | Pager needs IPC lock held by faultable usercopy | circular wait/deadlock |
| P0-X21 | Endpoint message cycle on Task death triggers recursive destroy | unbounded kernel stack/cleanup |
| P0-X22 | GC after K-I while DeviceCap DMA not fenced | premature frame reuse |
| P0-X23 | T2 self-verifier approves mutable bytes before K-U | false independent policy gate |
| P0-X24 | backend effect after Vp but NO_KI, guest retry new q | duplicate user effect despite native no-install |

## 8. Required proof obligations (P0-I01…P0-I20) — NONE PROVED

| ID | Candidate falsifiable invariant |
|---|---|
| P0-I01 | A(q) is unique identity admission, never blanket later grant permission. |
| P0-I02 | For relevant B vs K-S, one whole-vector outcome orders both cuts without post-B new auth. |
| P0-I03 | E1 public owner uniqueness and ownerless InTransit preserve K-S FIFO cut order. |
| P0-I04 | Same GrantID not simultaneously effective at sender/queue/receiver due to MOVE. |
| P0-I05 | A failed K-S does not consume source *by that SEND* or undo independent Close/Revoke. |
| P0-I06 | Receiver Claim is exclusive, with authenticated Task/Egen/issuer audience at visibility cuts. |
| P0-I07 | Secret Vm/Vp/Vt exposure flags precede publication and persist through fault/terminal. |
| P0-I08 | After relevant V, same m never automatically requeued to foreign recipient/incarnation. |
| P0-I09 | K-I is one all-N-Grant install+queue retirement+receipt phase, or none. |
| P0-I10 | Revoked mandatory queued grant cannot produce silent all-cap committed N−1. |
| P0-I11 | Maybe-published token stays inert until K-I and never externally rebinds unrelated authority. |
| P0-I12 | Historic protected receipt never remints CapGrant, rewinds K-S or implies current usability. |
| P0-I13 | Both Task receipt sequences qS/qR remain nonreplayable inside true TaskGeneration after final GC. |
| P0-I14 | Post-V NO_KI/terminal status cannot imply no prior disclosure/effect. |
| P0-I15 | Usercopy/Pager cannot participate in an incompatible global IPC/VM wait-for cycle. |
| P0-I16 | Ordinary receipt/Claim/cleanup cannot spend M0 reserved CPU/IPC/frame/IRQ/stack credit. |
| P0-I17 | Close/revoke/Task death doesn't prematurely refund queued grant/lineage/physical DMA obligations. |
| P0-I18 | Task/ServiceIncarnation restart never implicitly reattaches stale Endpoint/claim/result authority. |
| P0-I19 | Optional T2 method gate requires independent verifier with immutable Grant/protocol evidence and live policy/revoke check. |
| P0-I20 | Native COMMITTED/UNKNOWN does not certify or deny backend effect, hardware fence or storage durability. |

**Known non-mechanical dependencies:** P0-I15/I16 need realistic budget/wait-for and scheduler abstraction; P0-I17 physical R2-F requires hardware evidence beyond an abstract checker; P0-I20 is an *interface assertion* whose real backend effects must be separately modeled/tested. No checker has evaluated these properties.

## 9. Proof-budget plan — validation scope, NOT effort estimate

| Evidence batch | Input contract to freeze for paper review | Future checking target | Must not conflate |
|---|---|---|---|
| **PB-1 atomic SEND** | A, K-S, B, E-M, Close, TaskDeath / exact multi-Grant vector | no split MOVE, B/E-M coherent result + FIFO | admitted q vs commit rights |
| **PB-2 identity/receipt** | TaskGeneration, qS/qR, C/Used, R, SKIP, query/GC | no readmission, truthful Pending/Expired/committed, result loss | receipt GC vs cap/queue lifetime |
| **PB-3 receive-publication** | Claim/audience Vm/Vp/Vt, Token state, exact K-I order, no-K-I terminal | no foreign redelivery, whole install, token ABA, old/current rights separation | bytes seen vs grant installed |
| **PB-4 memory/recovery** | finite Q/W/queue/claim/token/lineage charge, Pager wait graph, M0 reserves, Task death | bounded cleanup or honest fail-stop, no forbidden waits | “finite model passed” vs real time/IRQ hardware proof |
| **PB-5 external effects** | SABI effect ledger, guest FD/OFD/errno and physical/media proof boundaries | reject universal native exactly-once; specify per-provider evidence | K-S/K-I receipt vs service effect/durability |
| **PB-6 optional T2** | issuer/verifier actual trust principal, immutable method/native grant map, B/PolicyEpoch | no postrevocation K-U and no self-approval | untrusted Task AcceptCommit vs independent policy |

**Nonvacuity requirements for a later separately authorized model:** successful SEND/RECEIVE, revoked-before-K-S, revoked-after-K-S-before-K-I, Vp fault, Vt fault, Task death, lost returns, q gap, final GC and M0 saturation must each be reachable; every intentionally unsafe rule mutation must produce a checker counterexample. A checker run reporting “no violations” with unreachable K-I is not acceptable.

**No numbers selected:** W, Q, cap-vector length, public token bit width, per-Task byte quota, worst-case cleanup bound and M0 reserve are not benchmarked or committed. Numerical examples in earlier studies remain illustrative only.

## 10. Decision dependency ordering and stopping conditions

**Phase R1 — paper semantic-cut resolution first:** narrow ADR-A + ADR-C + ADR-E joint review. Precisely name all K-S/B/E-M and K-C/V/K-I cut guards, keep resource/lock ceilings in same review. Reject any language promising byte-atomic usercopy or universal progress.

**Phase R2 — receipt and audience contract:** ADR-B + ADR-D; specify preknown qS/qR, SKIP/GC/EXPIRED, audience/projection, public token exhaustion. Any unresolved R2 effects must be visible to the earlier ADR-A/C guard decisions; do not isolate the papers artificially.

**Phase R3 — recovery and optional/outer policy:** ADR-E/C cleanup and ADR-F/G; preserve true Task/ServiceIncarnation generation, independent DMA/FS/effect semantics and honest fail-stop.

**Phase R4 — future separately authorized verification decision:** ADR-H supplies formal-tool version/assumption, finite-state and negative-control evidence plan. Writing/executing an actual formal model and kernel code are outside current user authorization.

### Mandatory STOP/NO-GO triggers

- K-S can split one Grant vector; a relevant B can “lose” on one CPU while later K-S commits as if before B.
- A source or Endpoint may have two effective owners; a returned no-cut response lies about an already committed operation.
- Vp may occur with no durable disclosure fence and m may later be auto-delivered to another recipient.
- K-I can commit a revoked mandatory grant as active rights or silently install only N−1 while reporting N.
- Exposed public ID can later authorize an unrelated native Grant in same live Task.
- Pager requires a global lock held by a faulting IPC, or ordinary malicious Claim/Q can consume M0 recovery reserves.
- Finite receipt q/ID wraps and reopens old identity, or GC clears the only dedup fence while Pending effects may commit.
- Task restart/receipt expiry incorrectly becomes app “no effect”, or hardware quiescence is falsely inferred from native IPC success.

These are **future refusal gates**; the study has not witnessed any such bug in code because no code was built.

## 11. Proposed decision register P0-D01…P0-D12 — ALL PROPOSED

- **P0-D01:** Record TR-01/02/03 as the coupled critical safety chain before preparing an Accepted CAPIPC ADR.
- **P0-D02:** Keep CO-S as first send B-vs-K-S research profile, CO-E-C as bounded endpoint owner-handoff comparator.
- **P0-D03:** Retain HPA-B+ *family* as first ordinary T1 receive research target but do not select Vt/Vp/K-I micro-order yet.
- **P0-D04:** First revoked-in-transit N-vector candidate is all-or-none K-I denial/terminal, never silent N−1 install.
- **P0-D05:** Keep Task qS/qR receipt identity separate from message and ServiceId; KR-M2 leader/KR-M1 baseline unresolved.
- **P0-D06:** Treat audience Vm projection and possible Vp/Vt disclosure as independent irreversible information concerns.
- **P0-D07:** Require Task-public token nonrebind after possible exposure and fail-closed exhaustion, subject to future feasibility.
- **P0-D08:** Separate M0/Pager/queue/claim/lineage/DMA charge/cleanup budget from record TTL and ordinary Task borrowing.
- **P0-D09:** Scope T2 verifier as optional independent trusted method-before-native-use profile; default T1 has only Task-wide bearer trust.
- **P0-D10:** Require kernel-level historical K-S/K-I truth, no blanket app effect, durability or guest retry claims.
- **P0-D11:** Do not promote mere documentary consistency to D2/D3 Accepted, implementation-ready or production guarantee.
- **P0-D12:** Only design/research docs may change now; no executable model, source implementation or ABI freeze.

## 12. Remaining explicit unknowns

**Major unresolved feasibility issues, not merely to-do-list wording:**

1. A single coherent atomic K-S+multi-Grant+revocation/Endpoint owner cut without locking across Pager and without broad TCB expansion.
2. Whole-vector K-I+result coherence with pageable, aliased outputs and concurrently observable Vt/Vp, including failure between exposing a token and installing usable caps.
3. Task-wide public Handle exhaustion and KR-M2 q hole liveness under deliberate partial copyout sabotage and many Threads.
4. Bounded iterative queue/Endpoint cycles, sponsor remapping and physical DMA quarantine when Tasks or Services die.
5. M0's actual admission-reserve upper bound and wait-for graph; abstract liveness may fail on real hardware/driver outage.
6. A genuine independently trusted T2 verifier bridge **if** the product later requires method-gated rights.
7. Effect/idempotency and guest restart translations without confusing native K-S/K-I with remote file/network durability.

**No P0 is declared resolved here.** The matrix prioritizes and decomposes what must be demonstrated. If a first-profile kernel IPC cannot satisfy these gates at a reasonable trusted-computing-base cost, future ADR reviewers must evaluate **narrower IPC payload/registration bounds or a different receive ABI**, not waive no-double-grant, information-exposure or memory-safety requirements.

## 13. Sources and reproducibility limits

- [S1] Fuchsia Channel kernel-object documentation (whole message+Handle ownership and Endpoint handoff ordering): https://fuchsia.dev/fuchsia-src/reference/kernel_objects/channel
- [S2] Fuchsia Channel read and read_etc (whole-message API and output metadata): https://fuchsia.dev/reference/syscalls/channel_read and https://fuchsia.dev/reference/syscalls/channel_read_etc
- [S3] Fuchsia channel_write_etc (MOVE/DUPLICATE distinctions): https://fuchsia.dev/reference/syscalls/channel_write_etc
- [S4] Linux kernel usercopy context (may sleep/fault and partial copy reporting): https://docs.kernel.org/kernel-hacking/hacking.html
- [S5] seL4 official IPC tutorial (receiver Capability slot/IPC contract as different synchronous architecture): https://docs.sel4.systems/Tutorials/ipc
- [S6] UniOS cross-RFC linked source papers and v0.2 architecture (written Proposed design only).

**Method:** independent consistency and adversarial option review of UniOS main branch, plus first-party external references. No new evidence that UniOS currently implements this protocol; no model checker has run. The heuristic scores are for **relative design triage only**, not measured engineering duration, security risk probability or ABI selection.

---

**History:** 2026-10-25 initial Proposed v0.1. **18** Unified blocker mappings; **8** draft ADR packages; **9** heuristic triage workstreams; **12** cross-RFC owner edge cases; **24** unexecuted adversarial schedules; **20** unproved invariants; **12** Proposed decisions. No kernel source, runnable spec, implementation, tests or Accepted ADR.

## 14. 2026-10-26 A/C/E joint semantic-cut research follow-up — Proposed, D0 remains

[CAPIPC Joint A/C/E Semantic-Cut Resolution v0.1](../models/capipc-joint-semantic-cut-resolution-a-c-e-v0.1.md) narrows, **on paper only**, CO-S/CO-E-C K-S/B/E-M ordering, HPA-B+ T1 candidate **Vp→Vt→K-I** (inert TokenIDs until full K-I) and precharged independent Pager/M0 sponsor/terminal budgets. Its central **NEW explicit blocker** is the audience/Endpoint/PolicyEpoch change while usercopy is faulted or in flight: a later K-I rights check cannot undo confidential bytes written after a visibility barrier. Bounded publication fences versus register-only/restricted outputs must be compared in a follow-on study. These remain D0 research options, not D1-resolved guard semantics, verified models, approved ADRs or source code.

# UniOS CAPIPC — Paired Mapping–Publication Cut & Invalidation Ordering v0.1

> **Status:** Proposed / D0 / OPEN, design-only feasibility and contract research — 2026-10-26 NZDT.
> **No implementation authority:** This is neither an executable model nor kernel synchronization source, a model-checker result, a selected Native ABI, an Accepted ADR, hardware evidence or permission to code. All actions and proposed invariants below are conceptual.
> **Focus:** one receiver publication slice competing with independent (1) VM destination binding invalidation and (2) issuer/Endpoint disclosure-authority change, plus the smaller register-only ReturnRelease option and M0/Pager conditions.
> **Parents:** [Cross-RFC Destination Decision Table](../reviews/capipc-destination-memory-publication-cross-rfc-decision-table-v0.1.md), [Publication Permit & Destination Memory](capipc-publication-permit-destination-memory-contract-v0.1.md), [Publication Barrier/Pager-Safe Copyout](capipc-publication-barrier-pager-safe-copyout-feasibility-v0.1.md), [Joint A/C/E Cuts](capipc-joint-semantic-cut-resolution-a-c-e-v0.1.md), [Unified SEND–RECEIVE](../contracts/capipc-unified-send-receive-semantic-contract-v0.1.md), [P0 ADR Readiness](../reviews/capipc-p0-conflict-resolution-adr-readiness-v0.1.md).
> **Contract owners:** [RFC-002 Capability](../rfcs/rfc-002-object-capability-lifetime-delegation-revocation.md), [RFC-003 IPC](../rfcs/rfc-003-ipc-wire-protocol-and-synchronization.md), [RFC-004 VM](../rfcs/rfc-004-virtual-memory-pager-cache-and-oom.md), [RFC-005 Scheduler](../rfcs/rfc-005-scheduling-preemption-and-cpu-budget.md), [RFC-006 M0](../rfcs/rfc-006-boot-trust-chain-service-startup-and-recovery.md), [RFC-009 KABI](../rfcs/rfc-009-native-abi-and-system-call-contract.md), [RFC-018 SABI](../rfcs/rfc-018-versioned-idl-abi-evolution-and-compatibility-contracts.md). [RFC-007 DMA](../rfcs/rfc-007-user-space-drivers-dma-and-iommu.md) separately covers physical DeviceGroup fences.

## 0. Executive conclusion and feasibility limit

The correct *abstract permission predicate* for confidential publication into a Task-owned VA is **both**:
- **M authority**: a current VM DestinationBinding of an exact TaskGeneration, AddressSpaceGeneration, MappingGeneration, target VA interval and prepared writable backing, including its explicitly permitted readable aliases;
- **P authority**: current issuer-authorized audience/PolicyEpoch, owner EndpointGeneration and exclusive message Claim, with a reserved receiver receipt and credit.

**One joint publication-start cut** must bind a *single definite pair* (M,P) for a bounded slice, order that start against either kind of DENY, and charge an active publisher until it truly cannot store again. Two independent observations followed by an irreversible user write **do not suffice**. Neither do physical page pins, a postcopy sequence retry, a plain cancel flag, later K-I validation, or Task death.

**Research-leading candidate U-J:** a **single logical paired PermitStart decision** using separately owned M and P protection records, with a bounded live publisher reference counted against *both* scopes, no faultable work in the actual protected publish slice, and two independently honest drains. This is **an abstract contract, not a proposal to take one global kernel lock**. Concrete fine-grained synchronization has not been shown feasible.

**Alternative U-T:** paired two-resource ticket acquisition with rollback is useful as a comparison; it must forbid a cycle in which M invalidator waits on an M ticket held by a Thread waiting to acquire a P ticket while P issuer waits on a P ticket held by another Thread waiting for M. Tickets must be nonblocking and bounded or a canonical admission ordering must be proved.

**Fallback U-R:** the smaller register-only control IPC profile can bypass dynamic user VA output entirely, yet still needs its own **ReturnRelease** authorization cut versus PolicyEpoch, Endpoint owner and Task death, with independently truthful K-I.

**Status:** **D0 / OPEN; no proof that U-J/U-T can be implemented with bounded SMP/VM costs.** The first proof path should restrict the destination to *small resident private bindings* or use U-R rather than broad pageable/unbounded usercopy. Do not call D1 complete merely for naming one logical cut.

## 1. Independent objects and exact scope

| Object/record | Authentic identity | Independent source of truth |
|---|---|---|
| Message m | immutable actual kernel message from committed K-S | RFC-003 queue; never sender qS alone |
| Receiver Claim c | m, TaskGeneration, EndpointGeneration, issuer Audience and ClaimEpoch | RFC-003; exclusive but not K-I cap ownership |
| Receiver receipt R[qR] | TaskGeneration + admitted qR with protected Pending/final and sponsor credit | RFC-009/003; qR distinct from qS |
| Destination binding d | TaskGeneration, AddressSpaceGeneration, MappingGeneration, VA range, exact backing identity/version and share policy | RFC-004 VM; pin/reference lifetime not current VA by itself |
| Publication scope p | Claim, Endpoint owner generation, issuer authority, Audience and PolicyEpoch | RFC-002/003/018; distinct from native CapGrant B |
| Publisher ref x | tuple(m,c,d,p,slice), active bounded store permission, independently charged credits | paired IPC–VM admission |
| Deny state M-DENY | forbids *new* publisher refs on old d | RFC-004 |
| Deny state P-DENY | forbids *new* publisher refs on old p | issuer/IPC |
| Drain M-DRAIN | no older x with d can store again; only then safe retire/reassign that binding per separate VM/TLB rules | RFC-004 |
| Drain P-DRAIN | no older x with p can store again; not erasure of old plaintext | issuer/IPC |
| Vp/Vt | monotone *possible* bytes/public TokenIDs exposed before first store | RFC-003 |
| K-I | whole CapGrant vector install + message retirement + R[qR] committed | RFC-002/003, separate from M/P publication |
| R1/R2-F/R2-O | CPU mapping, physical device and object-wide lifetime evidence | RFC-002/004/007, none equivalent to M/P drain |

**Scope identity rule:** "same Task name," "same ServiceID," raw VA, old pin pointer, old Endpoint number or numerically equal qR/qS cannot replace authenticated generation/object identities. At most one current E1 effective owner; no old Claim automatically migrates across owner MOVE.

## 2. What exactly must be linearized

### 2.1 Four outcomes for PublishStart versus two deny cuts

| Ordering | May an old-scope slice start? | What a denial completion means |
|---|---|---|
| **M-DENY wins before paired Start** | No; even if P was still allowed | M may report DENY_ESTABLISHED, not necessarily M-DRAIN |
| **P-DENY wins before paired Start** | No; even if M was still valid | P may report DENY_ESTABLISHED, not necessarily P-DRAIN |
| **Paired Start wins before either DENY** | Yes, exactly one bounded preauthorized slice under both scopes | later DENY forbids new starts; old slice may finish **only under explicitly allowed pre-DENY in-flight semantics** |
| **M-DENY and P-DENY race with Start** | One consistent winner relation for each scope must exist; no half-admitted permission authorizing stores | blocked pair must cancel without storing or truthfully retain pending reference/debt |

**NO implicit atomic user memory:** one PublishStart may precede several possibly observable CPU stores; Vp/Vt means any byte may be seen, not that all were copied. CPU cache propagation or R's later forwarding isn't erased by P-DRAIN.

### 2.2 Two-stage issuer and VM invalidation — independent cut pairs

- **M-DENY** closes admission into the old MappingGeneration's write binding. **M-DRAIN** requires no old publisher can issue a further store before VM retires old output semantics and backing refs; a further CPU/TLB R1 condition may separately apply before frame reuse.
- **P-DENY** closes issuer/Endpoint audience publication admission. **P-DRAIN** requires no old publisher can issue another store before a *strong publication-quiescence* acknowledgement.
- **Weak owner/issuer handoff** can report only DENY_ESTABLISHED if it expressly permits bounded older-authorized slices to remain; **strong handoff** cannot report old-publication stopped until P-DRAIN (and any required M-DRAIN) is established.
- **Task death** or receipt GC is neither drain. K-I success is neither drain. Native capability revoke barrier B may be correlated with issuer cutoff under an explicit policy, but cannot be silently conflated with P-DENY.

**Different old scopes may have distinct credits and drain completion**; no single global "all IPC publication stopped" boolean should be introduced to make paper invariants trivial.

### 2.3 Destination identity cannot be chosen after the store

- **V mode:** output promise is for actual receiver Task/AddressSpace/MappingGeneration at a *defined protected completion cut* and exact authorized backing. If unmap/remap races, a stale pinned-frame store cannot be advertised as "delivered to current VA".
- **O mode:** explicit typed MemoryObject/version/range output, independent of whether any user VA currently maps it; requires separate map/view release, private/shared alias policy and TLB proof.
- **R mode:** bounded Thread/register ReturnRelease cut, independent of user buffer. Saved register values staged earlier aren't a blanket right to release after later P-DENY.

**Privacy statement:** a PrivateAtRelease issuer policy must account for unauthorized readable **and writable** cross-Task aliases during the named window. R may deliberately forward legitimately observed plaintext afterward; UniOS cannot promise enduring secrecy against its authorized recipient.

## 3. Candidate U-J joint admission, stated as a logical protocol — NOT code

**Design obligation groups rather than an implementation recipe:**

1. **Outside the publisher cut:** authenticate actual m/c/qR; stage immutable exact bytes+GrantIDs; select V/O/R profile; resolve needed frames/COW, private/shared audience class and exact M backing; reserve complete Claim/receipt/token/sponsor/terminal/publisher resources. Faultable Pager work and allocation occur **outside locks required by Pager/M0**.
2. **Enter joint logical Start(m,c,d,p,slice):** all of M and P are current at one serialization outcome; neither M-DENY nor P-DENY won first; atomically charge/mark an *active publisher x on both old scopes* before the first possible store; bind a bounded slice, output range and backing. A prepared destination that becomes stale must fail/reprepare before this cut.
3. **Publish portion:** set Vm/Vp/Vt as appropriate *before* first observable metadata/payload/TokenID output. For initial V candidate, x has a strictly bounded **nonfaulting, nonallocating** CPU store window. Inert public TokenIDs do not become invokable before all-N K-I; exposed IDs cannot later rebind unrelated rights.
4. **Finish/Fence:** record that no execution path under x can perform further stores; release active refs only after coherent cross-CPU evidence. P-DRAIN and M-DRAIN become eligible only when the relevant scopes have no active store-capable publishers. Count reaching zero without safe CPU/store ordering is not evidence of drain.
5. **K-I or terminal:** independently perform full native N-Grant install/queue retirement/protected receiver receipt if all current Claim/Grant/revoke guards hold. B on mandatory Grant before K-I prevents silent N−1. Otherwise settle terminal with honest possible exposure, lost-return and resource debt; **no same-m foreign requeue after sensitive V**.
6. **Receipt/cleanup:** receipt query is history only, cannot remint CapGrants or undo committed sender K-S. Task death, TokenID retire, pin/MemoryObject and DeviceGroup debt require separate lifetime ledgers; ordinary credits cannot borrow M0.

**Crucial unresolved physical feasibility:** keeping x "store-capable" on all affected CPUs without an unbounded spin/no-preemption section, while VM invalidation and issuer cutoff make progress, needs a concrete memory-order and scheduling/ref model. This study deliberately does **not** pick one lock or claim SMP correctness.

## 4. Candidate U-T two-resource ticket alternative and deadlock counterexample

A naive "acquire M, then acquire P" fails if acquisition of P can block while keeping an M ticket and P-DRAIN/Handoff needs a VM lock or vice versa. The opposite acquisition order has a symmetric failure. Therefore U-T is acceptable only with **all** of:

- admission tickets are bounded in count and lifetime and have a failure/rollback path that itself needs no unavailable Pager/M0 credit;
- if either DENY wins before **actual joint authorization**, no user-visible bytes are stored, even if an earlier ticket was held;
- each DENY can safely count an in-progress ticket as a drain liability without forcing issuer or VM controller to block under a global lock;
- pair validation and active writer enablement have a defendable coherent cut, not merely two sequential bool reads;
- TaskDeath and preempted Threads cannot indefinitely pin resources while a strong drain is falsely reported done.

**Paper deadlock U-T-X:**
- Thread T1 holds M ticket for d, waiting for P ticket; issuer controller holds P management lock awaiting P-DRAIN;
- T2 holds P ticket for p, waiting for M ticket; VM controller holds M management lock awaiting M-DRAIN;
- both drains wait on held tickets, no bounded release path. A scheduler boost cannot break logical lock dependency.

**Provisional research preference U-J over naive U-T** for the *small first proof model* because it exposes the desired one-cut property, not because a workable implementation or performance advantage has been demonstrated.

## 5. Special case U-R: short register-only control receive

For a native minimal kernel/boot control profile, bounded register output removes **user VA, COW, pin and M-INV from the output publication cut**, but not from all operations in the OS.

- A **ReturnRelease(r,p,ThreadGeneration,small-result)** must serialize with issuer P-DENY, Endpoint owner E-M and Task death. Preparing a saved trapframe is not yet necessarily visible.
- **Cut-first semantics candidate:** once ReturnRelease wins, the register values are treated as *possibly exposed* even if the Thread has not resumed; a later P-DENY cannot undo earlier authorized release. If P-DENY wins, stale confidential prepared return values must not enter a user context.
- Return preparation, K-I actual whole Grant vector install and caller receipt/result knowledge remain **distinct**. If K-I commits but caller loses the return, another Thread in the Task may already have native bearer rights; no reissue on Query.
- The initial control profile may intentionally support **zero-cap or finitely bounded, exactly representable small cap vectors**. A larger message must be refused under explicit size limits or handled by a **separately specified protocol**; never silently split one promised all-or-none transfer.
- A Thread might be preempted or CPU-migrated between ReturnRelease and eventual user execution. The candidate confidentiality contract is authorization at release, **not guaranteed impossibility of future observation after revocation**.
- seL4's *documented* message-register fast path is evidence that limited small-message profiles are architecturally meaningful; it is neither UniOS's implementation nor a performance proof.

**Early proof-surface implication:** U-R isolates issuer/Task generation, qR, K-I and receipt correctness from the much harder VM paired (M,P) problem. It is a sensible *design scope-reduction candidate*, not proof the whole independent OS is ready to implement.

## 6. M0/Pager and lifetime ledger

| Liability | Who charges it before which cut | Release must not imply |
|---|---|---|
| Immutable queued message bytes+Grant refs | sender before K-S | delivery/sender restoration |
| Sender qS/R_S and receiver qR/R_R | actual Task admission before each irreversible native cut | replay allowed after history GC |
| Claim/Endpoint generation | before exclusive K-C | message CapGrants already installed |
| Destination backing and MappingGeneration | VM sponsor before U-J joint permit | pin alone proves current VA, private audience or DMA quiescence |
| Publisher M/P active references | receiver/issuer/VM charged before possible V | deny alone implies old publishers drained |
| TokenID reservations/retirement | Task public namespace before first possible Vt | expired receipt permits rebind unrelated Grant |
| VM invalidation pending M-DRAIN | mapping/VM sponsor while old writer active | old frame reusable before CPU/TLB/drain evidence |
| Issuer P-DRAIN pending | issuer/Endpoint policy sponsor while old writer active | previously exposed plaintext erased |
| FaultWaiter/preflight Pager resources | VM/Thread/M0 path before page-in wait | privileged recovery waits on pageable stack, global cap lock or normal depleted queue |
| Object lineage, terminal Claim and DMA pins | object/DeviceGroup sponsor to actual safe terminal/R2-F | receipt ACK/TaskDeath means physical fence |
| Wired M0 CPU/IRQ/frame/IPC/cleanup | boot owner, never ordinary Task-borrowable | a finite operation count proves worst-case critical reserve |

**Wait-for proof target:** Pager faultable preflight → Pager service → permitted IPC → fault supply must never require a global lock/credit held by preflight; VM invalidation/P-DRAIN may not wait while holding one needed by a live publisher/abort path. A malicious preempted publisher may still postpone strong drain unless bounded CPU/abort assumptions hold. Honest "PENDING/FAIL-STOP" is preferred over false "drained."

## 7. Conflict/counterexample register PM-X01…PM-X22 — paper-only, NOT EXECUTED

| ID | Interleaving | Violation to detect |
|---|---|---|
| PM-X01 | validate M then P-DENY then check P stale | unauthorized joint Start |
| PM-X02 | validate P then M-DENY then check M stale | wrong current-VA binding |
| PM-X03 | paired Start races both DENYs on different CPUs | inconsistent half-authorization |
| PM-X04 | Start before P-DENY; issuer falsely reports P-DRAIN immediately | old publisher writes after strong acknowledgement |
| PM-X05 | Start before M-DENY; VM recycles old backing before M-DRAIN | use-after-retire |
| PM-X06 | after P-DRAIN, CPU cached permit issues one more store | bad SMP/store fence |
| PM-X07 | physical page pin survives Task unmap/remap | old frame incorrectly reported as current VA success |
| PM-X08 | COW/hugepage split after preflight, before Start | stale/nonresident backing in claimed nofault slice |
| PM-X09 | new unauthorized read-only alias made during PrivateAtRelease | unintended cross-Task plaintext |
| PM-X10 | postcopy epoch retry finds P change | falsely treats already written bytes as unpublished |
| PM-X11 | T1 holds M while waits P; T2 holds P while waits M | admission/death/lock cycle |
| PM-X12 | old publisher preempted indefinitely after Start | unsupported finite drain guarantee |
| PM-X13 | usercopy faults despite claimed pre-resolved nofault slice | Pager wait under active publisher permit |
| PM-X14 | Task B dies and B2 reuses ServiceID while old Claim exposed Vp | old claim/rights migrate to B2 |
| PM-X15 | endpoint E1 B→C after Vp, before K-I | automatic foreign same-m redelivery |
| PM-X16 | queued g2 revoked after K-S, before K-I | silent N−1 CapGrant install |
| PM-X17 | public TokenID Vt prefix visible, K-I denied | stale ID rebind unrelated Grant |
| PM-X18 | K-I completes, result lost, old qR query or new request | cap remint/duplicate install |
| PM-X19 | register result prepared then P-DENY, Thread resumes | missing ReturnRelease guard |
| PM-X20 | ReturnRelease before P-DENY, CPU descheduled | falsely claim prior release impossible to observe |
| PM-X21 | M0 ordinary reserve full while Pager completion required | critical service self-deadlock |
| PM-X22 | Task death or receipt GC while DMA still active | false R2-F and premature frame recycle |

## 8. Proposed invariants PM-I01…PM-I18 — NONE PROVED

| ID | Property to falsify when future verification is authorized |
|---|---|
| PM-I01 | Every V publication uses an authentic same-m Claim and actual current Task/Endpoint/issuer Audience identity. |
| PM-I02 | Paired Start may commit only if both M and P deny gates have not already won for its exact scope. |
| PM-I03 | Every successful Start charges an active writer to BOTH M and P before possible store. |
| PM-I04 | Neither M-DENY nor P-DENY grants fresh old-scope Start after its cut. |
| PM-I05 | M-DRAIN implies no prior publisher with that binding can issue further stores. |
| PM-I06 | P-DRAIN implies no prior publisher with that audience scope can issue further stores. |
| PM-I07 | Old plaintext knowledge is never assumed erased by M/P drain, revoke B or receipt GC. |
| PM-I08 | VM backing lifetime/ref and actual VA delivery are separately checked; pin alone cannot prove either audience or current VA. |
| PM-I09 | PrivateAtRelease cannot have unauthorized readable/writable aliases in the stated protected release interval. |
| PM-I10 | Vm/Vp/Vt are monotone and set before potentially observable output. |
| PM-I11 | P0-R ReturnRelease is a genuine issuer/Endpoint/Task-generation-ordered publication event. |
| PM-I12 | R0 output truncation never falsely commits full native message/cap vector. |
| PM-I13 | K-I and receiver R[qR] atomically install exact N grants/retire m once, or install none. |
| PM-I14 | A revoked mandatory queued grant cannot yield active unauthorized/N−1 reported K-I success. |
| PM-I15 | No foreign same-m redelivery after sensitive possible V; no sender K-S rollback because receiver failed. |
| PM-I16 | Faultable Pager waits never hold locks/credits required by own Pager/M0 recovery; live publisher is bounded or honestly pending. |
| PM-I17 | Terminal, token, claim, mapping and DMA debt survives receipt/Task GC until actual scoped settlement/fence. |
| PM-I18 | Native K-S/K-I/P-DRAIN do not establish service effect, data durability or physical DeviceGroup quiescence. |

**No unconditional liveness theorem asserted.** If CPU/Pager fail permanently, an acknowledged strong drain cannot be guaranteed at a fixed time without an independently justified kill/fence mechanism. A model pretending every publisher eventually runs under unconditional fairness would mask the central failure case.

## 9. Unresolved decisions PM-O01…PM-O12 — ALL OPEN

| ID | Priority | Narrow follow-on decision owner |
|---|---|---|
| PM-O01 | P0 | Define actual paired Start implementation class and nested M/P denial linearization on x86_64 SMP (002/003/004). |
| PM-O02 | P0 | Can a bounded private MappingGeneration binding remain genuinely nofault across remap/COW/migration without arbitrary global locks (004)? |
| PM-O03 | P0 | Enforce private-readable-alias exclusion and authorized shared audience while backing changes (002/004/018). |
| PM-O04 | P0 | Define exact per-CPU store-fence and Finish before M/P-DRAIN; no guarantee merely from refcount zero (001/004/005). |
| PM-O05 | P0 | Show nonrecursive Pager/CPU/IRQ/M0 wait-for and worst-case sponsor bounds (003/004/005/006). |
| PM-O06 | P0 | Scope strong versus weak E1 owner handoff acknowledgements against P/M drains (002/003/006). |
| PM-O07 | P0 | Specify register ReturnRelease vs prepared trapframe and lost qR result/Task death (001/005/009). |
| PM-O08 | P0 | Specify restricted first control IPC maximum message/token count with full-vector truth, without fixing final ABI numbers (003/009). |
| PM-O09 | P0 | Reconcile K-I against native B, Claim terminal, E-M/Task death and possible V (002/003). |
| PM-O10 | P1 | Determine bulk-object identity/version and ObjectViewRelease strategy separately, no VA implicit promise (002/004). |
| PM-O11 | P1 | Formal paper transition refinements/nonvacuity and future implementation-test instrumentation (research/001). |
| PM-O12 | P1 | Real hardware CPU/TLB/IOMMU/IRQ assumptions for bounded drain, cross-core validity and R2-F (004/005/007). |

## 10. Concrete D1 and implementation-readiness distinction

**This paper progresses CRDM-O03/04 cut specificity but does not close the blockers or promote draft ADR-CAPIPC-C/E to D1.**

- **Paper D1-required:** exact paired authorization guard and winner/loser semantics, destination lifetime/alias promise, safe terminal state, weak/strong drain acknowledgement, finite per-cut credits and absence of forbidden Pager wait edges. Competing outcomes must be distinguishable and unambiguous.
- **Later model D2** requires separately authorized bounded concurrency exploration with nonvacuous success, negative controls and independent review; an abstract checker pass is not an implementation proof.
- **First narrow implementation gate** must be documented separately for the *entire* UniOS MVP, not inferred from a single RFC/CAPIPC study. A minimal boot/diagnostic kernel could legitimately be scoped to omit pageable usercopy, third-party DMA, general Capability transfer, Linux/Windows/Darwin compatibility and a stable public ABI **only if that explicit scope and its own safety/validation plan are approved**.
- **Product/production gate** remains a later stage after actual implementation/refinement, hardware/negative tests and product security review. It is unsound to demand post-implementation results as prerequisites for *writing first experimental code*, and equally unsound to equate writing experimental code with validated native IPC.

**Immediate next recommended design-only task:** assess **UniOS Minimal Kernel Implementation Entry Gate** and, in parallel, fully specify **U-R ReturnRelease + zero-cap/small-control IPC** as a bounded early verification target. If a credible restricted MVP isolates open general V-mode risks, the correct next decision may be to seek explicit approval for that MVP rather than endlessly design all optional compatibility features. Until then, no implementation is authorized.

## 11. Sources and applicability notes

- [S1] Linux kernel page pins: https://docs.kernel.org/core-api/pin_user_pages.html
- [S2] Linux MMU notifier/COW and secondary address translations: https://docs.kernel.org/next/mm/mmu_notifier.html
- [S3] Linux usercopy faultable locking: https://docs.kernel.org/kernel-hacking/hacking.html
- [S4] Linux sequence-counter lifetime caveats: https://docs.kernel.org/locking/seqlock.html
- [S5] seL4 official IPC manual, message registers versus IPC buffer: https://sel4.com/Info/Docs/seL4-manual-latest.pdf
- [S6] seL4 official FAQ, short register messages/shared-buffer alternatives: https://sel4.systems/About/FAQ.html
- [S7] UniOS proposed parent RFCs and research contracts linked above.

**Evidence limitation:** a paper semantic ordering is not a real SMP memory-fence algorithm. Linux/seL4 references help frame feasibility; they are not implementations or proven correctness claims for UniOS. No code, no formal model execution, no benchmark or hardware experiment was performed.

---

**History:** 2026-10-26 Proposed v0.1. U-J/U-T/U-R alternatives, two independent deny/drain scopes, 22 unexecuted paper counterexamples, 18 unproved invariants, 12 open questions. No accepted ADR, selected ABI or implementation.

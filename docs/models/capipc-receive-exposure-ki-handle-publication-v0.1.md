# UniOS CAPIPC — Receive Exposure, K-I Commit & Public Handle Publication Ordering v0.1

> **Status:** Proposed independent semantic-cut, trust and adversarial-feasibility research — 2026-10-25 NZDT. **NOT Accepted, implemented, model-checked, proved, benchmarked, security-certified or ABI-frozen.**
> **Scope:** paper-only comparison of K-C Claim, K-Vm metadata, K-Vp payload, K-Vt possibly published public token IDs, K-I atomic receive/install, T1 capability invocation and optional T2 escrow release K-U. No executable model, kernel source, syscall ABI, proof results, performance measurements or automatic implementation authorization.
> **Predecessors:** [HPA-B+ vs HPA-C v0.2](capipc-handle-publication-activation-v0.2.md), [CAPIPC SEND–RECEIVE–REVOKE v0.2](capipc-send-receive-revoke-state-model-v0.2.md), [Task Handle Namespace & Receipts v0.1](capipc-task-handle-namespace-and-receipt-v0.1.md), [Commit/Revocation/Endpoint Ordering v0.1](capipc-commit-revocation-ordering-v0.1.md), [Formal Model Preparation v0.1](capipc-formal-model-preparation-v0.1.md), [CAPIPC Joint Integration Contract v0.2](../contracts/capability-ipc-sabi-integration-contract-v0.2.md), [Cross-RFC Safety Case v0.1](../reviews/capipc-cross-rfc-safety-case-v0.1.md).
> **RFC owners:** [RFC-002 Capabilities](../rfcs/rfc-002-object-capability-lifetime-delegation-revocation.md), [RFC-003 IPC](../rfcs/rfc-003-ipc-wire-protocol-and-synchronization.md), [RFC-004 VM/Pager](../rfcs/rfc-004-virtual-memory-pager-cache-and-oom.md), [RFC-006 M0/restart](../rfcs/rfc-006-boot-trust-chain-service-startup-and-recovery.md), [RFC-009 KABI](../rfcs/rfc-009-native-abi-and-system-call-contract.md), [RFC-018 SABI/IDL](../rfcs/rfc-018-versioned-idl-abi-evolution-and-compatibility-contracts.md). Adjacent RFC-005 scheduling, RFC-007 DMA and RFC-008 external durability retain their own proofs.
> **Target blockers:** FM-O07/FM-O08, SC-O03, H2-Oxx receive token/publication, RFC-003 §§6, 23–25. This document refines candidate semantics only and **does not mark any prior blocker CLOSED**.

## 0. Executive finding: one receive has at least three independent irreversible boundaries

**Research-leading family:** Preserve **HPA-B+ as the first *T1 bearer receiver* proof candidate**, with one logical receive and protected preidentified result record, inert reserved public token IDs, conservative exposure accounting and one atomic **K-I** CapGrant transfer+message retirement. HPA-C staged Task acceptance remains optional; **T2** requires an independently trusted policy/verifier and separately gated K-U. None is selected or proved.

The central mistake to avoid is saying “either receive succeeded or nothing was seen.” Three different effects are independent:

1. **Information:** metadata (K-Vm), payload (K-Vp), and public Handle-number bytes (K-Vt) can become *possibly observable* before a complete receive. A fault may leave a prefix in B's memory. The kernel must assume external knowledge cannot be erased.
2. **Authority:** receiving Task B does **not** own invokable new transferred CapGrants until one protected **K-I** installs the entire exact vector and retires that message. After K-I, granted authority is Task-wide, even if the initiating Thread hasn't observed success; another Thread in B may race to use it. This is the **T1 bearer trust ceiling**, not method-level consent.
3. **Knowledge/effect:** result copyout or Task death can hide the actual K-I outcome from the caller, and B can already cause external effects with other existing rights after seeing payload before K-I. No K-I or status receipt by itself proves an SABI method ran, an external effect did not happen, or storage is durable.

**Proposed default disclosure policy:** bind a message to an authenticated, eligible receiver audience at **K-C**. Record *potentially sensitive* K-Vm/K-Vp before a corresponding write/map and forbid requeue of that **same message** to a *different Task/ServiceIncarnation* after possible exposure; preserve same-authorized-context continuation when provably safe or terminal settle with sponsor/quarantine accounting. Token-only publication may not be confidential but still creates public token anti-ABA and a possible recipient-knowledge issue; treat foreign requeue as **disallowed by default** once any receiver-specific output could have become observable, unless a narrower explicit policy proves why it is safe. Do not claim that identical byte content can never be independently sent in a new message.

**Important non-choice:** This review favors the *candidate family*, **not a fixed micro-order between K-Vt, K-Vp and K-I**. An inert-token-first HPA-B+ branch is a narrower paper target; whether output copies and safe K-I have a valid bounded, non-pager-locked linearization remains **P0 OPEN**. Do not equate its readable narrative with an implementable algorithm.

## 1. Official precedents and evidence limits

| Official technical source | Supported external fact | Non-transferable inference |
|---|---|---|
| [S1] Fuchsia `zx_channel_read` | whole-message API, bytes buffer written before handles buffer; overlaps can overwrite | does **not** prove UniOS usercopy is all-or-none at byte granularity, nor an observed Fuchsia leak on failed call |
| [S2] Fuchsia `zx_channel_read_etc` | outputs actual typed Handle/right information; untrusted peer should be checked | caller IDL type string isn't native type/rights authority |
| [S3] Fuchsia Channel kernel object | queue and Handle ownership move under Channel send/receive; Endpoint ownership and FIFO have special guarantees | does **not** guarantee UniOS's K-I/public-token receipt design |
| [S4] Linux kernel usercopy guidelines | generic usercopy may fail with uncopied count and may sleep/fault, not safe under spinlock/IRQ disabled context | does **not** itself define a UniOS Pager or partial write protocol |
| [S5] seL4 IPC tutorial | explicit capability IPC and distinct CSpace/receiver configuration | synchronous IPC contract is not UniOS queued E1 nor a proof of B+ |
| [S6] Existing UniOS design models/RFCs | documents HPA-B+ candidate, exposure-before-store, Claim and authority ownership | their “MUST” statements are **unproved design obligations** |

**Accuracy guard:** Fuchsia's “partial reads not possible” describes its logical receive API. It is not evidence that its kernel leaks secret bytes after an error. This paper studies **hypothetical failure traces in UniOS** and must not attribute those failures to an external implemented kernel.

## 2. Five distinct visibility/custody axes (not one success flag)

| Axis | Protected actual state | Potential user-visible observation | Failure that does *not* undo it |
|---|---|---|---|
| **Recipient/audience claim K-C** | message exclusively bound to (actual TaskGeneration B, Egen, authorized policy/audience) | claim existence may reveal readiness/presence | Thread died; another reader arrived |
| **Metadata exposure K-Vm** | projection-classified and exposure logged before disclosure | size, cap count, object type/rights, timing/presence if in claimed scope | K-I later fails |
| **Payload exposure K-Vp** | monotone possibly-exposed record before first user-byte write or shared map | one or more content bytes may be seen by any authorized B Thread/aliases | usercopy fault, source death, revoked claim |
| **Token-number exposure K-Vt** | token IDs pre-reserved, still inert until K-I; possibly published token values tracked | old token integers may persist in B's Task memory | aborted receive; failed second array write; old-token ABA |
| **Native rights K-I** | whole CapGrant vector installed as B-owned with current effective rights, queue message retired coherently | issued tokens *can become invokable* Task-wide once K-I commits | lost return/result; B's other Thread close/MOVE; later revoke |
| **Method/effect K-U/S-A** | optional T2 trusted activation distinct from backend method effect | application policy may allow further operations | Task-local kernel success alone cannot prove user-level effect |

**No conflation:** public token **value** exposure is not new CapGrant issuance. A numeric token in user memory can be inert until K-I. Conversely K-I might install grants before the initiating Thread sees or retains token results; that Task may still own usable grants or other Threads may already have copied token IDs. K-I receipt requery reports history only and cannot reissue the cap vector.

### 2.1 Exposure is conservative evidence, not proof somebody read it

K-Vp/K-Vm/K-Vt is registered **before first potentially observer-visible action**. It records that information *may* have become observable, not that a full message was delivered or definitely read. A failed first byte store may cause conservative terminalization even when zero bytes were actually copied. This is an explicit availability-versus-confidentiality tradeoff, not false evidence of guaranteed disclosure.

Metadata might be public under an explicitly issuer-approved receiver/protocol projection, so not every length query forces a high-confidentiality barrier. A secret size/count/type/right projection does. A “BUFFER_TOO_SMALL” error may reveal length if policy permits; mere Handle guessing by another Task or stale ServiceID must not query confidential queue metadata.

**Scope:** a simple boolean `Exposed` can be enough for a coarse no-foreign-requeue first model, but safety and useful availability analysis need separate Vm/Vp/Vt markers and public/sensitive audience classification; otherwise overly strong/no-op invariants hide failures.

## 3. Required trust and identity checks at every boundary

- **K-C:** receiving TaskGeneration, actual E1 Endpoint object/generation and current effective owner, RECEIVE right, bounded claim/receipt sponsor, issuer-authorized audience (including ServiceIncarnation/policy scope where trusted). SABI session labels alone cannot mint access.
- **K-Vm:** check current claim, Endpoint owner/issuer projection and relevant revoke/policy; returning secret count/type/rights is information publication, not harmless preflight. A generic untrusted size probe must fail closed or expose only declared nonsecret aggregate.
- **K-Vp:** active exclusive claim, actual authorized audience, valid current receiver Task/Endpoint/lineage/policy after faults, and first-possible-exposure flag before a user store/map. If ownership moves or Task dies before disclosure, don't write to old B or new C merely because one previously held the Endpoint.
- **K-Vt:** output may reveal numeric reserved ID(s) before K-I. A token remains **non-invokable** until matched genuine grant is installed in that exact TaskGeneration at K-I; exposed IDs cannot later bind unrelated grants in same Task. Do not rely on secrecy of integer IDs.
- **K-I:** win exactly once against claim abort, Task death, Endpoint MOVE, K-S source queue destruction, lineage barrier B, copyout side effects and receipt GC. Atomically retire one message and install complete exact grant vector with one coherent protected R[q] phase, or no installation.
- **Optional K-U T2:** independently trusted verifier/issuer proves actual *immutable* message/cap vector/rights/lineage/receiver/method role/policy epoch and live revoke scope; Task-supplied accepted=true isn't authorization. K-U also needs separate known-before-commit receipt.
- **S-A:** service method policy admission and backend effect remain userspace layer; neither a numeric token nor native K-I status automatically authorizes external writes.

**No new intra-Task principal:** after K-I a B Thread that acquired a public token through shared memory may use it within B's Task grant scope. If application requires per-Thread role secrecy, consent or method validation before *any* use, ordinary B+ T1 is the wrong trust contract; isolate actors into separate Tasks or use independently trusted T2.

## 4. Candidate receive order families (all unapproved)

| Family | Simplified cut order | Safety upside | Still-unsolved risk |
|---|---|---|---|
| **R-B+T:** inactive token/slot reservation, K-C, preidentified R, V-m/p/t as authorized, then one K-I, report | avoids live grants before K-I, one logical T1 receive | partial bytes/tokens, possible head-blocking, precommit after V error, token burn, rights become live Task-wide on K-I |
| **R-A:** K-I first, copyout outputs later | simpler cap ownership cut | live installed-but-unreported orphan grants, malicious other-Thread use, result loss/cleanup |
| **R-C:** K-C Stage, V, explicit Task AcceptCommit→K-I | allows intentional Task-level hold before activation | lost first ClaimID/second status, malicious self-accept, queue/CPU/quota cost; no new security principal |
| **R-T2:** trusted escrow and externally verified K-U | can enforce policy-required inactive rights until independent verification | verifier TCB, immutable message/grant binding, revocation/policy epoch race, extra receipts and cost |
| **R-R:** copy-before-commit then requeue to new receiver after V | apparent fast recovery | **unsafe default**: potential secret redisclosure/duplicate effects |
| **R-S:** small register-only/no-usercopy IPC profile | narrower fault exposure and ABI surface | cannot satisfy arbitrary bytes+caps receive requirements |

**Research preference:** Compare R-B+T first against R-A/R-C under the *same* confidentiality, revocation, Task-generation and resource assumptions. R-C/T2 are justified only when a named real preactivation decision is required; don't pretend added calls alone provide independent authorization.

### 4.1 Proposed HPA-B+T logical path — NOT an executable recipe

1. **Source K-S already committed:** kernel queue holds whole bytes+Grant vector; sender can't recover MOVEd originals.
2. **Receive A(qB) and K-C:** preknown receiver qB/reserved scoped receipt precedes irreversible receive cuts; bind eligible B TaskGeneration, EndpointGeneration, message ID and sponsor. Other readers cannot take same message.
3. **Prepare/reserve:** confidential copy bounded in kernel staging; destination inactive public IDs and entire cap-table vector capacity pre-reserved; reject bad lengths, aliases, quotas or invalid permissions *before any sensitive V when possible*.
4. **Controlled publication:** authorize Vm projection; record Vp before any possibly visible user payload byte/map; record Vt before any public token output. All pre-K-I published token IDs remain inert and checked against TaskGeneration. **No full user-buffer atomicity claim.**
5. **K-I decision:** after current receiver/Endpoint/lineage/revoke/claim/death checks, commit full CapGrant vector install + message retirement + R[qB]=COMMITTED_KI as one protected cut. If not safe, terminal or constrained same-B recovery according to exposure scope; don't falsely say no bytes possibly seen.
6. **Postcommit return:** caller may fail to observe result due copyout fault or termination. Historical receipt query may return committed token identities (not recreate grants). Another B Thread may already have used/closed/MOVEd them; receipt doesn't promise current invokability.
7. **Cleanup:** after failed K-I, no unauthorized destination grant; burned exposed public tokens never rebind unrelated grant, and claim/queue/lineage/DMA/sponsor debt persists until actual settle/fence. After genuine K-I, handle ownership and message retirement are not reverted by a later return error.

**No claim that steps 4 and 5 can be implemented with no nonfaulting race.** Snapshotting user outputs before publication is insufficient if caller remaps memory or another Thread observes partially copied data. A trusted prepared grant vector and publication marker must be coordinated with the protected K-I cut *without* holding a global faultable lock. Exact lock/free-list/page-alias/SMP protocol remains P0.

### 4.2 Alternative branches whose result semantics differ

**After V before K-I fault:** actual claim exposure may be true; R[qB] must not become a misleading generic FINAL_NOT_COMMITTED understood as “nobody learned anything.” Use scoped **TERMINAL_EXPOSED_NO_KI** (logical result vocabulary only) or retain a structured final `NativeCapsNotInstalled` and independent `PossibleDisclosure` bit. The presence of two axes is essential.

**After K-I but output lost:** historic COMMITTED_KI and Task-wide rights may exist; return `UNKNOWN` to observer until protected result query, with no cap-vector second issue. A callback or loser Thread can't “retry receive” and receive that same grant again.

**No V and no K-I:** if exclusive claim is provably unexposed and no user-visible receiver-specific metadata/token was released under its privacy contract, requeue to an appropriately authorized receiver may be possible, *after* race-safe abort, policy and claim-generation checks. Do not claim that this is always safe merely from `Vp=false`; Vm/Vt and other visible metadata matter.

**After token-only Vt, no Vp:** for strictly nonconfidential numeric IDs it may be technically possible to retire/burn the token IDs, abort the claim and safely requeue if **both** the interface's at-most-one-recipient contract and audience metadata policy permit it. Prefer conservative terminal/no-foreign-requeue unless a separate narrow design proves that exception; numeric token history still must never alias unrelated grants in the same TaskGeneration.

**T2 self-approval pitfall:** an untrusted B that already sees bytes can call AcceptCommit or exert other existing capabilities before K-U. Only an independently authorized verifier/issuer can enforce stronger resource gating, and even T2 cannot unpublish bytes or undo external effects made with other rights.

## 5. Public token lifetime and copyout alias analysis

### 5.1 Public ID states and anti-ABA

| Public ID state | Native invokability | Reuse/cleanup rule |
|---|---|---|
| **ProvisionalHidden** (provably never visible, never active) | none | conditional reclamation possible **only if proof of no prior publication** |
| **ReservedInert** | none | held by current claim; generation/Task binding checked; no new grant by guessing |
| **MaybePublishedInert** | none | may be copied partially to B memory; **never reassign identical public bits to different grant in live Task** |
| **ActiveForG** after K-I | exact B CapGrant G with rights/current revoke policy | historic old ID cannot later authorize unrelated grant; close/MOVE invalidates it |
| **RetiredKnown** (closed, moved or failed publication) | none | no unrelated native rebinding before TaskGeneration end under currently Proposed strong anti-ABA |
| **ExpiredEvidence** (receipt GC) | none by history alone | lost receipt doesn't authorize public token reuse or a new cap |

Do not infer `PublishedToken` from whether *copyout returned success*; output may have been observed by another Thread or through shared aliases before fault. Even a successful copyout doesn't prove the initiating Thread read all bytes. Reserving all IDs before V alone doesn't solve finite exhaustion. Actual numeric widths remain undecided (HN-A/HN-B research).

### 5.2 Alias, size/count and fault rules

- Reject dangerous apparent overlaps between byte output, public token array, lengths, receipt/status outputs, and claimed input regions **before** exposure where possible.
- Virtual address disjointness does not imply different underlying writable physical backing: shared mappings, remaps, aliases, COW, concurrent same-Task Thread stores and DMA may alter or reveal contents.
- Prevalidation of user pointers is not sufficient. A buffer may unmap or change after validation; a fault after K-Vp requires exposure-aware settlement.
- `BUFFER_TOO_SMALL` may return an issuer-approved projection without committing K-I, but unauthorized secret lengths/type/rights must not leak through generic errors.
- Writing token integers after K-I risks active-but-unreported rights; writing inert integers before K-I risks observed-but-inert token burn. Both have distinct P0 proof and availability costs.
- CPU release/acquire ordering of protected R, token invokability and cap table publication requires formal/concrete SMP proof; merely calling a software field `active` doesn't make observed user bytes atomic.
- No global IPC/MM/cleanup/IRQ lock may be held across faultable Pager copyout; no unbounded Task-wide Handle-table spin while waiting for hardware/user memory.

### 5.3 Task death, Endpoint MOVE, revocation, and current rights

If Task B dies after V and before K-I, bytes may already have left B via another service. Revocation B or a dead Task does not erase knowledge; settle same queue message terminally where foreign disclosure is possible. If B's receiving E1 Endpoint MOVEs before V, abort/revalidate under a fresh claim and issuer audience; if it MOVEs after V, do not retarget claim to C automatically. At K-I, only live authorized receiver/Endpoint-generation/grants may become usable; historic sender K-S remains committed even if receiver gets zero caps.

Once K-I succeeds, later B, close, MOVE or other Thread actions may render individual public tokens unusable. A protected receipt truthfully remains COMMITTED_KI historically; independent `QueryCurrentHandle` may be added only if authorized and generation-checked.

## 6. Receive result classification (logical cross-product, NOT numeric ABI)

A single `SUCCESS/FAILED` status hides distinct dimensions. Proposed result should encode/correlate:
- **Native cap transfer:** NOT_KI, PENDING_KI, COMMITTED_KI, TERMINAL_NO_KI or UNKNOWN/EXPIRED as a caller knowledge class.
- **Exposure scope:** NONE_PROVEN, MAYBE_METADATA, MAYBE_PAYLOAD, MAYBE_TOKEN, or combinations. **NONE_PROVEN requires credible evidence that none of the relevant V cuts could have occurred**, not simply absence of a byte count.
- **Current grant usability:** separate live CapHandle query; never inferred from historic COMMITTED_KI.
- **External app effect:** OUT_OF_SCOPE / UNKNOWN, not native cap success.
- **Physical cleanup:** QUEUED, RETAINED, FENCED, QUARANTINED under own resource and DMA authority.

| Actual event sequence | Native grant truth | Exposure truth | Recipient/retry rule |
|---|---|---|---|
| buffer too small before authorized Claim/V | no K-I | only policy-approved metadata projection | re-attempt authorized receive permitted; no false secret-size claim |
| exclusive K-C then no V, no K-I | no K-I | none if correctly proved | unexposed claim may abort/requeue under issuer policy |
| Vp then copyout fault, no K-I | no cap install | payload possibly disclosed | no different Task/incarnation requeue; terminal or exact-B continuation |
| Vt of first token, second token faults, no K-I | no cap install | maybe numeric ID disclosed | never rebind IDs; conservative terminal unless proven safe exception |
| Vp then K-I commits, result lost | full vector installed once | payload possibly disclosed | query same protected result, **no new receive/second install** |
| K-I first in R-A, token output faults | full vector installed once | token output incomplete | orphan live grants require accounted cleanup/query; closing doesn't undo earlier native uses |
| Vp exposed to B, then Endpoint owner moves to C | pending or terminal, not C's automatic K-I | B may have seen content | foreign requeue/migration forbidden |
| K-I before B revoke; later token close | historical COMMITTED_KI | independent | no grant resurrection on receipt |
| B revoke before K-I | must fail-new-rights under applicable lineage scope or explicitly proved old-effect rule | earlier V still possible | no unauthorized install, terminal/same-audience handling |
| result record GC after final state | old actual cut immutable | evidence may be expired | q replay-fenced, historical result possibly UNKNOWN; no fresh grant |
| Task dies; new ServiceIncarnation appears | old custody settles independently | prior exposure may persist | fresh registry grant isn't old claim/receipt; SABI idempotency separate |

**Important definition:** A receive terminal result with `NO_KI` is a claim that **native cap installation didn't happen**; it is *not* automatically a claim that data was never exposed, that another Thread didn't act, or that the sender got original caps back. The exact result taxonomy and public ABI remain open.

## 7. Adversarial schedule register RP-X01…RP-X30 — NOT EXECUTED

| ID | Interleaving / attack | Failure to exclude |
|---|---|---|
| RP-X01 | K-C reservation returned only in lost ClaimID | no way to reconcile earlier possibly exposed receive |
| RP-X02 | B1 and B2 Threads race K-C against same queue head | two effective claims / two-cap deliveries |
| RP-X03 | K-Vm secret count before authorized claim | queue metadata oracle |
| RP-X04 | size-error reveals type/rights without issuer projection | secret cap metadata leak |
| RP-X05 | first payload byte possibly written; second faults | falsely `no disclosure` or foreign requeue |
| RP-X06 | usercopy returns EFAULT but old copied bytes still readable | false rollback of information |
| RP-X07 | first public token number copied, second faults | public ID later rebinds unrelated grant |
| RP-X08 | Vt token copied by B Thread2, K-I commits, Thread1 loses result | token callable before initiating Thread knows |
| RP-X09 | receipt query after committed K-I | duplicate cap vector installation |
| RP-X10 | K-I first, output faults; installed live orphan caps remain | uncharged leaked authority, false no-KI |
| RP-X11 | token pre-reserved but genuinely never visible | unnecessary permanent burning without proof policy |
| RP-X12 | token MAYBE-visible reservation reclaimed by mistaken optimistic GC | stale-public-token ABA |
| RP-X13 | metadata output alias writes over token output | wrong handle bits/metadata mismatch |
| RP-X14 | disjoint virtual outputs alias same writable backing | TOCTOU despite no VA overlap |
| RP-X15 | another B Thread changes user output mapping between preflight and V | unvalidated destination/publication scope |
| RP-X16 | Pager usercopy faults while holding global Claim/VM lock | deadlock against Pager/service recovery |
| RP-X17 | receiver E1 Endpoint MOVEs B→C after K-C before V | claim leaks to stale or new unauthorized audience |
| RP-X18 | E1 MOVE after Vp before K-I | requeue to C duplicates B disclosure |
| RP-X19 | ServiceIncarnation B dies after V, B2 starts | stale claim/receipt migrates automatically |
| RP-X20 | K-I wins against revoke B | grant installed consistent with actual barrier order |
| RP-X21 | revoke B wins before K-I | forbidden new revocable grant becomes usable |
| RP-X22 | staged token from stale TaskGeneration used in new B2 | cross-Task public identity confusion |
| RP-X23 | corrupt/unknown SABI Handle role, T1 K-I already occurred | wrong promise of method authorization/security |
| RP-X24 | T2 B self-calls `validated=true` instead of real verifier | fake independent policy approval |
| RP-X25 | trusted T2 checks mutable B buffer, subsequent alteration before K-U | TOCTOU wrong-role release |
| RP-X26 | T2 policy epoch revoked after verifier, before K-U | stale release grants rights |
| RP-X27 | committed K-I then B other Thread closes/MOVEs handle | old receipt incorrectly promises current invokability |
| RP-X28 | claim times out while Vp possible, queue sponsor refunded | foreign redisclosure and lineage/physical debt leak |
| RP-X29 | finalized receive receipt GC while DMA pins outstanding | premature hardware reuse / false R2-F |
| RP-X30 | malicious receiver repeatedly faults partial public tokens | finite token exhaustion / M0 starvation |

## 8. Proposed invariants RP-I01…RP-I30 — NONE PROVED

| ID | Required candidate invariant |
|---|---|
| RP-I01 | Each K-C binds exactly one authentic receiver TaskGeneration, EndpointGeneration and authorized audience. |
| RP-I02 | At most one active Claim owns a message; no foreign K-I can race it successfully. |
| RP-I03 | K-S sender receipt q and K-I receiver receipt q are independent authenticated namespaces. |
| RP-I04 | Preidentified receiver R[q] exists before irreversible publication/K-I, bounded with sponsor credit. |
| RP-I05 | Confidential K-Vm disclosure requires current authorized audience/projection. |
| RP-I06 | K-Vp becomes true before first possible user-visible payload store or shared map. |
| RP-I07 | K-Vt marks possibly published public token IDs before corresponding user output. |
| RP-I08 | `Exposed` flags never reset to false by copyout fault, timeout, ACK or GC. |
| RP-I09 | After relevant exposure, same queue message never transfers to unrelated receiver Task/incarnation. |
| RP-I10 | An unexposed claim requeue requires proof *all relevant Vm/Vp/Vt/audience effects* are safe. |
| RP-I11 | Pre-K-I public tokens are inert and cannot invoke native grants through mere numeric guess. |
| RP-I12 | Every maybe-published token remains nonrebind to a different grant for same live TaskGeneration. |
| RP-I13 | K-I atomically installs *exact whole cap vector* and retires exactly one queue message. |
| RP-I14 | K-I can win at most once against competing claim, revoke, E1 MOVE and Task-death cuts. |
| RP-I15 | R[q]=COMMITTED_KI iff actual full K-I cut happened for that invocation, while evidence retained. |
| RP-I16 | No receipt query can install/reissue closed/MOVEd/revoked CapGrants. |
| RP-I17 | At K-I T1 native grants become available only to authentic current B Task within actual rights, not new principal by caller ID. |
| RP-I18 | T1 grants may be used by other B Threads; no false per-calling-Thread or method-admission guarantee. |
| RP-I19 | After V but no K-I, any terminal result retains possible disclosure, never clean `no information` claim. |
| RP-I20 | K-I denied by B/revoke/owner change does not make earlier V disappear. |
| RP-I21 | No usercopy fault/block happens under incompatible global IPC/VM/IRQ locks. |
| RP-I22 | Output length/pointer arithmetic and aliased mappings cannot silently relabel one GrantID or leak to unauthorized Task. |
| RP-I23 | Receiver policy/SABI validation cannot enlarge kernel rights granted at K-I. |
| RP-I24 | T2 K-U requires independent current verifier grant bound to immutable message+native grants+policy epoch. |
| RP-I25 | T2 release after revoked lineage/policy is forbidden without fresh real authorization. |
| RP-I26 | Receipt/Task death/Service restart never permits re-admission of old q or stale claim reattachment. |
| RP-I27 | Queue/grant/claim/receipt/tombstone/pin charges have independent bounded lifetime and cannot starve M0. |
| RP-I28 | Revocation B/R1/R2-F/R2-O and external service effects are never inferred solely from K-I. |
| RP-I29 | No forced token recycling at identifier exhaustion; fail closed under finite public namespace. |
| RP-I30 | Observer UNKNOWN after K-I does not cause a second K-I or false NO_KI. |

**Scope of RP-I22:** ordinary user data can be corrupted by another Thread in B; the kernel cannot guarantee B's mutable memory stays semantically stable. Its enforceable target is accurate native Grant identity and no cross-Task unauthorized installation, not permanent integrity of user-readable bytes after return.

## 9. Trade-offs, quota analysis and liveness limits

| Design concern | B+T one logical receive | C/T2 staged variants |
|---|---|---|
| protected claim duration | potentially shorter, yet usercopy may Pager-block | potentially long while Task/verifier schedules |
| syscalls and result-loss cuts | one main receive (but prior identity/query still needed) | ClaimID, Stage/AcceptCommit and K-U each may have lost results |
| native TCB | snapshot/claim/V/tokens/K-I/status | all B+ plus extended escrow, release and verifier bridge |
| security boundary | T1 Task bearer grants after K-I | real independent T2 approval *only* if separate verified authority |
| failure after V | same no-foreign-requeue/terminal debt | same, plus longer visibility-without-K-I period |
| availability | may lose message on partial V without K-I | may lose message on Stage timeout; stalled queue head |
| quota | bounded queue, claim, tokens, R[q], issuer Task sponsor | extra held escrow/validation/receipt credit |
| performance assertion | **not measured** | **not measured** |

**Memory/identifier cost variables (symbolic):** let M be max queued messages, H max transferred CapGrants per message, Q max protected R records, X max active claims, T max public token IDs consumed/retired per TaskGeneration, c_i charged per receipt/claim, and F phys pinned/fence debt. “Bounded” requires explicit maxima across the combined queue+Claim+token+R+lineage+M0 design, not just a small X. Repeated partial Vt failures may consume token IDs without successful K-I; fail-closed policy must avoid using M0 reserves to fund attacks.

**Progress limits:** A malicious B can hold an exclusive claim and force terminal message loss, not a guarantee that an alternate C will get the message. A recipient waiting on Pager/Verifier whose IPC credit is held by that Claim can deadlock unless wait-for cycles and M0 reserved resources are proven. Explicit timeouts can cause terminal rather than safe redelivery. No exact deadline/quota is selected.

**SABI/application effects:** An exposed B may already have authority to send a network request or write a file before K-I even without transferred new caps; if so later terminalization does not assert external no-effect. A service-specific idempotency key and durable ledger, not native q or K-I alone, handles meaningful “exactly once” if the service claims it.

## 10. Candidate decisions RP-D01…RP-D12 — NONE Accepted

- **RP-D01:** Keep HPA-B+ as the first *normal T1 bearer* receive proof research candidate, not ABI/default implementation.
- **RP-D02:** Split metadata Vm, payload Vp, public token Vt, all-cap K-I and optional K-U into distinct protected conceptual cuts.
- **RP-D03:** Bind K-C to genuine TaskGeneration/Egen/issuer-authorized audience; Endpoint MOVE and service restart cannot silently migrate an exposed Claim.
- **RP-D04:** Conservatively mark potential sensitive disclosure before first user store/map and default to no-foreign-requeue once relevant V occurs.
- **RP-D05:** Reserve public IDs as inert until K-I; maybe-published IDs cannot later authorize unrelated CapGrants in that Task lifetime.
- **RP-D06:** Keep true K-I exactly-once full-vector install+queue retirement+protected result irrespective of usercopy/report loss.
- **RP-D07:** TerminalNoKI must retain possible-disclosure truth and sponsor/lineage debt; no inference of sender restoration or external no-effect.
- **RP-D08:** Handle current invokability is distinct from receipt historical COMMITTED_KI; query may never duplicate native grants.
- **RP-D09:** T1 bearer grants are Task-wide after K-I; T2 independent authorization is a different, conditional feature and trust cost.
- **RP-D10:** Recheck actual revocation B, Endpoint owner and current claimant at K-I/K-U; avoid unbounded Pager-lock scope.
- **RP-D11:** Metadata policy, copyout alias safety, token namespace exhaustion, claim timeout and Q/M0 quotas remain P0 proof blockers.
- **RP-D12:** No firmware, kernel, IDL/SDK, executable state model, ABI layout, proof or benchmark is approved here.

## 11. Blocking questions RP-O01…RP-O18 — ALL OPEN

| ID | Priority | Exact unresolved proof/decision | Owners |
|---|---|---|---|
| RP-O01 | P0 | Prove K-C audience under E1 Endpoint MOVE and issuer policy epoch change before any Vm/Vp | 002/003/018 |
| RP-O02 | P0 | Decide exact K-Vt versus Vp order and whether potentially published token arrays must precede K-I | 003/009 |
| RP-O03 | P0 | Prove K-I+complete cap install+queue retirement+R[q] phase coherence on SMP | 002/003/009 |
| RP-O04 | P0 | Specify Claim/receipt ID preknown before first irreversible V without lost return bootstrap circularity | 003/009 |
| RP-O05 | P0 | Bound Vp/Vt copyout and fault/Pager wait without any incompatible global lock | 003/004/006 |
| RP-O06 | P0 | Define when confidential metadata projections make foreign redelivery impermissible | 003/018 |
| RP-O07 | P0 | Show anti-ABA public token issuance/retire/possible exposure with finite HN-A/HN-B budget | 002/009 |
| RP-O08 | P0 | Define truthful `TerminalNoKI+PossibleExposure` statuses without new numeric ABI | 003/009 |
| RP-O09 | P0 | Verify race ordering K-I against B, E1 MOVE, Task death and queue close | 002/003/006 |
| RP-O10 | P0 | Document HPA-B+ current bearer rights ceiling vs schema/role verification for T1/T2 | 002/018 |
| RP-O11 | P0 | Define real independent T2 verifier attestation, immutable original message, native GrantIDs and K-U | 002/003/018 |
| RP-O12 | P0 | Prevent token publication with mismatched current grants under shared backing aliases and concurrent writes | 003/004/009 |
| RP-O13 | P0 | Define bounded claim expiration and terminal sponsor/lineage/DMA charges independent of final receipt GC | 002/006/007 |
| RP-O14 | P0 | M0 budget / Pager / WaitSet / claim progress and nonrecursive cleanup bounds | 004/005/006 |
| RP-O15 | P1 | Compare token-first/Install-first/register-only with equal assumptions and measurable workload later | 003/009 |
| RP-O16 | P1 | Decide narrow per-Task result discoverability vs Supervisor post-Task death authority | 002/006/009 |
| RP-O17 | P1 | Reconcile native recv knowledge/error semantics with Linux guest FD/errno and SABI effect | 009/010/018 |
| RP-O18 | P1 | Name exact counters/queues for future bounded formal model and negative controls before acceptance | model research |

## 12. Formal model readiness mapping — no executable spec

**Future tiny model actors:** sending Task A, receiver B with two competing Threads, competing Task C/new ServiceIncarnation B2, one E1 Channel/Endpoint generation, two distinct CapGrants, one queue message, exclusive Claim, one qB receipt, one revoker B, bounded output token namespace and one Pager fault/Task death.

**Required state variables:** MessageCustody, GrantCustody[g], ClaimOwner/ClaimEpoch, ReceiverTaskGeneration, AuthorizedAudience, Vm/Vp/VtExposure, PublicTokenState[id], R[qB] actual phase, B lineage/Endpoint owner epoch, UserObservation (separate), SponsorClaim/Queue/Token/R charges. Distinguish B revoke barrier name from receiver Task B.

**Directed negative controls:** (a) requeue after Vp; (b) accept K-I twice; (c) expose token then reuse its public value for different grant; (d) false `NO_KI/no info` status after V; (e) stale claim migrates to new Task; (f) K-I accepts revoked native Grant; (g) result query remints handles; (h) usercopy fault while kernel owns Pager lock. Future checker must discover a corresponding violating trace if the intentionally wrong rule is introduced, or reconsider model vacuity.

**Exit gate RP-R1 = OPEN / NO-GO:** no fixed default receive ABI until independent formal proof of audience/visibility, one exact K-I, concurrency, all-cap ownership, public token anti-ABA, lost-result receipt, bounded cleanup/M0 and correct T1/T2 authorization. A passing finite model alone would still need implementation/refinement/hardware evidence and quantitative resource measurements.

**Suggested next paper-only milestone:** produce a **joint SEND–RECEIVE semantic-cut matrix and canonical state ownership glossary** reconciling CO-S/CO-E-C and RP-B+T with all RFC-002/003/004/009/018 responsibilities. Then prioritize P0 blockers and ADR candidate review without prematurely introducing runnable models or kernel implementation.

## 13. Source references and limits

- **[S1] Fuchsia `zx_channel_read`:** https://fuchsia.dev/reference/syscalls/channel_read
- **[S2] Fuchsia `zx_channel_read_etc`:** https://fuchsia.dev/reference/syscalls/channel_read_etc
- **[S3] Fuchsia Channel kernel object:** https://fuchsia.dev/fuchsia-src/reference/kernel_objects/channel
- **[S4] Linux kernel usercopy reference:** https://docs.kernel.org/kernel-hacking/hacking.html
- **[S5] seL4 IPC tutorial:** https://docs.sel4.systems/Tutorials/ipc
- **[S6] UniOS internal Proposed contracts:** links in header.

**Limitations:** Source review and adversarial paper reasoning only; **30 unexecuted scenarios, 30 unproved invariants, 18 open blockers, 12 proposed decisions**. No direct test of any UniOS program exists. Other kernels' documented mechanisms do not constitute implementation feasibility proof, nor license/source import approval.

---

**History:** 2026-10-25 initial Proposed RP v0.1 research. Separate K-C / Vm / Vp / Vt / K-I / optional K-U, user-copy/Endpoint/Task-gen races, message/audience terminal semantics, historic vs current cap rights, T1 bearer ceiling and T2 verifier trust; no implementation or verified proof.

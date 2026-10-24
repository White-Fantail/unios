# UniOS RFC-017 — Verified Boot, Updates & Rollback Protection

> **Status:** Proposed — Specification v0.2 (adversarial re-review; not Accepted, implemented, hardware tested, cryptographically audited or formally verified)
> **Date:** 2026-10-24 NZDT
> **Architecture:** [UniOS v0.2](../architecture/unios-architecture-v0.2.md) — D7 accepted as *direction*, this RFC NOT accepted.
> **Cross-RFC:** [RFC-002 Capabilities](rfc-002-object-capability-lifetime-delegation-revocation.md), [RFC-003 IPC](rfc-003-ipc-wire-protocol-and-synchronization.md), [RFC-004 VM/Pager](rfc-004-virtual-memory-pager-cache-and-oom.md), [RFC-005 CPU/IRQ](rfc-005-scheduling-preemption-and-cpu-budget.md), [RFC-006 Boot/Recovery](rfc-006-boot-trust-chain-service-startup-and-recovery.md), [RFC-007 DMA](rfc-007-user-space-drivers-dma-and-iommu.md), [RFC-008 Storage](rfc-008-storage-filesystems-and-crash-durability.md).
> **Never authorize code/hardware operations:** no new kernel/UEFI loader, TPM NV/floor writer, key generation/enrollment, production signing or root rotation, GPT/UEFI/firmware modifications, actual update staging, real-media tests or hardware resets.

## 0. Executive recommendation — TRUST-A as conditional research baseline

**TRUST-A:** firmware UEFI-verified minimal EFI loader -> independently rooted UniOS BootPolicyManifest and exact kernel/BootBundle/M0 verification -> kernel-enforced small typed bootstrap grants -> runtime immutable SystemImage authentication -> TUF-inspired signed update repository with distinct Root/Targets/Snapshot/Timestamp roles -> crash-consistent A/B trial plus physically verified rescue -> monotonic hardware-policy security floor raised ONLY after independent healthy-success and verified usable rescue conditions. TPM measured boot is optional evidence, not code admission or proof of safety.

**v0.2 critical corrections:** per-variable UEFI crash atomicity does **not** imply access-controlled monotonicity; TPM NV COUNTER is an incremental hardware *type*, not an arbitrary writable security version; TUF Root rotation is sequential with old/new threshold checks, bounded update and role metadata rebootstrap; rollback of the client's stored trusted Root/Timestamp/version state is independent from target hash validation; signing/verified modes must distinguish OEM-root, user-owner root, enrolled development keys and unlocked firmware; boot-good and physically checked recovery are conditional evidence, not future hardware guarantees.

This is a promising hypothesis rather than final crypto format, parser location, PK/KEK/db policy, EFI implementation, TPM NV index layout, role thresholds, on-disk metadata protocol or product assurance. It preserves RFC-006 BOOT-A with wired M0 RecoveryAuthority and RA-2 fail-stop/verified reboot on unrecoverable root failure. It preserves RFC-008 STORE-A split signed read-only SystemImage vs separate writable DataVolume.

**Non-equivalent predicates:**
1. Firmware grants EFI executable admission under PK/KEK/db/dbx and current enforcement mode.
2. UniOS independently authenticates exact kernel, M0, BootBundle, immutable page backing under a trusted boot root.
3. Distribution repository vouches for signed target hash/size, metadata consistency and freshness under trusted time.
4. Device enforces a local security version floor independent of old signed disk data and a compromised normal service.
5. TPM PCR/event log records measurements; remote quote has distinct nonce/verifier semantics, not permission to boot.
6. Slot is bootable, then trial, then independently marked Good; rescue is actually readable, eligible and schema/key-compatible.
7. Boot/slot/floor metadata persists under declared write and power-loss model; no assumed multi-store transaction.
8. Actual firmware/CPU/IOMMU/drive and power-loss hardware obey explicitly named assumptions.

**P0 conflict:** Irreversibly increasing the minimum acceptable security version prevents booting old vulnerable binaries but may prohibit the only fallback after a bad update. This must not be waved away by two A/B slots or an untested signed rescue digest. The answer may be *reject floor promotion/update* or safely fail-stop until an independently usable recovery path exists. Security and offline availability are competing goals, not guaranteed both.

Alternatives: TRUST-B locally signed images and updates without a TUF-like role-separated repository (lower delivery complexity, weaker role compromise/freeze resistance); TRUST-C OEM-assisted firmware-rooted distribution on one reference PC (hardware dependence); TRUST-D explicit owner-unlocked developer mode with reduced security/data-key policy. All remain research comparisons; TRUST-D must never be presented as production verified.

## 1. Goals, constraints and threat model

**Goals:** prevent unsigned boot code, revoked signing keys and known vulnerable image rollback in a chosen production profile; limit online update repository compromise; staged offline-friendly consumer recovery; protect OS root and user data separately; avoid normal Supervisor self-promoting Good/floor; demonstrate an actual route to usable rescue after each allowed power cut; preserve user-data schema and encryption key compatibility when promised; prevent guest Linux root or ordinary VFS from becoming native signing/floor authority.

**Non-goals:** a working bootloader, UEFI Secure Boot enrollment, TPM policy, firmware flashing, production update service, valid signing keys, cryptographic implementation, key custody operations, tested recovery, physical device support, whole-disk encryption, stable ABI or zero-downtime update.

**Adversary capabilities:** attacker controls update mirror/CDN/network and stale files; modifies writable disk/GPT/slot metadata; forges ServiceID/health messages; compromises normal Supervisor/Pager/VFS/StorageHost, temporary online timestamp keys, maybe independent release keys; sends malformed manifests and huge compression metadata; stages old signed vulnerable images; interrupts power at any update transition; forces EFI NVRAM/TPM storage failure and boot loops; controls hostile PCI DMA-capable device. Full firmware/SMM/CPU and physical attacker compromise are stronger external trust assumptions, not automatically blocked by a signed OS.

**Assets:** boot code authenticity, constrained initial capabilities and M0 readiness, user-data read access/recovery, release root keys and signer revocation, monotonically enforced version floor, persistent trial attempts, and truthful security-mode UX.

A correctly signed malicious/vulnerable release can pass signature verification. Validity of bytes is not correctness of code or hardware. A signature of a receipt cannot force an SSD to persist earlier writes.

## 2. First-party official evidence and comparisons

| Source | External technical evidence | Proposed UniOS use/limit |
|---|---|---|
| UEFI 2.11 Boot Manager / Secure Boot [S1] | PK, KEK, db and dbx, Setup/User/Audit/Deployed modes, authenticated firmware policy | firmware validates EFI application only, not all later kernel/system bytes |
| UEFI 2.11 Runtime Variable Services [S2] | one NV variable has old-or-new persistence when interrupted and is saved before EFI_SUCCESS; **general variable service does not itself enforce anti-rollback/access policy** | distinguish persistence, authenticated-write attrs, monotonic replay protection and multi-store crash consistency; ordinary NV variable is not floor |
| TUF v1.0.36 [S3] | Root update fetches sequential N+1 versions signed to both old/new thresholds; refresh can invalidate trusted Timestamp/Snapshot after key change and requires fixed update-start time | specify intermediate versions, threshold unique key IDs, persisted trusted metadata and rotation recovery before any claimed TUF compatibility |
| TUF roles and metadata [S4] | Root, Targets, Snapshot, Timestamp have different signing roles and risk | online Timestamp compromise does not automatically mint boot Root |
| Android Verified Boot 2.0 [S5] | rollback-index locations in tamper-evident storage; for A/B update floor from slots marked SUCCESSFUL only | floor promotion gate plus stronger UniOS rescue/data conditions |
| Android A/B OTA [S6] | active, bootable, successful and bounded trial retries separate | old signed slot not necessarily allowed/usable or DataVolume-readable |
| Android Virtual A/B [S7] | snapshot merge may resume after restart; early snapuserd boot dependency | storage merge/boot critical resources only if a comparable scheme selected |
| TCG PC Client RIM/firmware profile [S8] | PCR measurement log needs verifier reference information and quote validation | measurements and enforcement distinct; optional privacy-scoped attestation |
| TPM 2.0 Library and errata [S9,S18] | TPM NV COUNTER type uses TPM2_NV_Increment rather than arbitrary value assignment; PCR and NV auth are different | cannot map release floor F→F' by silently setting arbitrary counter value; evaluate indexing and write/availability limits |
| Windows TPM data protection [S10] | PCR/boot-policy changes can interrupt transparent key unseal and force recovery | explicit user-data recovery key, profile migration, consent |
| Microsoft UEFI Secure Boot keys [S11] | firmware allow/revoke databases, dbx precedence and OEM ownership | firmware signer policy independent from UniOS signed targets |
| TUF FAQ [S12] | Root/Targets generally offline, Timestamp often online; metadata freshness/expiry matter | trusted time problem and offline update limits |
| Linux fs-verity / dm-verity [S13,S14] | immutable pages/blocks verified against authenticated content roots | read-time verification of later pager-backed code, not data durability |
| Linux block cache barriers [S15] | device completion differs from persistent Flush/FUA | RFC-008 crash-safe image and boot metadata stage protocol |
| Microsoft UEFI signing key rotation guidance [S16] | real PC PK/KEK/db CA replacement has compatibility and recovery impact | never enroll/reset another OS keys or silently trust arbitrary EFI loader |
| Android hardware attestation boot state [S19] | device's manufacturer root, user-installed trusted root and unlocked state can be distinguished in attestation | UniOS must distinguish OwnerVerified vs VendorVerified candidate and development/unlocked states; do not pretend boot root provenance is identical |
| TUF metadata root rollback and trusted-state persistence [S3,S20] | client keeps trusted Root/metadata versions and validates monotonic updates; full local cache rollback is a separate platform trust risk | define trust-anchor for the **client's last accepted state**, not only hash of new package |
| TPM NV counter attribute reference [S18] | NV counter is 8-octet increment-only logical type | define mapping from security-index vector, counter headroom, resource cost and key migration |

Source data belongs to other projects, and does NOT prove UniOS kernel safety, crypto implementation or device support. Relevant AVB documentation is about Android's own A/B rules, not automatically UniOS policy.

## 3. Boot trust chain and executable use-time identity

### 3.1 Candidate graph

~~~text
External OEM/owner firmware trust & effective UEFI Secure Boot policy
    -> EFI loader accepted under firmware signer/db/dbx rules
    -> independently rooted, protected UniOS boot verifier
    -> trusted Root/Release policy and current local security floor
    -> authenticated BootPolicyManifest (canonical fields)
    -> exact kernel + BootBundle + resident M0 code/data bytes
    -> small fixed typed BootstrapDescriptor under kernel-enforced grant ceilings
    -> authenticated immutable SystemImage hash/Merkle root
    -> read-time Pager page/hash/root check on every needed execution mapping
    -> normal Services/Apps governed by native typed capabilities, not signer name
~~~

**Boot-path provenance and data key boundary:** if generic firmware db also admits a different boot manager, the system may be able to start *another* OS even while UniOS's own code is correctly verified. UniOS's ProductionVerified label must be derived from trustworthy loader evidence, and access to sealed user data must be bound to appropriate boot identity and owner policy rather than assuming every firmware-authorized EFI app is UniOS. A local adversary booting another allowed signed EFI image is a separate attack against stored user data and mutable metadata; OS signatures alone do not provide confidentiality or anti-rollback.

A widely trusted firmware CA may authorize non-UniOS bootloaders; this does not imply that firmware authorized them to grant UniOS ProductionVerified state. Under a strict profile, the actual verified UniOS loader/verifier chain and protected policy provenance must match. The manifest being verified cannot freely choose its own validator root, version floor or unsigned exception list. A legitimate signed service with excessive grants can still be dangerous; kernel cap admission and manifest limits are independently audited (RFC-002/006).

### 3.2 Trust role boundaries

| Role | Authorized action | NOT inherited from |
|---|---|---|
| Firmware Owner / PK / KEK / db / dbx | EFI image allow and revoke, authenticated firmware updates | general user app UID0, UniOS Targets key |
| UniOS Offline Root Authority | versioned Root rotation and role thresholds, release policy | online Timestamp / ordinary device owner label |
| Image/Targets Release Signer | exact kernel/SystemImage/BootBundle hashes, version and hardware conditions | CDN delivery request |
| Snapshot signer | mutually consistent repository target metadata versions | ability to rewrite Boot Root or local floor |
| Timestamp signer | bounded fresh snapshot advertisement | right to sign arbitrary executable bytes |
| BootSlotStateWriter | scoped durable trial/success/unbootable/attempt records | ReleaseSigner or Task named Supervisor alone |
| Independent BootGood Witness | bounded essential service checks for exact boot image/epoch | self-declared normal Supervisor READY |
| SecurityFloorWriter | authorized monotonic floor promotion when gates met | BootGood event alone or signed manifest field |
| Rescue Image Issuer | release-approved image and recovery mode/policy | permission to boot revoked vulnerable image in production |
| Read-time Image Verifier | authenticates demand-paged bytes against signed root | general FileCap read/mmap access |
| Optional TPM/Key Policy | PCR measurements, quote, conditional data key unseal | ability to sign executable or change rollback floor |

### 3.3 Version namespaces are distinct

Release marketing version, signer/revocation epoch, anti-rollback security index vector, native ABI version, SystemImage hash, metadata Root/Targets/Snapshot/Timestamp version, BootEpoch/slot generation, hardware profile, filesystem schema and encryption-key wrapping policy must not be conflated. A higher version string is not a higher security floor; a newer signed kernel may still be vulnerable. Do not allow signed manifest to claim a numerical floor that unilaterally permits old revoked code.

### 3.4 Bounded parsing and post-verification TOCTOU

BootPolicyManifest research fields: format/version; canonical encoding and required-fields behavior; permitted signer/key epoch; exact image lengths/digests; trusted root and signature threshold; hardware/CPU/IOMMU requirements; kernel/BootBundle/M0 image hash; immutable SystemImage root; service ID, initial budgets and attenuated caps; update slot/recovery conditions; min/max supported DataVolume schema and key policy; security version vector; staged update and boot health policy; signed required feature extensions.

Parser location P0: compare trusted loader/resident M0 bounded manifest validation with small fixed kernel BootstrapDescriptor; do not implicitly import a complicated network certificate/JSON parser into Ring 0. Reject duplicate/conflicting keys, integer overflows, unknown required versions, ambiguous canonicalization, too-large decompressed content, hash algorithm downgrade, signer confusion and paths whose bytes change after check. Verified code pages and loader memory may not be modified by writable alias, untrusted DMA or recycled backing before being executed; revalidate later page faults as needed (RFC-004/007/008).

### 3.5 Development versus production states

**v0.2 trust provenance axes (NOT a final UI protocol):** VerifiedByOwnerKey, VerifiedByVendorPolicy, DevSignedByTestKey, FirmwareUnlocked/AuditOnly, and MeasurementOnly should be distinguishable and should not automatically inherit each other's guarantees. Android boot attestation distinguishes device-owned vs user-set root and unlocked state [S19], but UniOS does not inherit Android's GREEN/YELLOW/ORANGE certification. Even a valid owner-selected signed boot may be strong against unsigned tampering under the *owner's* threat model without being vendor/enterprise-managed verified. Public words like ProductVerified must name root provenance, firmware enforcement, local floor protection and actual platform profile.

- DevUnsigned (unlocked/offline VM lab): no production verified/rollback claims, no silent reuse of production sealed data keys.
- DevSigned (developer key/test OVMF): only a controlled local test signer is validated; cannot state real OEM chain or physical NV floor verified.
- ProductionCandidate (not a user-facing green state): enough design evidence to plan controlled real-hardware security test; still not verified.
- ProductionVerified (future only): effective firmware mode and complete UniOS chain/floor/immutable runtime proven on named hardware and software versions.
- RejectedUntrusted/RecoveryOnly/HardwareUnsupported: fail closed or explicitly restricted access; never show the same security assurance as ProductionVerified.

A user-authorized owner unlock/debug transition may legitimately change trust; it requires unmistakable disclosure, bounded scope, revocable device privileges, user-data recovery/key impacts and a distinct boot status. No trust promise on a user-managed firmware that is physically switched into non-enforcing mode.

## 4. Signed update delivery vs local boot policy

### 4.1 TUF-inspired role separation

Root is a rarely rotated threshold signer controlling keys and role policy. Targets signs exact bytes/hash/length and release constraints. Snapshot signs consistent metadata sets, stopping arbitrary mix-and-match of different target revisions. Timestamp signs the newest referenced Snapshot and expiration, defending against stale metadata **only when the client has trustworthy time and update-start policy**; it cannot prevent arbitrary network denial or indefinitely missing repository data merely by being signed. Delegation must be scope-limited and cannot increase parent authority. TUF v1.0.36 is a specific formal update standard; this RFC does not assert conformance without implementing and testing its exact metadata and refresh procedures.

**TUF v1.0.36 rotation precision:** when updating Root N→N+1, verify **both** the old root's and new root's required signature thresholds using distinct key identities (not duplicates), require exactly sequential versions, bound fetched bytes/iteration count, persist trusted intermediate Root, and process the appropriate Timestamp/Snapshot cache reset after rotated keys to recover from fast-forward metadata [S3]. A generic two-signature check or skipping version N+1 is not a TUF-compliant replacement. Exact persistent version/root state must also survive client restart or a disk rollback threat may undo its memory of accepted metadata.

A compromise of an online Timestamp key cannot mint a new Boot Root or a new kernel target if the other roles remain secure. A Targets key compromise can authorize malicious release artifacts within its scope (serious!); Root threshold compromise can authorize new keys and is a critical incident. A signature is not a quality audit or transparency guarantee. Treat publication/CI/role key custody/reproducible builds as independent supply-chain trust decisions.

### 4.2 Rollback, freeze, fast-forward, mix-and-match

**v0.2 attacker restores entire local trust cache:** even if every downloaded target hash is valid, an attacker who can restore the complete previously accepted Root/Targets/Snapshot/Timestamp metadata+highest-seen versions on disk may defeat *client-history* rollback detection. Per-update TUF metadata checks assume a correct locally trusted starting state; protect that root/version history as part of the client trust anchor or reduce the promise explicitly. A TPM floor for boot images is not automatically a monotonic record of every TUF metadata role version; these have different storage/frequency and threat-model needs.

- **Target rollback:** older signed target still verifies; local security floor and metadata monotonic versions may reject it.
- **Snapshot mix-and-match:** server combines older targets metadata with newer payload; Snapshot consistency/hashes should reject.
- **Freeze:** attacker indefinitely serves an old signed timestamp/metadata; expiration works only against a trustworthy time/freshness basis.
- **Fast-forward:** attacker with an affected role key publishes extremely high metadata version to block legitimate future update; root rotation/recovery must follow threshold policy.
- **Key replacement:** old trusted Root may need to sign authorization of new Root and new Root sign itself under exact TUF root-rotation rules; a new server-provided key ID alone cannot rotate local root.

### 4.3 Offline time is a hard product choice

Consumer UniOS should be able to boot previously authorized immutable OS image *offline* using locally enforced floor and trusted key epoch. Freshness rules for accepting a **new update** may require current clock, signed newer metadata and comparison to previously persisted versions. An incorrect RTC after years powered off cannot magically prove target Timestamp expiry. If time/metadata cannot be verified, reject a new online update or present FreshnessUnknown; don't silently allow stale package as current or brick normal offline boot merely from no network. A stricter managed profile may require online time and attest availability, but is not an implied consumer default.

### 4.4 Staging image does not grant privilege

Successful network download/TUF manifest validation means candidate update content is eligible for further local policy checks, not that it may set floor, Root key, firmware dbx or arbitrary Task capabilities. Before use, validate release/hardware/schema compatibility, key revocation, version floor, image bytes at install and boot, and independent signed bootstrap ceiling. Ordinary UpdateAgent must never be able to rewrite the trust anchors that authorize itself.

## 5. A/B trial, boot success and security floor

### 5.1 State and separate authority ledgers

| Record | Trusted writer | Promise / limitation |
|---|---|---|
| Slot content/immutable root | signed release manifest and scoped inactive-slot installer | bytes verifiable, not yet verified boot |
| Slot flags ImageVerified/TrialBootable/Good/Failed | limited BootSlotStateWriter | crash-consistent old/new update; not security floor |
| Persistent boot retry count | limited preboot trial recorder | survives reboot/failed trial; BootEpoch RAM counter insufficient |
| Bounded independent HealthReceipt | M0/trusted witness with exact policy and image identity | only checks named services/conditions for an observed time |
| SecurityFloor vector | independent trusted monotonic FloorWriter | irreversible security authorization constraint; not just slot flag |
| RescueVerified | independent authorized verifier + physical read/boot probe | actually present, accessible, eligible and keys/schema-compatible |
| UserData schema | separate DataVolume writer/migration protocol | changing schema can strand old software despite signed image |

### 5.2 Candidate update transaction with all cuts

~~~text
A = Good and SecurityFloor = F, DataVolume schema V1
  -> obtain signed update metadata / exact B payload
  -> stage B to inactive slot (A not overwritten)
  -> flush B and hash-verify stored exact image/manifest
  -> record authenticated B ImageVerified and finite TrialBootable attempts
  -> select/boot B Trial (A retained while floor permits)
  -> gather independent bounded HealthReceipt (NOT normal READY alone)
  -> persist B Good via authorized SlotWriter, check readback/error
  -> verify actual floor-eligible rescue/alternate, data schema and key path
  -> FloorWriter MAY advance F to F' under separate accepted policy
  -> revalidate remaining rescue eligibility under new F'
  -> A may now be ineligible; do not call it fallback
Power cut: deterministic recoverable previous/new state OR explicit
           RescueOnly / RecoveryUnavailable / Unsupported / FailStop.
~~~

Single UEFI variable writes have limited per-variable old/new semantics, but writing GPT, disk image, slot flags and TPM NV floor is not one transaction. If the writer fails during a critical point, prove which of the old and new states remains valid. Boot attempt decrement must persist before risky new trial to avoid infinite reboot loops. Persistent records need duplicate generations/corruption handling and finite retry failure policy, not blind loop.

### 5.3 Three constraints for usable fallback

1. **Eligible:** rescue or alternate signer not revoked, version meets current floor, release/hardware policy allows it.
2. **Present and executable:** exact verified bytes are physically readable from supported storage and can boot on available firmware, drivers, key material and devices.
3. **Data compatible:** system/rescue can safely read the current writable data schema and encrypted user key, or it is explicitly a non-destructive rescue with limited data access.

Signed digest in a manifest proves none of physical presence, read stability or future repairability. A new B image being Good does not guarantee permanently operational hardware. Floor promotion may eliminate A; verify a truly available independent authorized alternative before any irreversible promotion (RFC-006 BOOT-38/40).

### 5.4 HealthReceipt and progress predicates

HealthReceipt binds the exact B SystemImage digest, boot epoch, verified manifest health policy version, protected M0 witness, essential StorageHost/IRQ/Pager/IPC usability and independent protected issuer. No ordinary service may mark Good by self-reporting READY or sending an IPC message containing Good. A signed HealthReceipt does not prove future uptime, all devices resettable, dirty user files durable or CPU watchdog functional when hardware has died.

### 5.5 Recovery attempts and failure

If trial/reboot attempt record is stored only in RAM, it resets each boot. A persistent retry ledger must have well-defined failure modes if EFI variables full/unsupported, TPM NV exhausted or disk unavailable. If decrement/mark-failed cannot be committed, fail boundedly or enter RecoveryOnly; **never** interpret failed record persistence as unlimited attempts. A legitimate production endpoint may fail closed without boot if every eligible alternative is unavailable.


## 6. Anti-rollback floor storage and transactions

**The floor protects against a correctly signed but vulnerable old image** whose bytes and signature are otherwise valid. Product version, signed metadata version, key epoch, hardware policy and per-component security index are distinct. A root manifest cannot unilaterally choose its own validation floor; the floor must derive from a previously trusted independent store/policy.

**Counter-type correction:** The TPM NV COUNTER entity, when supported, is an 8-octet value updated using TPM2_NV_Increment; it is **not** an ordinary arbitrary writable integer. A security-index F may jump by a large amount on one release and include several component locations. Mapping those values to counters requires an approved bounded representation (e.g. a distinct certified anti-rollback monotonic epoch with authenticated floor vector), headroom and crash-correct verification; a naive loop doing hundreds of thousands of increments per upgrade would be a wear/performance and availability hazard. Nor may the floor vector be kept in a replayable disk file with only a TPM 'counter exists' flag [S18].

A normal GPT/ESP/DataVolume file, including a signed old file, is replayable by a disk attacker. Candidate storage includes a specifically authorized TPM 2.0 NV counter/record, platform-protected signed EFI variable, RPMB or independently trusted external anchor. **TPM existence is not proof of an available usable monotonic index; firmware authenticated variables are not necessarily a trusted nondecreasing counter.** The UEFI spec [S2] specifies per-variable old/new persistence under failure, limited NV space, and runtime Unsupported/OutOfResources; **it does not generally secure read/write/delete access for arbitrary variables** and does not promise one monotonic counter or atomic transaction across separate slot/floor/data stores. Authenticated-write attrs have their own validation and firmware trust requirements; merely naming a variable 'UniOSFloor' is not security.

### 6.1 Safe floor promotion predicate

For a candidate new floor F', require all of the following: actual stored F <= F'; appropriate independently authenticated policy grants FloorWriter; B's exact signed image/manifest/hardware profile is verified and a separate authorized health witness generated a durable Good receipt; at least one image other than revoked old A remains *physically present, readable, signer/floor-eligible and bootable*, including available device drivers and DataVolume schema/key path; firmware dbx or key rotation will not invalidate that image; rollback-index domains can be updated with recoverable intermediate states; read-back validates actual stored floor. If these cannot be established, deny irreversible floor promotion, enter explicit rescue/degraded mode or fail-stop as appropriate. Health checks cannot promise indefinite future physical availability.

### 6.2 Alternative trusted stores and failure modes

| Candidate | Possible benefit | Limit |
|---|---|---|
| TPM 2.0 NV protected index | may resist disk rollback when correct authorization and index type semantics verified | an NV COUNTER is increment-only, cannot directly set F'=300 from F=12, needs mapping and per-index access/space/repair design; NV availability/locking/TPM replacement |
| Authenticated UEFI variable | firmware policy and per-variable durable write semantics | not generalized monotonic floor, tiny storage, firmware/runtime failure |
| RPMB/secure flash | device-backed anti-replay on selected hardware | not universal x86_64 PC capability |
| Independent online verifier | remote latest-state/freshness anchor | no offline liveness guarantee |
| Plain GPT/file journal | crash consistency for honest storage | attacker can restore previous signed data/floor |
| No suitable protected medium | development or explicitly reduced-security signed boot possible | **no production strong anti-rollback claim** |

The floor counter, authenticated Good state, slot retry record, bootable rescue and mutable user-data schema may inhabit **independent nontransactional devices**. Individual atomic writes do not make cross-device transactions atomic. An irreversible-last stage and explicit recovery from every interrupted phase are proof obligations, not implementation details.

## 7. Key lifecycle, compromise and revoked loaders

Offline Root role controls signing thresholds and release-role keys; Targets signs exact image bytes/hardware constraints, Snapshot signs metadata consistency, online Timestamp signs frequently refreshed freshness. Root rotation needs authorization by previously trusted Root and new Root per chosen standard, not just a signed manifest telling clients to accept a new untrusted key. Role threshold is effective only if signing custodians and private keys are truly independent.

If CDN or Timestamp private key is compromised, valid independent Targets/Root policy should continue to reject malicious new code (not necessarily denial/freeze). If Targets signing key is compromised, attacker may legitimately sign malicious code within the role scope; rebuild, independent Root revocation and physical rescue may be needed. If Root quorum is compromised, purely in-band software update may be unable to re-establish trust. Correct signatures do not prove the signed binary's safety or uncompromised build.

**Owner root and product root provenance:** vendor-managed signed release, user-enrolled root, test/dev signers and unlocked firmware are separate assurance states. A user-controlled root may be deliberately authorized and securely enforced under its owner's threat model, but the platform cannot label that path as UniOS-vendor-rooted without actual matching provenance. Root provenance must be included in boot status and DataVolume sealed-key policy, not merely the word 'verified'.

UEFI OEM PK, KEK, db and dbx are **not the same** as UniOS Root/Targets roles. Updating firmware dbx or replacing firmware PK/KEK can also disable an installed Windows/Linux loader or the only rescue USB. No automatic firmware enrollment/revocation in this RFC. Emergency key revocation can make an older rescue unacceptable; do not silently enable revoked image to improve availability. Owner physical override must have visibly weaker/unlocked trust mode and assess data-encryption consequences.

Trusted keys may need algorithm migration, new Root dual authorization, removal of obsolete keys and compatible old/new loader verifiers in a carefully planned stage sequence. No algorithm, threshold value, private-key HSM, team custody or release ceremony selected yet.

## 8. TPM measurements, attestation and user-data encryption

**Verified Boot** enforces that code meets signer/content/floor rules; **Measured Boot** extends available measurements into TPM PCRs and provides an event log; **remote attestation** requires a fresh verifier nonce, trusted quote identity, reference data and consistency of event/PCR log. PCR hash values by themselves do not prove that a component never loaded or that a component was safe; measurement gaps and untrusted event-log interpretation remain threats. TCG PC Client firmware profile/RIM describes evidence and reference matching [S8], not a substitute for launch prevention.

PCR-bound sealed encryption keys may stop unsealing after legitimate firmware/bootloader/dbx changes and new OS releases [S10]. A physically valid old slot may therefore be unable to decrypt user files even if the DataVolume format is compatible. Design explicit recovery secrets, user consent, PCR policy migration and key rotation. Do not claim TPM implies automatic disk encryption, nor disk encryption prevents hostile apps while volume is unlocked.

Attestation can reveal persistent device/firmware state and enable tracking; keep opt-in for consumer research baseline with clear data minimization. Offline M0 failure/recovery cannot depend on remote attestation/network or exact UTC availability. TPM replacement/motherboard repairs create a separate user-data recovery and hardware floor migration process that may not be automatable.

## 9. Recovery modes and physical availability

| Class | Failure | Contractual possible outcome |
|---|---|---|
| Local service restart | Supervisor, Pager or driver process crash | resident M0 with valid cap/CPU/IRQ/IPC and device fence path; no durability guarantee |
| A/B trial fail | B does not pass Good | select physically available allowed A/rescue only if floor and data permit |
| Read-only data recovery | user volume corrupted or unsupported | verified immutable rescue can attempt limited data access without false write safety |
| Storage lost | only SSD/controller failed | **same-disk rescue not independently available**; another tested physical medium needed |
| Firmware/TPM update mismatch | signer keys/dbx/PCR policy changed | physical owner repair/recovery keys; never silent signature bypass |
| Broken hardware isolation | malicious DMA before effective IOMMU | isolation may not be possible; fail-stop/platform unsupported |

An image's digest in a manifest is not a verified physical alternate. Actual fallback must be signer/key-eligible under post-update floor, present/readable, bootable with the needed storage/IRQ/CPU devices, and capable of reading current DataVolume keys/schema or clearly labeled nondestructive offline rescue. Boot attempt counters need genuine persistence between reboots; a per-BootEpoch counter is not a reboot-loop guard.

If no authorized physically usable slot exists, ProductionVerified may have to refuse boot. Without an independent working display/keyboard, even "repair screen available" is not guaranteed. A production-level fallback promise must specify physical media independence or disclose single-drive failure risk.

## 10. Storage/DMA/schema coupling with RFC-006/007/008

Staged B image hash verification before writing doesn't prove B persists. Installer must re-read/reverify stored exact bytes, flush image and metadata with RFC-008's validated controller/Flush/FUA semantics, then mark image trial-eligible. Neither successfully staged image, kernel boot, independent Good receipt nor TPM floor proves the SSD will survive future physical failure.

Update from A DataSchema V1 to B V2 may make A unreadable after B's Trial even without floor promotion. A signed previous SystemImage and valid rollback key do not undo B's user-file migration, external application effects, encryption key wrapping or snapshot garbage collection. Validate safe migration/older reader compatibility or a durable retained recoverable V1 snapshot with enough storage and actual rescue drivers/keys.

RFC-007 R2-F fence allows safe DMA buffer reuse if hardware actually stopped; it does not undo already persisted storage commands, journal effects or corrupt files. A timed-out write request may have reached disk; IPC COMMITTED, StorageHost ACK and FileSync DurableReceipt have distinct result semantics (RFC-003/008).

## 11. Failure and adversarial matrix

| Attack/failure | Forbidden simplification | Candidate response |
|---|---|---|
| UEFI Secure Boot disabled/SetupMode | signed path = production verified | declare Dev/Unlocked/Unsupported state |
| EFI loader UEFI authorized but kernel altered | firmware accepted all later code | independent manifest/kernel/M0 hashing |
| Manifest names its own trust root/floor | self-referential policy accepted | anchored verifier, bounded parser |
| Verified bytes changed via DMA/writable alias | load-time check still valid | protect immutable extents/reverify at use time |
| Guest administrator requests boot signer rights | UID0 equals native authority | deny typed cap escalation |
| Timestamp expired and RTC wrong offline | new update freshness still certain | FreshnessUnknown/reject update; offline allowed boot separate |
| Replay old snapshot/targets | old signed image may run | metadata version consistency + local floor |
| Compromised Timestamp online key | new malware automatically signed | distinct Targets/Root trust |
| Compromised Targets signer | cryptography guarantees safe code | signed-code compromise response and independent Root |
| Root signer threshold compromised | purely remote auto-recovery assured | require independent physical/out-of-band recovery |
| Staged B without media Flush | trial boot after power cut safe | do not mark ImageVerified until durable |
| B Trial fails but floor already F' | A forbidden, B broken | floor-last policy and eligible rescue gate |
| Normal Supervisor forges READY | mark B Good automatically | independent scoped HealthReceipt writer |
| NVRAM Good write succeeds but TPM floor interrupted | one atomic transaction assumed | cross-store crash graph/recovery statuses |
| TPM NV full/missing | disk slot file is trusted floor | deny strong anti-rollback claim |
| Driver reports successful DMA stop but writes persist | boot image remained immutable | RFC-007 trusted fence or fail-stop |
| New B migrates user data V2 | old A is usable fallback | schema/key compatibility gate |
| Floor rejects old rescue | automatic revoked boot anyway | alternate verified rescue or explicit recovery unavailable |
| Key rotation invalidates only EFI rescue signer | signed image digest implies bootable | signer/dbx compatibility before rotation |
| B infinite reboots, BootEpoch resets | retry limit still enforced | persistent attempt budget or recovery mode |
| TPM PCR policy changes after firmware update | user data auto-unsealed | recovery key and policy migration |
| Disk removed after rescue scan | rescue still present | runtime physical availability recheck |
| Attestation stale nonce/log | trusted current verified boot | freshness and reference log validation |
| Physical firmware compromised | signed OS repairs prior RAM corruption | explicit platform assumption limitation |
| Offline old verified OS never gets security updates | timestamp prevents compromise without time | detect/update-liveness policy, no impossible freshness |

## 12. Research-only model and evidence plan

A finite model should include two slots, a rescue image present/corrupt/missing, two old/new signer generations, independent TUF Root/Targets/Timestamp metadata, per-location floor F/F', broken TPM NV or UEFI variable, one dishonest Flush, slot Good/Trial/attempt counters, an independent health witness vs forged normal service, DataVolume V1/V2 and PCR-wrapped key, plus the boot/root-capability state. Enumerate staged writes, power cuts, health forging, signer revocation, floor promotion, recovery boot and hardware replacement interleavings.

Counterexamples: authorized production boot of revoked image; self-signed manifest root; rollback bypass by restoring old signed disk; floor promotion before a usable rescue; boot retry loop after failed persistent decrement; false Good from compromised Supervisor; Good true but immutable image unreadable; user schema/key incompatible with all rescued images; a stolen online role can authorize offline Root; hardware firmware trust false positive.

Test tools *if future implementation is authorized*: bounded TLA+/PlusCal model, fuzz manifest parsing and canonical encoding, simulate interrupted device/TPM/NVRAM slot writes in QEMU/OVMF only with explicit virtual hardware, then inspect a sacrificial Reference PC for actual EFI, TPM NV and SSD features. Real key enrollment, firmware updates, power interruption or destructive disk tests require separate authorization and precautions. **None were executed here.**


## 13. Proposed safety, security and liveness invariants (TRUST-01…TRUST-40) — NOT proved

| ID | Candidate requirement, without proof or implementation |
|---|---|
| TRUST-01 | UEFI admission of EFI image is not equivalent to authentication of kernel, M0, immutable backing or later app pages. |
| TRUST-02 | ProductionVerified can be asserted only under effective platform trust mode and verified full UniOS boot path; DevUnsigned/DevSigned never silently qualify. |
| TRUST-03 | A manifest cannot choose its own independent Root trust anchor or lower the floor against which it is checked. |
| TRUST-04 | All image signatures bind exact canonical bytes, digest, byte length, permitted role, release policy and key epoch without ambiguous required fields. |
| TRUST-05 | Kernel/bootstrap grant vector is typed, bounded and independently enforced; signed ServiceID alone never mints native global privileges. |
| TRUST-06 | Verified boot code/data cannot change by memory alias, PagerSupply or untrusted DMA between authentication and use without renewed validation. |
| TRUST-07 | Runtime immutable executable pages derive from an authenticated root and fail closed on digest/backing identity mismatch. |
| TRUST-08 | Firmware secure boot signer/PK/KEK/db/dbx and UniOS Root/Targets roles are not treated as interchangeable. |
| TRUST-09 | No unsigned, revoked or below-floor image executes as ProductionVerified, irrespective of valid older signature. |
| TRUST-10 | Update repository Root/Targets/Snapshot/Timestamp signatures and thresholds remain distinct; online Timestamp cannot mint release authority. |
| TRUST-11 | Metadata rollback, freeze and mix-and-match checks rely on verified versions, hashes, root delegation and explicit time assumptions. |
| TRUST-12 | An offline stale RTC cannot be silently called a proof of current update freshness; offline authorized boot policy is separately defined. |
| TRUST-13 | Trusted Root rotation follows current-and-successor authorization policy; no untrusted server-declared key is accepted as new Root. |
| TRUST-14 | Firmware dbx update and UniOS package/update signers do not implicitly share writer authority or recovery permissions. |
| TRUST-15 | Slot staged bytes and manifest are flushed, read-verified and reauthenticated at boot before marked TrialBootable. |
| TRUST-16 | A/B ImageVerified, TrialBootable, Good, security floor, health and actual rescue availability have distinct predicates. |
| TRUST-17 | Normal Supervisor READY/heartbeat alone cannot authorize Good or security floor promotion. |
| TRUST-18 | HealthReceipt binds specific verified image, policy, bounded essential check set, issuer and boot generation; cannot promise future liveness. |
| TRUST-19 | BootSlotStateWriter, independent Good witness and security FloorWriter are separate typed scoped authorities. |
| TRUST-20 | Persistent trial retry budget survives normal reboot or fails boundedly; a BootEpoch-local RAM retry counter never substitutes. |
| TRUST-21 | Security floor is monotonic under declared device threat model, not replayable by restoring a signed old GPT/DataVolume record. |
| TRUST-22 | Floor promotion is gated by actual authorized readable bootable alternative/rescue and compatible keys/schema, not rescue hash alone. |
| TRUST-23 | Every permitted crash cut among slot, floor, attempt, boot-good and data migration records has a valid policy-defined outcome or explicit failure. |
| TRUST-24 | Independent UEFI variable, TPM NV and disk records are not claimed to be one atomic transaction. |
| TRUST-25 | TPM PCR measurement/event log or remote quote alone does not grant execution permission or certify bytes never measured. |
| TRUST-26 | Attestation freshness and expected reference policy are validated separately from signer enforcement; optional privacy controls apply. |
| TRUST-27 | TPM presence does not imply available protected monotonic counter, no NV errors, or universal key unsealing after firmware updates. |
| TRUST-28 | Encrypted DataVolume recovery depends on key/PCR/recovery material compatibility, not only filesystem format and signed image. |
| TRUST-29 | Valid A/B fallback code does not imply it can read user data migrated to a newer schema. |
| TRUST-30 | A security floor cannot be advanced solely because B is signed, downloaded, booted or a GUI process appeared. |
| TRUST-31 | Rescue on the same failed physical medium is not independent availability; advertised recovery scope names physical assumptions. |
| TRUST-32 | Unavailable rescue and no authorized image may end in RecoveryUnavailable or fail-stop; no silent auto-boot of revoked image. |
| TRUST-33 | Device DMA isolation/reset and storage Flush/FUA truth remain external required gates to boot image immutability/durability. |
| TRUST-34 | Boot/status/attestation report names effective enforcement, issuer, checked artifact/version and unverified hardware dependencies, not vague Secure labels. |
| TRUST-35 | User/guest UID0, network package path and unsigned disk metadata cannot write firmware keys, Root, Floor or foreign slot state without explicit native authority. |
| TRUST-36 | Key compromise/revocation and rotation cannot silently orphan every eligible rescue without approved fail/recovery policy. |
| TRUST-37 | Recovery/developer override never silently retains ProductionVerified mode or promises data unseal when policy changed. |
| TRUST-38 | A future TPM/UEFI/EFI/OEM profile is declared per actual PC; OVMF emulator behavior is not physical Reference PC evidence. |
| TRUST-39 | Signing code, measuring PCR, marking slot Good, and reaching storage durability are separate proof obligations. |
| TRUST-40 | No detailed TRUST decision is Accepted and no real key/firmware/boot implementation is authorized by this RFC. |

## 14. Architecture choices and trade-offs

| Option | Possible benefit | TCB/correctness risks | Current recommendation |
|---|---|---|---|
| TRUST-A independently verified loader + TUF-inspired delivery + A/B and optional TPM | limits release-role compromise, offline boot, explicit recovery and floor | largest integration graph; complex crash cuts and key management | first research target, unverified |
| TRUST-B local signed update without repo role separation | smaller release infrastructure | distribution freeze/compromised online key consequences | low-complexity comparison only |
| TRUST-C firmware/OEM trust-rooted selected PC | narrower first physical test set | OEM dependency, key enrollment/boot coexistence, limited rescue control | alternative for a fixed supported device |
| TRUST-D unlocked/dev keys | rapid VM research | not anti-rollback/protected user data; no production assurance | explicit Development mode, not substitute |
| No hardware floor | boot code signatures possible | signed vulnerable old code still eligible under disk replay attacker | visibly weaker claim or refuse ProductionVerified floor |
| TPM PCR-sealed DataVolume key | conditional hardware binding | legitimate update/repair may lock out users; key recovery needed | optional future independent user-data policy |

A specific algorithm or role threshold must not be selected solely by mirroring another OS. Release chain size, reproducibility, signer custody, CPU/boot performance, firmware compatibility and physical PC TPM/NV constraints need feasibility evidence.

## 15. Open design blockers O17-01…O17-36

| ID | Priority | Question and proof gap | Co-review |
|---|---|---|---|
| O17-01 | P0 | Real UniOS Root trust anchor independent from attacker-controlled manifest and EFI signer scope? | 001/006/017 |
| O17-02 | P0 | Loader vs resident M0 parser, canonical signed schema and bounded kernel bootstrap descriptor? | 001/006/017/018 |
| O17-03 | P0 | Use-time immutable image/pages and writable alias/DMA protection? | 004/007/008/017 |
| O17-04 | P0 | Actual physical firmware Secure Boot state, owner PK/KEK/db/dbx and enrolled signing route? | 006/017 |
| O17-05 | P0 | Hardware-backed anti-rollback floor store exists with exact auth/monotonic behavior on reference PC? | 006/017 |
| O17-06 | P0 | Security version index vector, component signer/key epoch and release schema compatibility? | 006/017/018 |
| O17-07 | P0 | A/B trial Good witness independence, exact policy and self-report forgery defense? | 005/006/017 |
| O17-08 | P0 | Durable slot state, persistent attempts and interruption between distinct storage stores? | 006/008/017 |
| O17-09 | P0 | Irreversible floor promotion without stranding last physically eligible rescue? | 006/008/017 |
| O17-10 | P0 | Independent rescue exact bytes readable, usable drivers and hardware boot path? | 006/007/017 |
| O17-11 | P0 | User DataVolume schema/key/PCR recovery after B trial changes and A rollback? | 006/008/017 |
| O17-12 | P0 | Firmware dbx/key revocation and installed other OS/USB rescue compatibility? | 017 |
| O17-13 | P0 | TPM NV absent/full/worn/replaced or EFI variable unsupported: safe production policy? | 006/008/017 |
| O17-14 | P0 | Key role compromise, emergency Root rotation and valid successor rescue? | 015/017 |
| O17-15 | P0 | Root/Targets signing and online metadata threshold custody/legal operational feasibility? | 015/017 |
| O17-16 | P0 | Trusted time and offline update freshness/expiry without bricking authorized old image? | 017 |
| O17-17 | P0 | Malicious local disk rollback of signed old floor or BootGood record bypassing verification? | 008/017 |
| O17-18 | P0 | Multi-store crash cut includes partial secure-floor update after Good but before new rescue? | 006/008/017 |
| O17-19 | P0 | Dev vs product trust mode effective firmware enforcement, visible status, user-data unlock? | 006/016/017 |
| O17-20 | P0 | Physical platform pre-boot DMA/firmware authority assumptions for signed image immutability? | 006/007/017 |
| O17-21 | P1 | TUF threshold, root rotation and timestamp/snapshot metadata version storage format? | 017/018 |
| O17-22 | P1 | TUF freeze and fast-forward handling under failed RTC, offline mirror and revoked signer? | 017 |
| O17-23 | P1 | Signed rescue semantics when root revoked/floor raised and recovery is read-only only? | 006/008/017 |
| O17-24 | P1 | Remote attestation verifier nonce/log/reference/TPM AK handling and consent? | 015/016/017 |
| O17-25 | P1 | PCR bank/platform measurements and sealing policy changes across OS/firmware updates? | 016/017 |
| O17-26 | P1 | User data recovery key escrow/local-only/backup and consent after hardware replacement? | 016/017 |
| O17-27 | P1 | Partial image/package hash/hardware compatibility and decompression parser bounds? | 006/017/018 |
| O17-28 | P1 | A/B image vs package/component update version coupling, drivers and ABI migration? | 006/011/017 |
| O17-29 | P1 | Root key lifecycle release build supply-chain and transparency evidence? | 015/017 |
| O17-30 | P1 | Firmware EFI runtime variables reset/power-cut effect and write budget? | 006/008/017 |
| O17-31 | P1 | Snapshot merge first-stage daemon boot dependency and crash recovery if selected? | 004/006/008 |
| O17-32 | P1 | Persistent attempt budget tamper/availability when media state and NV writes diverge? | 006/017 |
| O17-33 | P1 | Error/repair UX when no display/keyboard/storage works and all slots rejected? | 006/016/017 |
| O17-34 | P1 | Stable native trust report/boot success/floor control errors and versioned manifest ABI? | 009/017/018 |
| O17-35 | P2 | Reproducible build provenance, signing transparency, managed-device attestation policy? | 015/017 |
| O17-36 | P2 | Firmware capsule updates and Secure Boot CA future rotation on broader PC catalog? | 007/017 |

## 16. Proposed decision register TRUST-D01…TRUST-D22 — NONE Accepted

- **TRUST-D01:** TRUST-A independently rooted loader validation, TUF-inspired update roles, scoped A/B Good, physically available rescue and conditional protected floor is first research baseline.
- **TRUST-D02:** UEFI firmware signer trust, UniOS release Root, measured boot and version floor are different objects and permissions.
- **TRUST-D03:** Loader/M0 verifies kernel/BootBundle/immutable image against independently trusted manifest/root; kernel consumes bounded typed grants.
- **TRUST-D04:** A signed BootPolicyManifest cannot self-authorize a signer, floor downgrade, firmware key enrollment or global grant expansion.
- **TRUST-D05:** Use-time immutable bytes and all demand-paged executable data require authenticated content lineage and protected backing.
- **TRUST-D06:** TUF Root/Targets/Snapshot/Timestamp threshold separation and metadata freshness are delivery research targets, not accepted implementation or automatic spec compliance.
- **TRUST-D07:** Local authorized offline boot and checking freshness of a new online update have explicitly different clock/network requirements.
- **TRUST-D08:** A/B slot flags, durable retry count, independent HealthReceipt, authenticated BootGood and monotonic FloorWriter are separate state/authority.
- **TRUST-D09:** New floor promotion is not triggered before B is durably Good and a physically accessible, post-floor eligible recovery path is verified including data/key compatibility.
- **TRUST-D10:** Signed but missing/incompatible rescue or bootloader revoked in dbx is not an eligible safety fallback.
- **TRUST-D11:** No claim that EFI per-variable writes, TPM NV floor and DataVolume schema/slot records form one atomic transaction.
- **TRUST-D12:** With no suitable protected monotonic floor medium, strict anti-rollback claim must be denied, although an explicit lesser DevSigned profile may boot.
- **TRUST-D13:** Default consumer research baseline does not require remote attestation or online time to launch an already authorized local image.
- **TRUST-D14:** Measured TPM PCRs and quote are evidence, not signature enforcement; remote verifier and consent required before attestation trust.
- **TRUST-D15:** PCR-bound user-data key access requires separate recovery material and update/firmware migration; signed fallback alone is insufficient.
- **TRUST-D16:** Firmware dbx/PK/KEK changes are independent owner-approved hardware-sensitive operations, not routine OS package actions.
- **TRUST-D17:** Revoked/compromised Root or Targets signer triggers explicit incident/key rotation, no false assurance of self-repair under stolen Root quorum.
- **TRUST-D18:** Startup Good receipts are bounded observations of essential services, not future success or durable user data evidence.
- **TRUST-D19:** Boot attempt budgets must persist across reboot or fail boundedly, not be reset by BootEpoch alone.
- **TRUST-D20:** Product trust mode distinguishes unlocked, developer-signed, unsupported hardware, production candidate and future verified chain.
- **TRUST-D21:** Every permitted crash cut across image staging, slot, floor, rescue and DataVolume migration must have an explicit safe outcome or documented fail-stop.
- **TRUST-D22:** No implementation, key operation, TPM/EFI write, physical boot experiment or crypto security certification authorized before open P0 gates close.

## 17. Cross-RFC correctness and authority integration

| RFC | Linked proof obligation |
|---|---|
| RFC-001 | actual kernel/loader/manifest verifier trusted computing base, asset-specific firmware/platform trust |
| RFC-002 | signer policy cannot mint arbitrary caps; issuer/revocation/sponsor and bootstrap grant ceilings |
| RFC-003 | health/BootGood messages cannot be forged by ServiceID or ordinary IPC COMMITTED; timeout/UNKNOWN |
| RFC-004 | signed immutable page root and PagerSupply use-time content identity, wired M0 bootstrap |
| RFC-005 | finite CPU/IRQ for trusted HealthWitness and M0; watchdog not proof of successful boot |
| RFC-006 | BOOT-A root trust, RA-2, A/B slot ledger, GoodReceipt, floor-safe rescue and persistent attempt budget |
| RFC-007 | pre-IOMMU DMA, trusted device mapping/reset and physical rescue availability |
| RFC-008 | Flush/FUA honest slot metadata/image persistence, user data V1/V2/key, snapshot replay and cross-device crash states |
| RFC-009 | typed native trust report/health status/floor metadata boundaries; not yet ABI |
| RFC-010 | guest Linux UID0 and raw EFI/device calls do not become native root capabilities |
| RFC-011 | native package/update Target roles and versioned installed app/service signing policies |
| RFC-015 | crypto parser fuzz, boot root/role threat model, signature-key security, model-check/fault injection |
| RFC-016 | user owner consent for firmware keys, unlock, recovery key/attestation privacy |
| RFC-018 | signed manifest canonical schema, version vectors, service/slot/boot epoch and status encoding |

**P0 co-review clusters:** (006+008+017) authenticated/ durable slot Good vs floor + media + rescue/data schema; (001+002+006+017) loader manifest verification, kernel grant attenuation; (004+007+017) verified file bytes vs DMA/writable alias; (015+017) root compromise, signed release, trusted time and algorithm assurance. These cannot be signed off by calling one layer "secure boot."

## 18. Research exit gates and refusal criteria

**R1 trust-chain design exit: NOT PASSED.** Before Accepted detailed spec or product claims:
1. Specify exact independent trust roots and firmware UEFI mode plus owned vs third-party PK/KEK/db/dbx on named hardware.
2. Show loader/M0 verification chain, typed kernel bootstrap descriptor, canonical signed manifest and immutable/read-time SystemImage proof.
3. Model TUF signature thresholds, Root replacement, Snapshot/Targets consistency, Timestamp expiration and offline bad RTC.
4. Determine actual selected hardware TPM NV/UEFI authenticated variable semantics or explicitly unsupported anti-rollback profile.
5. Enumerate all power-cut transitions from B staged image to B Good and F→F', including independent variables/TPM/media and failed writes.
6. Prove every irreversible floor/key revocation leaves an actually readable, signer/floor-eligible, device-bootable and data-schema/key-compatible recovery path, or abort promotion.
7. Produce independently authorized essential-health/BootGood issuer threat analysis and finite attempts across reboot.
8. Specify PCR/event log/quote/measurement scope and user data seal/recovery migration without promising universal TPM access.
9. Contrast TRUST-A/B/C/D cost, TCB, platform lock-in, developer UX, key custody, user consent and offline fallback on identical threat model.
10. After an explicit later implementation authorization, test safe QEMU/OVMF simulated modes and one *sacrificial reference PC* without touching existing personal boot keys or user disk, then independently evaluate key/TPM operations.

**Explicit rejection outcomes:** no working trusted verifier, no robust floor store on strict-profile PC, no eligible alternate after promotion, unavailable physical rescue, inability to recover DataVolume keys/schema, insecure loader rollback, unbounded persistent reboot loops, or fundamental platform DMA compromise -> decline ProductionVerified/anti-rollback or return RecoveryUnavailable/FailStop. A partial document does not grant go-live.

**Suggested next design step:** independent adversarial review RFC-017 v0.1→v0.2, then RFC-009 Native KABI and RFC-018 versioned status/canonical manifest bindings. The eight D1–D8 direction decisions remain Accepted only at top-level, not this proposal.

## 19. Official first-party references (reviewed 2026-10-24)

- **[S1] UEFI 2.11 Boot Manager, PK/KEK/db/dbx and Secure Boot state:** https://uefi.org/specs/UEFI/2.11/03_Boot_Manager.html
- **[S2] UEFI 2.11 Runtime Variable Services and authenticated variable semantics:** https://uefi.org/specs/UEFI/2.11/08_Services_Runtime_Services.html
- **[S3] The Update Framework Specification v1.0.36:** https://theupdateframework.github.io/specification/v1.0.36/
- **[S4] TUF required Roles and Metadata:** https://theupdateframework.io/docs/metadata/
- **[S5] Android Verified Boot 2.0 (official source):** https://android.googlesource.com/platform/external/avb/+/master/README.md
- **[S6] Android A/B system update slot/trial controls:** https://source.android.com/docs/core/ota/ab
- **[S7] Android current Virtual A/B overview:** https://source.android.com/docs/core/ota/virtual_ab
- **[S8] TCG PC Client Reference Integrity Manifest and TPM PCR log context:** https://www.trustedcomputinggroup.org/resources/documents/tcg-pc-client-reference-integrity-manifest-specification
- **[S9] TCG TPM 2.0 Library latest specs and errata:** https://www.trustedcomputinggroup.org/resources/documents/tpm-library-specification
- **[S10] Microsoft How Windows Uses TPM (PCR and key unlock):** https://learn.microsoft.com/en-us/windows/security/hardware-security/tpm/how-windows-uses-the-tpm
- **[S11] Microsoft Secure Boot PK/KEK/db/dbx explanation:** https://learn.microsoft.com/en/windows-hardware/design/device-experiences/oem-secure-boot
- **[S12] TUF key security and expiry FAQ:** https://theupdateframework.io/docs/faq/
- **[S13] Linux fs-verity authenticated read-only file backing:** https://docs.kernel.org/filesystems/fsverity.html
- **[S14] Linux dm-verity read-only block backing:** https://docs.kernel.org/admin-guide/device-mapper/verity.html
- **[S15] Linux volatile block writeback cache/Flush/FUA:** https://docs.kernel.org/block/writeback_cache_control.html
- **[S16] Microsoft 2026 UEFI Secure Boot key rotation and coexistence guidance:** https://learn.microsoft.com/en-us/windows-hardware/manufacture/desktop/windows-secure-boot-key-creation-and-management-guidance
- **[S17] TUF specification threat defenses (rollback/freeze/mix-and-match):** https://theupdateframework.github.io/specification/v1.0.36/

Third-party sources are technical comparisons and not proof of UniOS correctness or an authorization to import code, root certificates or license-restricted implementations. Actual UEFI/TPM functionality depends on a selected hardware/firmware configuration; none was tested.

---

**Document history:** 2026-10-24 — RFC-017 Proposed Specification v0.1 initial design research draft, no implementation/firmware enrollment/TPM write/signing ceremony/boot experiment/benchmark/formal model.

# UniOS RFC-017 — Verified Boot, Updates & Rollback Protection

> **Status:** Proposed — Specification v0.1 (design research only; not Accepted, implemented, benchmarked, independently audited or formally verified)
> **Date:** 2026-10-24 NZDT
> **Architecture:** [UniOS v0.2](../architecture/unios-architecture-v0.2.md) — D7 accepted as *direction*, this RFC NOT accepted.
> **Cross-RFC:** [RFC-002 Capabilities](rfc-002-object-capability-lifetime-delegation-revocation.md), [RFC-003 IPC](rfc-003-ipc-wire-protocol-and-synchronization.md), [RFC-004 VM/Pager](rfc-004-virtual-memory-pager-cache-and-oom.md), [RFC-005 CPU/IRQ](rfc-005-scheduling-preemption-and-cpu-budget.md), [RFC-006 Boot/Recovery](rfc-006-boot-trust-chain-service-startup-and-recovery.md), [RFC-007 DMA](rfc-007-user-space-drivers-dma-and-iommu.md), [RFC-008 Storage](rfc-008-storage-filesystems-and-crash-durability.md).
> **Never authorize code/hardware operations:** no new kernel/UEFI loader, TPM NV/floor writer, key generation/enrollment, production signing or root rotation, GPT/UEFI/firmware modifications, actual update staging, real-media tests or hardware resets.

## 0. Executive recommendation — TRUST-A as conditional research baseline

**TRUST-A:** firmware UEFI-verified minimal EFI loader -> independently rooted UniOS BootPolicyManifest and exact kernel/BootBundle/M0 verification -> kernel-enforced small typed bootstrap grants -> runtime immutable SystemImage authentication -> TUF-inspired signed update repository with distinct Root/Targets/Snapshot/Timestamp roles -> crash-consistent A/B trial plus physically verified rescue -> monotonic hardware-policy security floor raised ONLY after independent healthy-success and verified usable rescue conditions. TPM measured boot is optional evidence, not code admission or proof of safety.

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
| UEFI 2.11 Runtime Variable Services [S2] | nonvolatile variable atomic old/new behavior *per variable*; capacity limits and unsupported runtime operations | no cross-variable transaction for slot+TPM floor+disk, no unlimited retry counters |
| TUF v1.0.36 [S3] | root rotation by signed threshold, targets hash/length, snapshot consistent metadata, timestamp and expiry checks | TUF-inspired delivery; spec conformance not asserted |
| TUF roles and metadata [S4] | Root, Targets, Snapshot, Timestamp have different signing roles and risk | online Timestamp compromise does not automatically mint boot Root |
| Android Verified Boot 2.0 [S5] | rollback-index locations in tamper-evident storage; for A/B update floor from slots marked SUCCESSFUL only | floor promotion gate plus stronger UniOS rescue/data conditions |
| Android A/B OTA [S6] | active, bootable, successful and bounded trial retries separate | old signed slot not necessarily allowed/usable or DataVolume-readable |
| Android Virtual A/B [S7] | snapshot merge may resume after restart; early snapuserd boot dependency | storage merge/boot critical resources only if a comparable scheme selected |
| TCG PC Client RIM/firmware profile [S8] | PCR measurement log needs verifier reference information and quote validation | measurements and enforcement distinct; optional privacy-scoped attestation |
| TPM 2.0 Library and errata [S9] | PCR/NV/authorization policies are actual TPM facilities with platform-specific configuration | TPM presence not proof of usable monotonic NV floor or sealed key recovery |
| Windows TPM data protection [S10] | PCR/boot-policy changes can interrupt transparent key unseal and force recovery | explicit user-data recovery key, profile migration, consent |
| Microsoft UEFI Secure Boot keys [S11] | firmware allow/revoke databases, dbx precedence and OEM ownership | firmware signer policy independent from UniOS signed targets |
| TUF FAQ [S12] | Root/Targets generally offline, Timestamp often online; metadata freshness/expiry matter | trusted time problem and offline update limits |
| Linux fs-verity / dm-verity [S13,S14] | immutable pages/blocks verified against authenticated content roots | read-time verification of later pager-backed code, not data durability |
| Linux block cache barriers [S15] | device completion differs from persistent Flush/FUA | RFC-008 crash-safe image and boot metadata stage protocol |
| Microsoft UEFI signing key rotation guidance [S16] | real PC PK/KEK/db CA replacement has compatibility and recovery impact | never enroll/reset another OS keys or silently trust arbitrary EFI loader |

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

- DevUnsigned (unlocked/offline VM lab): no production verified/rollback claims, no silent reuse of production sealed data keys.
- DevSigned (developer key/test OVMF): only a controlled local test signer is validated; cannot state real OEM chain or physical NV floor verified.
- ProductionCandidate (not a user-facing green state): enough design evidence to plan controlled real-hardware security test; still not verified.
- ProductionVerified (future only): effective firmware mode and complete UniOS chain/floor/immutable runtime proven on named hardware and software versions.
- RejectedUntrusted/RecoveryOnly/HardwareUnsupported: fail closed or explicitly restricted access; never show the same security assurance as ProductionVerified.

A user-authorized owner unlock/debug transition may legitimately change trust; it requires unmistakable disclosure, bounded scope, revocable device privileges, user-data recovery/key impacts and a distinct boot status. No trust promise on a user-managed firmware that is physically switched into non-enforcing mode.

## 4. Signed update delivery vs local boot policy

### 4.1 TUF-inspired role separation

Root is a rarely rotated threshold signer controlling keys and role policy. Targets signs exact bytes/hash/length and release constraints. Snapshot signs consistent metadata sets, stopping arbitrary mix-and-match of different target revisions. Timestamp signs the newest referenced Snapshot and expiration, defending against indefinite freeze only if client has trustworthy freshness/time policy. Delegation must be scope-limited and cannot increase parent authority. TUF v1.0.36 is a specific formal update standard; this RFC does not assert conformance without implementing and testing its exact metadata and refresh procedures.

A compromise of an online Timestamp key cannot mint a new Boot Root or a new kernel target if the other roles remain secure. A Targets key compromise can authorize malicious release artifacts within its scope (serious!); Root threshold compromise can authorize new keys and is a critical incident. A signature is not a quality audit or transparency guarantee. Treat publication/CI/role key custody/reproducible builds as independent supply-chain trust decisions.

### 4.2 Rollback, freeze, fast-forward, mix-and-match

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

A normal GPT/ESP/DataVolume file, including a signed old file, is replayable by a disk attacker. Candidate storage includes a specifically authorized TPM 2.0 NV counter/record, platform-protected signed EFI variable, RPMB or independently trusted external anchor. **TPM existence is not proof of an available usable monotonic index; firmware authenticated variables are not necessarily a trusted nondecreasing counter.** The UEFI spec [S2] specifies per-variable old/new persistence under failure, limited NV space, and runtime Unsupported/OutOfResources; it does not promise one atomic transaction across separate slot/floor/data stores.

### 6.1 Safe floor promotion predicate

For a candidate new floor F', require all of the following: actual stored F <= F'; appropriate independently authenticated policy grants FloorWriter; B's exact signed image/manifest/hardware profile is verified and a separate authorized health witness generated a durable Good receipt; at least one image other than revoked old A remains *physically present, readable, signer/floor-eligible and bootable*, including available device drivers and DataVolume schema/key path; firmware dbx or key rotation will not invalidate that image; rollback-index domains can be updated with recoverable intermediate states; read-back validates actual stored floor. If these cannot be established, deny irreversible floor promotion, enter explicit rescue/degraded mode or fail-stop as appropriate. Health checks cannot promise indefinite future physical availability.

### 6.2 Alternative trusted stores and failure modes

| Candidate | Possible benefit | Limit |
|---|---|---|
| TPM 2.0 NV protected index | may resist disk rollback when correct auth and anti-replay semantics verified | NV availability, platform firmware, wear/locking, ownership loss, TPM replacement |
| Authenticated UEFI variable | firmware policy and per-variable durable write semantics | not generalized monotonic floor, tiny storage, firmware/runtime failure |
| RPMB/secure flash | device-backed anti-replay on selected hardware | not universal x86_64 PC capability |
| Independent online verifier | remote latest-state/freshness anchor | no offline liveness guarantee |
| Plain GPT/file journal | crash consistency for honest storage | attacker can restore previous signed data/floor |
| No suitable protected medium | development or explicitly reduced-security signed boot possible | **no production strong anti-rollback claim** |

The floor counter, authenticated Good state, slot retry record, bootable rescue and mutable user-data schema may inhabit **independent nontransactional devices**. Individual atomic writes do not make cross-device transactions atomic. An irreversible-last stage and explicit recovery from every interrupted phase are proof obligations, not implementation details.

## 7. Key lifecycle, compromise and revoked loaders

Offline Root role controls signing thresholds and release-role keys; Targets signs exact image bytes/hardware constraints, Snapshot signs metadata consistency, online Timestamp signs frequently refreshed freshness. Root rotation needs authorization by previously trusted Root and new Root per chosen standard, not just a signed manifest telling clients to accept a new untrusted key. Role threshold is effective only if signing custodians and private keys are truly independent.

If CDN or Timestamp private key is compromised, valid independent Targets/Root policy should continue to reject malicious new code (not necessarily denial/freeze). If Targets signing key is compromised, attacker may legitimately sign malicious code within the role scope; rebuild, independent Root revocation and physical rescue may be needed. If Root quorum is compromised, purely in-band software update may be unable to re-establish trust. Correct signatures do not prove the signed binary's safety or uncompromised build.

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


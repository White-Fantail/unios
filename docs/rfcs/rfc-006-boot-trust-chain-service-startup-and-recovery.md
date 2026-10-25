# UniOS RFC-006 — Boot Trust Chain, Service Startup & Recovery

> **Status:** Proposed — Specification v0.2 (adversarial architecture review, 미승인·미검증)
> **Date:** 2026-10-24 (NZDT)
> **Baselines:** [Architecture v0.2](../architecture/unios-architecture-v0.2.md), [RFC-001 v0.2](rfc-001-kernel-boundary-and-tcb.md), [RFC-002 v0.2](rfc-002-object-capability-lifetime-delegation-revocation.md), [RFC-003 v0.2](rfc-003-ipc-wire-protocol-and-synchronization.md), [RFC-004 v0.2](rfc-004-virtual-memory-pager-cache-and-oom.md).
> **Constraint:** Research and specification only. No executable loader, OS implementation, Secure Boot enrollment, tested recovery, new ABI, benchmark, formal proof, Accepted detailed design or approval to implement.

## 0. Executive conclusion — proposed BOOT-A

**BOOT-A: signed/verified minimal BootBundle + kernel-supplied typed root grants + independently resident minimal RecoveryAuthority + manifest-constrained normal ServiceSupervisor.**

The boot trust chain and the operating-system recovery chain have **different failure assumptions**. A signed UEFI executable does not verify all subsequently loaded kernel, M0 services or disk data. A crashed ordinary ServiceSupervisor cannot be trusted to restart itself. A running Pager does not imply storage durability or full memory-service recovery. A kernel safely denying unsafe operations does not imply desktop availability. These five non-equivalences are the central contract of this RFC.

**v0.2 검토:** M0 RecoveryAuthority 자체의 실패 시 RA-2 fail-stop/검증된 복구 부팅을 보수적 첫 대안으로 변경한다. 커널 직접 재실행 RA-1은 추가 privileged image loader와 권한 재발급 TCB가 필요하므로 미검증 연구안으로만 유지. EFI watchdog 단절, A/B floor·실제 rescue, reboot loop 및 verification-parser 권한 경계를 추가한다.

BOOT-A is a *conditional comparison hypothesis*, not final adoption. Keep BOOT-B (one root-task PID1-style privileged supervisor plus explicit fail-stop/reboot) and BOOT-C (broader kernel-owned service launcher/recovery) active. If a separate RecoveryAuthority merely duplicates an omnipotent supervisor, BOOT-A might **increase** the effective TCB instead of reducing it.

**Early M0 profile:** A small verified bootstrap artifact resides fully in memory. Core kernel, boot authority, critical stacks/page tables, IPC/IRQ/timer/watchdog and bounded teardown/recovery resources operate independently of file Pager, VFS, network and normal app resource pools (RFC-004). Any promise about M0 must cite finite admission, storage independence and error paths; merely stating that something is "wired" is not a proof.

**Four distinct axes:**
- Boot admission/verification: did this image meet firmware and UniOS image/auth version policy?
- Startup/availability: can verified essential services actually provide usable IPC and file faults?
- Update slot/rollback: is this boot trial accepted, is fallback still allowed by security floor and data format?
- Data protection: are dirty pages, pending storage writes, DMA and user files actually durable/safe?

D1–D8 Accepted Direction and previous Proposed RFCs are unchanged.

## 1. Goals, non-goals and threat model

**Goals:** explicit verified-image chain; least-privilege boot resource handoff; manifest-scoped service launch; typed object/sponsor/revocation lifecycle; pager-independent M0, multiple service criticality levels; bounded restart/no reboot loop; crash-safe conceptual A/B trial and anti-rollback policy; reproducible and auditable failure/health states.

**Non-goals:** functioning EFI loader, production Secure Boot key or TPM policy; signing/key ceremonies, verified A/B code, supported suspend/hibernate, general device drivers, guaranteed recovery of all GUI apps, auto-healing firmware, clean shutdown of every dirty data write, final KABI/IDL, stable package format or verified real-device hardware. RFC-017 owns full cryptographic trust/update model; RFC-008 owns filesystem durability; RFC-007 owns physical DMA resets; no code implementation is requested.

**Adversaries:** attacker-controlled boot media/filesystem or EFI boot arguments, unsigned/old kernel, compromised normal supervisor/service/compatibility layer, malicious Pager and driver, forged manifest and ServiceID, stale reply/fault after restart, kernel resource exhaustion, incomplete memory/DMA fence, update power cuts and tampered slot metadata. Compromised firmware/platform keys, malicious CPU/device firmware or physical tampering are external trust assumptions that ordinary UniOS services cannot repair.

## 2. Primary-source architecture comparison

| Primary source | Existing-system design fact | UniOS inference / limit |
|---|---|---|
| UEFI 2.11 Secure Boot [S1] | Firmware authenticates EFI images using platform signature policy, setup/user/audit/deployed modes and db/dbx | EFI loader acceptance **does not** verify all kernel/M0/userspace data or ensure anti-rollback |
| UEFI Boot Services [S2] | GetMemoryMap current MapKey and descriptor size/version, ExitBootServices terminates boot services after success | kernel must not invoke boot services after exit; retry rules and RAM handoff are critical |
| seL4 capabilities/untyped [S3,S4] | initial root task receives bootstrap capabilities and resource descriptions | typed first-process grants can be explicit, but indefinitely privileged runtime root is not required |
| Fuchsia userboot/bootfs [S5] | first userspace loader gets handles/ZBI and loads first process from BOOTFS | independent resident BootBundle avoids mandatory early file Pager |
| Fuchsia early component manager [S6] | root-job critical component manager failure may trigger reboot in the published guide | **guide itself warns it can be stale**; treat as precedent rather than a current validated deployment guarantee |
| Android legacy A/B updates [S7,S8] | retry/slot-success metadata distinct from trial | Modern Android Virtual A/B uses dynamic partition snapshots [S14]; UniOS does not copy Android's partition model |
| Android Verified Boot [S9] | A/B rollback metadata must not advance before new slot is marked successful | wrong ordering can make the old valid slot forbidden, stranding device |
| Chromium OS verified boot/recovery [S10,S11] | verified firmware/OS and separate recovery workflows | recovery artifact requires independent authorization, not merely older copy |
| TCG PC Client RIM [S12] and Firmware Profile [S15] | RIM supplies reference integrity information; Firmware Profile defines PCR/event logs | neither constitutes code-execution authorization by itself |
| UEFI runtime variable service [S13] | runtime variables may be unsupported/error/resource-limited and access is platform-dependent | NVRAM is not infinite transactional durable storage |

Do not equate source architecture claims with UniOS implementation, formal proof or acquired legal rights to source code.

## 3. Trust graph, root keys and boot mode

~~~text
Hardware, firmware, CPU and UEFI implementation: EXTERNAL PLATFORM ASSUMPTION
  -> Firmware-authorized EFI loader (PK/KEK/db/dbx mode and policy)
  -> Loader-verified kernel, minimal BootBundle and trusted manifest
  -> Kernel validates BootContext; constructs minimal typed object authority
  -> M0 wired RecoveryAuthority and admission/failure path
  -> Manifest-scoped normal ServiceSupervisor and resource/device authorities
  -> Authorized Pager, storage/driver, normal desktop services
  -> App-specific grants and compatibility sandboxes (not root privilege)
~~~

A "signed image" is not synonymous with a "safe image"; the verified signature must bind exact bytes, trusted signer and accepted version/security floor under a **verified verifier**. UEFI may authorize a loader but cannot be assumed to authenticate the kernel/bootfs that loader reads later. A manifest's hash field without an authenticated chain is untrusted text. **Trust cycle forbidden:** the manifest being verified cannot unilaterally choose trusted verifier keys or relax the independent version floor.

### 3.1 Explicit boot trust modes

| Mode | Required meaning | Disallowed claim |
|---|---|---|
| DevUnsigned | local emulator/unlocked experimentation, optional disabled firmware enforcement | production secure boot |
| DevSigned | developer/test key used with explicitly chosen local artifacts | production release or user-data trust |
| ProductionVerified (future only) | all required firmware, loader, kernel, BootBundle, floor and platform gates verified | "verified" absent actual hardware and RFC-017 |
| RestrictedRecovery | separately authorized recovery image and limited repair rights, offline possible | normal desktop automatically trusted |
| RejectedUntrusted | required verification failed, no ordinary production boot | silently fallback to unsigned image |

No automatic developer-key-to-production-key promotion. Whether an authenticated user can deliberately disable production policy, and whether that should invalidate sealed data access, belongs to RFC-016/017; visible mode is a security requirement, not an app decoration.

### 3.2 Verification, measurement and freshness are separate

Secure Boot/enforcement decides *may run*, TPM PCR/event log records *what was measured* on a certain boot, remote attestation attempts to prove that measurement to a verifier with fresh quote, and anti-rollback enforces *minimum allowed old version*. A valid signature alone cannot reject a once-valid compromised old build. Measured boot without enforcement can faithfully record execution of unauthorized code. Offline M0 cannot rely on remote attestation network.

## 4. BootBundle and signed manifest contract

**BootBundle (conceptual):** trusted kernel image/entry and minimal M0 RecoveryAuthority executable, immutable/resident early code+data, validated boot configuration, bounded resource budget plan, optional verified recovery metadata. Whether these are one signed EFI binary or multiple independently signed pieces is open and must not be guessed from UEFI acceptance.

**BootPolicyManifest (conceptual, not ABI):** schema version, image digest and signer/key identity, kernel ABI compatibility, release version and security floor, hardware/CPU/driver requirements, ServiceID/image digest and dependency types, initial resource/capability envelope, allowed device groups, service readiness and criticality, restart attempts/windows, allowed recovery/slot relationships and signed policy extension. All required fields must have strict parser behavior for duplicates/overflow/unknown required versions. **Complex manifest parser location is unresolved P0:** prefer a trusted loader or verified M0 policy interpreter with a minimal fixed typed kernel bootstrap descriptor over silently installing full schema/cryptography policy into Ring 0; compare actual code/authority trust.

**Integrity rule:** the kernel/boot validator must not consume an attacker-replaceable policy file *after* trusting it simply because path/name matches. Signed manifest commits to service image contents or immutable verified backing; writable disk path alone cannot be a trust identity. Runtime executable file faults remain a verification issue for RFC-004/017.

**Use-time identity:** verified bootstrap/code bytes must stay immutable or be reauthenticated on use; a writable alias, untrusted DMA, changed backing file or recycled loader buffer can invalidate a correct earlier hash check.

**Policy rule:** an authorized image may still be malformed, compromised by a legitimate signing key or unsafe for a particular hardware profile. Use bounded parser and least-grant policy, no ambient DMA, debug or global TaskControl from a manifest string. Do not freeze hash algorithm, descriptor size or signature wire layout until RFC-017/018.

## 5. EFI to kernel handoff (x86_64 first)

The firmware/UEFI layer is outside UniOS privileged kernel but within the boot trust dependency. An EFI image's successful load does not make all physical memory immediately owned by UniOS.

**BootContext conceptual fields:** firmware memory map descriptors (including DescriptorSize and DescriptorVersion), kernel/resident image extents, reserved stack/page-table/critical allocations, validated system tables/ACPI information, selected boot slot and trust mode provenance, supported early console/GOP framebuffer, TPM/event-log pointer only if validated, boot epoch initialization inputs and explicit handoff-state flags.

### 5.1 GetMemoryMap, ExitBootServices and PMM

- Obtain latest memory map with current MapKey, DescriptorSize and DescriptorVersion; buffer resizing can itself change the map. **After the first ExitBootServices call, firmware may partially shut down**; subsequent Boot Services calls are limited by UEFI to Memory Allocation Services (including necessary GetMemoryMap). Retry must not call arbitrary console, filesystem, event or image protocols [S2].
- After successful ExitBootServices, UEFI Boot Services and their device protocols cannot be called. **The UEFI Boot Services watchdog is disabled after success**, so M0 timers and liveness need independent OS IRQ/timer or separately verified hardware watchdog, not the preboot watchdog. EFI Runtime Services are separately optional, with mapping/attributes/support/return failures defined, not a universal storage API [S2].
- Memory map may contain firmware reserved, runtime, boot-services, ACPI, MMIO and untested regions. Do not hand all physical addresses to a normal frame allocator. Preserve kernel, BootBundle, firmware runtime and M0 wired extents.
- Validate descriptor overlap, alignment, arithmetic overflow, range bounds, page sizes, physical page zeroing and trusted provenance before kernel stores memory objects.
- UEFI GOP output does not prove persistent GPU/USB/driver support. A physical Reference PC and UEFI configuration must later be tested independently of QEMU/OVMF.

**Security failure:** a forged boot memory descriptor or incorrect ExitBootServices assumptions can corrupt kernel memory before sandboxing exists. No capability mechanism repairs a malicious loader's already executed arbitrary code.

## 6. Initial kernel capabilities and authority partition

**BOOT-A proposal:** kernel creates explicitly typed, narrow first-process bootstrap grants and corresponding Budget/RevocationControllers; minimal wired RecoveryAuthority handles only essential service launch, error detection, bounded restart authority and protected resource budgets. A normal ServiceSupervisor manages broad ordinary service lifecycles but **is not boot root by default**.

| Actor | Candidate minimal rights | Forbidden automatic rights |
|---|---|---|
| Kernel | privileged CPU/PMM/MMU/IRQ enforcement, cap table, fault state, secure metadata | application file content parsing/desktop policy |
| M0 RecoveryAuthority | fixed verified essential image launch, limited Task restart, reserve allocation and critical error detection | arbitrary app debug, raw user files and global device ownership |
| Normal ServiceSupervisor | manifest-approved service start/stop and scoped admission, generation/registry update | unrestricted kernel root, all device groups, key-signing authority |
| Budget/ResourceIssuer | child budget issue/adopt and committed resource accounting | arbitrary existing object read or unbounded allocate |
| DeviceAuthority | named physical group, scoped IRQ/IOMMU map/stop/fence | untrusted driver arbitrary MMIO/whole-RAM DMA |
| Pager/StorageHost | bound MemoryObject/ranges and authorized backing storage access | arbitrary Task resume, foreign addressspace map |
| Registry/PolicyPortal | service discovery and app approved portal grants | root of trust via string lookup |
| Application/Guest | per-app memory and explicit service endpoints | kernel root or guest UID/Administrator to native grant |

A process being signed or marked "essential" does not grant all possible runtime capabilities. Object origin holder, grant issuer/controller, resource sponsor and live reference are distinct (RFC-002). Each service's grants derive from an allowed parent with rights/scope attenuation and explicit create authority plus budget for new objects.

**Trusted source boundary:** manifest signer, minimum security floor, selected slot, BootContext and root grant vector must have provenance independent from writable ServiceRegistry values. Normal Supervisor READY never upgrades boot signer policy. Kernel need not parse a rich signed ServiceManifest in the proposed small-TCB variant.

**Root lifecycle:** investigate disposing of broad boot-only rights after initial partition, yet retain precisely scoped restart/revocation/driver safety grants if actual recovery needs them. Do not claim the root has vanished while a hidden unrestricted service still exists. Even a tiny RecoveryAuthority remains TCB-B/TCB-S and is only worthwhile when reduced code and grant surface is real.

## 7. M0 resident cut and protected recovery budget

**M0 must start and respond to a bounded essential failure without normal file Pager, filesystem, network and optional application allocations.** It is a research goal, not an achieved feature. Code-only wiring is insufficient if stack, IPC message buffers, handler page tables, IRQ/timer path or cleanup nodes may fault or OOM.

**Symbolic reserve** (numbers require selected finite hardware/configuration):
- R_frames: fixed kernel and RecoveryAuthority resident code+data+stack, boot bundle and designated minimal early driver memory.
- R_pt: page table creation/splits/shootdown+per-core translation bookkeeping for recovery operations.
- R_ipc: message/Reply/Wait/exception/fault queues, handle slots, cap lineage and service-death notifications.
- R_irq: timer, essential IRQ/interrupt routing, watchdog plus device stopping path if required.
- R_cleanup: bounded nonrecursive cleanup after crashed Task, grant revocation, in-transit handle cycles, OOM accounting.
- R_diag: bounded diagnostic memory; serial/GOP only when hardware path verified, no durable disk assumption.

**Admissibility inequality:** reserved critical capacity is at least the maximal simultaneously admitted essential operations on the agreed CPU/service/Task/device profile. This sentence is a **proof obligation**, not a verified bound. Must define max task count, per-core reserve, recursion depth, simultaneous faults, queue capacity and teardown steps. When reserve itself runs out, halt/fail-stop deterministically without infinite kernel allocation.

### 7.1 Start DAG is not runtime wait-for DAG

~~~text
Possible boot: Kernel -> M0 RecoveryAuthority -> Storage/Driver -> Pager -> VFS -> App
Allowed only if early code/stack/IPC/IRQ is resident and credits are reserved.

Forbidden runtime cycles:
RecoveryAuthority -> normal Supervisor -> RecoveryAuthority
Pager -> VFS -> same blocked Pager worker
Pager -> ResourceAllocator -> Pager page-in
Kernel Copyout lock -> Pager IPC -> waiting same lock
StorageDriver handler -> page fault -> Pager -> blocked StorageDriver
OOM reclaim -> normal allocation -> OOM reclaim
~~~

Service-start order may be acyclic while its IPC queues are full or IRQ handler cannot run. Include *CPU runnable state, memory/page-fault dependency, queue credits, locks, timer progress and device interrupts* in runtime wait-for proof. RFC-003/004/005/006 joint audit is mandatory.

### 7.2 Fail-stop is preferable to false recovery

When file Pager dies, M0 can fail pending faults/deny unsafe allocations and report degraded state. If even kernel/RecoveryAuthority/timer cannot run or the verified recovery artifact is unavailable, stop safely/reboot where hardware permits rather than claim automatic restoration. Safety isolation is weaker/different from data integrity and user-session availability; power loss can destroy unsaved data.

## 8. ServiceManifest and admission policy

| Manifest part | Proposed check |
|---|---|
| ServiceID and ImageDigest | exact expected image/content and generation; no PID/path authentication |
| Required dependencies | hard startup readiness vs soft optional runtime dependency, hard start DAG cycle detection |
| Requested grants | type, rights, Task/object/range/device group/lineage; deny ambient escalation |
| Sponsor budget | RAM/frame/metadata/handle/IPC/reply/fault/task/thread budget charged before admission |
| Isolation class | separate Task/AddressSpace by default, co-location loss documented |
| Health policy | bounded independent READY check, not service self-report alone |
| Criticality | root/essential/interactive/optional/driver and failure escalation |
| Restart policy | finite attempts, retry windows/backoff and circuit-breaker |
| Shutdown policy | bounded drain/abort, remote effect ambiguity, safe fencing |
| Recovery policy | last trustworthy good slot and explicit rescue paths |
| Version/schema | reject unknown required fields, duplicate or downgrade/incompatible major versions |

Launch transaction candidate: validate manifest and image identity → reserve required budgets/handles → create scoped Task/caps/Endpoints → start → independently observe readiness within deadline → publish ServiceRegistry incarnation and allow clients. On failure do not expose a partly admitted highly privileged Task with old grants; revoke/reconcile actual references before refund. If failure is after creation/side effects, teardown is not an atomic rewind.

**No implied cross-service superuser:** regular Supervisor is not permitted to edit trusted manifest, increase budgets by re-labeling services, map arbitrary physical pages, or grant unrelated apps debug/driver authority.

## 9. Service restart and reincarnation — phased, not atomic

~~~text
Declared -> Admitted -> Starting -> Ready/Running
                       |           |
                       +-> Failed <-+
                            -> Quiesce / Deny New Grants (B)
                            -> Fence + Reconcile (R1 / R2-F / maybe R2-O)
                            -> Authorized New Task + Budget + Image
                            -> New ServiceIncarnation + New Endpoints
                            -> Revalidate READY -> Publish Fresh Registry
                            OR Quarantined / Disabled / Degraded / Fatal
~~~

**S0 detect:** distinguish actual Task death, process stalled, health probe failed, device hung, backing unavailable or memory OOM. A compromised service may lie in health self-report.

**S1 B barrier:** block new grants via revoked lineage; stop discovery of stale endpoint, but registry removal itself is not a kernel revoke. Old in-flight calls may already have completed.

**S2 safe quiescence:** close or retire old Endpoint E1, pending ReplyTokens/WaitSets/ReceiveEscrow and PagerRequest/Kernel FaultWaiters, and stop/revoke mappings. R1 requires CPU TLB cutoff; DMA/IRQ cannot be assumed safe until R2-F device fences (RFC-007). Dirty file pages may remain outstanding or be lost in terminal conditions (RFC-004/008).

**S3 sponsor cleanup:** account old Task refs, in-transit caps, grant anchors, queue messages, shared mapping refs, dirty pages and pinned DMA. No unbounded recursive collector or assumed instant charge refund.

**S4 fresh admission:** validate image version/digest, service policy, new Task/kernel generations and scoped budgets. Never retarget old ReplyToken/PagerBinding/in-transit Endpoint to new incarnation by matching a service name. Explicitly grant fresh caps.

**S5 READY and rebind:** independent health test, then Registry publish. Client detects peer change, reacquires scoped endpoint and reconciles UNKNOWN_REMOTE_EFFECT/idempotency using actual durable application transaction state. Never blind replay writes merely because the service restarted.

**Independent work can progress:** a stalled old R2-F DMA fence must not automatically block every unrelated software service recovery. For the *same DeviceGroup, pinned frames or coupled mutable resource*, no new grant until actual R2-F fence; for unrelated, separately scoped resources, new software admission may proceed after necessary B/R1 isolation and budget checks. Whether a replacement may share older backing depends on its own provenance.

**Hardware failure:** driver Task death while bus master still DMA-active cannot release its frame/DeviceGroup; quarantine or trusted hardware reset/fence. If IOMMU unsupported, either reject untrusted driver assignment or state the weaker trusted-only hardware class.

**Root failure:** a RecoveryAuthority responsible for normal Supervisor restart cannot itself rely on that dead Supervisor. Candidate RA-1 kernel-mediated bounded re-exec of M0 from resident verified boot artifact, RA-2 fail-stop/reboot into signed rescue, RA-3 independent second wired monitor. RA-1 adds kernel hooks; RA-3 adds TCB/SplitBrain risks; RA-2 reduces availability. No general winner yet.

## 10. Tiered criticality and bounded restart

| Tier | Kind | Conditional failure plan |
|---|---|---|
| T0 | Kernel/MMU/boot verification | fail stop or verified recovery reboot; never claim in-place kernel restart |
| T1 | M0 RecoveryAuthority, essential IRQ/watchdog path | RA-1 only with provable reserved restart, else reboot/fail-stop |
| T2 | Pager, essential storage/DeviceAuthority | bounded restart, fail dependent faults, enforce device fencing/quarantine |
| T3 | VFS, input/compositor, policy service | bounded restart and user-visible degraded desktop where possible |
| T4 | Optional and per-app compatibility services | leave stopped or per-sandbox restart without global reboot |

A service's trusted *asset* impact overrides its marketing name; a compromised driver co-located with a benign compositor changes the effective TCB. Restart budget (per ServiceID+image+BootEpoch), bounded exponential backoff, circuit-breaker, deadline and escalation prevent infinite reboot/CPU/metadata cycles. If restart failed due to OOM, retry only with real reclaimed frame/queue reserve evidence, not merely another health notification. **BootEpoch-local retry counters are reset on reboot and cannot prevent reboot loops.** A crash-consistent persistent attempt budget or independent firmware/rescue intervention is needed, with a safe outcome when the counter cannot be recorded.

A process existing != Ready, and Ready != healthy *forever*. Boot success must be based on defined checks and recovery classes rather than a permanently optimistic registry.


## 11. A/B slots, trial success, anti-rollback and rescue

**P0 danger:** If minimum accepted security version is advanced before a newly installed image has actually succeeded, the older known-good A/B slot can become forbidden although the new slot is broken. Android's official Verified Boot boot flow explicitly requires successful-slot marking before rollback protection metadata updates in A/B configurations [S9]. This is not merely a QA or UX concern: it can permanently remove the fallback permitted by the current security policy.

### 11.1 Four orthogonal ledgers

1. **Slot state:** Missing / Installing / ImageVerified / TrialBootable / FailedTrial / Good / Unbootable. A slot can contain a signed image yet be rejected by the current version floor.
2. **Boot attempt state:** active slot, boot epoch, trial retries left, last failure class and authenticated successful health receipt. Booting a kernel is not marking a slot Good.
3. **Security state:** authorized signer/keys and revocations, minimum security version, product hardware profile, acceptable recovery version. Valid signature alone is not freshness.
4. **Persistent user data state:** versioned filesystem/on-disk schema, migration and rollback compatibility. Returning from software B to A does **not** roll back user files or volume metadata changed under B.

### 11.2 Candidate trial progression (conceptual, not durable implementation)

~~~text
Good A at minimum security floor F
 -> Prepare inactive B (A untouched, B marked nonbootable during writes)
 -> Verify signed B image+manifest+compatible rescue >= F
 -> Write crash-consistent trial B metadata / finite retry budget
 -> Boot B as TRIAL, not as Good
 -> M0 + essential Pager/storage/IPC/manifest health predicates
 -> Authenticated successful-slot receipt, durable Good B state
 -> Only then consider F -> F' security-floor promotion
 -> Before F' promotion, verify actual readable/bootable, authenticated floor-eligible rescue or alternate, plus user-data compatibility
 -> If trial fails: return A iff A.version >= current F
 -> If A now below F or invalid: only an authorized rescue >= F, else fail-stop
~~~

**Not a solved protocol:** crash occurs between Good B and advancing floor, between writing slot metadata and image content, or while rescue media is unavailable. **Signed rescue reference != physically available readable recovery image** and does not establish compatible filesystem/user-data schema. Floor advancement requires a usable, bootable, authorized alternative, not merely a hash in BootPolicyManifest. RFC-008/017 must model every cut and trusted storage semantics. Promoting F after Good B may immediately forbid A; an eligible independent rescue still needs to be verified before update promotion. No expectation that an internet connection can repair an offline unusable machine.

### 11.3 Trust floor and storage choice

Options include authenticated UEFI variables, trusted monotonic TPM NV resources, RPMB or a verified secure counter. Standard files, plain GPT slot flags or user-modifiable NVRAM without actual trustworthy policy are *not* hardware anti-rollback. TPM support/enrollment/monotonic durability can be absent or constrained on reference PCs; platform feature classes must be checked before any production claim. EFI variable services have resource/policy failure modes and are not a transactional multi-record database [S13].

An unauthorized recovery media version below the minimum security floor must not launch as ProductionVerified. Developer override, key recovery and user access after key loss need explicitly signaled separate mode (RFC-016/017). Key revocation and offline recovery sometimes conflict; neither "always boot older" nor "permanently brick rather than repair" can be chosen without policy, threat model and recovery trust evidence.

### 11.4 Boot-good receipt requirements

A receipt candidate is bound to slot image identity, BootEpoch, version/manifest, approved health policy and a trusted ability to mark success. **Health reporter, boot-slot success writer and irreversible secure-floor writer are different authorities.** A compromised ordinary Supervisor must not directly advance the security floor through a self-declared READY response; independent M0/loader/privileged storage control and policy needed. GUI appeared or one service self-reported READY is too weak. Essential Pager/backing readiness, IPC/RecoveryAuthority response and defined storage read-only integrity may be required, but **there is no single proof of future availability**. A success receipt certifies only a bounded set of checks over a defined time/profile, and malicious health services remain threat-model concerns.

## 12. Verified vs measured boot, TPM, user data and privacy

- Firmware Secure Boot authenticates firmware-executed EFI images under the enrolled platform policy. It does not itself establish a full trust chain through every post-UEFI file page [S1].
- TPM measured boot extends selected measurements into PCRs and supplies an event log; a quote can support independent verifier checks against trusted reference values [S12]. Measurement without enforcement cannot forbid a malicious image.
- Remote attestation needs fresh nonce, verifier trust, expected reference measurements and a privacy model. Incomplete or malformed log values cannot become proof of authorized execution merely because they contain hashes.
- Sealing encryption keys to exact PCRs may cause legitimate firmware/OS update, repair or different UEFI settings to lock users out of data. Recovery key policies and user consent belong to RFC-017; no claim of seamless data preservation.
- M0 research should boot offline without remote attestation, cloud sign-in, network time or mandatory TPM. A future stronger product profile may require particular hardware features but must not silently assume all UEFI systems satisfy them.
- Log and measured version values can disclose personal device/software identity. Remote sharing requires separate explicit policy.

## 13. Power cut, shutdown, storage loss and suspend

### 13.1 Controlled shutdown sequence candidate

1. Stop granting new user operations and report planned shutdown/drain state with current service incarnation.
2. Finish bounded IPC work or explicitly cancel/mark remote effects uncertain. RFC-003 transport COMMITTED is not completed file storage.
3. Ask relevant Pager/VFS/storage to write back dirty data and metadata under the still-running driver stack, with bounded failure/error reporting.
4. Stop new device commands; obtain actual DMA quiescence or retain quarantine; never recycle unsafe frames.
5. Retire mappings and cap ancestry, performing required R1 shootdown and R2-F/R2-O checks without unbounded recursive cleanup.
6. Update verified boot slot state only through separately proven crash-safe storage; clean shutdown cannot substitute for a verified trial-health receipt.
7. Stop hardware, reboot or expose a controlled fail-stop status under agreed physical profile.

**No guaranteed flush:** actual durability requires filesystem/storage/controller barriers and power-loss semantics (RFC-008). Dirty memory and queued IPC may be lost on sudden power cut. A watchdog reboot during an incomplete writeback cannot be labeled as preserved user data.

### 13.2 Suspend, resume and hibernate boundary

Suspend/resume are **not automatically normal boot**. They must reconcile CPU security state, MMU/PCID/PKRU, device reset/DMA continuation, clocks/deadlines, event/IRQ routing, PagerBinding incarnation and memory lifetimes. Disk hibernation additionally needs signed/verified image identity and confidentiality plus rollback protection. These are future independently gated features, not part of a presumed working M0.

## 14. Service registry, identity and application reconnection

A ServiceID is a manifest-authenticated *logical name* at the Supervisor layer. Kernel authority remains typed Endpoint, Task and grant identity/generation. Registry lookup alone grants **no capability**. For RFC-003 E1 (exclusive ChannelEndpoint ownership), separate client channels/dispatcher are needed; a globally duplicated Endpoint is not assumed.

When a service is relaunched:
- the registry publishes a fresh service incarnation only after authorized image/budget admission and an independent readiness test;
- previous endpoint, queued rights, ReplyTokens, WaitSet completions and PagerBindings remain tied to their old scopes and cannot silently point to new Tasks;
- clients explicitly reacquire service permission; a previously cached ServiceID is not an automatic regrant;
- service work with UNKNOWN_REMOTE_EFFECT can only be replayed based on idempotency and authoritative persisted state (RFC-003/008);
- compatibility guest UID 0, Windows Administrator and Darwin entitlement labels are user-space protocol values and never become native root capabilities;
- user policy/portal approval must not silently vanish from new service trust checks.

**Race to model:** old ServiceSupervisor or Registry worker remains alive after being declared dead and continues sending requests. New and old incarnations cannot both claim identical privileged IRQ/group/memory grants; resolve through kernel revoke/fence and generation control, not untrusted string checks.

## 15. Observability and trusted diagnostic boundaries

Diagnostic schema candidates: boot stage, boot mode, slot and manifest version, kernel-generated BootEpoch, service incarnation plus kernel object-generation reference, failure class, bounded attempt counts, Pager/IPC condition, resource charge category, device reset/fence state, recovery transition and health-check provenance.

**BootEpoch is a boot-local discriminator, not a trusted anti-rollback monotonic counter.** An initial random epoch may not be cryptographically strong without verified entropy, and uniqueness/guessability cannot be relied on for authority. Capability checks and generation scope remain the security boundary.

- Avoid logging raw secrets, encryption keys, user file content, kernel memory dumps or stable personal identifiers into ordinary reboot logs.
- Bounded kernel in-memory logs may be overwritten or lost during power failure; do not claim forensics persistence.
- Persistent dump requires RFC-008 storage path and user privacy/security policy; no arbitrary disk write in M0 when file Pager is unavailable.
- Record failure as detected, attempted, recovered or unresolved; merely scheduling a restart is not evidence it succeeded.
- Serial/GOP/early console are hardware capabilities requiring validation, not guaranteed by QEMU-only demonstration.

## 16. Failure-injection and race matrix

| Failure/race | Incorrect outcome to prevent | Required design reaction |
|---|---|---|
| EFI loader accepted but kernel or BootBundle replaced | fake verified boot claim | loader-chain image/manifest verification |
| Secure Boot setup/audit/dev mode | production trust falsely asserted | separate boot trust mode |
| Memory map changed after GetMemoryMap | wrong MapKey and unsafe page reuse | valid ExitBootServices retry and fail-safe |
| Untrusted ServiceManifest duplicates service identity | cap escalation via parser ambiguity | strict signed manifest validation |
| Broad boot root retained by normal Supervisor | compromise becomes whole OS root | partition initial grants, minimal RecoveryAuthority |
| Normal Supervisor death | cannot restart own supervisor | independent wired RA or fail-stop |
| RecoveryAuthority death | attempts unbounded self-restart | bounded kernel M0 re-exec or signed rescue |
| Pager handler stack depends on Pager | no fault progress | M0 wired resources |
| Driver IRQ delivery needs pageable driver code | Pager/storage cycle | resident bounded IRQ completion |
| IPC queue full during OOM cleanup | no error path | reserved credits, fail-fast |
| Service readiness self-report compromised | trial slot falsely marked Good | independent health conditions |
| Service restart with in-transit Endpoint | stale reference resurrected | E1 generation and lineage |
| Reply arrives after client timeout/restart | wrong client completed | ReplyToken scoped object/gen |
| Fault page supplied by old Pager generation | maps wrong new VA/backing | independent Kernel FaultWaiter revalidation |
| Stale DMA after driver death | unsafe frame/group reused | device fence/IOMMU or quarantine |
| Repeated restart at memory pressure | endless CPU/metadata exhaustion | finite retry/circuit breaker |
| Slot B signature valid but old vulnerable version | version downgrade called secure | floor/key authorization |
| B fails after floor updated too early | A fallback forbidden, B broken | success-before-floor plus rescue gate |
| Crash between A/B metadata writes | both or neither bootable | journal/redundancy/replay proof |
| Floor advanced and rescue old | no trusted path to restore device | verify eligible rescue before floor |
| User file schema changed under trial B | rollback A corrupts user data | migration/backward compatibility contract |
| TPM event log exists but no signature verification | measured bad code still runs | separate enforcement/measurement |
| Unexpected power loss while file dirty | false preserved file claim | incomplete durability/explicit risk |
| General app spends emergency memory pool | pager restart OOM deadlock | scoped protected Reserve Budget |
| Source of boot crash diagnosis lost | no log interpreted as no failure | best-effort telemetry and unknown status |

## 17. Proposed security and correctness invariants BOOT-01…BOOT-40

| ID | Proposed invariant — not verified |
|---|---|
| BOOT-01 | UEFI loader admission does not imply verification of later kernel, BootBundle, manifest and service images. |
| BOOT-02 | Development, TestSigned, Recovery and future ProductionVerified states are not silently conflated. |
| BOOT-03 | Authentication, measurement, attestation and anti-rollback have separate predicates and trust roots. |
| BOOT-04 | EFI memory map handoff uses valid MapKey/descriptor version or fails safely without unauthorized RAM reuse. |
| BOOT-05 | Kernel/MMU/firmware runtime/BootBundle/critical DMA regions are not freed as ordinary usable RAM prematurely. |
| BOOT-06 | Initial kernel grants are typed, scoped, charged and derived from legitimate root authority. |
| BOOT-07 | Service name, PID, guest UID or a signed executable alone cannot create global kernel privileges. |
| BOOT-08 | RecoveryAuthority grants are independently scoped; normal ServiceSupervisor has no implicit Root privilege. |
| BOOT-09 | Critical M0 boot/failure path requires no ordinary user file Pager or network to run within its stated scope. |
| BOOT-10 | General Task allocations cannot spend M0 emergency frames/IPC/IRQ/page-table/cleanup quota. |
| BOOT-11 | Mandatory service startup and runtime critical wait dependencies do not contain a self-requiring Pager/IPC/driver cycle. |
| BOOT-12 | Manifest-controlled launches reject invalid image identity/schema/dependency/grant request. |
| BOOT-13 | READY process self-assertion alone cannot mark update trial slot trusted Good. |
| BOOT-14 | Restart assigns fresh Task/Endpoint/service generation; old credentials cannot transparently bind new instance. |
| BOOT-15 | Stale ReplyToken, ReceiveEscrow, PagerRequest or IRQ completion cannot authorize a fresh service. |
| BOOT-16 | Service death does not instantly complete revocation B/R1/R2-F/R2-O across memory and DMA. |
| BOOT-17 | Cap lineage anchors, in-transit handles and sponsor debt survive origin Task death until correctly settled. |
| BOOT-18 | Critical recovery operates with finite cost or an explicit bounded fail-stop outcome; no limitless fault recursion. |
| BOOT-19 | Keeping kernel isolation under OOM does not automatically claim service availability or dirty data durability. |
| BOOT-20 | Restart retry/circuit-breaker and boot slot attempts have finite limits and escalation. |
| BOOT-21 | Slot trial success and version-floor promotion follow an explicitly crash-consistent policy, not a single Boolean. |
| BOOT-22 | Promoting anti-rollback floor cannot silently remove last policy-eligible rescue/fallback without an approved alternative. |
| BOOT-23 | A signed image that is revoked or below security floor cannot appear as ProductionVerified. |
| BOOT-24 | A successful health receipt is bound to verified slot identity, version, boot epoch and explicit checked conditions. |
| BOOT-25 | UEFI NVRAM/GPT/normal filesystem metadata is not assumed to provide atomic secure multi-record transactions. |
| BOOT-26 | TPM PCR/event-log observation alone is not code-signature authorization or trusted boot policy enforcement. |
| BOOT-27 | Service restart never automatically undoes completed file/network side effects or magically resolves UNKNOWN results. |
| BOOT-28 | Unsafe device DMA cannot permit memory reuse or DeviceGroup reallocation without actual fence/quarantine. |
| BOOT-29 | Shutdown and boot metadata success cannot claim all dirty user data persisted absent storage durability evidence. |
| BOOT-30 | Critical RecoveryAuthority failure may cause controlled fail-stop/reboot, never falsely reported healed. |
| BOOT-31 | Dev/recovery trust-policy bypass is explicitly represented; no silent access to production-protected data. |
| BOOT-32 | Boot/restart telemetry labels source certainty and persistence; volatile missing logs are not proof no crash happened. |
| BOOT-33 | No UEFI Boot Service outside Memory Allocation Services after initial ExitBootServices attempt; after successful exit Boot Services watchdog cannot be M0 watchdog. |
| BOOT-34 | Verified image/manifest bytes and their trust keys/version floor cannot be changed between verification and use without reauthentication. |
| BOOT-35 | Complex signed service manifest parsing may not enter Ring 0 implicitly; kernel bootstrap metadata/provenance stays bounded. |
| BOOT-36 | M0 RA own-death cannot claim automatic restart without bounded privileged re-exec; fail-stop is valid. |
| BOOT-37 | Boot retry budgets survive reboot via trusted durable state or explicitly safe fallback when state unavailable. |
| BOOT-38 | Floor promotion requires physically available readable, bootable, authorized and recovery-compatible rescue/alternate. |
| BOOT-39 | Independent software service restart does not imply unsafe reuse of stalled old DMA group or pinned frames. |
| BOOT-40 | Boot-good receipt, privileged version floor writer and user-data migration each have distinct trust and durability checks. |

**Status:** these are proposed predicates/proof obligations only. Exact hardware, fairness, timer progress, secure storage and threat conditions must be supplied before any formal correctness claim.

## 18. Formal design verification plan and future tests

**Conceptual bounded-state model:** two A/B slots, physically present/absent rescue, floor/version+data-schema compatibility, BootEpoch, two service incarnations, cap lineage/sponsor, PagerBinding plus per-Thread FaultWaiter, ReplyToken, DMA pin, both *volatile* and *persisted across reboot* attempt counters, independent health/slot-success/floor writers, verified-vs-executed byte identity, emergency reserve and torn journal metadata.

Safety states to explore: unauthorized/floor-rejected image starting; manifest grant escalation; restart with stale cap/Reply/FailRange; R1 vs queued page fault; DMA frame prematurely reclaimed; A/B floor raised before a remaining eligible rescue; OOM consuming critical reserve; duplicated "Good" slot after torn metadata update.

Liveness states (under explicitly stated CPU/IRQ/fair scheduler/working storage assumptions): Supervisor crash eventually yields fresh authorized Supervisor or a bounded fail-stop; Pager crash eventually fails dependent faults or recovers from an independent M0 path; trial B either earns Good under health policy or exhausts attempts and follows a policy-eligible fallback/rescue. A killed kernel or malicious firmware is outside such liveness proof.

**Future experiments only after explicit separate implementation approval:**
- OVMF/QEMU GetMemoryMap/ExitBootServices retry/fault handling under controlled faults;
- invalid firmware memory descriptors, signed manifest parser fuzz, denied root grants and service dependency cycles;
- kill Supervisor/RecoveryAuthority/Pager while IPC/COW/DMA/metadata are active, then test bounded cleanup;
- exhaust normal resources and even approach the reserved pool, verify no deadlocks/privilege amplification;
- safe dedicated update media power-cut tests before/after B ImageVerified, Trial, Good and security floor; do not risk user disks;
- actual selected x86_64 UEFI reference PC for firmware Secure Boot mode, TPM presence, runtime variable reliability, PCI/IOMMU/IRQ and reset assumptions;
- separate boot time, fail detection, recovery p50/p95/p99 and user-data durability results only after implementation. QEMU does not prove physical storage/firmware atomicity.

**No tests or model checker runs are reported by this RFC.**

## 19. Design alternatives and refusal criteria

| Candidate | Potential advantage | Architectural cost | When to reject |
|---|---|---|---|
| BOOT-A dedicated M0 RA + normal Supervisor | crash observation survives ordinary Supervisor, explicit limited root grants | more trusted code/processes and sophisticated reserved recovery state | if M0 cannot be bounded or RA is effectively another omnipotent supervisor |
| BOOT-B single privileged root task | simple initial boot/authority accounting | root death has no in-place recovery, broad TCB-S | if desktop service death must be recoverable without reboot |
| BOOT-C kernel broad service management | stronger root-independent supervision | service policy/manifest parsing in Ring 0, larger TCB-K | if verified code surface/upgrade complexity becomes unacceptable |

**Conditional selection:** BOOT-A for research only. Comparison must use equivalent failure semantics and actual authority graphs, not marketing labels, initial process count or unmeasured microbenchmarks. If BOOT-A requires so many kernel exceptions that its TCB exceeds BOOT-C's cost, narrow it or consider BOOT-B with explicit fail-stop and trusted offline rescue.

## 20. Open issues B6-01…B6-36

| ID | Priority | Unresolved decision | Co-review |
|---|---|---|---|
| B6-01 | P0 | BOOT-A RecoveryAuthority vs BOOT-B/C TCB and actual privilege cost? | 001/006 |
| B6-02 | P0 | Loader verification of kernel/BootBundle/manifest after UEFI authorizes EFI image? | 006/017 |
| B6-03 | P0 | GetMemoryMap/ExitBootServices errors and BootContext PMM ownership? | 004/006 |
| B6-04 | P0 | M0 full wired reserve of stack, IRQ, IPC, page tables, cleanup under all CPU profiles? | 003/004/006 |
| B6-05 | P0 | Minimal RA own-death recovery primitive vs controlled reboot? | 001/006 |
| B6-06 | P0 | Scoped root cap derivation, sponsor adoption and revocation anchor survival? | 002/006 |
| B6-07 | P0 | Runtime Pager/VFS/Storage wait-for graph including queue credit and scheduling? | 003/004/005/006 |
| B6-08 | P0 | PagerBinding/FaultWaiter cleanup and old/new incarnation scopes? | 003/004/006 |
| B6-09 | P0 | ReplyToken/ReceiveEscrow/IPC UNKNOWN when service dies after COMMITTED? | 002/003/006 |
| B6-10 | P0 | Real hardware IOMMU group, IRQ and driver reset before fresh DeviceGrant? | 004/006/007 |
| B6-11 | P0 | A/B slot trial-success receipt and power-cut-safe success metadata? | 006/008/017 |
| B6-12 | P0 | Anti-rollback floor promotion, surviving alternate/rescue and trusted monotonic state? | 006/017 |
| B6-13 | P0 | Reboot loop finite termination after critical process/slot repeated failure? | 006/017 |
| B6-14 | P0 | Dirty user file/writeback after Pager/storage loss during shutdown/reboot? | 004/006/008 |
| B6-15 | P0 | Secure Boot disabled/dev signed/real platform keys vs user-visible verified claim? | 006/017 |
| B6-16 | P0 | Independent critical service health, forged READY and update-good decision authority? | 006/015/017 |
| B6-17 | P1 | Per-client E1 endpoints and scoped ServiceRegistry discovery? | 003/006/018 |
| B6-18 | P1 | Restart attempts, deadlines/backoff and CPU-budget donation for recovery? | 005/006 |
| B6-19 | P1 | Signed manifest unknown fields/schema/key rotation and policy downgrade rules? | 006/017/018 |
| B6-20 | P1 | Firmware NVRAM/TPM absent, full or worn vs metadata durability? | 006/008/017 |
| B6-21 | P1 | Trusted user-data repair and dev-mode/recovery consent? | 006/016/017 |
| B6-22 | P1 | TPM quote and event-log privacy, freshness and remote attestation scope? | 006/017 |
| B6-23 | P1 | Executable bootfs/immutable code pages under subsequent Pager faults? | 004/006/017 |
| B6-24 | P1 | DMA DeviceGroup regrant after crash/fence, unsafe hardware exclusion? | 006/007 |
| B6-25 | P1 | Diagnostics privacy, persistent crash logs vs M0 no storage? | 006/008/015 |
| B6-26 | P1 | Suspend/hibernate/resume vs cold boot epoch and old DMA/IRQ state? | 005/006/007 |
| B6-27 | P1 | Shared on-disk data format migration while B trial, fallback to A? | 006/008/017 |
| B6-28 | P2 | Trusted local/offline rescue media and optional network repair provisioning? | 006/017 |
| B6-29 | P0 | EFI partial shutdown allowed retry calls and independent post-ExitBootServices watchdog/timer? | 005/006 |
| B6-30 | P0 | Root own-death RA-2 vs kernel re-exec RA-1 TCB cost and initial grants? | 001/006 |
| B6-31 | P0 | Complex signed ServiceManifest parser location, verifier key/floor trust provenance and TOCTOU? | 001/006/017/018 |
| B6-32 | P0 | Floor promotion: actual rescue readable/bootable plus user-data schema compatibility? | 006/008/017 |
| B6-33 | P0 | Cross-boot attempts: trusted persistent budget/crash policy and no infinite reboot? | 006/008/017 |
| B6-34 | P0 | Health/slot-good/floor writer separation under malicious Supervisor? | 006/015/017 |
| B6-35 | P1 | Restart overlap of unrelated software vs blocked old device R2-F hardware quarantine? | 002/006/007 |
| B6-36 | P1 | Historical Fuchsia guide and Android legacy/virtual A/B source applicability? | 006/017 |

## 21. Proposed decision register B6-D01…B6-D21 (not Accepted)

- **B6-D01:** BOOT-A signed minimal BootBundle + kernel initial typed caps + wired small RecoveryAuthority + restricted normal Supervisor is priority hypothesis; BOOT-B/C remain alternatives.
- **B6-D02:** Verify exact chain from firmware EFI image policy through loader, kernel, BootBundle, signed manifest and service images; no automatic trust transitivity.
- **B6-D03:** Distinguish verified execution, TPM measurement, attestation, rollback floor, development and recovery trust modes.
- **B6-D04:** Typed scoped initial grants and resource budgets with accountable sponsor/lineage; string lookup or executable signature does not grant root.
- **B6-D05:** M0 is fully resident for defined errors and operates without file Pager, normal allocations or network.
- **B6-D06 [revised]:** Normal Supervisor gets no root recovery grants; **RA-2 bounded fail-stop/authorized reboot is first M0 self-death study default** until RA-1 kernel-side re-exec is proved safe and justified.
- **B6-D07:** Signed versioned ServiceManifest controls image identity, dependencies, grant envelope, admission budget, health, restart and criticality.
- **B6-D08:** Service restart has explicit deny/drain/fence/accounting/fresh-grant phases and does not roll back completed external effects.
- **B6-D09:** Tiered failure escalation and finite retry/circuit-breaker prevent endless service or boot loops.
- **B6-D10:** Runtime wait-for resource graph, not just service startup DAG, is joint RFC-003/004/005/006 P0.
- **B6-D11 [revised]:** Slot health success, floor writer and data migration are separate. Floor promotion requires physically readable, bootable, authorized floor-eligible rescue/alternate and compatible data recovery.
- **B6-D12:** Slot/floor metadata durability and secure monotonic storage require RFC-008/017, not unsupported NVRAM assumptions.
- **B6-D13:** Successful boot, operational availability, durable user data and attested state are separate success claims.
- **B6-D14:** Lost driver with unverified DMA fence leaves device/frame quarantined, not reassignable.
- **B6-D15:** Required manifest image/policy mismatch fails closed; recovery image must meet separate signed/current security policy.
- **B6-D16:** All choices remain Proposed until trust, failure, metadata, hardware and proof gates pass; no implementation is authorized.
- **B6-D17 [new]:** Post-ExitBootServices M0 watchdog independent of firmware boot watchdog; UEFI retry restrictions explicit.
- **B6-D18 [new]:** Complex signed manifest verified outside Ring 0 where feasible; fixed typed kernel bootstrap descriptor with trusted provenance is studied.
- **B6-D19 [new]:** Root RA-2 fail-stop/authorized reboot research default; RA-1 requires bounded kernel loader+cap reissue proof.
- **B6-D20 [new]:** A/B durable cross-boot retries, boot-good and floor-writer permissions plus actual rescue readability are separate gates.
- **B6-D21 [new]:** Recover unrelated scoped software while quarantining unsafe DeviceGroup until genuine R2-F fence.

**ADR effect:** Architecture D1–D8 Accepted Direction remains; RFC-001~004 decisions remain Proposed. Nothing in this new file modifies any existing ADR or implementation status.

## 22. Cross-RFC interface contracts

| Adjacent RFC | Required contract |
|---|---|
| RFC-001 | asset-specific TCB-K/B/S and separate native kernel vs user service policy, restricted guest isolation |
| RFC-002 | typed boot grants, sponsor accounting, Cap lineage, R0/B/R1/R2-F/R2-O, restart across Task death |
| RFC-003 | E1 endpoint, Wait/Reply/ReceiveEscrow, backpressure recovery pool, COMMITTED/UNKNOWN and stale calls |
| RFC-004 | M0 resident cut, PagerBinding/FaultWaiter, CPU page table protection, dirty data vs OOM |
| RFC-005 | RecoveryAuthority runnable fairness, watchdog deadlines, scheduling inversion without assumed donation |
| RFC-007 | true IOMMU group, device stop/reset fence, IRQ and pinned frame lifecycle |
| RFC-008 | A/B metadata atomicity, migration durability, flush/replay/crash consistency |
| RFC-009 | BootContext/native Task start, usercopy/error and fatal failure contracts |
| RFC-010 | guest supervisor isolation and no Linux root/Windows admin to native root conversion |
| RFC-015 | threat/quality gates, restart/power-loss/timing tests and diagnostic privacy |
| RFC-016 | user consent, developer mode and recovery permissions |
| RFC-017 | full signing hierarchy, image verification, security floor, trusted A/B/rescue and key recovery |
| RFC-018 | BootBundle, signed ServiceManifest and service protocol version/migration compatibility |

**P0 joint review cluster:** (003+004+006) IPC/Pager/boot wiring; (002+006+007) cap/device/DMA reassign; (006+008+017) anti-rollback/fallback/slot metadata; (005+006) watchdog recovery runnable guarantees. The RFCs cannot independently claim these cross-boundary properties are solved.

## 23. Proof-oriented exit gates and research road map

**R1 design cannot pass yet.** Need:
1. Verified image/BootContext provenance table and root authority graph with narrowed grants.
2. Fully costed finite M0 emergency memory/queue/IRQ/timer/page-table budget and allocation-free terminal error path.
3. Service start and runtime wait-for graphs with bounded Pager/storage IRQ cycles and a health observer independent of normal Supervisor.
4. Service-restart state machine proof with old cap lineages, R1 CPU fences, R2-F DMA, Reply/Pager generations and outstanding remote effects.
5. Crash-state A/B metadata and security floor model proving availability of an authorized eligible rescue path under stated hardware/security assumptions.
6. Explicit distinction of safety failure, performance/availability degradation and on-disk unsaved data loss.
7. BOOT-A/B/C evaluation under equivalent grants, failure recovery and product goals before committing to more Ring 0 code.

**Next design tasks:** RFC-005 Scheduling/Preemption and RFC-007 Hardware DMA for actual restart/fence feasibility; RFC-008 storage durability and RFC-017 verified boot/anti-rollback should be completed before product claims. A second adversarial review of RFC-006 is recommended before acceptance.

## 24. Official primary technical sources (checked 2026-10-24)

- **[S1] UEFI 2.11 Secure Boot and Driver Signing:** https://uefi.org/specs/UEFI/2.11/32_Secure_Boot_and_Driver_Signing.html
- **[S2] UEFI Boot Services GetMemoryMap/ExitBootServices:** https://uefi.org/specs/UEFI/2.10/07_Services_Boot_Services.html
- **[S3] seL4 initial capability root task:** https://docs.sel4.systems/Tutorials/capabilities.html
- **[S4] seL4 initial untyped memory and BootInfo:** https://docs.sel4.systems/Tutorials/untyped.html
- **[S5] Fuchsia userboot/BOOTFS:** https://fuchsia.dev/fuchsia-src/concepts/process/userboot
- **[S6] Fuchsia early boot component manager:** https://fuchsia.dev/fuchsia-src/concepts/process/everything_between_power_on_and_your_component
- **[S7] Android A/B system updates:** https://source.android.com/docs/core/ota/ab
- **[S8] Android bootloader slot selection:** https://source.android.com/docs/core/architecture/bootloader/updating
- **[S9] Android verified boot A/B failure/rollback policy:** https://source.android.com/docs/security/features/verifiedboot/boot-flow
- **[S10] Chromium OS firmware/recovery design:** https://www.chromium.org/chromium-os/chromiumos-design-docs/firmware-boot-and-recovery/
- **[S11] Chromium OS verified boot overview:** https://www.chromium.org/chromium-os/chromiumos-design-docs/verified-boot/
- **[S12] TCG PC Client RIM and PCR event log:** https://www.trustedcomputinggroup.org/resources/documents/tcg-pc-client-reference-integrity-manifest-specification
- **[S13] UEFI Runtime Variable Services 2.11:** https://uefi.org/specs/UEFI/2.11/08_Services_Runtime_Services.html
- **[S14] Android current Virtual A/B overview:** https://source.android.com/docs/core/ota/virtual_ab
- **[S15] TCG PC Client Platform Firmware Profile PCR/event-log:** https://trustedcomputinggroup.org/wp-content/uploads/TCG-PC-Client-Platform-Firmware-Profile-Version-1.06-Revision-49_31July2023.pdf

## 25. Critical adversarial review — v0.1 → v0.2 (2026-10-24)

**Method and scope.** Reconsidered BOOT-A against architecture v0.2 and RFC-001/002/003/004, current official UEFI ExitBootServices guidance, Android Verified Boot and A/B, seL4 initial capabilities, Fuchsia early boot description, and TCG measurement sources. The task is design research only; NO model checking, independent audit, hardware boot, firmware key provisioning, actual signed image, performance measurement, or OS implementation occurred.

### 25.1 Material corrections and counterexamples

| Topic | Counterexample against v0.1 assumptions | v0.2 safety response | Remaining evidence |
|---|---|---|---|
| EFI firmware interface after first ExitBootServices | A stale MapKey causes EFI_INVALID_PARAMETER. A naive recovery retries console/event/filesystem Boot Services even though firmware may already be partially shutting down. | Restrict retry calls to UEFI-permitted Memory Allocation Services, current MapKey and valid descriptor info. On failure, do not assume full firmware environment survives. | UEFI state-machine, buffer sizing and exit postconditions on OVMF and physical PC |
| Firmware boot watchdog | ExitBootServices succeeds; the preboot watchdog is disabled and cannot time out a wedged Pager/RecoveryAuthority. | Require separately initialized OS timer/IRQ path or independently supported hardware watchdog, including IRQ stack and reserved memory. | RFC-005 scheduling/timer, RFC-007 physical IRQ |
| Complex bootstrap verifier | Loader trusts a service manifest that declares its own acceptable key/version and demands broad boot grants; kernel parses unbounded rich manifest and increases TCB-K. | Establish verifier/key/floor from higher trusted stage; propose trusted loader or M0 policy parser and bounded fixed typed kernel descriptor. | Secure signer floor/origin and privilege reduction model RFC-001/017/018 |
| Check-time vs use-time | Correctly hashed kernel/M0 bytes later change via shared writable mapping, loader buffer reuse or unauthorized DMA before execution. | Protected immutable or reauthenticated executable/manifest bytes through handoff; W^X and physical driver isolation required. | RFC-004/007/017 boot DMA threat and firmware assumptions |
| RecoveryAuthority own crash | RA-1 seamless root re-exec requires image loader, Task construction, CPU/IPC/VM budgets and root grants in Ring 0; small kernel assumption fails. | RA-2 verified reboot/fail-stop is conservative M0 own-death baseline until an explicitly bounded RA-1 design is proven; ordinary Supervisor restarts may remain user-managed. | Minimum kernel hooks, no-image-Pager recovery cut and hardware restart policy |
| Volatile boot loop counter | System reboots after RecoveryAuthority crash and resets in-memory BootEpoch/restart counters each time, repeating failure forever. | Persistent trusted cross-boot attempt journal or independently triggered firmware/recovery fallback; unavailable persistence has explicit stop policy. | RFC-008 crash semantics, RFC-017 trusted slot counter |
| Rescue merely listed, not bootable | A/B floor raised after B marked Good and rescue hash validated, but recovery partition is missing, unreadable, signed below F' or cannot read migrated data. | Floor promotion requires physically present readable, bootable, signed and **at new floor** eligible rescue/alternate, plus required user-data migration compatibility. | Simulated all crash cuts and offline recovery on selected media |
| Boot-good controlled by compromised service | Normal Supervisor sends READY and can itself mark B Good and advance irreversible security floor without independent observation. | Separate health reporter, boot slot state writer and trusted floor writer; check identity/health source and strict capability budget. | Malicious Supervisor and forged receipt model |
| R2-F device quiescence stalls unrelated recovery | A crashed driver cannot safely quiesce a DMA group, leaving non-device software restart indefinitely blocked by global drain ordering. | Separate resource ownership: affected group remains fenced/quarantined while unrelated scoped software may start after its own B/R1 and budget checks. | Dependency and lifetime graph RFC-002/006/007 |
| Documentation drift | Fuchsia early boot guide explicitly warns it may be outdated. Android legacy A/B and modern Virtual A/B use different storage topologies. | Cite both historical and current references as design comparators; do not copy their layouts as current guarantees. | Confirm on future chosen hardware and actual updated RFC-017 design |

### 25.2 New proof obligations, organized by threat boundary

**BOOT-33/34 — handoff:** model first ExitBootServices failure→partial shutdown→legal GetMemoryMap retry→success; prove that afterward no Boot Services watchdog/device protocol remains assumed. Protect verification bytes from mutable alias and post-check DMA between load and execution.

**BOOT-35/36 — privileged root:** compare a signed manifest parsed by verified loader/M0 with a tiny kernel bootstrap record against a manifest parser in kernel; include new kernel TCB and lack of secure floor provenance. When RecoveryAuthority is destroyed, show fail-stop without invoking nonexistent kernel filesystem loader. If later selecting RA-1, explicitly budget trusted loader, authority reissuance and generation revocation before acceptance.

**BOOT-37/38 — update/reboot loop:** model failure on every power interruption between durable retry count, trial image, Good receipt, floor promotion and rescue. A hash/signer alone cannot make rescue bootable. The recovery artifact must actually exist at a version/format/floor and usable hardware configuration. A secure irreversibly advanced floor may intentionally forbid old A, so verified rescue is a product-recovery prerequisite, not an optional nice-to-have.

**BOOT-39/40 — bounded restart:** an unrelated software service can restart while a separate unsafe DMA group is quarantined; no cross-domain authority must leak. Boot-good receipt and monotonic floor writer are independently authenticated. A change to user-data schema before trial health may make A rollback logically impossible even if A remains signed.

### 25.3 Specific statements NOT supported by this design

- Not proven that RA-1 is possible within a microkernel's current KABI or without extra Ring 0 loading code.
- Not proven that signed boot chain, TPM quote, measured logs, no-downgrade state or secure NVRAM is available on a chosen PC.
- Not proven that A/B metadata or version-floor changes are crash-atomic, that a recovery image is actually readable, or that user data remains usable after update rollback.
- Not proven that an independent M0 timer/IRQ/watchdog/scheduler with bounded credits exists or can be restarted after a kernel-wide fault.
- Not proven that security isolation implies liveness, that restarting userspace repairs DMA, or that dirty memory is persisted at shutdown.
- Not proven that Fuchsia component manager or Android A/B historical behavior applies unchanged to UniOS.

### 25.4 Exit-gate changes

1. **RFC-003/004/005/006:** demonstrate an *actually bounded* M0 CPU/IRQ/timer/IPC/frame/cleanup wait-for graph under dead Supervisor, Pager and OOM.
2. **RFC-001/002/006:** map image verification→fixed boot descriptor→initial typed grants; ensure boot parser, trust key, secure floor and service grant lifetimes do not form a circular trust chain.
3. **RFC-006/008/017:** construct a product-state model with two slots, persisted attempts, monotonic floor, physically available signed rescue and user-data schema compatibility; enumerate every power-cut point.
4. **RFC-002/006/007:** separate software-process recovery from hardware DeviceGroup regrant and prove B/R1/R2-F boundaries in malicious driver reset cases.
5. **Architecture choice:** retain BOOT-A as research hypothesis; accept BOOT-B with an explicit fail-stop if BOOT-A requires an unbounded M0 loader or broad root privileges. No Approved/Accepted status until actual counterexamples are resolved.

### 25.5 Version status and source caveats

**RFC-006 v0.2 remains Proposed — Specification.** References [S1,S2,S3,S4,S7,S8,S9,S12,S13,S14,S15] are architecture comparison or normative specifications of other systems; UniOS adopts neither their source code nor their proof. [S6] specifically self-labels as potentially stale; [S7] is explicitly Android *legacy* A/B, not the Virtual A/B layout currently used for dynamic partitions [S14]. [S12] reference integrity manifest is not the platform event-log format source; [S15] is the relevant TCG firmware profile. All brand/implementation claims have to be rechecked when selecting physical hardware and release assurance.

**Review judgement:** The trust separation and scoped userspace supervisor of BOOT-A still make a promising design target. But v0.1 overstated the simplicity of recovering the privileged bootstrap authority and under-specified the firmware handoff/watchdog, secure-floor rescue and persistent reboot-loop controls. These are real blockers, not optional optimizations.

**Evidence and licensing note:** External technical sources describe those independent systems only; no UniOS kernel code, EFI binary, third-party source or working test was produced or imported. Any future use of code must undergo separate license/provenance review.

---

**Document history:** 2026-10-24 v0.1 original proposed design (retained in Git); 2026-10-24 v0.2 counterexample review of EFI handoff, watchdog, manifest TCB, root recovery, A/B floor and safe restarts. No code, key enrollment, actual boots, tests or benchmarks.

## 26. 2026-10-25 CAPIPC Receipt/Task death and M0 safety-case boundary — Proposed

**Joint study:** [CAPIPC Cross-RFC Safety Case v0.1](../reviews/capipc-cross-rfc-safety-case-v0.1.md). Kernel TaskGeneration and protected q/C/B/R[q] are not ServiceID, SABI RequestId, BootEpoch or persistent external-effect ledgers. Task death removes ordinary query authority but cannot roll back committed queue/cap exposure, DMA or durable storage effects. New ServiceIncarnation must not inherit old receipts/ReplyTokens from shared names. PENDING records/ReceiveClaim and outstanding hardware pins require independent custody and bounded M0-safe cleanup rather than expiration-driven refund. Supervisor post-death inspection requires separately narrow grants; no recovery proof or Accepted Boot/ABI changes.

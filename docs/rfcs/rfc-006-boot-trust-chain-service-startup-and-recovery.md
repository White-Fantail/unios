# UniOS RFC-006 — Boot Trust Chain, Service Startup & Recovery

> **Status:** Proposed — Specification v0.1 (미승인·미검증)
> **Date:** 2026-10-24 (NZDT)
> **Baselines:** [Architecture v0.2](../architecture/unios-architecture-v0.2.md), [RFC-001 v0.2](rfc-001-kernel-boundary-and-tcb.md), [RFC-002 v0.2](rfc-002-object-capability-lifetime-delegation-revocation.md), [RFC-003 v0.2](rfc-003-ipc-wire-protocol-and-synchronization.md), [RFC-004 v0.2](rfc-004-virtual-memory-pager-cache-and-oom.md).
> **Constraint:** Research and specification only. No executable loader, OS implementation, Secure Boot enrollment, tested recovery, new ABI, benchmark, formal proof, Accepted detailed design or approval to implement.

## 0. Executive conclusion — proposed BOOT-A

**BOOT-A: signed/verified minimal BootBundle + kernel-supplied typed root grants + independently resident minimal RecoveryAuthority + manifest-constrained normal ServiceSupervisor.**

The boot trust chain and the operating-system recovery chain have **different failure assumptions**. A signed UEFI executable does not verify all subsequently loaded kernel, M0 services or disk data. A crashed ordinary ServiceSupervisor cannot be trusted to restart itself. A running Pager does not imply storage durability or full memory-service recovery. A kernel safely denying unsafe operations does not imply desktop availability. These five non-equivalences are the central contract of this RFC.

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
| Fuchsia early component manager [S6] | critical component manager in root job may trigger whole-system reboot | service criticality sometimes requires controlled restart, not transparent self-healing |
| Android A/B updates [S7,S8] | trial slot, retry counters and successful status are separate metadata | launch success differs from accepted update; previous slot is conditional fallback |
| Android Verified Boot [S9] | A/B rollback metadata must not advance before new slot is marked successful | wrong ordering can make the old valid slot forbidden, stranding device |
| Chromium OS verified boot/recovery [S10,S11] | verified firmware/OS and separate recovery workflows | recovery artifact requires independent authorization, not merely older copy |
| TCG PC Client measurement model [S12] | firmware measurements/event log and PCRs support attestation | measurement/TPM log is not enforcement or image authorization |
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

A "signed image" is not synonymous with a "safe image"; the verified signature must bind exact bytes, trusted signer and accepted version/security floor under a **verified verifier**. UEFI may authorize a loader but cannot be assumed to authenticate the kernel/bootfs that loader reads later. A manifest's hash field without an authenticated chain is untrusted text.

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

**BootPolicyManifest (conceptual, not ABI):** schema version, image digest and signer/key identity, kernel ABI compatibility, release version and security floor, hardware/CPU/driver requirements, ServiceID/image digest and dependency types, initial resource/capability envelope, allowed device groups, service readiness and criticality, restart attempts/windows, allowed recovery/slot relationships and signed policy extension. All required fields must have strict parser behavior for duplicates/overflow/unknown required versions.

**Integrity rule:** the kernel/boot validator must not consume an attacker-replaceable policy file *after* trusting it simply because path/name matches. Signed manifest commits to service image contents or immutable verified backing; writable disk path alone cannot be a trust identity. Runtime executable file faults remain a verification issue for RFC-004/017.

**Policy rule:** an authorized image may still be malformed, compromised by a legitimate signing key or unsafe for a particular hardware profile. Use bounded parser and least-grant policy, no ambient DMA, debug or global TaskControl from a manifest string. Do not freeze hash algorithm, descriptor size or signature wire layout until RFC-017/018.

## 5. EFI to kernel handoff (x86_64 first)

The firmware/UEFI layer is outside UniOS privileged kernel but within the boot trust dependency. An EFI image's successful load does not make all physical memory immediately owned by UniOS.

**BootContext conceptual fields:** firmware memory map descriptors (including DescriptorSize and DescriptorVersion), kernel/resident image extents, reserved stack/page-table/critical allocations, validated system tables/ACPI information, selected boot slot and trust mode provenance, supported early console/GOP framebuffer, TPM/event-log pointer only if validated, boot epoch initialization inputs and explicit handoff-state flags.

### 5.1 GetMemoryMap, ExitBootServices and PMM

- Obtain latest memory map with correct MapKey. Firmware allocations can change the key between calls; do not allocate carelessly after obtaining the last map. A failed ExitBootServices may involve partial shutdown and requires strictly spec-compliant retry/failure behavior [S2].
- After successful ExitBootServices, UEFI Boot Services and their device protocols cannot be called. EFI Runtime Services are separately optional, with mapping/attributes/support/return failures defined, not a universal storage API.
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

A service's trusted *asset* impact overrides its marketing name; a compromised driver co-located with a benign compositor changes the effective TCB. Restart budget (per ServiceID+image+BootEpoch), bounded exponential backoff, circuit-breaker, deadline and escalation prevent infinite reboot/CPU/metadata cycles. If restart failed due to OOM, retry only with real reclaimed frame/queue reserve evidence, not merely another health notification.

A process existing != Ready, and Ready != healthy *forever*. Boot success must be based on defined checks and recovery classes rather than a permanently optimistic registry.


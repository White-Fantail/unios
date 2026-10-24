# UniOS RFC-007 — User-space Drivers, DMA & IOMMU

> **Status:** Proposed — Specification v0.1 (design research only; **not Accepted, implemented, tested, or proved**)
> **Date:** 2026-10-24 (NZDT)
> **Architecture:** [UniOS Architecture v0.2](../architecture/unios-architecture-v0.2.md)
> **Cross-RFC:** [RFC-001 Kernel & TCB](rfc-001-kernel-boundary-and-tcb.md), [RFC-002 Object Capabilities](rfc-002-object-capability-lifetime-delegation-revocation.md), [RFC-003 IPC](rfc-003-ipc-wire-protocol-and-synchronization.md), [RFC-004 Memory/Pager](rfc-004-virtual-memory-pager-cache-and-oom.md), [RFC-005 Scheduling](rfc-005-scheduling-preemption-and-cpu-budget.md), [RFC-006 Boot/Recovery](rfc-006-boot-trust-chain-service-startup-and-recovery.md).
> **No implementation authorization:** No hardware modified, no executable driver, no PCI enumerator, no IOMMU/page table configuration, no IRQ handler, no prototype, no DMA tests, no kernel code, no benchmarking and no source code imported.

## 0. Executive recommendation — DEV-A conditional research baseline

**DEV-A: kernel-enforced bounded MMIO/PIO/IRQ/DMA + userspace driver services + userspace device policy/topology with trusted group verification and privileged fail-safe hardware control.** A *DeviceGroup* is the smallest real physical isolation/ownership unit verified from topology, firmware/platform information and enabled IOMMU protections, not necessarily one PCI function. The driver cannot gain unrelated memory by raw physical addresses, a fake group ID, registering MSI, using a permissive default DMA domain, or obtaining an MMIO doorbell before isolation is ready.

Three candidate designs remain:
- **DEV-A — split control plane:** small privileged IOMMU/IRQ/PCI safety primitives and authoritative device-group assignment; authorized DeviceAuthority/BusManager userspace enumerates, applies policy, supervises typed grants and normal drivers. **Preferred research hypothesis**, not proven simplest.
- **DEV-B — trusted privileged driver/bus host for hard devices:** explicitly mark unisolatable DMA devices as part of broad hardware/software TCB, or disable them. Product may choose supported trusted storage/boot device exceptions; cannot advertise same security isolation as DEV-A.
- **DEV-C — broad kernel device stack / trusted hypervisor I/O layer:** stronger centralized ownership but substantial TCB and driver hardware complexity. Compare only if physical reference-PC device/interrupt safety requires it.

**Fail-closed rule:** A device may start CPU-independent DMA before its first driver command or continue after the driver exits. Kernel memory protection alone (MMU) is **not** a DMA boundary. A completed IOMMU unmap/invalidation is **not** itself proof that already issued DMA writes, device queues, posted transactions, peer-to-peer paths or device-private caches are drained. Driver death is **not** a quiescence signal; a software completion/fence from a dead or malicious driver is not physical evidence. Until an independent trusted fence/reset plus translation/cache invalidation completes, the affected DeviceGroup, IOVA reservations and physical frames must remain quarantined rather than reassigned.

**Minimum early hardware principle:** The chosen x86_64 UEFI reference PC may be less capable than an emulator. Initial real hardware work must identify actual IOMMU remapping enablement, PCI/ACS group topology, interrupt-remapping capability, reset granularity, device DMA masks, boot firmware residual DMA, storage boot pathway and group-specific isolation. QEMU/OVMF/VirtIO demonstrates only the configuration that was actually enabled/tested, and cannot certify another motherboard's isolation.

**Do not call this "safe userspace drivers" yet.** The entire document is a falsifiable technical specification proposal. Security invariants, P0 blockers, safety mode and feasibility gates must be met before a specific device is released to an untrusted DriverHost.

## 1. Scope, attacker and evidence goals

### 1.1 In scope

- PCIe/UEFI/ACPI device discovery and authoritative group formation; bus mastering and reset discipline;
- physical MMIO, port I/O and kernel-mediated device-object ownership;
- IOMMU translation domains, DMA frames/pins/IOVA, map/unmap, CPU-vs-device cache coherency, fences and secure reclamation;
- interrupt routing, MSI/MSI-X, interrupt-remapping and bounded user-level IRQ delivery/ack;
- device crash, stale completions, hotplug/removal, unsafe quiescence, fail-stop/recovery and inter-device sharing;
- security/power/error class of unsupported IOMMU/ACS, P2P, ATS/PRI/PASID and device reset;
- boot M0 essential storage/IRQ/Pager/driver dependency; capability R0/B/R1/R2-F/R2-O mapping;
- future model checking, fault injection and real-PC validation plan.

### 1.2 Out of scope / no implied promise

Full PCIe bus implementation or ECAM driver, ACPI AML interpreter, Thunderbolt/USB4 policy, NVMe command/queue driver, network/GPU/USB/storage stack, concrete chipset errata workarounds, modern PCIe security protocol compliance, display compositor or stable native I/O ABI. No guarantee all devices can be isolated, reset, recovered or are safe; no guaranteed performance, device support catalog or consumer production security level.

### 1.3 Threat model

**Untrusted actors:** a malicious app or compatibility guest, compromised user driver, forged service registry device name, maliciously shaped or stale DMA descriptor, unauthenticated device firmware/update, misreported completion, interrupt flood, hostile PCIe endpoint, untrusted peripheral on hotplug bus, or DMA-lifetime race. A legitimate authorized DriverHost remains a trusted party *for its own data and device operation*, but must not become global CPU/MMU/IOMMU or other-device authority.

**External trust assumptions:** CPU/IOMMU and interrupt-remapping hardware obey claimed isolation; firmware/ACPI tables and PCI topology correctly reflect actual pathways; trusted boot chain established (RFC-006/017); PCIe/CPU interconnect and chipsets implement routing and security controls; secure IOMMU protection is in effect before any untrusted DMA is possible. Compromised system firmware, SMM, rogue physical endpoint or malicious remapping hardware are NOT generally fixed by Ring 3 driver separation. A hardware isolation failure forces a weaker product profile or device denial.

**Asset-specific TCB:** kernel DMAR/IVRS or equivalent discovery validation and IOMMU/IRQ authoritative programming (TCB-K); BootAuthority/DeviceAuthority/BusManager supplying verified device grants and driver restart (TCB-B/S); hardware/firmware assumptions outside UniOS. "Userspace" is neither synonymous with untrusted nor automatically secure.

## 2. Official primary-source comparison

| Official source | Existing evidence | Consequence for UniOS (not a claim of implementation) |
|---|---|---|
| Linux VFIO [S1] | secure direct user access uses actual **IOMMU groups**; PCI multifunction/bridge/ACS limitations can make multiple devices inseparable | DeviceGroup must be topology-verified, never fabricated by a per-function string or assumed independent |
| Linux iommufd [S2] | I/O address space, device ownership and hardware I/O page-table attach are distinct operations | separate DeviceGroup, IOMMUDomain, DMA map/pin and attachment provenance; kernel remains authoritative |
| Linux DMA mapping guide [S3] | streaming/coherent DMA semantics differ; explicit sync, ordering barriers and direction matter | a mapping/pin doesn't make CPU/device caches coherent or prove transfer complete |
| Linux PCI P2P DMA [S4] | PCIe switches and ACS can route peer traffic without visiting root/IOMMU path | verify P2P containment / topology; no automatic universal IOMMU protection |
| Linux PCI device reset API [S5] | FLR, function/bus reset availability and effects depend on device and scope | any promised restart/reset must state exact reset granularity and effect on sibling functions |
| seL4 platform/FAQ [S6] | seL4 drivers run in userspace; formal kernel verification assumes MMU protection and DMA off, requiring separate assurance for DMA devices | user driver isolation proof ≠ proof of DMA safety |
| seL4 hardware I/O manual [S7] | IRQHandler grants, user notifications and x86 IO space/page-table capabilities | minimal kernel IRQ/IOMMU control is plausible but not sufficient without group and device semantics |
| Linux x86 IOMMU [S8] | DMA IOVA mapping lifecycle and exceptions including interrupt/P2P routing | MSI/interrupt remapping and IOVA map alone must not be conflated |
| Linux DMA buffer sharing/fences [S9] | dma-buf, async fences and reservations coordinate shared-device work | software completion fence ≠ trusted hardware bus quiescence, distinct scopes |
| Linux IOMMUFD user API current [S10] | nested hardware page tables, PASID attachments and cache invalidation have distinct lifetimes | advanced ATS/PRI/PASID/SVA disabled from initial profile pending exact security proof |
| Linux x86 Shared Virtual Addressing [S11] | SVA/PASID binds device to process memory and uses ATS/PRI/TLB invalidations | guest VA/CPU PTE invalidation alone cannot close device-side translations |
| Linux DMA attributes [S12] | cache synchronization and buffer-sharing contracts depend on ownership/direction | per-device mapped ownership and alias/access conflict must be modeled |
| Linux VFIO group cdev ownership [S13] | modern per-device cdev still respects **one DMA owner per group** | API shape does not waive group isolation requirements |
| UEFI Boot Services [S14] | firmware DMA/device services can be active before ExitBootServices; boot services end after exit | UniOS must define safe device handoff, not assume new IOMMU protection existed pre-boot |

**Do not overread source wording:** Linux VFIO's current no-IOMMU mode is explicitly unsafe, not evidence it can safely host hostile PCIe hardware. seL4's proof assumptions do not automatically transfer to UniOS. A Linux dma_fence is a software synchronization object whose correctness depends on participating driver/device; it is not a proof against malicious bus masters.

## 3. Physical isolation inventory and device-group truth

### 3.1 Discovery evidence and group computation

A trusted physical group assignment needs:
1. Firmware/UEFI/ACPI DMAR/IVRS-or-platform IOMMU information and validation of integrity, length, overlaps and DMA-remapping hardware identities;
2. PCI hierarchy: segment/bus/device/function, bridge/root port, bus numbering, requester IDs, multifunction functions, integrated devices, hotplug slots, ACS capabilities/settings and translation features;
3. Actual routing through enabled IOMMU(s), bridge or peer paths and PCIe ACS (not merely reading a Boolean flag);
4. MSI and interrupt-remapping coverage and control over interrupt generation/registration;
5. a documented power/reset topology, firmware-reserved devices and platform-critical components;
6. explicit hardware/evidence profile tag and generation. If unknown, group into a more conservative shared isolation set, or deny direct nontrusted access.

**Important distinction:** a *PCI function* identifies one BDF address; *IOMMU Group* is the set of devices mutually nonisolatable under actual hardware; *IOMMU Domain/IOAS* is translation state which may be shared by authorized groups; *DriverHost* is a user process. These identities are never interchangeable. A single function can have more than one associated DMA requester/SVA context on modern systems; this is **not** early-profile proof of independent trust. VFIO's device-file descriptor does not waive group membership [S1,S13].

### 3.2 Initial probe and classification

| Profile | Capability | Direct untrusted user-space driver? | Expected result |
|---|---|---|---|
| **HW-0 No usable IOMMU** | I/O MMIO/PIO and maybe CPU MMU, no trusted DMA remap | **NO** for DMA bus masters; PIO-only/non-bus-master subset evaluated individually | deny DMA device or knowingly trust whole hardware path, never market as isolated |
| **HW-1 IOMMU exists, group topology/ACS or interrupt remap unproven** | some DMA translation | **NO** until hardware-specific group+MSI threat review | conservative grouping, disable risky DMA/P2P features and verify |
| **HW-2 Verified group-aware DMA remap and IRQ safety** | assigned groups+domains with tested isolation and reset limits | candidate YES in approved profile only | bounded per-group grants, pin/maps/fences and fault audit |
| **HW-3 Advanced ATS/PRI/PASID/SVA/P2P/GPU** | shared VA, peer memory/device translation caches | **Deferred** pending independent model/hardware proof | support later only with device-cache invalidation and trust model |
| **HW-T Trusted-only exception** | no practical isolation, product accepts a chosen hardware/driver in TCB | Not called an isolated untrusted driver | documented weaker security class; scope limited, possible production exclusion |

**No-IOMMU caveat:** moving an unsafe device's DMA into a bounce buffer does not isolate a malicious device that can address arbitrary physical memory without IOMMU. A bounce copy may help DMA masks/performance/compatibility, not provide a memory protection boundary. Pure non-bus-master devices may be considered separately if their hardware behavior genuinely prevents DMA.

### 3.3 Recompute on topology change

Hotplug, function reset, re-enumeration, firmware/ACPI changes, a bridge becoming noncompliant, ACS disabled or a multi-function sibling entering a new driver state can invalidate the group authorization. Deny further grants until secure topology verified; do not assume a once-issued DeviceGroup capability remains correct across arbitrary hardware generations. Locking/configuring ACS can itself require trusted privileged actions; exposing raw PCI config writes lets a driver defeat the isolation model.

## 4. TCB and component responsibility

| Operation | Proposed kernel enforcement | User-space authorization/policy | Remaining hardware risk |
|---|---|---|---|
| PCI probe topology | access-controlled privileged config read, table validation where required, atomic group owner/security state | BusManager enumerates/proposes group graph | false firmware tables, stealth bridges, device quirks |
| DeviceGroup assign | exclusive group owner state and delegation gate, typed rights, generation | DeviceAuthority policy decides which named DriverHost | PCI P2P, ACS, reset coupling |
| MMIO map/PIO | checked BAR/port span, cache attributes, CPU mapping and revocation | driver protocol and queue layout | registers may enable DMA/reconfigure group |
| PCI configuration | restrictive privileged gates on BAR, bus-master, MSI, ACS, IOMMU-sensitive config | BusManager approved init and reset recipe | device may ignore expected reset semantics |
| IOMMU domain | own hardware-translation safety mechanism or tightly trusted equivalent, deny-by-default | scoped translation request for pinned object/range | stale IOTLB/ATS device cache and P2P |
| DMA memory | kernel frame owner, pin, IOVA mapping and lifetime record; no PFN authority from integers | driver chooses approved buffer/IOVA requests | device may DMA while driver process absent |
| IRQ/MSI | vector routing, interrupt remap, ack/mask, grant provenance, bounded notification | driver services designated IRQ messages | IRQ spoof/flood and MSI bypass |
| Quiesce/reset | control-gated bus master stop/fence and conservative hardware evidence | DeviceAuthority orchestration and error decisions | bus-level reset affects siblings, I/O in flight |
| Service crash/restart | kernel cap B/R1 and DMA R2-F/R2-O gate; prevent stale IRQ/IOVA reuse | Supervisor launches fresh incarnation under approved manifest | lost dirty storage writes or device irrecoverability |

No blanket policy that the entire PCI allocator and complex bus protocol must reside in Ring 0. But placing it in userspace cannot permit raw unsigned config/port writes able to override IOMMU group membership, bus-master denial, MSI/interrupt remapping or firmware-critical registers. The privileged *security-relevant configuration subset* must be controlled even if a broader PCI service remains userspace.

## 5. Authority and object taxonomy (candidate, not native ABI)

| Logical object | Permissions (illustrative, subject to RFC-002) | Lasting resource/lifetime conditions |
|---|---|---|
| DeviceGroupResource | GROUP_OBSERVE, ASSIGN, RELEASE, RESET_SCOPE, QUARANTINE | actual group ID/topology epoch, exclusive owner and no unsafe sibling |
| DeviceFunctionView | CONFIG_SAFE, MMIO_MAP_RANGE, QUERY_BAR, OBSERVE_FAULT | subordinate to group; not independent unrestricted DMA ownership |
| IOMMUDomain / IOAddressSpace | DOMAIN_ATTACH, MAP_IOVA, UNMAP_IOVA, SYNC_INVALIDATE | group attach, IOTLB/ATS cache invalidation and mapping refs |
| DmaRegion / DmaPin | PIN_MEMORY, DMA_READ, DMA_WRITE, SHARE_AUTHORIZED | authorized MemoryObject/frame references and charge; non-reclaim until safe fence |
| DmaMapping | IOVA range, group/domain, direction, generation and mapping rights | no stale alias or reuse after unmap while device may access |
| IrqSource / IrqBinding | BIND_IRQ, MASK, ACK, UNBIND, WATCH | group owner and generation; interrupt remap and level semantics |
| MmioWindow / IoPortWindow | CPU_MAP, READ, WRITE scoped cache type/range | cannot write protected bus-master/ACS/MSI config indirectly |
| DeviceControlGrant | privileged assignment/reset/config class, restricted scope | revocation/lineage persists beyond Task handle close |
| DeviceResetFence / QuiescenceRecord | trusted ordered hardware reset/invalidation stages | never fabricated by a driver numeric cookie alone |
| DmaFaultReport | typed fault source, IOVA, access type, group/domain gen | advisory observation vs authoritative kernel hardware fault |
| DeviceBudget | pinned bytes, map metadata, IOVA space, IRQ credits/reset attempts | sponsor charges persist during quarantine and crash |
| DeviceHealth / DriverIncarnation | user service progress and restart state | not hardware DMA quiescence evidence by itself |

A DriverHost can receive a DeviceFunctionView/MemoryObject but **not a broad DeviceGroupResource ASSIGN** merely from possession of a sibling function or a ServiceID. Create privileges and budget are separate. Integer BDF, physical MMIO/PA, IRQ vector, IOVA and API-provided cookie are **locators, not kernel capabilities**.

### 5.1 Revocation ownership

- R0: close a Task-local handle; other mappings, IOMMU translations, DMA engine and descendants remain potentially live.
- B: deny new device-cap invocations/grants under revoked family; possibly freeze submissions via privileged control. Not proof previously submitted DMA stopped.
- R1: remove CPU MMIO/mapping access through applicable grant, synchronize CPU TLB and prevent new MMIO/bus-master config. Does not erase device caches/old DMA.
- R2-F: specific revoked family has no *continued physical DMA/IRQ/use effects* after trusted device disable/fence/reset, IOMMU/device invalidation and pending-access barrier under a stated hardware profile. May require group quarantine if unprovable. **This is a property of verified physical stop**, not just control-register write.
- R2-O: object-wide reclamation requires *all independent roots*, mappings/pins/IRQ references/other-device access and R2-F conditions drained; one family's R2-F does not imply it. Zeroing/scrubbing before memory moves to another security owner remains RFC-004.

**Rights lifecycle:** a DriverHost can die but its kernel DeviceGroup owner lease, grant controller, pinned pages and charge cannot evaporate. On death, owner is moved to a quarantined state under a privileged recovery actor until verified release; old endpoints/IRQs/grants are never auto rebound to the new process by name. A service restart does not release outstanding DMA for free.

## 6. DeviceGroup state machine and exclusive assignment

~~~text
FirmwareOwned / Unknown
 -> Discovered (no untrusted DMA)
 -> TopologyValidated (group/ACS/IRQ/IOMMU evidence)
 -> BlockedDomainInstalled (default deny; no bus-master acceptance)
 -> AuthorizedGroupAdmitted (owner, budget, mandatory shared siblings)
 -> Prepared (safe config + pinned buffers + IOVA + IRQ)
 -> Active (enabled bus-master within scoped domain)
 -> Quiescing (new work denied, IRQ masked, DMA stop requested)
 -> Fenced (trusted device and caches quiesced)
 -> Detached (domain/IOVA retired, device owners drained)
 -> Reusable or Quarantined/Offline/Fatal
~~~

**Not all actual firmware boot paths begin at a safe FirmwareOwned DMA-off state.** UEFI/storage/network firmware may already have enabled bus mastering and left buffers or device transactions. The firmware-to-kernel handoff, before fully trusted remapping is installed, is a **separate exposure window**. The requested security profile must either establish early DMA protection with platform evidence or explicitly treat that interval as unprotected external trust. Later IOMMU initialization does not retroactively repair corruption that occurred earlier.

### 6.1 DeviceGroup assignment transaction

1. Validate current topology/ACPI/IOMMU routing, firmware reservation, group epoch and no conflicting owner; deny unsupported hardware state.
2. Ensure a **blocked/default-deny IOMMU domain** is installed as the device attachment (or establish trusted equivalent). Do not enable bus mastering or grant direct MMIO/PCI config before this barrier.
3. Atomically charge group owner, pinned memory allowance, map/IRQ quotas and named DriverHost image/incarnation rights against validated manifest and sponsor.
4. Validate **all functions/siblings in the group**, reset and bus-master implications. A trusted kernel driver cannot simultaneously own another member in a way that lets userspace bypass DMA group ownership.
5. Create scoped Function/Domain/IRQ/MMIO/port grants; disallow unsafe config writes that alter ACS/root port remap or MSI/IRQ ownership.
6. Optionally allow bus mastering only after required IOVA mappings, coherency and IRQ routing are established, under a verified device profile.
7. Commit group Active state and publish service access. Failure at any point releases only **proven unused/unexposed** resources; once hardware may DMA, failure requires quiescence or quarantine rather than an imaginary atomic rollback.

### 6.2 Multi-function/shared group example

NIC function F0 and a sibling F1 may share a nonisolatable PCIe path. Even if only F0 is handed to an untrusted DriverHost, F1 running as a trusted native/firmware/storage driver may route DMA or shared device state in a way that collapses the boundary. Treat the whole group as one owner security decision, or deny direct assignment. One IOMMU page table can serve multiple authorized independent groups, but this is an explicit trust-domain sharing choice that widens DMA reach.

### 6.3 Security-relevant PCI configuration writes

Access to configuration space can change bus-master bit, decode MMIO, MSI/MSI-X tables, BAR windows, ATS enablement, ACS, SR-IOV virtual functions and reset state. Thus **raw unrestricted PCI config access** cannot be given to untrusted DriverHost even when its MMIO span seems constrained. Device-specific MMIO registers can themselves instruct DMA or alter on-device address translation; trusted domain and mapped frame scope are the ultimate protection boundary.

## 7. DMA buffer ownership, IOVA mapping and cache coherence

### 7.1 Distinguish five addresses/rights

- VA (Task virtual address): User mapping under kernel CPU page tables; **not** an IOVA and may disappear while physical buffer stays pinned.
- Physical frame (PA/PFN): kernel tracks real RAM and reserved holes; raw integer PFN not authorization.
- MemoryObject/DmaPin: eligible owned backing RAM and immutable pinned reference while device may access.
- IOVA: device-visible address in an attached IOMMU domain; can be reused only after all relevant stale device translations/operations safely retired.
- Device CPU MMIO: control-register aperture for host driver; does not grant arbitrary DMA MemoryObject authority.

Pinning can outlive process/handle death. Mapping *the same MemoryObject into multiple groups* creates multiple independent device access lifetimes. Closing one mapping does not stop other groups, and freeing one pin cannot release the last shared frame unless **all CPU and DMA references** satisfy RFC-004/RFC-002 lifetime requirements.

### 7.2 Direction and lifetime

Candidate DmaRegion rights include DEVICE_READ (memory-to-device), DEVICE_WRITE (device-to-memory) and bidirectional, with device-specific DMA mask, length/alignment/boundary, IOVA range and mapping generations. A WRITE-capable mapping cannot be made through a READ-only backing by pretending directions are advisory. Validate offset/length overflow and sub-page permissions: if IOMMU maps an entire granule, placing sensitive unrelated bytes in the same granule can leak or corrupt memory.

**Safe grant proposal:** isolate DMA-specific frames or bounce regions when page granular mapping would expose neighboring data, charge extra copies and avoid declaring the bounce *a security solution for a device without IOMMU*. A malicious driver cannot specify arbitrary physical addresses or map privileged kernel memory as a DMA buffer via a fake object ID.

**Mapping lifetime records** must link group+domain+IOVA range+physical frames+grant lineage+sponsor+generation+device generation. PIN_MEMORY on a revocable MemoryObject does not by itself create an eternal DMA privilege; attempts to revoke must either fence and release or report pending unsafe effect and quarantine.

### 7.3 CPU-device coherence is a separate synchronization protocol

- Coherent mappings allow particular CPU/device cache visibility properties, **not** ordered command descriptor writes without proper barriers [S3].
- Streaming mappings need explicit CPU→device and device→CPU ownership transfer/synchronization. A READ of a DMA-written page before cache synchronization can observe stale contents; a CPU write while device writes can corrupt shared data.
- For noncoherent devices/architectures, cache maintenance and partial cache-line sharing must be analyzed. An x86_64 assumption of general coherency cannot be generalized to every future device or platform; PCIe/device caches and posted writes differ from CPU cache coherence.
- Mapping/unmap TLB invalidation is **not** device completion. DMA completion/fence must be scoped to actual queues, outstanding write paths, device reset and all relevant caches. A driver-set software completion flag without independent hardware semantics is insufficient against a malicious endpoint.
- Sharing with GPU/network/storage requires per-device access rights, explicit fences for content readiness and independent hardware quiescence for frame recycling. A Linux dma_fence is not a magical physical group shutdown [S9].
- Runtime Pager/file-cache dirty/writeback state is not equivalent to a device completed DMA read/write; persistence is a storage barrier problem in RFC-008.

### 7.4 DMA map/update/unmap candidate transaction

~~~text
Authorize DeviceGroup + MemoryObject range + direction + sponsor
  -> charge region/map/pin metadata
  -> pin physically owned frames, reject unsafe overlap/alias
  -> select IOVA within domain and device addressing mask
  -> program IOMMU mappings and synchronize IOMMU device caches
  -> publish scoped DmaMapping/IOVA to owning DriverHost
  -> (per driver) prepare descriptors, CPU/device ownership barriers
  -> enable submission only under current owner/group generation
  -> transfer may be in flight or outlive DriverHost
  -> stop submissions; verify device/queue quiescence and required sync
  -> remove translation + IOTLB/ATC invalidation with completion
  -> release pin/ref only after R2-F; otherwise Quarantined
~~~

**Unmap timeout is not success.** An invalidation API returning an error or unavailable reset means device translations and/or older bus transactions may survive. Keep unsafe frames pinned and unavailable for cross-domain reuse. Drivers may be malicious and hide pending work. Implementations must specify trusted hardware evidence at the chosen device profile, not accept user-space "I am done" status as proof of physical safety.

## 8. IOMMU, PCI P2P, ATS/PRI/PASID and DMA remapping security

### 8.1 Initial IOMMU policy

Proposed initial default: no identity mappings or broad passthrough domains for untrusted groups; only declared pinned pages in explicitly created translation domain(s). Protect kernel text/page tables, M0 recovery reserve, other Tasks, firmware runtime and unrelated device buffers from any untrusted IOVA. Avoid leaking real physical addresses when possible, but confidentiality of PA alone is not the security boundary.

An IOMMU remap may not isolate peer-to-peer PCIe transactions if routing under switches/bridges bypasses host IOMMU or ACS is absent/misconfigured [S1,S4]. The group graph and P2P security class must account for such devices. A malicious device that programs MSI messages also attacks interrupt delivery if interrupt remapping is absent/disabled [S8].

### 8.2 Deferred advanced features

- ATS (Address Translation Service) can cache translations at device. Ordinary CPU TLB flush or IOMMU table edit alone may not invalidate a device's ATC.
- PRI (Page Request Interface) permits device page fault requests and may couple a device's progress to Pager/MemoryObject availability, adding boot/cycle and malicious fault flooding concerns.
- PASID/SVA can bind device operations to process address spaces or nested mappings. That increases the authorization graph (Task, group, PASID and page table generation) and rollback complexity.
- SR-IOV virtual functions and mediated devices may share resets, firmware and device capability, so cannot be assumed independently isolated by function names.
- P2P device-to-device DMA bypassing RAM still requires an isolation/owner policy and may affect other groups.
- PCIe hotplug/USB4/Thunderbolt and GPU command processors raise external DMA exposure and firmware attack surface.

**Recommendation:** early group profile disables ATS/PRI/PASID/SVA/nested IOMMU and unapproved P2P even if the silicon supports them. Re-enable only with a profile showing group/requester attribution, cache/device invalidation ordering, reset and denial on error. Support is not granted merely by a feature bit. Linux modern IOMMUFD treats device, IOAS, nested HWPT, PASID and invalidation as distinct controls [S10,S11].

## 9. IRQ, MSI/MSI-X and notification lifecycle

### 9.1 Trusted IRQ objects and routing

IRQ authority is group/device-scoped, not user-chosen global APIC vector. Kernel mediates interrupt source binding, masking, acknowledgment and mandatory secure remapping for untrusted MSI/MSI-X where supported. Driver can receive an IRQ object/notification, not arbitrary ability to redirect interprocessor interrupts or privileged vectors.

**Interrupt remapping matters independently from DMA translation.** x86 IOMMU presence does not automatically establish working interrupt-remapping protection on every firmware or chipset. On unavailable/uncertain interrupt remap, deny untrusted MSI assignment or explicitly restrict device/profile. MSI messages, IRQ delivery, CPU wake and device signal tables need current group epoch/generation.

### 9.2 IRQ state machine

~~~text
Unbound -> AuthorizedMasked -> Armed
  -> Pending (hardware interrupt, bounded kernel notification)
  -> MaskedAwaitAck / CoalescedPending
  -> ReArmed on correctly scoped ack
  -> Quiescing -> Detached or Quarantined
~~~

Level-triggered interrupts require coordination so a continuously asserted line does not produce an endless storm. MSI/MSI-X may need different rearming semantics. Mask/ack must not be confused with software RPC reply or physical DMA completion. A lost or delayed ack can block service while the kernel remains safe; the relevant driver may be restarted or isolated but no guaranteed availability is claimed.

On Task death or cap B: stop new bind/ack authority, mask/disable source using privileged controls, retire old IRQ binding generation and account queued notifications; do not deliver old notifications into a replacement DriverHost. A stale IRQ vector value is not a capability.

### 9.3 IRQ flood and scheduler

- Short bounded top-half in privileged code; deferred driver handling runs under service's CPU ResourceGroup (RFC-005), not free CPU time.
- Hardware mask/ack paths need wired minimal resources if used by M0. Ordinary app or untrusted driver cannot spend the entire emergency IPC/CPU reserve by flooding IRQs.
- Acknowledge priority may matter for progress but cannot override unsafe device-group ownership or grant global Kernel IRQ rights.
- A crashed user driver cannot be allowed to block all critical timer/reschedule IPIs merely by failing to ack a distinct PCI IRQ. This needs actual physical interrupt-controller routing proof.
- A source spamming MSI despite driver shutdown may require privileged mask/remapping disable/reset and quarantine. Merely removing the user-space callback does not stop physical interrupts.

## 10. Device reset, DMA drain and trusted quarantine

**Four separate events:** (A) user DriverHost stops, (B) new command submission is denied, (C) outstanding hardware DMA operations are physically quiescent, and (D) IOMMU/device cache translations and CPU mappings are invalidated sufficiently for reassign/reuse. Neither A nor B alone implies C; C without D can retain unsafe stale IOVA-to-frame translation when hardware restarts.

### 10.1 Quiesce candidate ladder

1. **Revoke new commands/grants B** and stop user driver IPC/doorbell rights. Existing doorbells/DMA already submitted may remain.
2. **Mask IRQ and disable new bus-master submissions** under privileged control, noting device-specific command queues and chipset bridge behavior.
3. **Device-specific orderly drain** when accessible, bounded; do not trust malicious service alone to report completion.
4. **Hardware reset** at exact proven scope—FLR if supported, device reset, secondary bus reset only when it will not silently disrupt unowned siblings; some devices have no usable reset and may remain nonresponsive.
5. **IOMMU/ATC invalidation** completion and any outstanding-device-access ordering proof necessary to assert R2-F; CPU TLB/R1 separately.
6. **Fence evidence verified:** device group/epoch and all DMA pins/IOVA regions are safe for explicitly authorized next generation. Only then refund/reallocate affected memory and DeviceGroup; otherwise Quarantined/Offline/fail-stop/reboot.
7. **If reset stops storage/Pager path** needed for M0, fail boundedly or invoke truly independent recovery path. A software restart cannot be assumed sufficient when the only boot storage controller is unresponsive.

### 10.2 Device reset hazards

An FLR attempt is not an FLR guarantee. Some PCIe functions do not implement individual function reset, bridge bus reset affects children, and an unexpected reset may invalidate MSI, BARs or device state needed for subsequent driver initialization [S5]. Config-space availability itself can fail while device is hung. A completed IOMMU invalidation does not retroactively revoke writes that occurred before the fence or guarantee recovery of corrupted kernel/file memory. Firmware may be controlling a device even after user driver exit if firmware/runtime services or SMM take ownership: product trust profile must include that possibility.

### 10.3 Quarantine semantics

**Quarantine keeps *affected* physical frames and group resources unavailable for unsafe reuse** until a trusted fence; it does not necessarily require freezing every unrelated software service or rebooting a healthy device. Pin/map metadata, sponsor debt, lineage and group generation remain in kernel even if user process has died. Quarantine may last until reboot or operator recovery and can consume real physical RAM—OOM policy must explicitly count quarantined memory and refuse new workloads rather than silently reclaim.

**Hard limit caveat:** if no real IOMMU or trustworthy reset and DMA hardware can target arbitrary RAM, "quarantining a few buffers" does not confine its future writes. The only honest options can be trusted-only device, physical disable/power removal, full fail-stop, or refusing to boot in the stronger security profile. Kernel software cannot quarantine against arbitrary DMA that bypasses its protection.


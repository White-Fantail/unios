# UniOS RFC-007 — User-space Drivers, DMA & IOMMU

> **Status:** Proposed — Specification v0.2 (adversarial re-review; **not Accepted, implemented, tested, or proved**)
> **Date:** 2026-10-24 (NZDT)
> **Architecture:** [UniOS Architecture v0.2](../architecture/unios-architecture-v0.2.md)
> **Cross-RFC:** [RFC-001 Kernel & TCB](rfc-001-kernel-boundary-and-tcb.md), [RFC-002 Object Capabilities](rfc-002-object-capability-lifetime-delegation-revocation.md), [RFC-003 IPC](rfc-003-ipc-wire-protocol-and-synchronization.md), [RFC-004 Memory/Pager](rfc-004-virtual-memory-pager-cache-and-oom.md), [RFC-005 Scheduling](rfc-005-scheduling-preemption-and-cpu-budget.md), [RFC-006 Boot/Recovery](rfc-006-boot-trust-chain-service-startup-and-recovery.md).
> **No implementation authorization:** No hardware modified, no executable driver, no PCI enumerator, no IOMMU/page table configuration, no IRQ handler, no prototype, no DMA tests, no kernel code, no benchmarking and no source code imported.

## 0. Executive recommendation — DEV-A conditional research baseline

**DEV-A: kernel-enforced bounded MMIO/PIO/IRQ/DMA + userspace driver services + userspace device policy/topology with trusted group verification and privileged fail-safe hardware control.** A *DeviceGroup* is the smallest real physical isolation/ownership unit verified from topology, firmware/platform information and enabled IOMMU protections, not necessarily one PCI function. The driver cannot gain unrelated memory by raw physical addresses, a fake group ID, registering MSI, using a permissive default DMA domain, or obtaining an MMIO doorbell before isolation is ready. **Caveat:** a raw MMIO register span may itself control bus master or peer routing and require mediation/deny; a safe-looking BAR aperture is not always a safe untrusted grant.

Three candidate designs remain:
- **DEV-A — split control plane:** small privileged IOMMU/IRQ/PCI safety primitives and authoritative device-group assignment; authorized DeviceAuthority/BusManager userspace enumerates, applies policy, supervises typed grants and normal drivers. **Preferred research hypothesis**, not proven simplest.
- **DEV-B — trusted privileged driver/bus host for hard devices:** explicitly mark unisolatable DMA devices as part of broad hardware/software TCB, or disable them. Product may choose supported trusted storage/boot device exceptions; cannot advertise same security isolation as DEV-A.
- **DEV-C — broad kernel device stack / trusted hypervisor I/O layer:** stronger centralized ownership but substantial TCB and driver hardware complexity. Compare only if physical reference-PC device/interrupt safety requires it.

**v0.2 critical corrections:** (i) ACPI reserved DMA regions / RMRR/IVRS can conflict with blanket default-deny, requiring a constrained firmware-trust exception or rejecting the device; (ii) DMA isolation group, hardware-reset collateral group, interrupt domain and actual resource sharing are different graphs; (iii) driver process/queue stop, IOMMU invalidation, pending bus transactions, software synchronization and actual physical memory-reuse eligibility need separate scoped evidence; (iv) direct MMIO can defeat filtered PCI configuration if a device exposes equivalent controls; (v) hardware with an active rogue bus master and no effective remapping may be **unsafe even to continue the rest of the desktop**. None of these gates is verified.

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
| Linux VFIO [S1] | secure direct PCI access relies on **IOMMU groups**; ACS/bridge/multifunction can make devices inseparable; even VFIO cdev keeps group DMA-owner exclusivity | verified group is a *minimum assignment unit for a given routing topology*, not itself proof all side channels, firmware DMA or reset effects are isolated |
| Linux iommufd [S2] | I/O address space, device ownership and hardware I/O page-table attach are distinct operations | separate DeviceGroup, IOMMUDomain, DMA map/pin and attachment provenance; kernel remains authoritative |
| Linux DMA mapping guide [S3] | streaming/coherent DMA semantics differ; explicit sync, ordering barriers and direction matter | a mapping/pin doesn't make CPU/device caches coherent or prove transfer complete |
| Linux PCI P2P DMA [S4] | PCIe switches and ACS can route peer traffic without visiting root/IOMMU path | verify P2P containment / topology; no automatic universal IOMMU protection |
| Linux PCI device reset API [S5] | FLR, function/bus reset availability and effects depend on device and scope | any promised restart/reset must state exact reset granularity and effect on sibling functions |
| seL4 platform/FAQ [S6] | seL4 drivers run in userspace; formal kernel verification assumes MMU protection and DMA off, requiring separate assurance for DMA devices | user driver isolation proof ≠ proof of DMA safety |
| seL4 hardware I/O manual [S7] | IRQHandler grants, user notifications and x86 IO space/page-table capabilities | minimal kernel IRQ/IOMMU control is plausible but not sufficient without group and device semantics |
| Linux x86 IOMMU [S8] | ACPI DMAR/IVRS enumerates hardware; **RMRR** can require restricted unity identity DMA mappings for firmware devices | strict default-deny requires explicit reserved-memory and firmware ownership exception audit, not implicit broad identity map |
| Linux DMA buffer sharing/fences [S9] | dma-buf, async fences and reservations coordinate shared-device work | software completion fence ≠ trusted hardware bus quiescence, distinct scopes |
| Linux IOMMUFD user API current [S10] | nested hardware page tables, PASID attachments and cache invalidation have distinct lifetimes | advanced ATS/PRI/PASID/SVA disabled from initial profile pending exact security proof |
| Linux x86 Shared Virtual Addressing [S11] | SVA/PASID binds device to process memory and uses ATS/PRI/TLB invalidations | guest VA/CPU PTE invalidation alone cannot close device-side translations |
| Linux DMA attributes [S12] | cache synchronization and buffer-sharing contracts depend on ownership/direction | per-device mapped ownership and alias/access conflict must be modeled |
| Linux VFIO group cdev ownership [S13] | modern per-device cdev still respects **one DMA owner per group** | API shape does not waive group isolation requirements |
| UEFI Boot Services [S14] | boot services may stop on ExitBootServices but that is not a hardware DMA drain acknowledgement | boot firmware-to-kernel handoff and hardware quiescence are separate |
| UEFI PCI Bus Support [S15] | EFI_PCI_IO_PROTOCOL Map/Unmap and required completion/flush ordering for bus-master DMA | EFI itself distinguishes device completion, posted-write flush and unmap; do not claim ExitBootServices automatically drained every transaction |
| Linux PCI reset and group coupling [S5,S16] | function reset may be unavailable and bridge reset can affect several functions | isolation group and reset collateral set need independent graph; reset success must be scoped to exact affected hardware |

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
7. **separate reset-coupling inventory:** what other devices/functions/storage controllers are lost when function/bridge/bus/power reset is performed; do not merge this with IOMMU grouping merely because they overlap.
8. **firmware reserved region inventory:** ACPI DMAR RMRR/AMD IVRS exclusions, access to SMM/firmware reserved RAM and possible mandatory identity mappings, with validated scope and explicit protected-memory exclusion.

**Important distinction:** a *PCI function* identifies one BDF address; *IOMMU Group* is the set of devices mutually nonisolatable under actual hardware; *IOMMU Domain/IOAS* is translation state which may be shared by authorized groups; *DriverHost* is a user process. **ResetCouplingSet**, **IRQ/InterruptRemappingDomain**, **P2PRouteGroup** and **FirmwareReservedMapSet** are separate overlays, not automatically identical to IOMMU Group. A group is the minimum VFIO-like DMA *ownership* granule under supported PCI topology, not a universal guarantee of safe device firmware or reset/IRQ containment. These identities are never interchangeable. A single function can have more than one associated DMA requester/SVA context on modern systems; this is **not** early-profile proof of independent trust. VFIO's device-file descriptor does not waive group membership [S1,S13].

### 3.2 Initial probe and classification

| Profile | Capability | Direct untrusted user-space driver? | Expected result |
|---|---|---|---|
| **HW-0 No usable IOMMU** | I/O MMIO/PIO and maybe CPU MMU, no trusted DMA remap | **NO** for DMA bus masters; PIO-only/non-bus-master subset evaluated individually | deny DMA device or knowingly trust whole hardware path, never market as isolated |
| **HW-1 IOMMU exists, group topology/ACS or interrupt remap unproven** | some DMA translation | **NO** until hardware-specific group+MSI threat review | conservative grouping, disable risky DMA/P2P features and verify |
| **HW-2 Verified group-aware DMA remap and IRQ safety** | assigned groups+domains with tested isolation and reset limits plus RMRR/reset-coupling review | candidate YES in approved profile only | bounded per-group grants, pin/maps/fences and fault audit; no unsupported firmware identity exceptions |
| **HW-3 Advanced ATS/PRI/PASID/SVA/P2P/GPU** | shared VA, peer memory/device translation caches | **Deferred** pending independent model/hardware proof | support later only with device-cache invalidation and trust model |
| **HW-T Trusted-only exception** | no practical isolation, product accepts a chosen hardware/driver in TCB | Not called an isolated untrusted driver | documented weaker security class; scope limited, possible production exclusion |

**No-IOMMU caveat:** moving an unsafe device's DMA into a bounce buffer does not isolate a malicious device that can address arbitrary physical memory without IOMMU. A bounce copy may help DMA masks/performance/compatibility, not provide a memory protection boundary. Pure non-bus-master devices may be considered separately if their hardware behavior genuinely prevents DMA.

### 3.3 Recompute on topology change

Hotplug, function reset, re-enumeration, firmware/ACPI changes, a bridge becoming noncompliant, ACS disabled or a multi-function sibling entering a new driver state can invalidate the group authorization. Deny further grants until secure topology verified; do not assume a once-issued DeviceGroup capability remains correct across arbitrary hardware generations. Locking/configuring ACS can itself require trusted privileged actions; exposing raw PCI config writes lets a driver defeat the isolation model.

## 4. TCB and component responsibility

| Operation | Proposed kernel enforcement | User-space authorization/policy | Remaining hardware risk |
|---|---|---|---|
| PCI probe topology | access-controlled privileged config read, table validation where required, atomic group owner/security state | BusManager enumerates/proposes group graph | false firmware tables, stealth bridges, device quirks |
| DeviceGroup assign | exclusive group owner state and delegation gate, typed rights, generation | DeviceAuthority policy decides which named DriverHost | PCI P2P, ACS and distinct reset/firmware coupling |
| MMIO map/PIO | checked BAR/port span, cache attributes, CPU mapping and revocation | driver protocol and queue layout | registers may enable DMA/reconfigure group |
| PCI configuration | restrictive privileged gates on BAR, bus-master, MSI, ACS, IOMMU-sensitive config and **dangerous equivalent MMIO command registers** | BusManager approved init and reset recipe | device may ignore expected reset semantics; mediated command interface required for some BARs |
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
| DeviceResetFence / QuiescenceRecord | trusted ordered hardware reset/invalidation stages | **stage evidence is distinct:** no new grant, IRQ disable, translation invalidate, in-flight drain, source power isolation, and safe reuse; cannot be fabricated by a numeric cookie |
| DmaFaultReport | typed fault source, IOVA, access type, group/domain gen | advisory observation vs authoritative kernel hardware fault |
| DeviceBudget | pinned bytes, map metadata, IOVA space, IRQ credits/reset attempts | sponsor charges persist during quarantine and crash |
| DeviceHealth / DriverIncarnation | user service progress and restart state | not hardware DMA quiescence evidence by itself |

**v0.2 overlays:** ResetCouplingSet (reset affects which siblings), FirmwareReservedDmaRegion (firmware/ACPI reserved physical scope and trusted policy), UnsafeMmioCommandProfile (register operations able to alter isolation), FenceEvidence (physical stop/posted-write drain/cache invalidation/readiness) are conceptual typed policy records; none is a frozen kernel object/ABI.

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
 -> TopologyValidated (group/ACS/IRQ/IOMMU AND reset/P2P/firmware-reserved overlays evidence)
 -> FirmwareReservationResolved / DeniedDevice (RMRR/IVRS identity-map exception explicitly authorized or rejected)
 -> BlockedDomainInstalled (default deny for UNTRUSTED groups; no bus-master acceptance)
 -> AuthorizedGroupAdmitted (owner, budget, mandatory shared siblings)
 -> Prepared (safe config + pinned buffers + IOVA + IRQ)
 -> Active (enabled bus-master within scoped domain)
 -> Quiescing (new work denied, IRQ masked, DMA stop requested)
 -> FencedForReuse (hardware evidence of stopped submitted work, pending writes ordered, translations invalidated)
 -> Detached (domain/IOVA retired, device owners drained)
 -> Reusable or Quarantined/Offline/Fatal
~~~

**Not all actual firmware boot paths begin at a safe FirmwareOwned DMA-off state.** UEFI/storage/network firmware may already have enabled bus mastering and left buffers or device transactions. The firmware-to-kernel handoff, before fully trusted remapping is installed, is a **separate exposure window**. The requested security profile must either establish early DMA protection with platform evidence or explicitly treat that interval as unprotected external trust. Later IOMMU initialization does not retroactively repair corruption that occurred earlier.

### 6.1 DeviceGroup assignment transaction

1. Validate current topology/ACPI/IOMMU routing, firmware reserved-RMRR/IVRS spans, reset/P2P group overlays, current group epoch and no conflicting owner; deny unsupported hardware state. **Treat mandatory firmware unity maps only under a separate tightly constrained trusted-device profile; never pass arbitrary firmware identity mappings to untrusted DMA groups**.
2. Ensure a **blocked/default-deny IOMMU domain for untrusted group** is installed as the device attachment (or establish trusted equivalent), explicitly reconciling protected firmware-required mappings under a separate trusted class; if no safe split exists, refuse untrusted assignment. Do not enable bus mastering or grant direct MMIO/PCI config before this barrier.
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

**Mapping lifetime records** must link group+domain+IOVA range+physical frames+grant lineage+sponsor+generation+device generation. A *domain mapping* is not permission to reach the entire mapped frame's neighboring security data; IOMMU granule expansion and PCI/device access controls decide that risk. PIN_MEMORY on a revocable MemoryObject does not by itself create an eternal DMA privilege; attempts to revoke must either fence and release or report pending unsafe effect and quarantine.

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

**Ordering and evidence:** Denying future IOVA translation is one safety stage; stopping already queued submissions and draining previously accepted posted writes is another. A successful invalidate acknowledges the defined IOMMU cache operation only under its hardware contract; it is NOT a universal DMA completion event. Device reset response or driver fence token cannot establish stronger ordering than hardware defines [S3,S5,S15].

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

An FLR attempt is not an FLR guarantee. Some PCIe functions do not implement individual function reset, bridge bus reset affects children, and an unexpected reset may invalidate MSI, BARs or device state needed for subsequent driver initialization [S5]. **ResetCouplingSet is not the same as the IOMMU isolation DeviceGroup:** a bus reset may take down a trusted boot-storage sibling outside the requested driver's group, and independent isolation does not imply independent reset. Config-space availability itself can fail while device is hung. A completed IOMMU invalidation does not retroactively revoke writes that occurred before the fence or guarantee recovery of corrupted kernel/file memory. Firmware may be controlling a device even after user driver exit if firmware/runtime services or SMM take ownership: product trust profile must include that possibility. **ACPI reserved mappings (RMRR/IVRS) can be required for firmware emulation and cannot be silently made available to an untrusted driver as identity-mapped arbitrary RAM;** require reserved frame exclusion, explicit narrow trusted exception, disabling BIOS feature/device if supported, or reject strict HW-2 assignment [S8].

### 10.3 Quarantine semantics

**Quarantine keeps *affected* physical frames and group resources unavailable for unsafe reuse** until a trusted fence; it does not necessarily require freezing every unrelated software service or rebooting a healthy device. Pin/map metadata, sponsor debt, lineage and group generation remain in kernel even if user process has died. Quarantine may last until reboot or operator recovery and can consume real physical RAM—OOM policy must explicitly count quarantined memory and refuse new workloads rather than silently reclaim.

**Hard limit caveat:** if no real IOMMU or trustworthy reset and DMA hardware can target arbitrary RAM, "quarantining a few buffers" does not confine its future writes. In this scenario, **continuing unrelated applications may also be unsafe**, because the rogue device can still overwrite their or kernel memory. Local software quarantine only isolates what effective hardware remapping can actually stop. The only honest options can be trusted-only device, physical disable/power removal, full fail-stop, or refusing to boot in the stronger security profile. Kernel software cannot quarantine against arbitrary DMA that bypasses its protection.


## 11. Boot, M0 and essential driver dependency

### 11.1 Boot ownership handoff

Early x86_64 UEFI may use firmware storage/display/network drivers and DMA-capable devices before UniOS can construct its IOMMU and device-group model. ACPI **RMRR / IVRS reserved DMA and identity-translation exceptions** may survive firmware handoff or be necessary for firmware emulation. Their existence is not an automatic entitlement for an untrusted DriverHost; default-deny applies to *untrusted domains* while any firmware-trusted exception must be narrowly scoped or the device remains unsupported. RFC-006 BootContext can describe physical resources, but a boot metadata pointer or ACPI table is **not evidence that devices are already quiesced or isolated**.

Candidate safe handoff study:
- determine trusted firmware/bootloader DMA security assumptions and whether remapping is enabled while firmware is active;
- freeze/stop EFI I/O and prevent use of Boot Services after ExitBootServices according to UEFI;
- validate physical memory types (firmware/runtime/ACPI/MMIO/kernel/BootBundle), discover PCI/IOMMU and reserve memory that may still be hardware-owned;
- install a validated blocked domain before enabling newly managed DMA traffic; do not casually disable an established protective remapper before replacement;
- establish minimal kernel-owned IRQ/timer path **independent** of UEFI Boot Services watchdog and pageable user storage;
- authorize only the minimal boot-critical DeviceGroup and its protected DMA frames under a specifically trusted bootstrap profile;
- launch wired critical user DriverHost without forcing it to file-fault through the Pager it must serve; then start file Pager/VFS in a validated dependency order.

### 11.2 No fantasy fully user-space boot paradox

If file Pager is on the same NVMe device whose user-space DriverHost executable and queues are themselves demand-paged from that Pager, cold start produces a cycle:
~~~text
Pager -> StorageHost IPC -> driver DMA completion/IRQ -> StorageHost executable page fault -> Pager
~~~

M0 needs resident storage-driver executable, stack, queue metadata, IRQ bindings, DMA buffers and IOMMU bookkeeping, or an explicit non-pager-dependent alternate medium. Even if all those components are resident, a dead storage controller and no independent physical rescue path may make restart impossible. RFC-006 RA-2 fail-stop/verified reboot is a permissible safe response, not a guarantee of data recovery.

### 11.3 Boot essential device class

Define an *EssentialDeviceProfile* within the authenticated BootPolicyManifest for required storage, early serial/debug, timer, critical interrupts and optional display. Not every one uses DMA. Its minimal required group membership, IOMMU/IRQ prerequisites, exact grant scope, MemoryObject reservations and failure policy are verified before granting service control. No runtime normal Supervisor may add new global device owners by editing a registry label.

**No code implied:** deciding whether an early storage driver lives in small trusted privileged bootstrap or a wired user process remains a P0 tradeoff if the physical Reference PC cannot meet strict IOMMU/fault-resident constraints.

## 12. Driver service lifecycle, IPC, CPU and resource accounting

### 12.1 Service identity vs device ownership

A hardware DeviceGroup generation, IOMMU-domain generation, CPU MMIO mappings, IRQ bindings and DriverHost service incarnation are distinct. A fresh DriverHost with identical image/ServiceID **does not inherit** the old group's hardware ownership merely because service discovery presents the same name.

Start candidate: topology validation → exclusive DeviceGroup/resource-budget reservation → blocked IOMMU attach → only approved pins/IOVA → IRQ/remap setup → scoped MMIO/config grant → driver Ready checked → enable submissions. A bootstrap essential driver may require a narrower version of this flow with wired resources; unsupported features fail closed.

### 12.2 Crash/restart candidate

1. Detect failure via Task death/timeout/driver health; a self-reported "all DMA done" from compromised DriverHost does not establish safety.
2. B stops new authority and normal Supervisor registry publication; retire old IPC endpoints/ReplyTokens/Wait registrations under RFC-003 semantics. Queue messages can have already COMMITTED; remote device effects are not undone.
3. Revoke affected CPU mappings and verify R1 CPU shootdown; prevent old MMIO doorbell/config access and guard against CPU cores still running old DriverHost.
4. Mask IRQ, prevent bus-master new work, bound drain/reset; establish independent trusted R2-F evidence. If impossible: **Quarantined** DeviceGroup and pinned frames. Other independent groups/services may continue only if truly isolated.
5. Retire old IOVA, IOMMU/ATC cache state, pins, IRQ binding, service credits and sponsor ledger only when references and physical fences permit.
6. Re-enumerate or validate topology/reset side effects, new group epoch, verified device image/firmware policy and fresh grant; assign a new domain/IOVA/IRQ generation as appropriate.
7. Independent service health test; then publish new endpoint. Older DMA completions/IRQ/driver messages must not be delivered into new service as if generated by it.
8. Reconcile durable storage operations separately per RFC-008. Replaying a storage/network transaction blindly may duplicate or corrupt work.

**Restart layering:** Software service restart and hardware DeviceGroup reuse are different gates; an unrelated VFS/GUI service can often restart after its own B/R1 even while another DMA group is quarantined. Do not make all process availability depend on one unquiesceable endpoint; but do not claim a dependent service has usable hardware.

### 12.3 CPU, IPC and pin sponsor conservation

ResourceBudget covers pinned physical pages, IOVA address space, IOMMU page-table metadata, IRQ/Wait registrations, queue DMA descriptor storage, process and object refs, and temporary/quarantined resources. These charges survive driver Task death when hardware may still write. Revoking a control grant does not refund real pinned physical RAM until R2-F and all independent refs settle (RFC-002/004).

Driver CPU execution charged to its own authorized ResourceGroup under RFC-005; IRQ top-half is bounded privileged overhead, deferred service work is charged or otherwise explicitly bounded. No assumption that client IPC automatically donates CPU time or that "essential device" bypasses all CPU caps. IRQ floods cannot consume protected M0 CPU/IPC credits without bounds.

### 12.4 Cleanup while out of memory

Unpin/unmap/fault handling and IRQ mask might allocate metadata at a bad time. M0 R_cleanup/R_irq/R_pt must reserve bounded critical operations or safely deny incomplete teardown. No unbounded recursive device-tree destructor with interrupts disabled or kernel scheduler locks held across a potentially hung device fence. Hardware reset time is external and may be unbounded or never complete; timeout is **not successful quiescence**.

## 13. Device discovery, hotplug, fault containment and removals

### 13.1 Enumeration/hotplug

A hotplug PCIe function may change bridge topology, bus numbering, group membership and upstream ACS behavior. Re-enumeration must create a new physical resource epoch; old DeviceFunctionView or PCI BDF integers cannot authorize a newly discovered function, even if the same address is reused.

On device removal, freeze new MMIO/IRQ/IOVA requests; old MMIO may raise machine-check/error if mapped and accessed. Mask/ack lost IRQ sources without assuming device accessible; revoke CPU mappings, quarantine outstanding DMA until physical removal/fence properties are established, and stop clients gracefully where possible.

**PCIe external peripherals:** USB-C/Thunderbolt/USB4 with PCIe tunneling pose hotplug DMA security issues. No general Thunderbolt hotplug support/authorization in M0; future permission model, physical-port authorization and device group routing require separate RFC-016/017/hardware policy.

### 13.2 IOMMU faults

A hardware IOMMU fault is evidence of attempted device memory access outside translated rights, not proof that no earlier write succeeded. Kernel logs only relevant IOVA, group/domain/gen, access type and fault reason; avoid dumping user memory or secrets.

Fault storm handling: per-group counters and bounded queue/reporting; disable a misbehaving DeviceGroup rather than logging forever or starving critical timer IRQs. Inconsistent IOMMU fault attribution, invalid requester ID or overflow of reliable fault reporting may require disabling direct assignment on that hardware. Lost logs are not proof that faults did not occur.

### 13.3 Driver fault categories

- Protocol bug: DriverHost refuses/forges IPC reply, illegal register, invalid DMA length/IOVA; isolate process and group only when hardware state can be made safe.
- Device malfunction: unresponsive reset, IRQ storm, missed command completion, DMA engine stuck; quarantine/fail-stop.
- Memory pressure: pin allocation denied, fragmented IOVA, IOMMU metadata OOM; no unsafe "fall back to physical memory without translation."
- Unsupported hardware: IOMMU absent, ACS insufficient, MSI unsafe, FLR missing, firmware control unresolved; trusted-only/restricted profile or no support.
- Device ownership theft: stale caps/IRQ tokens or hotplug epoch reuse; reject by object/generation/lineage.
- Platform failure: corrupted firmware/hardware remapper, SMM/CPU issues or pre-boot DMA compromise; beyond claimed user-space driver sandbox.

## 14. Storage, graphics, networking and compatibility personality implications

| Subsystem | Likely challenge | Design caution |
|---|---|---|
| NVMe/storage | DMA to queues/PRPs, interrupt processing needed for Pager/VFS boot and fsync | wired bootstrap; completion not durable flush; lost DMA can corrupt user data |
| VirtIO Block/Net | emulator/paravirtual queue and interrupt semantics | virtqueue may use shared buffers; guest IOMMU may be absent unless explicitly configured |
| Network | packet RX/TX DMA buffer rings and malicious device input | frame lifetime/refill, output duplication after driver restart, packet confidentiality |
| USB/XHCI | DMA transfer rings, external device/hub and hotplug topology | USB device is not always a separate PCI IOMMU group; physical controller owns DMA |
| GPU/display | commands, VRAM, P2P, dma-buf synchronization, user app GPU jobs | complexity and unverified isolation; not included in earliest HW-2 claim |
| Audio/input | latency/IRQ, USB controller DMA, multimedia deadline | SCHED-B deadline may help but does not replace DMA protection |
| Windows/Linux/Darwin guest | guest driver ABI, virtual PCI, passthrough permissions | no direct native DeviceGroup grants from guest admin/root; virtualization policy must authorize each mapping |
| Power/suspend | device DMA may persist across low-power/reinit | cold-boot proof not automatically applied after resume; future revalidations |

**User-space DeviceHost is not the same as exposing raw hardware to applications.** Native and compatibility applications use brokered typed driver services. Native capability rights only come through explicit authorized policy. A Linux app calling ioctl or mapping a BAR via a compatibility interface does not obtain physical IOMMU/IRQ privileges unless an external native grant authorized by UniOS exists.

### 14.1 Filesystem durability remains external

A successful NVMe queue completion can mean a command finished per device protocol; it is not universally equivalent to fsync persistence or controller power-loss protection. Ordered dirty-page flush, metadata journal and crash recovery belong to RFC-008. Driver crashes may leave result status UNKNOWN_REMOTE_EFFECT; service restart cannot guarantee exactly-once disk operations.

## 15. Architectural alternatives and physical feasibility

| Alternative | Benefit | Security/TCB costs | Decision gate |
|---|---|---|---|
| **DEV-A** kernel enforcement + user DeviceAuthority/DriverHost | least policy in Ring 0, service restart feasible for isolatable groups | hard group topology and reset validation, privileged IOMMU/IRQ path, service dependency | conditional first candidate |
| **DEV-B** trusted device/driver group | can support hardware without usable IOMMU or early boot security feature | expands TCB-S/B, potential arbitrary RAM DMA and no untrusted isolation | explicitly weaker tier or reject |
| **DEV-C** privileged kernel device stack | close to reset/IRQ hardware and perhaps simpler storage boot | broad kernel TCB, complex device protocol/firmware and update | reject unless hardware constraints prove necessity |
| **DEV-D** software bounce-only arbitrary DMA | can handle address-width/legacy buffers | **not a defense** against malicious unrestricted bus master without hardware remapping | not acceptable as isolated-driver security design |
| **DEV-E** virtualization/IOMMU nesting + SVA/PASID | advanced passthrough/per-app throughput | nested page faults/device caches, IOTLB/ATS and huge authority state | defer beyond M0/R1 |

A driver running in Ring 3 does not make its malicious DMA harmless. Conversely a signed/trusted Ring 0 driver may be operationally necessary for narrowly chosen test hardware but invalidates claims of minimal untrusted device isolation. The project should publish per-device trust/support class and feature-gated "unsupported" outcomes, not promise universal driver compatibility.

### 15.1 Security vs liveness selection

**P0 criterion:** a selected physical Reference PC cannot be used as an unrestricted user-driver security demonstration until isolation groups, IOMMU and IRQ routing, reset behavior and DMA stop are physically validated for the actual devices. If hardware fails, UniOS may still do design research in QEMU or with trusted-only/no-DMA hardware, but must not silently call this proof of a consumer secure isolated desktop.

### 15.2 Counterfactual: is IOMMU mandatory for all early boots?

Not necessarily. A software-only emulator, CPU-only boot and truly non-DMA serial/port devices can run without proving PCI DMA isolation, subject to explicit platform assumptions. But **enabling arbitrary bus-master-capable hardware under an untrusted DriverHost** without a trusted DMA isolation boundary contradicts RFC-001/002/004's security model. This product support decision must remain visible.

## 16. Race and failure matrix

| Adversarial scenario | Unsafe shortcut | Expected design outcome |
|---|---|---|
| PCI BDF reused after unplug/replug | stale function capability gains new device | physical epoch/generation and fresh grant |
| Device with 2 functions/no ACS | separate drivers own one function each as isolated | group as shared security owner, else deny |
| Firmware or driver toggles bus-master before attach | DMA runs to arbitrary PA | blocked domain+configuration control before untrusted grant |
| Device speaks MSI to protected vector | DMA mapping alone protects interrupts | mandatory remap/routing validation, deny unsafe MSI |
| VFIO-like cdev assigned by function only | cdev = independent IOMMU group | verify real group ownership |
| Driver process dies with pending DMA | release and zero pinned buffers immediately | Quarantine until trusted R2-F plus other refs |
| IOMMU unmap succeeds during in-flight write | assume already queued PCIe transaction cancelled | device stop/drain + ordering + invalidation proof |
| Device caches stale ATS translation | CPU TLB shootdown believed enough | disable ATS early or ATC invalidation plus fence |
| PCI P2P bypass route | endpoint writes peer resources outside allowed group | verify ACS/P2P topology or deny |
| IOMMU fault report appears late | no earlier corruption possible | treat as detection, containment and audit |
| IRQ replays after DriverHost restart | old vector wakes fresh owner | new IRQ generation and authorized binding |
| Driver sends false DMA completion | recycle frame from untrusted message | trusted hardware/ordered fence evidence |
| Group reset includes storage sibling | accidentally disable boot Pager path | group-level reset scope, independent recovery or fail-stop |
| FLR unsupported or device won't respond | endless retry then reuse | bounded retry, quarantine/offline |
| Driver Task closes handle while DMA active | refcount=0, pin freed | kernel pin/map/lineage refs outlive handle |
| Group has multiple IOMMU translation paths | one mapping removed, assume global fence | all relevant domains/caches validated |
| IOVA address reused under stale completion | old DMA writes new owner's frame | generation, no reuse until R2-F |
| CPU DMA coherent yet device descriptor reorder | corruption despite coherent mapping | memory ordering barrier and DMA direction rules |
| Cache flush incorrectly skipped | CPU sees stale data or device misses writes | explicit sync transition or supported coherent mode |
| User app requests arbitrary PFN for DMA | maps kernel/foreign memory | MemoryObject rights/pin/grant check |
| Driver pins entire RAM repeatedly | OOM despite normal app quota | sponsor-based pin/map budget and M0 reserve |
| Kernel cleanup attempts Pager fault | self-deadlock under OOM | wired handler/queue/metadata reserve |
| MSI storm while timer IRQ needed | M0 cannot progress | bounded top-half/mask/IRQ remap, fail-stop if not controllable |
| PCIe hotplug changes group while caps live | old group authority remains valid | topology revalidation, epoch revoke and quiesce |
| Device removal while MMIO mapped | CPU crashes or memory alias leak | revoke CPU map, bounded device-loss faults |
| Storage DMA completion before persistence | falsely confirms saved user file | RFC-008 fsync/journal/power-loss semantics |
| Guest Linux root opens physical driver | guest becomes native DMA root | no native group grants, policy broker required |
| QEMU no virtual IOMMU enabled | test result claimed IOMMU-protected | label tests actual emulated topology and mappings |
| No physical IOMMU but bounce enabled | claim hostile DMA confined | refuse untrusted DMA, trusted-only profile |
| Bootloader trusted but pre-existing DMA | post-boot isolate too late | preboot DMA trust/profile and handoff evidence |
| Device state lost on suspend resume | resumes bus-master with stale IOVA | separately gated resume re-attachment and revalidation |
| Malicious driver changes ACS/config | creates P2P escape | privileged filtered config/Reset/ACS controls |
| Driver death/quarantine leaves IRQ in kernel | orphan interrupt and resource leak | mask/detach/record debt and bounded error |
| Group quiesce blocked, unrelated UI healthy | whole desktop stuck unnecessarily | scope quarantine to affected resources; independent software may run |
| IOMMU/invalidation fails while mapping removed | memory recycled prematurely | fail fence, preserve pin+quarantine, error observable |

## 17. Security and correctness invariants (DEV-01…DEV-49) — Proposed

| ID | Proposed obligation, NOT proven |
|---|---|
| DEV-01 | No untrusted DMA-capable group is authorized before a validated remapping and group ownership boundary exists. |
| DEV-02 | DeviceGroup rather than PCI function/registry name is the minimum physical isolation/assignment unit. |
| DEV-03 | PCI group identification includes bridge/multifunction/ACS/P2P/firmware/IOMMU information and conservative denial on uncertainty. |
| DEV-04 | A group cannot have conflicting simultaneous DMA owners or bypass through another function's driver. |
| DEV-05 | An IOMMU domain attach or named IOVA is distinct from a kernel capability to arbitrary RAM. |
| DEV-06 | A raw PA/PFN/BDF/MSI/IOVA/IRQ number never grants DMA/MMIO/IRQ authority by itself. |
| DEV-07 | Bus mastering and unsafe PCI config privileges remain denied until domain/IRQ policy established and verified. |
| DEV-08 | A DriverHost cannot alter group isolation by unfiltered ACS, ATS, BAR or MSI configuration. |
| DEV-09 | DMA mappings are authorized by MemoryObject, typed direction, offset/length, group/domain/gen and sponsor. |
| DEV-10 | DMA physical frames remain pinned until all relevant existing device effects are safely fenced and refs settle. |
| DEV-11 | CPU R1 MMIO/TLB cutoff is not misrepresented as hardware DMA R2-F. |
| DEV-12 | DMA R2-F is not inferred from DriverHost death, handle close, IPC ack or software completion text. |
| DEV-13 | Device-group reuse or frame recycle after unsafe DMA requires trusted quiescence plus relevant IOMMU/device-cache invalidation. |
| DEV-14 | R2-O waits for all independent memory/device roots and refs, not just one revoked group's R2-F. |
| DEV-15 | IOVA range cannot be safely reused for a different physical owner before old accessible DMA effects are settled. |
| DEV-16 | Unsupported reset/FLR/ACS or failed invalidate causes quarantine/denial rather than unsafe regrant. |
| DEV-17 | A claimed interrupt isolation boundary needs independently verified remapping/MSI/vector ownership. |
| DEV-18 | IRQ bindings are scoped to group/Task/incarnation generation; old notifications cannot wake a new driver silently. |
| DEV-19 | IRQ storm/fault reporting cannot spend unlimited kernel CPU or reserved M0 CPU/IPC resources. |
| DEV-20 | Ordinary DMA cache-coherence and streaming ownership/barrier semantics are explicit and not conflated with map lifetime. |
| DEV-21 | A coherent buffer is not assumed to provide all command ordering or DMA completion fences automatically. |
| DEV-22 | A page granule mapping cannot expose unapproved neighboring secrets through partial buffer ranges. |
| DEV-23 | Multiple IOMMU/P2P/device-cache pathways and independent device mappings are reconciled before declaring safe reuse. |
| DEV-24 | Without usable IOMMU, bounce buffers alone are not represented as containing hostile arbitrary bus-master DMA. |
| DEV-25 | Hotplug/re-enumeration or topology policy changes revoke stale DeviceGroup/Function/IRQ generations before new grant. |
| DEV-26 | Failed device removal or MMIO access is safely reported/contained, not treated as proof a device still exists. |
| DEV-27 | User apps and compatibility guest admin/root cannot manufacture native IOMMU, DeviceGroup or MSI grants. |
| DEV-28 | DeviceHost signed image or manifest ServiceID is not unrestricted device ownership; typed DeviceAuthority controls assignment. |
| DEV-29 | Pinned DMA resources and lineage sponsor debt survive DriverHost crash until proved reclaimable. |
| DEV-30 | Active unrelated software services need not wait for impossible fence of separate quarantined device, absent real dependency. |
| DEV-31 | M0 storage/IRQ critical path is nonpageable and cannot depend on its own file Pager or failed DriverHost. |
| DEV-32 | A healthy user-space driver and scheduler budget alone do not prove device or DMA liveness. |
| DEV-33 | IOMMU fault detection does not imply earlier DMA writes were prevented or corruption repaired. |
| DEV-34 | DMA buffer availability, request completion, dirty-page writeback and durable filesystem persistence remain distinct. |
| DEV-35 | Firmware-to-kernel boot device handoff explicitly declares any pre-IOMMU DMA exposure/trust assumptions. |
| DEV-36 | Advanced SVA/PASID/ATS/PRI/P2P are disabled in first strict profile unless hardware-specific invalidation and isolation is proved. |
| DEV-37 | IRQ mask or PCI bus-master bit write is not asserted to end all already-submitted DMA transactions. |
| DEV-38 | Unsafe device power/reset operations respect actual multi-function/sibling group impacts or deny operation. |
| DEV-39 | Device fault/IRQ/IOVA/metadata memory pressure cannot silently consume the protected M0 cleanup reserve. |
| DEV-40 | QEMU simulated isolation and production physical Reference PC DMA safety are reported as separate evidence levels. |
| DEV-41 | IOMMU isolation DeviceGroup and ResetCouplingSet/IRQ/P2P/firmware-reserved overlays are analyzed independently; reset isolation is never inferred from group membership. |
| DEV-42 | Firmware RMRR/IVRS identity-map exceptions never automatically authorize untrusted DriverHost DMA access to platform-reserved memory. |
| DEV-43 | A physically active unrestricted DMA endpoint without effective isolation cannot be reported safely contained by software buffer quarantine. |
| DEV-44 | Completion of IOVA invalidate alone never asserts that previously accepted, posted or peer-path device writes are drained or undone. |
| DEV-45 | Hardware reset/fence evidence states a precise affected-device set, generation, physical stop/ordering mechanism and remaining uncertainty. |
| DEV-46 | Raw MMIO BAR access is denied or mediated where it can reconfigure device isolation, interrupt routing, firmware control or unauthorized DMA path. |
| DEV-47 | EFI PCI DMA flush/completion is separate from ExitBootServices and from new UniOS IOMMU policy; boot handoff doesn't invent a hardware drain. |
| DEV-48 | Security-protected device group and firmware reservation never share unrestricted identity mappings with normal untrusted DMA. |
| DEV-49 | A controller/sibling affected by bus-reset collateral may be independently isolated but still unavailable; recovery/liveness policy accounts for the actual ResetCouplingSet. |

The property names do not implement hardware state machines, prove DMA exclusion, guarantee reset/IRQ delivery or assert stable ABI. No DEV invariant is verified for UniOS hardware.

## 18. Candidate finite-state verification and test methodology

**Design-only bounded model:** two PCI functions sharing one group (plus one independent group); two DriverHost incarnations; two DMA frames and an independent owner; two IOMMU domains, one stale IOTLB/ATC entry; 1 IRQ vector+ack generation; 1 DeviceGroup with failed reset; a cap B/R1/R2-F/R2-O sequence; two concurrent CPU Threads; one Pager-dependent storage driver; service restart and OOM reserve.

Transitions: topology validation, grant issue, memory pin, IOVA attach, MMIO map, bus-master enable, IRQ arm/ack, DMA in-flight+completion, IOMMU invalidation, R1 TLB shot, driver crash, independent service restart, group FLR/bus reset, hotplug re-enumeration, denied stale completion and frame zero/reuse. Explicitly model failure before and after physical-write issuance (completed transactions cannot be undone).

### 18.1 Safety claims needing counterexamples

- attacker function from same group bypasses isolation when sibling attached elsewhere;
- IRQ register points to protected timer vector without interrupt remapping;
- driver dead but IOTLB still maps old IOVA, frame reused by new Task;
- device reset returned but posted write or ATS translation remained in-flight;
- pin double-refund after cap close and Task death while other group still maps object;
- newer DriverHost inherits old IRQ or DMA fence token via ServiceID;
- failure at any group-assignment commit leaves bus mastering enabled without domain;
- storage driver's own fault prevents Pager progress while its DMA is essential;
- P2P path does not visit IOMMU and escapes intended group;
- host QEMU reports successful mapping but hardware Reference PC has disabled IOMMU;
- queue depth/IRQ and kernel cleanup bypass ordinary quota under resource attack.

### 18.2 Liveness assumptions must be separate

Under an operating CPU/IRQ timer, finite kernel critical sections, healthy IOMMU and a resettable device, a quiesce may eventually finish and the group become reassignable. This is **conditional**; malicious or broken hardware, unresponsive PCI config, infinite outstanding transactions, unusable R2-F, SMM/firmware or absent reset can make physical progress impossible. The safety response is Quarantined/Offline/fail-stop, even if normal service restart would improve availability.

### 18.3 Tests only after separate implementation authorization

1. QEMU/OVMF: compare explicit virtual IOMMU enabled vs disabled, VirtIO queues, MSI, interrupt remapping **only if emulated**, hotplug, driver crash and stale device mappings.
2. Physical reference PC: inventory PCI vendor/device/bridge IDs, verified ACS, IOMMU group topology, ACPI DMAR/IVRS, remap enable and default DMA domain, interrupt remap and device reset capabilities.
3. Physical dedicated test medium: force controlled driver hangs, IOMMU invalidations, device reset and memory reuse checks under selected safe hardware; do not test on user data disk.
4. Controlled DMA out-of-bounds/IRQ fault injection only on sacrificial/isolated test hardware with documented risk and rollback; never run this merely because an RFC suggests it.
5. Failure scenarios: device does not support FLR, bus reset damages sibling, storage driver cannot restart, DMA pin remains after Process death, forced OOM with quarantine, IRQ storm.
6. Measure pin/map/unmap/invalidation latency, size/type CPU metadata, IOMMU/ATC cache costs, copy/coherence overhead, IPC overhead, throughput and tail latency with clear hardware/firmware/software conditions.
7. Repeated CPU TLB vs IOMMU invalidation races and physical fault containment after reset; report exact achieved isolation and known hardware gaps, not only pass/fail.
8. Virtual guest or compatibility layer sending raw PCI/IOVA/IRQ strings must be rejected absent actual typed grants.

**No test was run while writing this document.** QEMU-only success cannot validate real ACS group boundaries, PCI P2P, external Thunderbolt and chipset DMA trust.

## 19. Alternative security profiles and Go/No-Go criteria

| Gate | Pass condition proposed | Failure result |
|---|---|---|
| HW-0 boot CPU-only | firmware/boot memory and CPU safety documented; no untrusted DMA path enabled | disallow bus masters; can still research core OS |
| HW-1 discovery | accurate conservative group model, no privileged config leakage | deny group direct assignment |
| HW-2 group attach | enabled trusted IOMMU/IRQ remap, no harmful ACS/P2P bypass for scoped profile | group to Trusted-only or unsupported |
| HW-2 mapping | verified backing MemoryObject pin+IOVA, direction/cache rights, no foreign frame exposure | reject DMA map |
| HW-2 crash | reliable stop/fence/reset or safe indefinite quarantine with bounded resource use and fail-stop | no new owner or frame reuse |
| M0 boot driver | wired driver/execution/IRQ/queues and Pager independence on selected hardware | no claim boot service recovery |
| Product release | physical hardware safety and explicit supported device trust profile + incident recovery | cannot market "protected untrusted drivers" |

**Important impossibility case:** a PCI endpoint that can freely write arbitrary physical memory while no effective IOMMU exists cannot be confined by a Ring 0 CPU page table or a userspace service boundary. The answer may be "this device/hardware configuration is unsupported" instead of a kernel feature request.

## 20. Open research blockers O7-01…O7-43

| ID | Priority | Decision / missing evidence | Cross-RFC |
|---|---|---|---|
| O7-01 | P0 | DEV-A split versus DEV-B trusted-only, DEV-C privileged stack and actual TCB? | 001/007 |
| O7-02 | P0 | Reference PC actual IOMMU, isolation group, ACS, MSI-remap and boot DMA profile? | 001/006/007 |
| O7-03 | P0 | Group topology calculation and ambiguous bridge/P2P/firmware ownership? | 006/007 |
| O7-04 | P0 | Kernel minimal security-relevant PCI config/IOMMU programming vs userspace DeviceAuthority split? | 001/002/007 |
| O7-05 | P0 | Bus-master deny barrier at initial attach and firmware DMA exposure before kernel enforcement? | 006/007/017 |
| O7-06 | P0 | DMA mapping/pinning/page granularity and MemoryObject authorization/alias protection? | 002/004/007 |
| O7-07 | P0 | R2-F trusted device quiesce vs posted writes/IOTLB/ATC completion, no false fence? | 002/004/007 |
| O7-08 | P0 | Device reset capabilities and multifunction/sibling group control under failure? | 006/007 |
| O7-09 | P0 | MSI/IRQ remapping and safe interrupt source assignment on chosen hardware? | 005/007 |
| O7-10 | P0 | Device crash while Pager/storage waiting and M0 wired dependency/resource graph? | 003/004/005/006/007 |
| O7-11 | P0 | IOVA reuse and stale DMA/IRQ/service-gen tokens under concurrent revoke/restart? | 002/003/007 |
| O7-12 | P0 | No-IOMMU group risk profile and trusted-only vs unsupported consumer claim? | 001/007/015 |
| O7-13 | P0 | Shared buffer multi-device mapping/independent pin sponsors and safe frame reclaim? | 002/004/007 |
| O7-14 | P0 | Physical DMA quarantine budget/OOM if reset cannot safely complete? | 004/005/006/007 |
| O7-15 | P0 | Raw PCI config paths that can re-enable bus mastering or disable ACS/IRQ remap? | 001/002/007 |
| O7-16 | P0 | Capability lineage/object refs R1/R2-F/R2-O and group/driver generation lifecycle? | 002/003/007 |
| O7-17 | P0 | Actual PCIe P2P routing through/non-through IOMMU and group bypass analysis? | 007 |
| O7-18 | P1 | DMA cache sync/descriptor ordering/coherent vs streaming and atomic buffer ownership? | 004/007 |
| O7-19 | P1 | Device DMA masks, IOVA width, granule and partially shared page security? | 004/007 |
| O7-20 | P1 | Hardware fault/IRQ queue amplification and kernel scheduler reserved CPU credits? | 005/007 |
| O7-21 | P1 | Power management/suspend/resume and device re-attachment under stale mapping? | 005/006/007 |
| O7-22 | P1 | Virtualized MMIO/PCI/API for guest compatibility without native direct hardware grant? | 009/010/007 |
| O7-23 | P1 | PCI hotplug topology recomputation and external peripherals/USB4 exposure? | 007/016 |
| O7-24 | P1 | IOMMU fault attribution and reliable reporting privacy/security? | 007/015 |
| O7-25 | P1 | MSI ack/mask/rearm semantics and IRQ racing driver death/hotplug? | 003/005/007 |
| O7-26 | P1 | CPU TLB shootdown vs device IOTLB/ATC and DMA fence ordering proof? | 002/004/007 |
| O7-27 | P1 | Device firmware update/authenticity and reset trust? | 006/007/017 |
| O7-28 | P1 | Stable native IO DMA map/IRQ capability ABI and forward compatibility? | 007/009/018 |
| O7-29 | P1 | Bus error MMIO access after device removal and kernel fault containment? | 007/015 |
| O7-30 | P1 | Storage user-data durability vs hardware queue/flush after driver crash? | 007/008 |
| O7-31 | P1 | IOMMU group sharing with trusted driver/VM and separate DeviceFunctionView rights? | 002/007/010 |
| O7-32 | P1 | Hardware reset timeout/unsafe P2P causes indefinite quarantine vs safe recovery? | 006/007 |
| O7-33 | P2 | Advanced ATS/PRI/PASID/SVA nested translation and revocation design? | 004/007/010 |
| O7-34 | P2 | Device cost, pinned memory performance and impact of bounce/copy policy? | 004/007/015 |
| O7-35 | P0 | RMRR/IVRS device+reserved memory identity maps vs strict default-deny untrusted hardware profile? | 006/007/017 |
| O7-36 | P0 | ResetCouplingSet vs IOMMU DeviceGroup: FLR, AER, bus reset and boot-storage sibling effects? | 006/007/008 |
| O7-37 | P0 | Exact physical R2-F ordering: in-flight posted DMA, IOMMU invalidation, device reset and provenance of success? | 002/004/007 |
| O7-38 | P0 | Raw MMIO backdoor/control registers can bypass filtered PCI config/ACS/bus master guards? | 001/002/007 |
| O7-39 | P0 | Unsafe no-IOMMU device actively DMAing to arbitrary RAM: required whole-system fail-stop and protected future boot? | 006/007/017 |
| O7-40 | P0 | UEFI PCI driver DMA completion/Flush/Unmap vs ExitBootServices handoff of firmware-owned buffers? | 006/007/017 |
| O7-41 | P1 | Independent mapping/device/IRQ/P2P/reset overlays versioned across hotplug and driver restart? | 002/006/007 |
| O7-42 | P1 | Effective hardware bus/power isolation mechanism, trusted reset completion and failure containment evidence? | 007/015 |
| O7-43 | P1 | Vendor-specific IO ordering and device cache controls required to reclaim buffer after posted writes? | 004/007/015 |

## 21. Proposed decisions DEV-D01…DEV-D26 — NONE Accepted

- **DEV-D01:** DEV-A kernel-minimal protected IOMMU/IRQ/MMIO+userspace DeviceAuthority/DriverHost is first conditional research target.
- **DEV-D02:** DEV-B explicit broad-TCB trusted-only device and DEV-C larger trusted kernel driver remain alternatives for incompatible hardware.
- **DEV-D03 [revised]:** Assign direct DMA ownership at actual IOMMU *DeviceGroup* granularity; treat ResetCouplingSet, IRQ routing domain, P2P routes and FirmwareReservedMapSet as distinct additional security/availability gates, never assume every overlay has the same membership.
- **DEV-D04:** Typed DeviceGroup/FunctionView/Domain/IOVA/Pin/IRQ/Reset objects are candidate; raw hardware IDs are never capabilities.
- **DEV-D05 [revised]:** Deny untrusted bus-master/config/MMIO until secure group domain and protected-memory policy; firmware RMRR/IVRS exceptions are **trusted-only constrained mappings or unsupported**, never silently extended to untrusted devices.
- **DEV-D06:** IOMMU device ownership/attachment, pin/map and IRQ remap have distinct authorization and lifetime states.
- **DEV-D07:** Untrusted DMA maps only typed authorized MemoryObject frames/ranges/directions with sponsor and cache ownership.
- **DEV-D08 [revised]:** R0/B/R1/R2-F/R2-O are separate. R2-F requires source-specific trusted physical quiescence/posted-write ordering, relevant translation/device cache invalidation and proof of which group/mappings it covers; an invalidate completion or driver-reported fence alone is insufficient.
- **DEV-D09:** Failed reset/invalidation/fence retains relevant group, frames and metadata in Quarantine, no false safe reuse.
- **DEV-D10:** PCI P2P/ACS and MSI interrupt remap are independent isolation gates; an IOMMU present flag is insufficient.
- **DEV-D11:** No-IOMMU bounce buffering never counts as security isolation from hostile bus-master DMA; HW-T is visibly weaker.
- **DEV-D12:** Initial strict x86_64 profile disables unverified ATS/PRI/PASID/SVA, advanced nested domains and unsupported peer routes.
- **DEV-D13:** IRQ/mask/ack is kernel-enforced typed authority, generation-scoped to group and DriverHost; flood handling budgeted.
- **DEV-D14:** Driver service restart creates fresh identity and grants only after R1/R2-F-safe affected group; unrelated software may recover independently.
- **DEV-D15:** Hardware DMA completion, software fence, CPU/device cache synchronization and durable file flush remain distinct contracts.
- **DEV-D16:** M0 boot-essential storage/IRQ/DeviceAuthority run from wired resources without self-demand paging and with fail-stop allowed.
- **DEV-D17:** DeviceGroup topology and capabilities are revalidated across physical hotplug, reset, firmware changes and suspend/resume.
- **DEV-D18:** Device faults, pins, IOMMU metadata, IRQ credits and quarantine debt remain charged under sponsor budgets after service death.
- **DEV-D19:** QEMU/OVMF optional virtual IOMMU tests and early physical PC hardware isolation tests are separate mandatory evidence classes.
- **DEV-D20:** No user-space driver isolation/product trust claim and no code implementation until P0 group/DMA/IRQ/fence/boot gates are reviewed.
- **DEV-D21 [new]:** RMRR/IVRS firmware-reserved DMA must be explicitly mapped to a constrained trusted device context or denied under strict isolation, with protected RAM excluded.
- **DEV-D22 [new]:** Reset collateral domains are separate from IOMMU group; operation may be denied if the reset would disable boot-critical/persistent-data hardware.
- **DEV-D23 [new]:** Hardware fence proofs must report source of evidence, physical in-flight DMA/posted-write ordering, IRQ/ATS/IOTLB completion and exact scope; failed/incomplete evidence means quarantine.
- **DEV-D24 [new]:** Unsafe Device MMIO/PIO reconfiguration must be mediated or restricted; a safe PCI config filter alone does not authorize all BAR bytes.
- **DEV-D25 [new]:** Active unrestricted no-IOMMU malicious bus mastering may require full fail-stop/power isolation, not just local pinned-frame quarantine.
- **DEV-D26 [new]:** UEFI PCI DMA map/unmap/flush and ExitBootServices are separate transitions; early boot handoff needs trusted ordered termination or explicit unprotected assumption.

These remain **Proposed — Specification**, not ADR Accepted. They do not modify D1–D8 architecture directions, existing ABI or other RFCs. A security-profile exception must be an explicit future ADR with a restricted device/support claim.

## 22. Cross-RFC contract and consistency matrix

| Related RFC | Exact required integration |
|---|---|
| RFC-001 | TCB-K minimal trusted IRQ/IOMMU control, verified group vs raw device function, no firmware trust assumption hiding |
| RFC-002 | Cap lineage, sponsor, R0/B/R1/R2-F/R2-O, DeviceGroup right/control granularity, refs after Task death |
| RFC-003 | DriverHost IPC E1 endpoint generation, ReplyToken/UNKNOWN result, IRQ/Event WaitSet notification and queue credits |
| RFC-004 | MemoryObject frame ownership, DmaPin/granularity/zeroing, CPU vs IOMMU TLB fence, OOM quarantine accounting |
| RFC-005 | IRQ handler/preempt cost, CPU budget, limited M0 recovery run eligibility and physical timer independence |
| RFC-006 | firmware boot DMA handoff, wired boot StorageDriver/IRQ path, service restart/separate hardware fence, RA fail-stop |
| RFC-008 | NVMe/storage IO queue completion vs durable writeback/flush, shared persistent data crash consistency |
| RFC-009 | Native Driver/IRQ/IOMMU syscall/ABI status and explicit asynchronous fence result after proof |
| RFC-010 | Linux guest driver ABI isolation, no guest root→native DeviceGroup entitlement, user-space compatibility |
| RFC-013 | compositor/GPU mapping, display VRAM/P2P and shared buffer scheduling/trust |
| RFC-015 | privileged register parser/IOMMU state security audit, malicious DMA and hardware fault tests |
| RFC-016 | USB4/Thunderbolt/peripheral user consent and privileged DeviceGrant policy |
| RFC-017 | signed boot/firmware trust, pre-boot DMA exposure and device firmware revocation |
| RFC-018 | DeviceGroup/IRQ/IOVA/DriverHost generation and versioned manifest/ABI |

**Joint review P0 clusters:** (001+002+004+007) cap/MMU/IOMMU physical reuse; (003+005+006+007) IRQ/IPC/scheduler/wired boot progress; (006+007+008+017) boot storage/firmware handoff and crash persistence; (002+006+007) service restart with hardware-backed quarantine. A group-level physical reset proof is not implied by a clean software state machine.

## 23. Exit gates / future research milestones

**RFC-007 v0.2 is NOT design-accepted; its prior v0.1 remains in Git history.** The following research evidence is required before a "verified user-space DMA driver" product claim:

1. Select a concrete x86_64 physical Reference PC and enumerate real group/ACS/IOMMU/interrupt-remap topology **plus reset collateral sets and RMRR/IVRS firmware reserved mappings** from authoritative device/vendor/firmware sources. If unknown, use a weaker explicitly marked profile or disable direct DMA.
2. Draw initial firmware→blocked-domain→first DriverHost bootstrap protocol, covering residual boot DMA and safe ExitBootServices handoff. Not all physical firmware trust can be proven by UniOS.
3. Specify minimal privileged config/IOMMU/IRQ enforcement with a complete capability+budget table; compare DEV-A vs kernel TCB increase.
4. Model DmaPin/IOVA attach/cache sync/unmap, posted transactions, **completion/flush ordering under UEFI and PCI**, device reset and invalidation order with a precise **R2-F physical safety predicate** for at least one real device family; do not substitute driver-declared completion for a trusted fence.
5. Prove group owner exclusivity with sibling functions, P2P and MSI/interrupt remap, **and separately evaluate whether resetting the group affects a different essential sibling or device**. Report unsupported hardware rather than weakening isolation silently.
6. Model DriverHost death, old cap/IRQ/IOVA generation, resource sponsor debts, failed reset quarantine and unaffected software recovery.
7. Show protected M0 wired storage/IRQ/CPU/IPC graph; explicitly account boot driver executable and queue memory without file Pager recursion.
8. Define fatal/unsupported/degraded product disclosures and secure limited fallback; never claim dirty data persistence or global recovery merely because DMA was fenced.
9. Cross-check capability B/R1/R2-F/R2-O, VM pin and scheduler CPU charge against previously proposed RFCs before any Accepted state.
10. Future emulated and **real** fault-injection evidence must be collected on dedicated safe hardware after separate implementation authorization, not inferred from model text.

**Suggested next design research:** RFC-008 Storage, Filesystems & Crash Durability, because NVMe/DMA/IPC completion vs persistent user data is the remaining direct boot/Pager correctness gap. RFC-017 Verified Boot/Update should be co-reviewed before production security assertions.

## 24. Official primary technical references (reviewed 2026-10-24)

- **[S1] Linux VFIO, IOMMU groups and isolation:** https://docs.kernel.org/driver-api/vfio.html
- **[S2] Linux IOMMUFD / user IOAS mapping:** https://docs.kernel.org/userspace-api/iommufd.html
- **[S3] Linux Dynamic DMA Mapping Guide:** https://docs.kernel.org/core-api/dma-api-howto.html
- **[S4] Linux PCI Peer-to-Peer DMA:** https://docs.kernel.org/driver-api/pci/p2pdma.html
- **[S5] Linux PCI Support Library reset/FLR:** https://docs.kernel.org/driver-api/pci/pci.html
- **[S6] seL4 FAQ on devices and DMA proof assumptions:** https://sel4.org/About/FAQ.html
- **[S7] seL4 hardware I/O manual source, IRQ/IOMMU:** https://github.com/seL4/seL4/blob/master/manual/parts/io.tex
- **[S8] Linux x86 IOMMU and exceptions:** https://docs.kernel.org/arch/x86/iommu.html
- **[S9] Linux DMA buffers and async fences:** https://docs.kernel.org/driver-api/dma-buf.html
- **[S10] Linux IOMMUFD current detailed/nested API:** https://docs.kernel.org/next/userspace-api/iommufd.html
- **[S11] Linux x86 Shared Virtual Addressing:** https://www.kernel.org/doc/html/latest/arch/x86/sva.html
- **[S12] Linux DMA synchronization attributes:** https://docs.kernel.org/core-api/dma-attributes.html
- **[S13] Linux VFIO cdev, group ownership and no-IOMMU contrast:** https://docs.kernel.org/6.12/driver-api/vfio.html
- **[S14] UEFI Boot Services/ExitBootServices:** https://uefi.org/specs/UEFI/2.10/07_Services_Boot_Services.html
- **[S15] UEFI 2.10 PCI Bus Support, DMA Map/Unmap and posted-write Flush:** https://uefi.org/specs/UEFI/2.10/14_Protocols_PCI_Bus_Support.html
- **[S16] Linux PCI Error Recovery / bridge reset and collateral effects:** https://docs.kernel.org/PCI/pci-error-recovery.html

All links are references to third-party published technical descriptions. External correctness/security claims belong to their owning systems and **do not verify UniOS**. Platform documentation and API details may evolve; future hardware profiling must check relevant device/firmware specifications. No external source code was imported.

---

**Document history:** 2026-10-24 v0.1 original Proposed design retained in Git; 2026-10-24 v0.2 critical adversarial re-review corrected RMRR/IVRS firmware exceptions, reset collateral distinct from IOMMU groups, physically evidenced DMA fences, unsafe MMIO and uncontained no-IOMMU drivers. Research only: no actual PCI/IOMMU device configuration, driver code, boot, physical experiment, test or proof.

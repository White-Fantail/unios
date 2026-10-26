# UniOS — L1 UEFI Handoff & Kernel Entry Microcontract v0.1

> **Status:** Proposed / Design-Only / D1 candidate review — **2026-10-26 NZDT**. **NOT Accepted, implemented, boot-tested, formalized in executable notation, benchmarked, hardware-validated or ABI-frozen.**
> **Authorization:** User permits **research and design only**. Creating bootloader/kernel source, tests, build scripts, QEMU artifacts and executable formal models requires a separately explicit scoped Go decision.
> **Scope:** x86_64 **QEMU Q35 + OVMF / one vCPU / serial COM1** and one EFI image containing a separately linked **freestanding kernel ELF payload**, with a protected handoff to the kernel's first Rust instruction. Do **not** confuse this L1 probe with full UniOS Capability IPC, Pager, driver DMA, general supervisor, Verified Boot or Native ABI.
> **Parents:** [Minimal Boot/Exception/Memory MVP](unios-minimal-boot-exception-memory-mvp-contract-v0.1.md), [Implementation Entry Readiness Gate](../reviews/unios-implementation-entry-readiness-gate-v0.1.md), [Architecture v0.2](../architecture/unios-architecture-v0.2.md), [RFC-001 TCB](../rfcs/rfc-001-kernel-boundary-and-tcb.md), [RFC-004 VM/Pager](../rfcs/rfc-004-virtual-memory-pager-cache-and-oom.md), [RFC-005 CPU](../rfcs/rfc-005-scheduling-preemption-and-cpu-budget.md), [RFC-006 Boot](../rfcs/rfc-006-boot-trust-chain-service-startup-and-recovery.md), [RFC-017 Verified Boot](../rfcs/rfc-017-verified-boot-updates-and-rollback-protection.md).
> **External primary-source constraints:** [UEFI 2.11 Boot Services / GetMemoryMap / ExitBootServices](https://uefi.org/specs/UEFI/2.11/07_Services_Boot_Services.html), [UEFI 2.11 EFI system table / loader lifecycle](https://uefi.org/specs/UEFI/2.11/04_EFI_System_Table.html), [Rust x86_64-unknown-uefi](https://doc.rust-lang.org/rustc/platform-support/unknown-uefi.html), [Rust x86_64-unknown-none](https://doc.rust-lang.org/rustc/platform-support/x86_64-unknown-none.html).
> **Evidence:** source/manual reading and adversarial paper ordering only. No compiled UniOS image or VM boot is claimed.

## 0. Executive decision record

**Preferred first *research/implementation-entry candidate* L1-H1:**
1. **One UEFI application** (PE32+) contains an **immutable embedded, separately linked fixed-virtual-layout kernel ELF** as bytes. Do **not** depend on a second EFI filesystem lookup to load the payload. An embedded payload is *packaging*, not proof of cryptographic trust or compatibility between UEFI and Rust ELF.
2. **Kernel image loader accepts only a deliberately restricted ELF64 subset** for x86_64, with explicit \`PT_LOAD\`, non-overlap, canonical fixed virtual destinations, segment rights and bounds. The experimental image is **not** a runtime dynamic-linking ABI. Initial profile disallows \`PT_INTERP\`, \`PT_DYNAMIC\`, relocations that require runtime fixup, unsupported TLS, unresolved imports, unsupported ELF type/class/machine and W+X segments. Prefer a fixed **higher-half virtual kernel address compatible with the selected kernel code model** and distinguish linked virtual offsets from allocated physical pages. Final chosen address and linker contract still **OPEN**.
3. **Allocate everything before final \`GetMemoryMap\`:** kernel backing, BSS, first page tables, instruction/stack/trampoline pages, double-fault emergency IST, BootContext/map buffer (with reserved descriptor slack) and serial diagnostic metadata. After final map retrieval and first ExitBootServices call no normal allocation, image loading, protocol I/O or logging through firmware is permitted.
4. **Two-translation-domain handoff:** EFI loader executes on firmware-provided current mappings until the handoff trampoline. The loader constructs a **strict minimal new CR3 page-table root** with mappings for (a) a transitional code/stack area needed across CR3, (b) protected higher-half kernel text/data and dedicated kernel/DF stacks, (c) read-only BootContext and (d) minimal IDT/GDT/TSS/serial access requirements. Changing CR3 while executing and switching to a new stack must not fetch an instruction, descriptor, stack byte or branch target unmapped in either relevant phase.
5. **Normal EFI→kernel transition:** latest memory map + map key → successful \`ExitBootServices\` → no more Boot Services → verified handoff trampoline → CR3 switch to UniOS-owned initial tables → controlled stack/CPU/IDT/TSS/GDT state → **SysV-compatible freestanding kernel entry** → serial stage trace and exception/guard self-diagnostics → \`HALTED_PASS\` or fail-stop. **UEFI calling conventions are not passed through unchanged to Rust kernel.**
6. **Single-CPU / privileged only:** interrupts masked for the entire L1 path; no user page tables, Scheduler, Task/CapHandle/IPC, Pager, DMA drivers, network, writable disk, general PMM/free, production Secure Boot, or service restart.

**Verdict:** This microcontract sharply narrows L1 technical choices and supplies falsifiable pre-code requirements. **Do not call L1 READY FOR USER GO yet**: the final linked virtual region/page table layout, executable payload loader subset and CPU transition/early exception guarantee need one last concrete B02/B03/B07 sign-off, plus B10 user-specific scoped approval. That is a **finite short list**, not all L2 CAPIPC blockers.

## 1. Why this narrow boot design, alternatives rejected for first slice

| Option | Result for limited Tier0A probe | Why |
|---|---|---|
| **H1 embedded ELF + fixed link virtual layout + private page tables** | **Research preference** | separate Rust UEFI and bare-metal targets, removes late filesystem demand, explicit rights/stack ownership |
| H2 EFI-loader + external ELF on filesystem | **Defer** | adds EFI file protocol, path trust, late read/error state and payload provenance, especially unpleasant around ExitBS |
| H3 UEFI-only PE executable as “kernel” | **Reject as an independent-kernel handoff proof** | useful diagnostics but does not validate a freestanding ELF entry/CR3/BootServices exit boundary |
| H4 fully relocatable/dynamically linked ELF in first loader | **Defer** | expands parser/relocation and W^X proof surface without L1 value |
| H5 borrow firmware CR3 forever while claiming protected kernel W^X/guard proof | **Reject as sufficient L1 memory test** | firmware mappings/aliases are not the UniOS-owned protected map being tested |
| H6 kernel high-half only with no mapped transition code | **Reject** | instruction fetch/stack accesses during CR3 switch can fault before first handler |
| H7 loader switches all device/IRQ/CPU facilities simultaneously | **Reject** | unnecessary coupled failures with no diagnostic separation |
| H8 signed/verified full BootBundle and M0 RecoveryAuthority before first code | **Defer** | belongs to RFC-006/017 product security, not a DevUnsigned disposable VM-only diagnostic |

**Design balance:** H1 adds a small ELF parser in trusted experimental loader. Review its bounded format rather than pretending embedding an ELF image eliminates the need for image validation. Later, trust policy and signature checks could cover the EFI image and embedded kernel together, but **UEFI accepting DevUnsigned test media is not a proof of authenticated code**.

## 2. Image representation and linking contract — still no selected ABI numbers

### 2.1 UEFI loader output

- Rust target **\`x86_64-unknown-uefi\`**; image is UEFI-compatible **PE32+** with UEFI x64 calling convention (Windows-like), not bare-metal SysV.
- Loader owns firmware image handle/system table before exit, and it is the caller of \`ExitBootServices\`. It is not permitted to return to firmware after successful ExitBootServices.
- Embedded kernel ELF is immutable input **inside the selected experiment image**; it is not a user-provided arbitrary executable. Despite that, the embedded bytes are parsed as potentially malformed and all bounds validated to prevent a compromised/mispackaged loader from corrupting memory.
- Build must record exact compiler, target, linker, UEFI runtime dependencies, OVMF hash, QEMU version/config and binary provenance before an actual L1 run.

### 2.2 Embedded kernel payload subset

- \`x86_64-unknown-none\` ELF64 for x86-64 machine and little-endian encoding, fixed link-time virtual addresses. Freestanding \`no_std\`; panic abort; no \`std\`, allocator, file I/O or implicit UEFI dependency.
- ELF header and all program header offsets/sizes must be within complete embedded payload; overflow and misalignment rejected. The **program header count is bounded** by this experimental profile and cannot be used for unbounded allocation/recursion.
- Each supported \`PT_LOAD\`: \`filesz <= memsz\`; explicitly allocated physical backing for page-rounded \`memsz\`; copies exactly file bytes and **zero-fills remaining BSS**, without touching an ownership-excluded range. Virtual range must be canonical, aligned/compatible with page map and not overlap other segments with contradictory rights.
- Reject dynamic relocation requirements, loading/interpreter requirements, nonapproved machine/class/endian/unsupported required headers, executable+writable segments, entry outside an executable mapped range, and ambiguous page-overlap permissions. **No general ELF shared-library support is implied**.
- The target's default **kernel code model** may presume a particular negative canonical virtual address reachability; the exact linker virtual base and any large/small-model flags must be selected together and audited **before implementation approval**. Do not infer a valid address from the firmware-selected physical allocation.
- Boot-time physical segments do not need to share their linked virtual numerical addresses; the new page tables map the chosen virtual regions to allocated frame extents. Memory ownership must be recorded per segment and page.
- File-offset/code checksum or build manifest may identify experimental artifacts; no cryptographic authenticity or rollback policy asserted by an unauthenticated hash.

**Remaining H-OPEN-01:** choose and record **one** fixed virtual-link/physical-mapping scheme, list permitted ELF segment/relocation types, and explicitly validate the kernel entry instruction fetch under new CR3. Until then, calling this an approved loader implementation recipe would be premature.

## 3. Concrete physical memory ownership and map discipline

### 3.1 Ownership ledger per phase

| Resource | Before first ExitBS | After successful ExitBS | May be reclaimed in L1? |
|---|---|---|---|
| EFI loader's executing image/stack | loader/firmware-approved executable allocations | bootstrap transfer region until UniOS control settled | **No**, if transition still executes there; no general reclamation in L1 |
| embedded ELF bytes | immutable data in EFI image | loader-owned bytes not needed after segments staged, may remain reserved | deliberately **No** |
| kernel mapped segment frames | loader **explicitly allocates** these pages | UniOS kernel resident/protected | **No** |
| BSS/data backing | loader allocated and initialized | UniOS kernel data; NX | **No** |
| new CR3 page-table frames | loader allocated, describes exact mapping set | UniOS kernel privileged map authority | **No** |
| trampoline code and temporary stack | loader allocated; old and new map consistency required | interim code until high-half switch complete; intentionally retain | **No** |
| kernel main and IST stacks | loader allocated, guard ranges mapped invalid under final tables | kernel/exception TSS | **No** |
| BootContext/memory-map buffer | loader allocated before final map, guaranteed retained | kernel read-only metadata/reference | **No** |
| firmware runtime regions/ACPI/NVS/MMIO/unknown | firmware or non-UniOS reserved | kernel excludes from arbitrary allocation | **No** |
| EfiConventionalMemory and post-exit EfiBootServices* | firmware manages pre-exit | OS may own usable portions by UEFI type *but L1 lacks general PMM* | **No**, unused deliberately |

**Important distinction:** firmware memory descriptors indicating \`EfiConventionalMemory\` before ExitBootServices are not a blanket permission to write to arbitrary pages before exit; use UEFI allocation services and preserve its map protocol. After successful ExitBootServices certain types become OS-available, yet this particular no-PMM L1 probe elects to leave them unused. It cannot prove user-pager/physical memory allocator correctness from that policy.

### 3.2 Map buffer and last-key sequence

**Proposed bounded state cut:**
1. Stage ELF payload, tables, stacks, BootContext, COM1 and memory-map storage **first**; map buffer must allow actual descriptors plus margin because allocating additional storage changes map.
2. Use \`GetMemoryMap\` and parse by returned **DescriptorSize and DescriptorVersion**, validating stride, bounds and owner ranges. Save returned \`MapKey\` corresponding to **the map that will be used**.
3. Minimize work between final successful \`GetMemoryMap\` and \`ExitBootServices\`; in particular **no \`AllocatePool\`, \`AllocatePages\`, \`FreePool\`, \`FreePages\`, firmware console, protocol calls or dynamic logging** that could change map or rely on services being present during partial shutdown. Direct fixed UART diagnostics are a separate hardware path and should not silently use UEFI console.
4. Invoke \`ExitBootServices(ImageHandle, MapKey)\` using last valid key. Its success is the irreversible **L1-BS-CUT**. Do not call any Boot Service thereafter and do not return from EFI application.
5. On \`EFI_INVALID_PARAMETER\` / stale MapKey, **the first exit attempt may have partially disabled Boot Services**. Subsequent calls are conservatively **\`GetMemoryMap\` and \`ExitBootServices\` only** within a preallocated buffer/finite retry envelope. UEFI 2.11 permits memory-allocation services after first exit attempt, but the narrower profile intentionally does not use them; if map exceeds retained capacity or retry bound, **fail-stop**, do not call a forbidden service or pretend firmware is fully alive. If a future profile needs allocation after failed ExitBS, it must be separately reviewed against actual UEFI version.
6. Errors outside the specified retry set, map corruption, inability to retain needed allocations or repeated failure -> **fail-stop/serial best effort** after first attempt; avoid assuming normal EFI exit is still safe.

**Why no fixed memory map buffer size here:** actual OVMF/QEMU/memory descriptor count not measured. The future approved test chooses capacity after inspecting target firmware baseline; the design must nevertheless reject insufficient capacity and prevent unbounded retries.

### 3.3 Validation invariants

- Descriptors are iterated at \`DescriptorSize\` stride, with checked multiplication, bounds, number of entries and alignment; **never** assume a compiled Rust \`size_of\` equals firmware stride.
- Track all allocated segment/stack/trampoline/map/root frames independently. Reject physical overlap among regions with incompatible rights, integer wrap and unknown memory-type reinterpretation as ordinary RAM.
- Preserve kernel image, BootContext, page tables, main/IST guard backing; do not free a page still referenced by active CR3/CPU stack or exception handler.
- Identity mapping may make a supposedly protected high-half text frame writable through a lower alias; alias audit must cover **all** present translations to those frames, not only high-half virtual regions.
- Physical address 0, firmware runtime/ACPI/MMIO/NVS and unaccepted/unknown page types remain unavailable for ordinary experimental mappings unless explicitly required and separately approved.

## 4. First page-table and CR3/trampoline handoff micro-order

### 4.1 Page table candidate H-PT1: high-half kernel with minimal bridge

**Mappings needed in new CR3**, all supervisor and constrained by actual CPU/MMU features:

| Region | Mapped in new CR3 | Rights | Lifetime |
|---|---|---|---|
| short transition/trampoline instructions in **their actual current linear execution range** | yes | executable, not writable once finalized | until first kernel high-half entry and known safe state |
| current temporary bridge stack, if any access remains during transition | yes | writable+NX, guard no alias | until target RSP established |
| final higher-half kernel text | yes | read+execute, **not writable via any alias** | entire L1 |
| kernel rodata | yes | read-only, NX | entire L1 |
| kernel data/BSS/boot diagnostics | yes | read/write+NX | entire L1 |
| main kernel stack and #DF IST stack | yes | writable+NX, guard unmapped including no lower alias | entire L1 |
| BootContext and validated memory-map descriptors | yes | readable; preferably RO+NX after finalization | entire L1 |
| GDT/IDT/TSS, fault entry code and minimal CPU state | yes | appropriate supervisor rights (GDT/IDT RO, TSS writable if needed), execute handlers | entire L1 |
| general low RAM, EFI filesystem/protocol regions, video framebuffers, untrusted device BARs | **no** | not assumed | unavailable |

**Hard remaining H-OPEN-02:** determine the actual old UEFI linear address→physical backing relation for the trampoline and document both old and new CR3 accessibility. The loader cannot simply assume any physical address is valid as a *linear* pointer, nor that firmware's page tables are an acceptable permanent OS address space. Use a known allocated execution region and a transition plan proved not to fetch/fault between CR3 write and final entry.

### 4.2 Protected transition ordering and CPU state

**Candidate order (one or more assembly boundaries needed, not code):**

| Cut | CPU/VM state guarantee | If violated |
|---|---|---|
| **H-C01** before ExitBS | kernel frames loaded/BSS zero, new page tables and guard maps prepared, handoff BootContext complete, UART fallback ready; CPU necessary features checked | fail EFI app before first exit attempt |
| **H-C02** final map/ExitBS | current MapKey and last map validated; no subsequent Boot Services or firmware console use after first attempt except reviewed retry subset | no kernel entry, bounded fail-stop |
| **H-C03** enter resident handoff stub | interrupts masked; stub location/stack mapped under **both** current UEFI and proposed new CR3; current GDT/IDT/TSS references safe during swap or one temporary fault-safe handoff mechanism explicitly established | early fault/triple-fault classified as FAIL, no claim guaranteed serial |
| **H-C04** switch to own CR3 | new root frames and page translations valid, trampoline fetch and temporary stack unbroken, no unintentional RW alias to kernel RX | abort is hardware fail-stop; cannot return to UEFI |
| **H-C05** establish kernel stack and descriptors | explicit RSP aligned for SysV kernel entry; known DF=0, IF=0, GS/FS/segment assumptions, GDT and TSS, emergency IST, IDT handler addresses valid under own CR3 | negative VM test/fail-stop, not automatic recovery |
| **H-C06** enter first freestanding Rust function | transfer from UEFI ABI to kernel **SysV \`extern "C"\`**, no red zone, selected soft-float/vector assumptions and panic abort; BootContext pointer points into mapped validated memory | fail-stop; no unsafe BootService rollback |
| **H-C07** finish diagnostics | expected rights/exception checks passed, print unique final stage marker once, stop CPU with interrupts masked | any prior panic/exception/reset -> FAIL, never PASS |

**Attention to exception gap:** temporarily disabling maskable interrupts does **not** eliminate NMI, #PF, #GP or machine-check risks during the CR3 switch. A practical design requires a transitional IDT/TSS/handler stack valid across the switch, or explicitly treats any rare unhandled event in that gap as fatal and observable only by host reset/timeout. A fabricated “every fault yields serial” guarantee is unacceptable. An accepted L1 design must choose which of those promises its negative-test oracle makes.

**CPU bit-level details NOT frozen:** exact CR0/CR4/EFER, APIC, page size and TSS selector values, IDT gate type/error-code stubs, syscall-register ABI and code-model linker flags must be specified for the first controlled VM target before code. Generic CPUID detection plus UEFI's x64 entry does not prove every optional feature is enabled or safely configured.

### 4.3 Rust entry ABI must not inherit EFI ABI

UEFI x64 Rust target builds PE32+ with platform calling convention. Freestanding \`x86_64-unknown-none\` produces ELF and \`extern "C"\` uses the System V convention **without a red zone**. The handoff stub explicitly prepares arguments/callee-saved constraints/stack alignment and **does not directly call a SysV function using EFI compiler conventions**. A \`repr(C)\` BootContext is an *internal experimental handoff* with private version/length checks; not a promised native syscall ABI for user Tasks.

**Compiler/toolchain feasibility dependency:** x86_64-unknown-none defaults to a kernel code model. A fixed higher-half image/linker address, actual ELF relocation types and code-generation flags must be mutually compatible and reviewed from produced artifacts *after* future implementation authorization. This microcontract is not a substitute for an ELF/PE loader ABI test.

## 5. Early fault/diagnostic bounds

**Initial safety requirement:** no normal Pager, scheduler, heap, file system or storage. Use a known resident main stack, an independent resident double-fault IST, minimal GDT/IDT/TSS, a small **bounded fixed-record COM1** path (best effort) and a deterministic halt state.

**Positive oracle:** a recognizable series including loader final-map/ExitBS status, kernel-entry marker, validated BootContext and page-map/CPU checks, then \`HALTED_PASS\`. If exit never reaches kernel, host QEMU test distinguishes loader rejection, EFI retry exhaustion, VM reset/triple-fault and timeout; missing serial is **not** PASS.

**Negative controls:** intentionally stale MapKey; too-small map buffer; corrupt ELF header/entry/segment/overlap; unsupported CPU flags; bad CR3/trampoline mapping; incorrect stack/IST; text write and guard alias; COM1 stuck; exception error-code mismatch; unexpected NMI/triple fault and final PASS printed prematurely. Tests remain **future planned**, not run.

**Explicit fault visibility limitation:** an early malformed stack/IDT/CR3 can triple-fault before the serial path is executable; the correct test oracle is QEMU reset/timeout/host-observed failure, **not** a fraudulent promise of on-device panic output from every possible fault. A host-controlled VM watchdog is distinct from the UEFI Boot Services watchdog, which is disabled on successful ExitBootServices.

## 6. Cross-RFC boundary and trust limits

| Owner | L1 proposal / what must remain untouched |
|---|---|
| RFC-001 | preserve small trusted boundary; no user syscalls, cap tables or pretending a working secure microkernel |
| RFC-004 | private initial CR3/stack/map only; no general physical allocator, COW, Pager, mmap or Capability publication permit |
| RFC-005 | 1 BSP/IF masked, no periodic timer, WaitSet, priority inheritance, SMP concurrency or realtime availability |
| RFC-006 | QEMU lab DevUnsigned image; no M0 RecoveryAuthority, BootBundle signature, service restarts, A/B update |
| RFC-017 | production verified boot/TPM/key rotation/rollback are future, no unsupported security claims |
| RFC-007 | no guest physical DeviceGroup DMA, no IOMMU/fence safety proof |
| RFC-002/003/009 | L1 has no Task/Endpoint/CapGrant, no SEND/RECEIVE K-S/K-I, and no public native syscall ABI |
| Product | only "L1 experimental boot diagnostics under selected VM configuration," not usable desktop or guest compatibility |

**Rust-first Hybrid:** exception stubs, MMU/CR3 and UART I/O require tiny reviewed unsafe/assembly; no assertion that pure safe Rust alone can fulfill firmware/CPU handoff.

## 7. Future tests / paper races H-X01…H-X20 — NOT EXECUTED

| ID | Scenario | Required outcome |
|---|---|---|
| H-X01 | valid embedded ELF + prepared maps | BootContext valid, one successful ExitBS, first kernel entry, terminal PASS |
| H-X02 | segment filesz > memsz | reject before ownership transition |
| H-X03 | ELF header / program header arithmetic wraps | reject without copying outside owned pages |
| H-X04 | overlapping PT_LOAD with RW/RX conflicting rights | reject no double writable text alias |
| H-X05 | ELF entry outside executable segment | reject before ExitBS |
| H-X06 | kernel code model / linker VA mismatch | reject at build-contract review or VM negative, not unexplained execution |
| H-X07 | last GetMemoryMap key invalidated before ExitBS | constrained map/exit retry or fail-stop |
| H-X08 | first ExitBS may partially shut down Boot Services | no later EFI console/file/protocol misuse |
| H-X09 | final map buffer too small during restricted retry | fail-stop safely without calling unsupported post-exit APIs |
| H-X10 | reuse EFI loader stack after it becomes unmapped | prevent via explicit dual-mapped transition/final RSP |
| H-X11 | CR3 switch fetch enters unmapped trampoline next instruction | negative VM reset/FAIL, never PASS |
| H-X12 | GDT/IDT/TSS descriptor points to absent pages on new CR3 | reject design or explicit failure oracle |
| H-X13 | high-half kernel text exposed via low writable identity alias | guard and write-protect not satisfied |
| H-X14 | noncanonical RSP/entry/BootContext pointer | reject input/state before kernel execution |
| H-X15 | BSS/BootContext accidentally references unreserved EfiBootServices pages | prevent premature reallocation and bad data |
| H-X16 | #PF/#GP/#UD/#DF diagnostics when expected | bounded record or host-detected fail, not general recovery |
| H-X17 | COM1 absent and busy wait infinite | bounded serial fallback and deterministic halt |
| H-X18 | firmware disables watchdog after ExitBS | host VM test must not rely on it |
| H-X19 | L1 unexpectedly uses EFI Runtime Service/allocates after exit | architectural reject |
| H-X20 | loader or kernel emitted PASS before last validation then triple fault | host oracle rejects false PASS |

## 8. Proposed invariants H-I01…H-I20 — ALL UNPROVED

| ID | Intended property |
|---|---|
| H-I01 | Loader UEFI PE32+ and kernel freestanding ELF target/ABI conventions never silently mix. |
| H-I02 | Every embedded ELF load segment and entry is bounded, of supported type and mapped with explicit access rights. |
| H-I03 | No W+X kernel segment or alternative lower writable alias reaches RX frames in approved L1 mapping. |
| H-I04 | BSS is zeroed and image/stack/context pages are not physically overlapping incompatible owners. |
| H-I05 | Final GetMemoryMap descriptor stride/key preserved through initial ExitBS attempt. |
| H-I06 | After first ExitBS failure, no disallowed non-memory EFI Boot Services are called; retries bounded. |
| H-I07 | After successful ExitBS, **no** EFI BootService/device protocol call or attempted firmware return occurs. |
| H-I08 | New CR3 and handoff trampoline are valid for instruction fetch and temporary stack through full transition. |
| H-I09 | All final kernel/BootContext/IDT/GDT/TSS/IST mappings exist with correct supervisor rights before Rust entry. |
| H-I10 | Source EFI ABI calling convention is not assumed to be freestanding System V \`extern "C"\`. |
| H-I11 | Kernel entry stack alignment, direction flag, red-zone and floating/SIMD assumptions match selected target. |
| H-I12 | Critical exception vectors and #DF IST use owned resident mapped stacks or fail as externally observable VM fault. |
| H-I13 | No general frame allocator treats firmware runtime, ACPI/NVS, MMIO or reserved extents as free. |
| H-I14 | BootContext and map descriptor memory are owned, length-checked and not freed before kernel stops using them. |
| H-I15 | Serial/panic diagnostic path is finite without UEFI console, heap, Pager or potentially held global lock. |
| H-I16 | BSP-only and interrupts-masked L1 doesn't secretly start scheduler, SMP or user Tasks. |
| H-I17 | No mutable user disk, real device DMA or production-verified-boot promise arises from L1. |
| H-I18 | A RESET/triple fault before verified terminal marker is never reported as PASS. |
| H-I19 | Printed PASS is conditional on all admitted positive checks; negative test intentional faults have separate oracles. |
| H-I20 | No source code, executable formal model or VM execution occurs without explicit user-approved L1 implementation Go. |

## 9. Remaining H-O01…H-O08 and exact "READY FOR USER GO" threshold

| ID | Priority | Required final design choice (not actual code/test result) |
|---|---|---|
| **H-O01** | P0 | Pick one linked higher-half virtual base/code model and supported ELF image/payload embedding representation; decide how to reject relocations and segment page-overlap. |
| **H-O02** | P0 | Pick exact early page-table ownership and CR3 transition bridge: which code/stack/descriptors are valid under firmware map and new root, and at what cuts. |
| **H-O03** | P0 | Document literal x86_64 entry ABI / stack/GDT/IDT/TSS/IST/exception cut and a negative test for each nonrecoverable gap. |
| **H-O04** | P0 | Specify first-UART fallback and bounded after-first-ExitBS retry fail-stop semantics that require no forbidden services. |
| **H-O05** | P1 | Freeze test-version *metadata policy*, baseline QEMU Q35/OVMF/CPU/RAM/vCPU/COM1 configuration and initial source dependency governance (actual exact versions may be pinned when later authorized build starts). |
| **H-O06** | P1 | Complete 1-page L1 Go/No-Go scope & B01–B10 evidence references, with a clear "not full OS" warning and user approval needed before implementation. |
| **H-O07** | P1 | Align later test oracle for serial PASS vs host timeout/reset/expected faults; no actual test required before experimental entry. |
| **H-O08** | P1 | Issue and record specific **user** R6 L1 Go. The design cannot grant itself permission. |

**Current assessment:** H-O01/02/03 remain the **non-waivable technical P0**; H-O04 can be substantially satisfied in writing but needs a concrete bounded cut choice; H-O05/06/07 are documentation/governance before approval. **L1 remains NOT READY FOR USER GO**, L2/Capability IPC remains independent NO-GO. Do not hold the experimental L1 gate hostage to Linux/Wine/Darwin or pageable CAPIPC; likewise do not claim a secure multi-Task kernel just from an ELF/CR3 handoff.

## 10. Next focused design paper

**L1 Fixed-Link ELF, CR3 Bridge & Entry-State Closure v0.1.** Finalize the remaining technical H-O01/02/03 simultaneously so that no ABI/page-table assumption contradicts another. The goal is a *reviewable complete L1 design packet*; then run the [Implementation Entry Readiness Gate](../reviews/unios-implementation-entry-readiness-gate-v0.1.md) B01–B10 once more. If all required **pre-code design conditions** are satisfied, notify the user explicitly "**L1 최소 커널 설계 준비 완료 — 구현 승인 가능**" with excluded L2/L3 scope and request their explicit Go; do not write code automatically.

## 11. Sources, provenance and limits

- [UEFI 2.11 Boot Services](https://uefi.org/specs/UEFI/2.11/07_Services_Boot_Services.html): loader allocated-page ownership, current \`GetMemoryMap\` \`MapKey\`, descriptor stride, post-first \`ExitBootServices\` restricted calls, successful relinquishment.
- [UEFI 2.11 EFI System Table](https://uefi.org/specs/UEFI/2.11/04_EFI_System_Table.html): once an EFI OS loader calls \`ExitBootServices\` successfully, it cannot return to firmware before reset.
- [Rust \`*-unknown-uefi\` target](https://doc.rust-lang.org/rustc/platform-support/unknown-uefi.html): PE32+ UEFI application and calling convention.
- [Rust \`x86_64-unknown-none\` target](https://doc.rust-lang.org/rustc/platform-support/x86_64-unknown-none.html): bare-metal ELF, freestanding environment, kernel code model, no red zone and System V \`extern "C"\`.
- [Earlier UniOS minimal boot contract](unios-minimal-boot-exception-memory-mvp-contract-v0.1.md) and other linked RFCs: internal Proposed research, **not** independent evidence that the platform boots.

**Method/limits:** manual comparison and adversarial trace design only. This file does not establish an executable kernel or proof that the proposed CR3/GDT/IDT handoff will work; neither an external OS nor a book defines UniOS's particular private entry ABI.

---

**History:** 2026-10-26 proposed v0.1. H1 embedded ELF and restricted loader; finite map/ExitBootServices retry; high-half W^X + mapped trampoline; distinct EFI/SysV ABI; 20 paper attack cases, 20 unproved invariants, 8 open decisions. No implementation, executable formal spec or Accepted ABI.

## 12. 2026-10-26 Fixed-link/CR3/entry closure and scoped L1 Go readiness — Newer Proposed Finding

**Superseding scoped follow-up:** [L1 Fixed-Link ELF, CR3 Bridge & Entry-State Closure v0.1](unios-l1-fixed-link-elf-cr3-bridge-entry-state-closure-v0.1.md) narrows H-O01/02/03 into **LAB-H1**: fixed **development-only** high-half VA at `0xffffffff80000000`, bounded ELF64 ET_EXEC without dynamic relocations, preallocated four-level 4KiB NX/WP mapping, an EFI-stub with same *linear* VA→physical backing under old/new CR3 verified in preflight, old/new shared bridge stack and early resident IDT/GDT/TSS/DF-IST, then ABI-safe `extern "C"` SysV kernel call. The referenced [L1 Go/No-Go review](../reviews/unios-l1-implementation-go-no-go-assessment-v0.1.md) now judges **L1 *experimental boot* design READY FOR USER GO** only; earlier “not ready” passages remain **dated prior assessments**, not the latest L1 implementation-entry state. Real ELF layout, OVMF old mappings, CR3/IST and negative cases are **post-user-approval verification obligations**, not claimed tested/proven. No user approval or OS code exists.

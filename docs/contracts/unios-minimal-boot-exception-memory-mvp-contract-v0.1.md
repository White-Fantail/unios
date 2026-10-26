# UniOS — Minimal Boot, Exception & Memory MVP Contract v0.1

> **Status:** Proposed design-only *scoped L1 experimental-kernel entry candidate*, **NOT Accepted, implemented, tested, security-validated or ABI-frozen**.
> **Date:** 2026-10-26 NZDT. **User authorization:** design/research only. Do not create loader/kernel/source/build scripts/executable formal models or run QEMU tests until a separately explicit implementation Go decision.
> **Review baseline:** [Architecture v0.2](../architecture/unios-architecture-v0.2.md), [Implementation Entry & Design-Completion Readiness Gate](../reviews/unios-implementation-entry-readiness-gate-v0.1.md), [RFC-001 TCB](../rfcs/rfc-001-kernel-boundary-and-tcb.md), [RFC-004 VM/Pager](../rfcs/rfc-004-virtual-memory-pager-cache-and-oom.md), [RFC-005 Scheduler](../rfcs/rfc-005-scheduling-preemption-and-cpu-budget.md), [RFC-006 Boot/Recovery](../rfcs/rfc-006-boot-trust-chain-service-startup-and-recovery.md), [RFC-017 Verified Boot](../rfcs/rfc-017-verified-boot-updates-and-rollback-protection.md), [L2 IPC publication study](../models/capipc-paired-mapping-publication-invalidation-ordering-v0.1.md).
> **Boundary:** This is **L1**: Tier 0A VM boot/diagnostic exercise, **not** the full capability microkernel, not its stable syscall ABI, not the Native developer platform or a product. It intentionally isolates open L2 CAPIPC/Pager/driver/security debates. L1 may inform later designs without silently promoting any L2 RFC to Accepted.

## 0. Executive architecture and conditional feasibility

**Paper-only first candidate L1-A:** a deliberately restricted **x86_64 UEFI EFI application as loader**, handing off to a **separately linked freestanding x86_64 no_std kernel payload** in QEMU Q35 + OVMF. Loader consumes EFI boot services to validate memory map and stage resident kernel image, then calls ExitBootServices exactly according to firmware contract. Kernel takes complete ownership of its CPU context, *already provisioned* memory and early diagnostics. For initial experiment it does **not** launch user Tasks or use service IPC/Pager. The Rust language split is **x86_64-unknown-uefi loader (candidate target)** versus **x86_64-unknown-none freestanding kernel (Tier 2 target)**, with minimal specifically reviewed Assembly/unsafe for entry, CRx/MSR/GDT/IDT/TSS/serial and memory copy. UEFI and kernel targets have **different executable format/calling-convention/runtime assumptions**.

**Important selection fork, still D0:** the kernel payload may be an immutable *embedded* blob in one EFI application to avoid initial disk/filesystem/dynamic boot-policy code, or a separately packaged/verified payload later; an ELF loader must still validate every loadable segment, relocation, bounds and memory ownership. **For the first approved boot experiment prefer embedded immutable payload and a deliberately minimal payload format/relocation plan; no exact loader image/wire encoding is approved here.** Simply linking PE/COFF UEFI loader and kernel ELF into one binary does not make the x86_64-unknown-none kernel directly executable by firmware.

**First success promise:** from a reset with trusted-for-experiment OVMF and user-controlled read-only boot image, report a bounded boot-stage trace over a known emulated serial device, transfer control exactly once to a resident minimal kernel, establish guarded privileged memory/exception invariants and terminate with a deterministic **HALTED_PASS** status observed via serial/QEMU debug. No disk or guest data is modified. A reset, panic, invalid boot map, bad image or intentionally induced fault must produce a deliberate observable failure **if the serial/report path is available**, else a reproducible no-return fail-stop with VM watchdog/QEMU outer timeout. Success is not a "secure boot", usable operating system or proof of stability.

**Practical feasibility assessment:** good conceptual precedent from UEFI + Rust bare-metal compiler support, but no image format, CPU exception layout or VM scenario has been validated by actual artifacts; not READY FOR USER GO yet. ExitBootServices memory map ownership is tractable for a tiny single-CPU scope on paper; **the weakest portions** are careful loader→kernel page-table/stack/entry execution contract and precise recovery/error handling after partial firmware shutdown. These are targeted below.

## 1. L1 explicit inclusion and exclusion matrix

| Area | L1 includes | L1 explicitly EXCLUDES |
|---|---|---|
| Target | **QEMU x86_64 Q35 + OVMF Tier 0A**, single Bootstrap Processor only, controlled CPUID/VM device assumptions | real reference PC, ARM64, multiprocessor bring-up, hotplug |
| Entry | UEFI x86_64 loader, freestanding Rust-first kernel with separate limited Assembly/unsafe boundary | legacy BIOS boot, runtime EFI application services after ExitBS |
| Memory | firmware-memory map validation, reserved image/stack/page-table layout, nofault fixed early mappings, zero before reuse | user AddressSpaces, pageable VM, COW, swap, file cache, OOM reclaim |
| CPU | early known CR/GDT/IDT/TSS and exception handlers; interrupts masked until vector-safe | general scheduler, timer-driven preemption, inter-Task switching, user mode |
| Diagnostics | explicitly supported emulated **ISA serial COM1** on selected VM profile; small bounded in-memory panic record / QEMU outer exit optional separate decision | storage/file log, network console, production crash dump, trusted GPU/USB |
| Security | protect kernel image/stack, no writable+executable memory, fail on invalid boot inputs, no foreign guest tasks, controlled debug output | production Secure Boot/TPM/antirollback, DeviceCaps, sandbox promises |
| IPC/Capabilities | **none for L1 boot probe**; reserve no public ABI or Task handles | E1 CapGrant MOVE, K-S/K-I, receive publications, SABI, Linux/Windows/Darwin |
| Recovery | deterministic halt on invalid state, no normal Pager/service dependency | M0 RecoveryAuthority, durable A/B rollback, dynamic service restart |
| Storage | only firmware read of designated boot medium **before** ExitBS, using selected read-only image | mutable disks, FS write, persistent update or real user data |

**Scope safety statement:** running ring 0 Rust code in a VM is inherently potentially crash-prone. L1 is an isolated engineering prototype with no external input except the selected image/firmware/VM config and **not** a service that hosts untrusted applications. This exclusion reduces proof debt; it does not make memory-unsafe code harmless outside the VM.

## 2. Candidate machine profile and assumptions

| Property | L1-A paper proposal | First-run evidence required AFTER approval |
|---|---|---|
| Architecture | x86_64 long mode, BSP-only, Q35 machine | record QEMU version, CPU model, CPUID |
| Firmware | OVMF UEFI image whose provenance/version are documented | record OVMF version and image hash |
| RAM | fixed one-VM memory size in approved test matrix, chosen later | capture map, reserved regions and page-table use |
| Emulated output | **ISA COM1** at chosen configured I/O-port base, not a universal device | output can be observed during all intended phases |
| Firmware execution | UEFI Boot Services available only until successful ExitBootServices | simulate stale MapKey and retry |
| CPU protection | NX, required paging features and mandatory exception support validated via CPUID before use | known CPU-feature negative case |
| APs | no secondary CPU startup (SMP out of scope) | verify VM configured for one vCPU |
| Device DMA | never grant or initiate DMA and never claim PCI/IOMMU isolation | no DeviceGroup security proof claimed |
| Persistence | read-only volatile boot image, no writable user-data media attached | verify QEMU file attachment options |

**Unknown platform assumptions:** QEMU emulation fidelity to a physical UEFI PC, firmware configurations, exact UART readiness/timeout behavior, kernel entry virtual-address canonicality, CPU exception stack requirements and linker/relocation needs must be documented as assumptions, not measured properties.

## 3. EFI Loader → Kernel ownership transfer (B02 and B05)

### 3.1 State/owner table (conceptual, no code)

| State | Image/boot owner | Allowed services | Transition / failure |
|---|---|---|---|
| **L-0 EFI accepted** | OVMF accepted the EFI application for this test mode; firmware controls memory map | normal permitted EFI Boot Services | malformed/unsupported platform → show error if console available, exit EFI app |
| **L-1 Image staged** | EFI loader has explicitly allocated/verified kernel image pages, page tables, stack, diagnostic buffer | allowed EFI services | reject bounds/overlaps/CPU mismatch before irreversible transition |
| **L-2 Handoff prepared** | loader owns immutable prepared BootContext and fixed reserved ranges | only carefully defined map/exit path | no unbounded late allocations; boot-map buffer capacity prepared with slack |
| **L-3 Exit attempt** | firmware controls memory map until success | latest GetMemoryMap immediately before ExitBootServices; after a first failed ExitBS, firmware may partially shut down | stale key/EFI_INVALID_PARAMETER → limited permitted memory-service retry; no filesystem/console/other boot protocol assumption |
| **L-4 Boot Services gone** | UniOS loader/kernel exclusively responsible for continuation under validated map | **no EFI Boot Service or handle-based firmware protocol** | mark boot handoff committed, must not attempt boot-services rollback on fault |
| **L-5 Kernel entry** | resident kernel owns CPU stacks/tables/page maps/diagnostic path | no firmware boot services, EFI runtime deliberately unused in first profile | verify BootContext authenticity within shared trusted image; invalid → minimal serial + fail-stop |
| **L-6 Diagnostic done** | kernel kernel-only | no user/task execution | report final controlled state, halt with interrupts masked |
| **L-F Failure** | depends on phase | prior ExitBS: EFI app may return; after success: kernel only | never claim ability to safely return to UEFI after ExitBS; deterministic fail-stop |

**UEFI authoritative constraints:** GetMemoryMap returns actual map descriptor **size and version**, not an always-fixed structure stride. Buffer sizing requires extra headroom because buffer allocation changes the map. ExitBootServices requires the last current MapKey. If it returns EFI_INVALID_PARAMETER, get map+retry using only the service subset permitted after the first Exit attempt; prior boot console/filesystem operations are **not assumed safe** after failed ExitBS. Post-success no Boot Services calls; Firmware UEFI watchdog may be disabled. These follow UEFI spec, **not a UniOS test pass**.

### 3.2 BootContext conceptual fields (not a stable C/Rust ABI)

BootEpoch/boot mode = DevUnsigned in Tier 0A; serialized firmware memory descriptors + DescriptorSize/Version; actual ExitBS success provenance; kernel image/text/data/bss and (physical+virtual) reserved ranges; dedicated kernel guarded stack and double-fault emergency IST stack; page table root and mapping regime; reserved BootContext lifetime; instruction pointer/stack pointer handoff invariants; BSP CPU feature manifest; diagnostic serial base selected by VM profile; optional output of test-case ID; bounded physical diagnostic record. Header includes size/version/check-like integrity guard for **accidental corruption**, not a cryptographic claim. Every address/length/entry offset has nonwrap canonicalness/physical-range/alignment/memory-type checks. No kernel permission derives from a mutable external service registry string.

**Avoid hidden trust:** BootContext and kernel payload are both part of the same explicit trusted experimental boot image and pre-validated buffers. A checksum can catch accidental corruption but isn't equivalent to BootBundle signer validation, TPM measurement or production Secure Boot. No untrusted service can edit BootContext after ownership transfer.

### 3.3 Ownership and permissions

- Prior to successful ExitBS, only explicitly acquired/allocated loader pages may be used; firmware-reserved or memory-map "available" entries are **not** automatically writable by UniOS.
- Kernel, loader-needed resident handoff metadata, stack, exception IST, page tables and serial diagnostic backing remain reserved across handoff.
- After success, interpret EFI memory types explicitly: Boot Services Code/Data may become reclaimable **only** when no live handoff/kernel refs depend on them; preserve RuntimeServices, ACPI/NVS, MMIO/Reserved/unknown. L1 chooses **conservative no-general-frame-allocation policy**: leave excess free pages unused rather than introduce incomplete PMM.
- Loader must not inadvertently call \`ExitBootServices()\` and then rely on stale firmware page tables/console or old loader stack to transfer execution. Kernel entry needs a documented stack and valid mapping and ABI.
- Precisely account when loader temporary buffers may be discarded; the kernel must never use a pointer into retired or reclaimed pre-exit allocation without preserved ownership.

## 4. Initial memory safety envelope (B03)

### 4.1 Proposed L1 fixed mapping regime

**First proof target:** construct or validate an explicitly enumerated supervisor-only mapping set sufficient to execute the resident kernel and panic handlers. Whether loader installs a new page-table root immediately before handoff or the kernel installs it at the first safe entry is a **P0 selection still OPEN**; either case must prevent unmapped execution, mutable text alias and bad stack fault before IDT.

Required regions:

- **Kernel text:** executable, supervisor-only, read-only after final image setup; no RWX alias.
- **Kernel rodata:** read-only, NX and supervisor-only.
- **Kernel data/bss:** writable, NX and supervisor-only; bss zeroed before use.
- **Kernel main stack:** fixed resident, page-aligned, NX, explicitly guard-unmapped on overflow; known ABI alignment at entry.
- **Double-fault/critical exception IST stack:** separate guard and resident pages; IDT and TSS set before provoking fault.
- **Kernel page tables:** explicit ownership of each paging-structure physical frame; writable only in bounded privileged modification path, NX/no arbitrary user mapping.
- **BootContext and memory map:** fixed reserved lifetime; preferably immutable to ordinary kernel components after validation, not freed prematurely.
- **UART/MMIO:** COM1 in configured legacy I/O-port region; don't presume MMIO framebuffer or full identity map.

**Minimal paging anti-assumptions:** the loader may run under firmware-established page tables; the freestanding kernel cannot assume its linked virtual addresses are presently mapped just because CPU long mode is active. QEMU memory layout does not guarantee identity mapping at all desired physical ranges. Enabling NX requires checking CPUID support and setting correct CPU features. No user-mode mappings / W^X exceptions are needed for L1. Unknown firmware runtime regions and absent CPU features **fail** the chosen profile rather than silently request legacy unsafe mode.

### 4.2 Exact memory-map acceptance rules

- Validate all descriptor stride/length arithmetic, minimum descriptor size for known fields, memory type and attribute bounds, page alignment and address-space extent/nonwrap. Unknown descriptor versions/types are **not automatically free**.
- Represent every ownership-critical allocated range with a source: kernel payload, loader retained segment, BootContext, stack, IST, page tables, diagnostic static memory or firmware reservation; no writable overlap unless intentionally the same controlled alias and proven permission-compatible.
- Disallow physical address 0 and exception-critical unmapped pages from free/allocation pool. Guard pages must actually fault, not be shadow-mapped via an unintended broad identity alias.
- No third-party DMA; PMM does not reclaim or reuse external frames at L1. This simplifies, but doesn't prove, frame lifetime.
- Selftests must be able to observe that text write and stack guard produce expected synchronous protection faults in a **dedicated negative scenario**, not during ordinary success.

**Open design choice:** exact post-ExitBS mapping transition/trampoline, page sizes, internal ELF segment loading strategy and BootContext compatibility version are required to move B03 from D0 to D1. The initial prototype may stay permanently in its tiny fixed mapping set.

## 5. x86_64 CPU, exception and interrupt boundaries (B04)

### 5.1 Before kernel calls any fallible Rust code

Candidate contract:
- privileged BSP long mode with interrupts masked for entire L1 unless a dedicated vector test requires known controlled enabling (default: keep IF=0);
- establish known stack pointer/ABI alignment and **no red-zone** assumptions; avoid hidden floating-point/SIMD state instructions unless CPU state contract supports them;
- GDT + 64-bit code selector + TSS with a dedicated IST for **double-fault**; IDT entries for \`#PF\`, \`#GP\`, \`#UD\`, \`#DF\`, \`#BP\` diagnostic and NMI / machine-check minimal fail-stop as platform permits;
- explicitly preserve CPU error-code semantics per exception vector and avoid recursive serial/heap/lock use in the fault handler;
- a double fault or corrupted stack cannot rely on the ordinary panic stack; if DF IST cannot safely run, fail-stop without false diagnostic guarantees;
- reject unsupported CPUID/paging conditions before instruction path that requires them;
- no userspace interrupt/scheduler/preemption/APIC calibration; unexpected IRQ/NMI remains a controlled fault/fail-stop condition, not an implied supported device.

**Caveat:** \`#DF\` can fail catastrophically when page tables/TSS/GDT are broken, so never promise a readable serial trace from every possible hardware failure. "Observable when diagnostic channel available, otherwise VM outer timeout" is the truthful test oracle.

### 5.2 Exception taxonomy

| Class | Example | Allowed L1 outcome |
|---|---|---|
| **Expected negative test** | deliberate write-to-text, guard fault, invalid opcode, general protection violation | record vector/error and minimal CR2/PC/stack if safe; fail-stop or end explicit negative test without resuming unless recovery semantics separately defined |
| **Unexpected synchronous exception** | bad pointer, CPU feature mismatch, stack corruption | panic to bounded serial/record; stop, no nested C/C++ unwinding |
| **NMI/MCE** | injected/real machine check, unmaskable CPU event | best-effort emergency record and fail-stop; do not promise universal recovery |
| **Triple fault/unrecoverable state** | corrupt GDT/IDT/page tables/IST | VM outer abort/reset/timeout recognized as **failure**, never silent PASS |
| **Halt after success** | expected boot-stage validations pass | explicit HALTED_PASS marker and halt; no long-lived interactive OS promise |

One final global state must be distinguishable from kernel exceptions; invalid test that incorrectly prints PASS before a subsequent panic is a **test-oracle design flaw**. Final marker is issued only after all selected checks and deliberate no-return halt.

## 6. Diagnostics, debug and error disclosure (B06)

- **Primary first design candidate:** fixed emulated ISA COM1 UART, configured in QEMU launch profile and usable after ExitBS without UEFI ConsoleOut; no dependence on GPU, USB, filesystem, or EFI Runtime Services.
- UART write must be bounded/nonblocking at panic (if unavailable, truncate/omit safely); exception path must not deadlock waiting for a serial buffer or lock held by the interrupted code. Bounded small serial records may include stage, build ID, feature flags and fault vector; no uncontrolled raw memory dump.
- **Secret boundary:** DevUnsigned L1 contains no passwords, user disk or cryptographic keys. Still avoid leaking arbitrary firmware memory bytes, raw uninitialized stack, TPM material or DMA buffers into logs.
- **Optional separate host oracle:** a deliberately named QEMU exit/debug interface can supplement serial logs **only if** its VM ABI/emulator dependency is documented. Do not couple ordinary kernel functionality to it or claim it exists on physical PCs.
- No persistent kernel crash files. Failure before ExitBS can use permitted EFI error outputs; failure during/after a partial ExitBS attempt should use serial path or fail-stop, not arbitrary firmware console.
- Log construction has bounded stack/static storage and no heap/allocations in panic/DF/NMI.

## 7. Build/boot source-layout contract (B07) — design only

**Candidate layout for later authorized implementation:**

| Logical module (not new repository files now) | Responsibility |
|---|---|
| UEFI loader binary (x86_64-unknown-uefi) | firmware entry, validated image/BootContext staging, GetMemoryMap→ExitBS, CPU handoff |
| freestanding kernel image (x86_64-unknown-none) | Rust no_std/no implicit std, isolated CPU/IDT/TSS/VM/serial diagnostics |
| kernel architecture boundary (minimal Assembly/unsafe) | register/stack/CRx/GDT/IDT/TSS and I/O, with documented invariants |
| handoff contract (private to experimental build) | typed fixed-size/length-checked BootContext and residency, no public stable ABI |
| build/VM profile metadata | pinned Rust release/toolchain/crate hashes, OVMF/QEMU versions, exact VM command/test case |
| artifacts and checks (future) | source→kernel ELF→embedded payload→EFI boot image provenance, ELF/PE validation, reproducibility observation |

**Target details verified from Rust docs, not UniOS build:** \`x86_64-unknown-none\` is Tier 2, bare-metal ELF without std and no default allocator, kernel code model default and no red zone in SysV-style C ABI; UEFI applications use a distinct target and firmware calling contract. Disallow assuming the two targets have the same linker artifact/ABI. A precise target/edition/release/loader architecture is **not fixed to numeric version** until actual dependency audit. Architecture's Rust-first hybrid policy remains Accepted only at high level.

**No third-party kernel code copied into UniOS.** Library licenses, toolchain/OVMF packaging and graphics assets must be recorded under D6 before public binary release. Rust toolchain itself and firmware image licenses are distinct from UniOS's core policy.

## 8. Planned L1 observable validation matrix (B08; NO tests executed)

| ID | Input/fault class to stage in QEMU **after user authorizes code** | Expected oracle / gate |
|---|---|---|
| **L1-X01** | supported OVMF + valid image + correct map | staged trace; one ExitBS success; kernel reports HALTED_PASS |
| **L1-X02** | stale MapKey / map churn before ExitBS | limited valid GetMemoryMap/ExitBS retry; never arbitrary Boot Services after first failed exit |
| **L1-X03** | memory-map buffer too small then resized | no truncation/overrun; extra descriptor slack; map key refreshed |
| **L1-X04** | malformed descriptor stride/version/size | reject/stop before using data; no allocator ownership from it |
| **L1-X05** | overlapping kernel/stack/firmware reserved range | stop, no unsafe page free/write |
| **L1-X06** | zero/overflow kernel image segment/entry/relocation | reject before control transfer |
| **L1-X07** | unexpected CPU feature absent (NX/etc.) | reject selected VM profile safely |
| **L1-X08** | expected #PF on unmapped guard page | correct fault classification on separate exception stack where designed |
| **L1-X09** | write to RX kernel text | #PF protection, no silent execution, no PASS |
| **L1-X10** | controlled #UD/#GP/#BP scenarios | correct error-code handling, bounded diagnostic then halt |
| **L1-X11** | corrupt/tiny main stack to trigger #DF | IST or controlled VM failure, no false PASS; cannot promise trace on triple fault |
| **L1-X12** | UART unavailable or TX stuck | bounded no-return fail-stop; no blocking infinite serial wait |
| **L1-X13** | Boot Services call mistakenly attempted after successful ExitBS | architectural negative control; reject as design/code review/test failure |
| **L1-X14** | invalid image signature/hash (future DevSigned, **not** DevUnsigned core) | maintain explicit distinction; no fake production Secure Boot result |
| **L1-X15** | VM configured >1 vCPU despite BSP-only assumption | reject config/verify APs not enabled; no hidden SMP |
| **L1-X16** | QEMU reset/triple fault during "success" path | host oracle says FAIL, never rely on missing console as PASS |
| **L1-X17** | deliberate panic with bogus format input/log length | bounded panic output, no unwinding/heap, no raw RAM dump |
| **L1-X18** | write-protect/guard aliases with broad identity map | fail if protected page also accessible through unintended writable alias |

**Success oracle requirements:** per-run build and VM identifiers; starting stage, protected exit cut and final marker; finite host timeout is an *external test harness rule*, not an OS scheduling guarantee. Distinguish "no serial", "VM crashed", "kernel halted after PASS", "loader rejected", "test expected fault", and "firmware hung". No test timing target or CI artifact is yet created.

## 9. Paper-only invariants L1-I01…L1-I20 — NONE PROVED

| ID | Intended property |
|---|---|
| L1-I01 | The first trusted boot artifact and its provenance are explicitly scoped to DevUnsigned Tier 0A. |
| L1-I02 | Kernel entry never depends on usable Boot Services after a successful ExitBS. |
| L1-I03 | After an initial failed ExitBS, no forbidden non-memory EFI service is used by the loader. |
| L1-I04 | The ExitBS key used is from the latest valid GetMemoryMap/descriptor stride, barring an explicitly retried map mutation. |
| L1-I05 | Every BootContext image/stack/page-table/descriptor range is aligned, bounded and integer-nonwrapping. |
| L1-I06 | Firmware-owned/reserved/runtime/MMIO/ACPI extents are not treated as free frames. |
| L1-I07 | Kernel text/rodata/data/stack/IST page rights are separate, guarded and not writable+executable through aliases. |
| L1-I08 | Entry PC/SP, CPU feature flags, executable payload placement and page-table root are coherent before entry. |
| L1-I09 | Kernel doesn't copy/relocate image into an unowned/overlapping frame. |
| L1-I10 | Uninitialized data exposed via BootContext/logs is excluded; bss is zeroed before use. |
| L1-I11 | IDT/GDT/TSS and DF IST are resident before any intentional exception fault test. |
| L1-I12 | Fault handlers never require ordinary heap, Pager, files, scheduler or a potentially held serial lock. |
| L1-I13 | No unhandled exception, triple-fault or host reset is marked a kernel HALTED_PASS. |
| L1-I14 | L1 does not create ambient native CapGrant/Task/IPC/Device/DMA privileges. |
| L1-I15 | All code paths remain BSP-only as configured; unexpected IRQ/NMI is fail-stop, not fake supported IRQ path. |
| L1-I16 | Kernel panic/serial reporting is bounded and does not reveal raw firmware/user secrets. |
| L1-I17 | No mutable user disk or firmware NVRAM update is performed as part of L1 boot success. |
| L1-I18 | Rust/no_std/kernel and UEFI calling convention/cpu-state assumptions are not conflated. |
| L1-I19 | The VM test profile has no physical-platform security inference or performance claim. |
| L1-I20 | No implementation is launched without separate explicit user Go for the named L1 scope. |

## 10. Preliminary B01–B10 readiness evaluation as of writing

| L1 gate | This contract's contribution | Residual decision / status |
|---|---|---|
| **B01** | QEMU x86_64 Q35/OVMF Tier0A and Rust-first loader/kernel chosen as **paper candidate** | **REVIEWABLE, NOT APPROVED**: final emulator/vCPU/image profile when experimental Go proposed |
| **B02** | ExitBS state machine and BootContext ownership/failure now explicit | **PARTIAL**: exact payload linking/entry/trampoline, error path and post-failed-ExitBS boot path need one definitive design |
| **B03** | fixed protected map + no frame reclaim + guard/IST | **PARTIAL**: page-table adoption/replacement and alias-safe entry not yet chosen |
| **B04** | BSP IF=0, IDT/GDT/TSS/IST, terminal no-resume fault semantics | **REVIEWABLE**, pending exact vector/CPU-state selection |
| **B05** | no Task/IPC/CapGrant/DevCaps/Scheduler/Pager — hard scope exclusions | **REVIEWABLE**, explicitly L1 only |
| **B06** | emulated ISA COM1 bounded diagnostic + halt, secret-minimized panic | **REVIEWABLE**, VM UART assumption/test oracle not yet demonstrated |
| **B07** | distinct UEFI/none Rust targets, minimal linked images, toolchain provenance plan | **PARTIAL**: exact private BootContext linkage/image loader options still OPEN |
| **B08** | eighteen paper-only valid/error test cases and pass/fail oracle | **REVIEWABLE**, no actual tests, which are post-approval |
| **B09** | no physical disk/user data, DevUnsigned lab-only, research/implementation separation, license audit | **REVIEWABLE**, branch/workspace governance needs Go scope |
| **B10** | full scope/exclusion outline and residual risks | **PARTIAL**: final cut/approval card and direct user Go after technical decisions |

**Result now: L1 experimental boot implementation entry remains NO-GO / NOT READY FOR USER GO.** This is progress from “undefined” to a concrete bounded target and test matrix, **not** a declaration of completed safe implementation design. L2-native remains NOT READY and independent. In particular, **B02/B03/B07/B10** are still material P0 for actual first boot. Other B statuses are *reviewable drafts*, not accepted ADRs.

## 11. Exact remaining design decisions (L1-O01…L1-O10) — OPEN

| ID | Severity | Question and next paper artifact |
|---|---|---|
| L1-O01 | P0 | Which **single boot artifact strategy** (one embedded kernel payload EFI application vs separate file) will L1 actually choose, and what bounds/relocation handling make it safe? |
| L1-O02 | P0 | Exact entry relocation/VA/CR3/trampoline and stack switch order across ExitBS into freestanding Rust, with no hidden firmware call. |
| L1-O03 | P0 | Exact UART/serial error path before/after first unsuccessful ExitBS and post-success, including nonblocking panic path. |
| L1-O04 | P0 | Settle protected page-table layout and guard-unalias strategy without a general PMM/heap/Pager. |
| L1-O05 | P0 | Define precise x86_64 CPU entry ABI, registers, SIMD-state, GDT/IDT/TSS/IST and no-resume interrupt semantics. |
| L1-O06 | P0 | Document valid EFI memory descriptor type→reserved/reclaimable table for this no-allocator prototype. |
| L1-O07 | P1 | Pin exact Rust/QEMU/OVMF/edk2 toolchain and third-party loader dependencies **before** source is introduced. |
| L1-O08 | P1 | Final validation oracle and QEMU profile: reset/triple fault, serial availability and non-user-data disk policy. |
| L1-O09 | P1 | Future branch/version layout and untrusted/malformed payload test provenance, no release claims. |
| L1-O10 | P0 | Issue a compact R6-scoped L1 implementation Go/No-Go proposal with residual risk and explicit approval controls. |

## 12. Recommended next design-only task, before asking for a Go

**L1 UEFI Handoff & Kernel Entry Microcontract v0.1** is the highest-value next paper:
- choose exact embedded image packaging/ELF loader vs simpler fixed-layout boot image;
- resolve KIMAGE placement, memory ownership and entry CPU/CR3/stack/guard/trampoline;
- define ExitBS last-map-key/first-failure restricted retry and serial failure path with no undefined boot-service use;
- decide whether first VM kernel adopts firmware paging briefly or creates its own protected initial page tables;
- produce a concrete x86_64 exception/CPU-state and B01–B10 exit judgment.

If these conditions converge and the small L1 scope stays frozen, publish **"L1 Design Ready — Implementation Can Be Authorized"** and request a user Go. This does not imply safe GUI, Linux/Windows/Darwin binary support, Capability microkernel, M0 production recovery or a stable public KABI.

## 13. Official sources and limitations

- [UEFI Boot Services: GetMemoryMap/ExitBootServices, descriptor stride and first-call failure constraints](https://uefi.org/specs/UEFI/2.11/07_Services_Boot_Services.html).
- [Rust bare-metal \`x86_64-unknown-none\` Tier 2 target](https://doc.rust-lang.org/rustc/platform-support/x86_64-unknown-none.html).
- [Rust supported targets and separate UEFI target](https://doc.rust-lang.org/rustc/platform-support.html).
- [UEFI Specifications index](https://uefi.org/specifications).
- [UniOS reviewed RFC-001/004/005/006/017, architecture v0.2 and readiness gate](../reviews/unios-implementation-entry-readiness-gate-v0.1.md).

All target/platform facts above are **published precedents**, not UniOS measurements. The design only reduces unknowns and records falsifiable post-implementation tests; it does **not** prove that a freestanding kernel will boot or exception entry/exit is correct.

---

**History:** 2026-10-26 initial **Proposed** L1 MVP contract. 18 paper-only fault/test cases, 20 unproved invariants, 10 OPEN scope/entry decisions; no loader/kernel/model code, ABI or implementation approval.

## 14. 2026-10-26 UEFI/ELF/CR3 entry microcontract follow-up — Proposed

[L1 UEFI Handoff & Kernel Entry Microcontract v0.1](unios-l1-uefi-handoff-kernel-entry-microcontract-v0.1.md) compares and narrows the L1-A boot choice to an EFI PE32+ application **embedding a restricted, separately linked ELF64 freestanding kernel**, preallocating resident payload/page tables/main+IST stacks/BootContext/map buffer, then using last GetMemoryMap Key and bounded spec-conforming ExitBootServices retry. It proposes **new high-half CR3 plus a transition trampoline that must remain mapped across the CR3 switch**, and explicit UEFI-to-SysV no-red-zone stack/CPU/IDT handoff. It does NOT show a working trampoline: exact linked base, initial map layout, early descriptor/exception window and loader subset are still H-O01/02/03 and L1 B02/B03/B07 open. [L1 readiness](../reviews/unios-implementation-entry-readiness-gate-v0.1.md) remains NO-GO and no code/test/ABI approved.

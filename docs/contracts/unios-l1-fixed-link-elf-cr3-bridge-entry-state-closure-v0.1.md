# UniOS — L1 Fixed-Link ELF, CR3 Bridge & Entry-State Closure v0.1

**Subsequent approval status (2026-10-26):** The user explicitly approved only the bounded L1 experimental scope in the [L1 User-Go Record](../reviews/unios-l1-scoped-user-go-2026-10-26.md). This is a historical design contract and not evidence of code or boots. Current repository work remains documentation/design only.


> **Status:** Proposed *L1 implementation-entry design closure* for a **disposable Tier 0A QEMU/OVMF experiment** — 2026-10-26 NZDT. Not an Accepted product architecture/ABI, runnable model, booted artifact, formal proof, verified x86 exception handler, measured test, or authorization to implement.
> **Strict project rule:** Only research and GitHub **documents** are authorized. Do not write even a loader/kernel/trampoline/build script until the user **explicitly approves the bounded L1 implementation** in the associated [Go/No-Go Assessment](../reviews/unios-l1-implementation-go-no-go-assessment-v0.1.md).
> **Upstream:** [L1 UEFI Handoff & Kernel Entry Microcontract v0.1](unios-l1-uefi-handoff-kernel-entry-microcontract-v0.1.md); [Minimal Boot, Exception & Memory MVP](unios-minimal-boot-exception-memory-mvp-contract-v0.1.md); [Implementation Readiness Gate](../reviews/unios-implementation-entry-readiness-gate-v0.1.md); [Architecture v0.2](../architecture/unios-architecture-v0.2.md); [RFC-001](../rfcs/rfc-001-kernel-boundary-and-tcb.md), [RFC-004](../rfcs/rfc-004-virtual-memory-pager-cache-and-oom.md), [RFC-005](../rfcs/rfc-005-scheduling-preemption-and-cpu-budget.md), [RFC-006](../rfcs/rfc-006-boot-trust-chain-service-startup-and-recovery.md), [RFC-017](../rfcs/rfc-017-verified-boot-updates-and-rollback-protection.md).
> **Authority:** UEFI 2.11 Boot Services; Rust target platform descriptions for x86_64 UEFI and x86_64-unknown-none; Intel 64 SDM Vol. 3A paging/interrupt/IST. Published platform descriptions are technical constraints, **not UniOS boot evidence**.

## 0. Executive answer and scope of decision

This study **selects the L1 laboratory-only, pre-implementation contract** sufficiently narrowly to let the user authorize the **first experimental boot implementation**. It is **not** a proof that this software will boot: the runtime old-to-new CR3 mapping, emitted ELF/linker sections and TSS/IDT entries are specifically marked as **first implementation verification gates**, not assumptions to silently trust.

**LAB-H1 declared choices:**

- **Machine:** QEMU Q35 x86_64, **one vCPU**, OVMF UEFI, 4-level IA-32e paging with LA57=0, controlled CPU with NX support, serial COM1 port I/O, no attached writable user disk. This is not a physical-PC security claim.
- **Image:** a single **UEFI PE32+** loader for the \`x86_64-unknown-uefi\` Rust target embeds a separately linked **ELF64 ET_EXEC freestanding** \`x86_64-unknown-none\` kernel. It is an *experimental image packaging contract*, not a cryptographically signed BootBundle. The loader accepts **only its own bounded embedded ELF subset**.
- **Development-only kernel virtual link base:** **\`0xffffffff80000000\`** (negative 2 GiB boundary, 2 MiB aligned; x86_64 kernel code model), initial reserved kernel VA *envelope* \`[0xffffffff80000000, 0xffffffff84000000)\` (**64 MiB maximum virtual span**). These are **lab configuration values**, not the future UniOS Native ABI, ASLR policy or production KASLR layout. If emitted ELF's first mapped address differs, fail the artifact review rather than quietly relocate.
- **Paging:** **4 KiB mappings and 4-level PML4** for all protected, guard and bridge-critical pages, **CR0.WP=1**, **EFER.NXE=1**, mapped kernel text RX, read-only data R/NX, mutable BSS/data RW/NX, separately controlled page tables, *no writable alternate alias of kernel RX frames*. No generic low-physical direct map. CPU features and LA57/PCIDE mode must match the restricted profile or boot is rejected.
- **Bridge:** a **small EFI-loader-resident executable transition stub and a distinct preallocated bridge stack**, both reachable by exactly the **same linear addresses under old firmware CR3 and the new UniOS CR3**. CPU obtains/validates the original physical backing of these linear ranges **before ExitBootServices**, and constructs matching mappings in the new root. The selected QEMU/OVMF implementation must actually satisfy this; do **not** assume all physical memory is identity-mapped.
- **Entry:** after valid **last GetMemoryMap→ExitBootServices**, enter bridge at controlled current linear RIP, mask IF, switch to bridge stack, install/retain mapped low emergency GDT/IDT/TSS/IST, switch CR3, move to higher-half final stack, then call a freestanding \`extern "C"\` Rust entry using the **System V x86_64 ABI (no red zone)**. No UEFI ABI crosses that call boundary or returns.
- **Termination:** bounded COM1 serial stage/fault record and single **HALTED_PASS** only after declared validations; unexpected exception and triple fault are **FAIL/host-detected reset or timeout**, never PASS.
- **Explicitly excluded:** user mode, Task/CapGrant/K-S/K-I, Pager, heap/PMM/reclaim, scheduler/SMP, filesystem writes, real device DMA, Verified Boot/TPM, GUI, Linux/Windows/Darwin compatibility, stable syscall ABI.

**L1 result distinction:** **Design is sufficiently scoped for a human Go/No-Go decision** after the [readiness assessment](../reviews/unios-l1-implementation-go-no-go-assessment-v0.1.md), but that is **NOT user permission**; both implementation and runtime verification are still absent. L2 native microkernel and L3 product remain NO-GO.

## 1. Fixed-link embedded ELF64 subset — LAB-ELF1

| Item | Selected experimental limit / rule | Why and failure |
|---|---|---|
| format | little-endian ELF64, \`EM_X86_64\`, \`ET_EXEC\`, single exact embedded immutable payload | reject arbitrary user/kernel ELF, mismatched class/endian/machine |
| placement | linked VAs in selected 64 MiB high-half window; segment PAs **allocated by loader** | never treat \`p_vaddr\` as physical or assume EFI directly executes ELF |
| program headers | **up to 8 \`PT_LOAD\`**; inspect \`PT_NOTE\`, \`PT_GNU_STACK\` (non-exec) and \`PT_GNU_RELRO\` as constraints, not mapped payload types | reject unknown required headers, \`PT_INTERP\`, \`PT_DYNAMIC\`, TLS/dynamic imports; segment count is bounded |
| relocation | no dynamic relocation, PIE, IFUNC, external unresolved symbol or runtime linker; **reject any ELF whose code/data require runtime fixups** | private L1 loader need not be a general dynamic relocator |
| alignment | validated nonzero power-of-two \`p_align\`, congruent \`p_offset/p_vaddr\`, page-granular mapping and no inconsistent page-share permissions | reject overflowing or ambiguous permission alias |
| bounds | checked 64-bit arithmetic; \`p_filesz <= p_memsz\`; all offsets/entry in embedded ELF; full **loaded data span <= 16 MiB** for lab profile | reject malformed rather than allocate unbounded |
| segment pages | no VA overlaps except explicitly validated compatible page ownership; segment bytes copied to own PAs; BSS zeroed | no writable+executable page and no uninitialized reads |
| entry | canonical high-half \`e_entry\`, within a **mapped executable** \`PT_LOAD\` page | reject branch to data/invalid VA |
| load envelope | \`[0xffffffff80000000,0xffffffff84000000)\` lab only | reject emitted linker layout outside window; not final ABI |
| metadata/provenance | immutable embedded image/build artifact manifest, no signer trust claim in DevUnsigned mode | a checksum isn't authenticated boot |

**Important compiler caveat:** Rust's \`x86_64-unknown-none\` uses the *kernel code model by default* and can emit ordinary ELF program headers depending on linker/flags. The exact allowed header set must be reconciled with the **real future linker artifact** before launching QEMU. A rejected initial build is an expected **engineering discovery**, not permission to weaken ELF bounds or add general dynamic loader support silently. Loader may parse ELF section headers solely for diagnostics; runtime load authority derives from validated program headers and explicit no-relocations contract.

**No numeric user-facing kernel ABI:** the fixed VA range, ELF bounds and BootContext size/version are private to disposable Tier 0A images. Their changes require a documented experimental manifest, not public backwards-compatibility guarantees.

## 2. Physical page ownership and development-only limits — LAB-MEM1

| Reserved object | Initial lab envelope | Ownership and audit |
|---|---|---|
| kernel staged \`PT_LOAD\` page backing | <=16 MiB loaded data; 4 KiB pages | AllocatePages before final map, exact segment-by-segment own PAs, no overlap with firmware Reserved/ACPI/runtime |
| page-table root and lower tables | fixed precharged **512 KiB** *tentative lab cap* | separate allocated page-table frames, prove sufficient for actual 4 KiB mappings or reject before ExitBS |
| bridge executable + bridge stack | bounded code from trusted EFI image + **8 KiB bridge stack**, no unrelated frame writable alias | executable image mapping and independent reserved stack, reachable old/new CR3 |
| main kernel stack | **16 KiB**, one **4 KiB virtual guard** below | supervisor RW/NX; no direct physical alias at guard |
| double-fault IST stack | **16 KiB**, separate **4 KiB virtual guard** | IST1 in resident TSS; no ordinary stack/heap dependency |
| IDT/GDT/TSS + minimal emergency handler | fixed resident preallocated pages | mapped under both CR3 while crossing; selector/IST/handler addresses valid |
| immutable BootContext + map snapshot | bounded by preallocated **64 KiB map buffer** plus fixed header/manifest | retained after ExitBS, read-only once loaded; DescriptorSize-aware and fail if too small |
| firmware-excluded ranges | UEFI Reserved, ACPI/NVS, RuntimeCode/Data, MMIO and unknown | no general PMM, no reclaim even if EFI type later may permit it |
| diagnostic | polled ISA COM1, **finite per-byte polling** and small static record | never wait infinitely or log raw memory dumps |

**Numerical nature:** the listed KiB/MiB values are **test-profile caps and failure criteria**, not measured safe minimums. A compiler/linker/runtime can legitimately exceed them, in which case the first image build is rejected and the manifest reviewed **before** broadening a cap. No high-assurance bound is claimed.

**Memory map validation:** use actual descriptor \`DescriptorSize\` and \`DescriptorVersion\`, bounded \`MemoryMapSize\`, valid alignment and checked physical extent; do not recast variable-stride descriptors as a tightly packed Rust array. Only pages explicitly allocated by the loader can be assumed writable **before** successful ExitBootServices. Keep firmware reserved/runtime, all kernel frames, bridge and stacks alive; don't rely on \`EfiBootServicesData\` automatically reusable in this no-PMM experiment.

## 3. Exact shared-linear bridge strategy — LAB-CR3-1

### 3.1 Old and new address-space overlap is an explicit condition

**First-prototype policy:** an EFI \`.text\` transition function with **known current linear RIP pages** remains resident through the handoff; the loader uses its image allocation provenance and a **pre-ExitBootServices translation audit** to determine the **actual physical frames backing these current linear pages**, bridge stack and low exception descriptors. These are mapped at **the identical old linear VA addresses** in the new CR3 with compatible R/X or RW/NX rights. The bridge ranges must not overlap kernel high-half link VAs, other frames or guard pages.

**Not an assumption of universal UEFI identity mapping:** if the selected OVMF revision doesn't provide a usable way to authenticate/stabilize old-linear→physical mappings through the transition, or uses unsupported CPU mode, **reject LAB-CR3-1 in that VM profile**. A raw UEFI pointer/physical number is not, by itself, a universal old-current-page-table translation proof. The future implementation must expose an auditable preflight check (and a negative test) for this. No general physical memory mapper is added to L1 merely to paper over the issue.

**Why a dedicated stack:** immediately after ExitBS, the loader's EFI stack need not be mapped in the new CR3 and does not satisfy the freestanding kernel ABI. The bridge switches to its own precharged **old/new-mapped stack before CR3 write**, then to the mapped high-half main kernel stack after switching. This prevents hidden reliance on old firmware stack pages.

### 3.2 Dual-mapping and exception guarantee

| Phase | Active CR3 | Instruction fetch must be mapped | Stack and critical descriptors |
|---|---|---|---|
| before ExitBS | firmware old CR3 | EFI PE loader \`.text\` | EFI stack, boot-service call state |
| after ExitBS / before bridge | old CR3 | resident EFI image transition stub | EFI stack only for the shortest return from ExitBS; no later BootService calls |
| bridge preparation | old CR3 | same transition stub code | **bridge stack** in old CR3; mapped resident GDT/IDT/TSS/IST, load descriptors before new CR3 |
| protected CR3 write | new CR3 | **same bridge linear VA** and same physical instruction backing | bridge stack at **same linear VA** in new CR3; emergency handler descriptors/buffers also mapped |
| high-half entry | new CR3 | fixed linked kernel entry RX at \`e_entry\` | new main stack, private BootContext, emergency IST still valid |
| success/fail stop | new CR3 | kernel diagnostic/fault code resident | bounded COM1 and halt; no EFI return |

**First-lab fail-stop semantics:** fault handlers are best effort only in the instant after ExitBS and before valid descriptor setup. IF=0 masks ordinary interrupts but **does not prevent synchronous faults, NMI or machine checks**. The design must not promise a serial error record from an unmapped IDT/TSS; host reset/triple-fault/timeout counts as FAIL. To reduce the interval, pre-stage and load a small resident **emergency GDT/IDT/TSS/IST while still on the old CR3** immediately after ExitBS and prior to its switch; verify that the same descriptor targets are mapped in new CR3.

**Guard and alias:** new CR3 maps **only** the required small low bridge/exception ranges, not an identity map of all RAM. Bridge frames and high-half kernel text frames are different allocations. No low writable alias of kernel RX physical pages and no present alias around guard pages may remain in new CR3. Old CR3 aliases cease to be CPU-visible on current BSP after new CR3 activation; no SMP/APs in this profile.

### 3.3 Lab paging/CPU preconditions

- BSP in 64-bit mode; **4-level IA-32e**, CR4.LA57=0 (fail profile if 5-level mode), CR4.PAE=1; CR4.PCIDE=0 for the first experiment or a documented safe no-PCID CR3 transition; no unreviewed PCID reuse/global mapping.
- CPUID NX supported; **EFER.NXE=1** before entering new PTEs with NX; **CR0.WP=1** for supervisor W^X enforcement. New image mappings honor NX and are supervisor-only; no NX write alias of RX text.
- CR3 physical root alignment/ownership valid, reserved bits appropriate for selected physical address width; no page-table walk accesses user/unowned frames. Page size for early protected regions **4 KiB**, later large-page optimization deferred.
- CPU address canonicalness, cache/memory-order implications of page-table update and CR3 write are **explicit future refinement tests**; one CPU significantly narrows but does not prove correctness.
- IF=0 for L1 through halt. No APIC or periodic timer setup. This is **not** proof that unexpected NMIs/triple faults are always reportable.

**Important portability limit:** this describes one **restricted QEMU CPU/OVMF mode**. If firmware enters with LA57 or PCIDE state not supported by the first contract, stop rather than attempt an unreviewed conversion inside a live firmware environment.

## 4. Literal initial x86_64 entry-state contract — LAB-ENTRY1

### 4.1 Before CR3 and before first Rust invocation

| CPU facet | Required experimental state / reviewer check |
|---|---|
| privilege/mode | CPL0, 64-bit long mode, supported x86_64 paging feature set |
| interrupts | \`RFLAGS.IF=0\`; unexpected external vector isn't treated as registered device |
| direction | **DF=0** before any Rust/String instruction |
| flags/stack | canonical aligned bridge RSP before CR3; canonical high-half RSP before \`call\` |
| page protections | CR0.WP=1, NXE enabled, new CR3 maps RX/RW+NX/R+NX correctly |
| segment selectors | correct 64-bit code GDT descriptor (L=1, D=0), legitimate data/stack selectors as required; explicit loaded CS (far control transfer if necessary) |
| TSS | available **64-bit TSS** GDT descriptor (two entries), \`LTR\` with resident TSS; **IST1** points to dedicated resident #DF stack |
| IDT | resident fault stub gates for **#DE, #DB, #NMI, #BP, #UD, #DF, #TS, #NP, #SS, #GP, #PF, #MF, #MC**; unexpected other vectors default fail-stop if safely recognized |
| fault arguments | normalize vector/error-code/no-error-code frame to a bounded diagnostic record, no unrestricted stack reads or unwinding |
| TLS / FS,GS | no TLS user/Task fields; per-CPU base is absent in L1; Rust must not depend on EFI FS/GS hidden state |
| FP/SIMD | Rust bare-metal default avoids floating/vector registers; do not enable extra target features without explicit XSAVE/XCR0 state contract |
| ABI bridge | **EFI ABI calls end in stub**; final kernel \`extern "C"\` is **SysV x86_64, no red-zone** |
| return behavior | entering kernel never returns to UEFI/firmware and no Rust panic unwinds through entry stub |

**SysV call alignment rule:** configure new main kernel stack so **RSP is 16-byte aligned at the call instruction** into Rust. The call pushes an 8-byte return address; the Rust function observes ordinary SysV entry alignment (RSP mod 16 = 8). Pass private BootContext pointer in the appropriate first-argument register (RDI for SysV) and ensure the return target is a resident *non-returning fail-stop* rather than former EFI application code.

**Exception caveat:** #DF uses dedicated IST1. For ordinary #PF/#GP a broken current stack could escalate to #DF; the handlers must not recursively call firmware, heap allocation, serializer/formatting facilities or a serial lock that was interrupted. An early broken GDT/IDT/IST may triple-fault without recording anything; outer QEMU oracle detects failure.

**ABI/CPU details remain private experimental assumptions**, not a public Native syscall ABI. The future compiled assembly/ELF disassembly must show actual selector widths, IST descriptor types, error-code normalization, instructions and register preservation. This requirement is *post-Go engineering validation*, not a claim of proof today.

### 4.2 User-level observation and HALTED_PASS

The kernel may report staged serial markers (e.g. boot image loaded, map accepted, ExitBS completed, CR3 changed, Rust entry reached, exception descriptors valid). **HALTED_PASS is issued only after the final positive test oracle** and is never emitted in any exception handler. Diagnostic output is *bounded* and COM1 may be absent; in that case the VM can halt with host-detected timeout/failure. A serial "PASS" printed then VM resets is **not PASS**; the future host script (only after approval) must correlate the expected end state and no unexpected reset.

## 5. ExitBootServices microcontract — LAB-EXIT1

**Before last GetMemoryMap:** allocate image frames, bridge stack, main/IST stacks, GDT/IDT/TSS, BootContext, memory map buffer, page tables, exception/serial stubs and all expected needed resident storage. Precompute ELF validation and initial maps; no late dynamic initialization that changes memory map.

**On last GetMemoryMap:** store actual descriptor stride/version and current MapKey. The lab map buffer capacity is **64 KiB**; if inadequate, reject the attempt or (before the first ExitBS call only) allocate a larger new buffer and repeat the full map preparation. Never truncate descriptors.

**Call ExitBootServices once with current key.** If EFI_INVALID_PARAMETER (stale key) and firmware is partially shut down, the **strict conservative retry subset** is \`GetMemoryMap\` followed by \`ExitBootServices\` again using already allocated memory. Limit to **two ExitBS attempts total** for the lab protocol, then fail-stop/host failure. Do not use EFI console, file protocol, events, timer, AllocatePages or FreePages after the *first* ExitBS attempt under this narrower design—even if some UEFI 2.11 Memory Allocation Services are permitted by specification. The no-extra-services policy is a conservative subset, not a false statement that UEFI prohibits every allocation call.

After success, **no Boot Services or handle-based device protocols**, no return to UEFI. Never call EFI watchdog service after successful ExitBS; the host's timeout/reset oracle is independent. The selected DevUnsigned boot environment provides **no production chain of trust**.

## 6. B01–B10 implementation entry evidence mapping and non-goals

| Readiness ID | Evidence recorded in this paper | Is proof needed before experimental code? |
|---|---|---|
| B01 profile | Q35+OVMF, one vCPU, 4-level paging, NX, COM1, read-only medium | explicit profile ready for decision; first runtime must verify CPUID/map/firmware |
| B02 firmware handoff | LAB-ELF1 + LAB-EXIT1 and phase ownership | full semantic design sufficient for a disposable prototype; actual ExitBS behavior untested |
| B03 memory | LAB-MEM1 + LAB-CR3-1, no direct map, W^X, guarded stacks | no executable proof before code; dynamic old mapping audit **mandatory in implementation before CR3** |
| B04 exception | LAB-ENTRY1, emergency IDT/TSS/IST, IF=0, host failure oracle | produced descriptors / CPU tests are post-Go |
| B05 scope | strictly kernel-only, zero Tasks/IPC/Pager/DMA; no physical PC | no general native OS behavior promised |
| B06 diagnostic | bounded COM1, explicit failure classes, cannot always print on triple fault | run actual COM1 negative tests after Go |
| B07 build/provenance | Rust UEFI PE32+ + kernel ELF, restricted linker contract + no relocation, pinned per-run manifest and dependency/license audit | choose exact dependency versions before first build, no ABI freeze needed |
| B08 verification plan | predecessor L1-X01–X18 + this paper's LC-X01–X22, isolated QEMU failure/reset/timeout oracles | actual tests **cannot** predate implementation |
| B09 repository/security | user approval, repo docs history retained, dedicated experimental branch recommended, read-only VM media, no secrets | explicit Go required before source modification |
| B10 Go card | [separate scoped decision review](../reviews/unios-l1-implementation-go-no-go-assessment-v0.1.md) fixes deliverables/exclusions, and user can approve/decline | user Go is **not granted** by this paper |

**Implementation entry means permission to begin writing a constrained prototype and validation harness, not endorsement of the finished design or kernel security.** Changes found during actual image build must remain within lab scope; if a core cut/assumption fails, stop and re-review before broadening capabilities.

## 7. Paper-only adversarial design tests LC-X01…LC-X22 — NOT EXECUTED

| ID | Scenario | Post-Go test or rejection oracle |
|---|---|---|
| LC-X01 | embedded ELF wrong class/endianness/machine/type | reject before final map |
| LC-X02 | \`p_filesz > p_memsz\`, overflowed \`p_offset+p_filesz\` | reject no write |
| LC-X03 | \`p_align\` invalid or \`p_vaddr/p_offset\` incongruent | reject |
| LC-X04 | PT_LOAD segment claims more than 64MiB VA /16MiB loaded cap | reject |
| LC-X05 | W+X segment or conflicting-page overlaps | reject no writable executable physical alias |
| LC-X06 | \`e_entry\` outside real executable mapped page | reject |
| LC-X07 | linker emits PT_DYNAMIC/required relocations unexpectedly | reject artifact, adjust *plan* not auto enable loader |
| LC-X08 | old EFI transition stub linear VA maps to unknown backing | fail pre-ExitBS, never assume identity |
| LC-X09 | new CR3 lacks bridge's exact old linear RX page | host failure, never PASS |
| LC-X10 | bridge stack not mapped same linear VA old/new | negative CR3 failure (must not be PASS) |
| LC-X11 | low writable alias reaches high-half kernel RX physical page | fail W^X alias audit |
| LC-X12 | stack guard accidentally mapped by bridge/direct alias | fail protection test |
| LC-X13 | wrong LA57/PCIDE/CR0.WP/NXE CPU mode | reject incompatible VM profile |
| LC-X14 | emergency GDT/IDT/TSS/IST unmapped after CR3 | reset/FAIL, no false serial guarantee |
| LC-X15 | kernel Rust entry gets EFI ABI registers/stack convention | reject produced entry disassembly/negative VM |
| LC-X16 | ExitBS fails first attempt and map key changes again | at most two sanctioned map/exit attempts then fail-stop |
| LC-X17 | last GetMemoryMap buffer insufficient or malformed descriptor stride | reject, never overrun |
| LC-X18 | attempted EFI console/file/Allocate after first ExitBS attempt | reject architecture/negative test |
| LC-X19 | #PF/#GP/#UD/#DF on ordinary/critical stacks | distinct expected fault/host oracle, no panic recursion |
| LC-X20 | COM1 absent, serial read busy forever, missing marker | finite fallback and host FAIL |
| LC-X21 | prints HALTED_PASS, then unexpected VM reset | host must reject completed outcome |
| LC-X22 | unsupported arbitrary media, 2+ vCPUs or real hardware | refuse/mark OUT OF SCOPE, no security or compatibility claim |

## 8. Unproved invariants LC-I01…LC-I20

| ID | Candidate invariant (neither model-checked nor tested) |
|---|---|
| LC-I01 | Embedded ELF subset is bounded and checked without source/target physical overlap or W+X. |
| LC-I02 | Fixed link VAs and emitted code model are compatible with mapped physical pages and entry. |
| LC-I03 | The EFI loader never treats a kernel ELF VA as automatically mapped under EFI. |
| LC-I04 | Old bridge linear ranges have verified physical backing and matching new CR3 translations. |
| LC-I05 | A temporary bridge stack, GDT/IDT/TSS/IST are valid under both old and new roots when used. |
| LC-I06 | New CR3 has no writable alias to kernel RX frames or present alias of guard. |
| LC-I07 | All allocated segment/stacks/map context/paging frames retained after ExitBS. |
| LC-I08 | Firmware map handled by returned DescriptorSize/Version and latest MapKey. |
| LC-I09 | After first ExitBS attempt no unsupported Boot Services; after successful exit none. |
| LC-I10 | CPU long-mode LA57/PCIDE/NXE/WP and 4 KiB page assumptions are checked. |
| LC-I11 | The CPU never fetches unmapped bridge/high-half instructions on a correct transition. |
| LC-I12 | SysV first Rust invocation has correct RSP alignment, RDI BootContext, DF=0 and IF=0. |
| LC-I13 | The kernel does not depend on EFI calling convention or standard library/allocator. |
| LC-I14 | Emergency #DF IST and diagnostic fault stubs are resident and nonrecursive if available. |
| LC-I15 | Unmapped IDT/triple-fault/COM1 failure never accidentally counts as PASS. |
| LC-I16 | BSP-only L1 makes no scheduler, Pager, Task/IPC, DMA or physical PC promise. |
| LC-I17 | BSS/BootContext and page permissions are initialized without revealing arbitrary old firmware memory. |
| LC-I18 | Unsupported ELF/CPU/OVMF profiles fail closed before new CR3 as far as possible. |
| LC-I19 | All test-lab numeric caps are private and are not an accepted KABI/GUI/compatibility claim. |
| LC-I20 | Only an explicit user Go authorizes starting even experimental loader/kernel source implementation. |

## 9. Implementation-time uncertainties, bounded and tracked — LC-O01…LC-O08

These are **validation obligations during/after an authorized L1 prototype**, not missing user-facing product specifications silently declared solved.

| ID | When evaluated | Required evidence / stop condition |
|---|---|---|
| **LC-O01** | first linker artifact | actual Rust target/code model emits supported fixed ELF64 ET_EXEC, expected PHDRs and no unsupported relocations; otherwise STOP and revise contract |
| **LC-O02** | before ExitBS runtime preflight | actual selected OVMF mapping provenance supplies trustworthy bridge old-linear→physical audit; otherwise STOP and assess alternate entry strategy |
| **LC-O03** | initial page-table construction | 512KiB page-table lab reserve and 64MiB kernel VA window enough; if exhausted fail before exit and re-review |
| **LC-O04** | exception bring-up | real IDT/TSS/IST selectors/stub error-code paths protect intended guard faults without recursive failure; else STOP |
| **LC-O05** | QEMU/OVMF config selection | exact toolchain, OVMF provenance hash and one-vCPU CPU model/CPUID match assumptions, pinned in reproducible experimental manifest |
| **LC-O06** | host test execution | serial and host reset/triple-fault/timeout oracles distinguish PASS and negative tests; no actual result yet |
| **LC-O07** | after first Run | mapping alias audit, EBS retry and no-UEFI-after-exit negative controls exercised; no security assurance until evidence |
| **LC-O08** | before any L2 scope enlargement | independent user decision for Task/CapGrant/IPC/Pager/hardware; L1 approval never auto-expands |

## 10. What is **not** claimed, and implementation-entry boundary

The design packet now names a finite **experimental x86_64 UEFI handoff contract** and a complete stop-on-violation test plan. **It has not produced a successful boot, verified W^X, protected #DF implementation or safe page-table switch.** No tool/runtime model was executed.

**Conditional L1-entry recommendation:** [Go/No-Go Assessment](../reviews/unios-l1-implementation-go-no-go-assessment-v0.1.md) may now classify **"L1 design ready for explicit user approval"** if it accepts the finite experimental constraints and validation-first workflow. That does **not** require demonstrably correct kernel code before code exists. L2 and L3 remain in research/NO-GO.

### 10.1 Subsequent work if Go is *not* given

Continue **design only** with a separate L2 tiny register-control IPC boundary or L1 hardware-variant threat review. Do not accidentally start the UEFI loader because this file contains a sufficiently exact VA/CPU plan.

## 11. First-party sources and evidence qualifications

- [UEFI 2.11 Boot Services: GetMemoryMap/ExitBootServices](https://uefi.org/specs/UEFI/2.11/07_Services_Boot_Services.html) — map key, descriptor stride, allocation rights, partially completed ExitBS and no Boot Services after success.
- [Rust bare-metal x86_64-unknown-none](https://doc.rust-lang.org/rustc/platform-support/x86_64-unknown-none.html) — Tier 2 freestanding ELF, kernel code model, default no floating/vector use, no red zone and System V \`extern "C"\`.
- [Rust x86_64-unknown-uefi target](https://doc.rust-lang.org/rustc/platform-support/unknown-uefi.html) — PE32+, UEFI-specific calling convention, EFI loader target distinct from kernel.
- [Intel 64/IA-32 Architecture Developer Manuals](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html) — CR3, paging, long-mode TSS/IST, IDT, CPU control register rules. The exact expected fault entry is **still future implementation-verification work**.
- [UniOS architectural direction v0.2](../architecture/unios-architecture-v0.2.md) and preceding two boot contracts are Proposed technical documentation, not independent runtime evidence.

**Method:** research-first, narrow lab parameter choices, cross-document semantic consistency review and paper counterexamples. These descriptions do not certify the executable map/CPU handoff until future approved implementation/refinement checks.

---

**History:** 2026-10-26 v0.1. Resolves earlier H-O01/02/03 to a *reviewable experimental LAB-H1* with fixed high-half VA, bounded ELF, explicit dual-linear trampoline, 4-level NX/WP paging, private SysV CPU entry and limited EFI map/exit retry. 22 paper cases, 20 unproved invariants, 8 implementation-validation blockers; **no OS code, checker or VM runtime run**.

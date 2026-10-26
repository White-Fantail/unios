# UniOS — L1 Experimental Kernel Implementation Go/No-Go Assessment v0.1

**Post-assessment decision (2026-10-26):** The user subsequently granted the explicit **scoped L1 Go** requested below. See the [L1 User-Go Record](unios-l1-scoped-user-go-2026-10-26.md). This dated assessment retains its *pre-approval* verdict as historical evidence. **No kernel code, QEMU run or runtime validation has been produced, and current project work remains design/research-only.**


> **Evaluation date:** 2026-10-26 NZDT.
> **Verdict:** **L1 DESIGN READY FOR EXPLICIT USER GO — NOT AUTHORIZED TO IMPLEMENT.**
> **Scope only:** disposable **QEMU Q35/OVMF one-vCPU UEFI boot + fixed-map, privileged freestanding kernel + COM1 diagnostics and guarded exception proof-of-concept**. It is **not** a conclusion that UniOS's independent capability microkernel, native IPC, driver platform, Windows/Linux/Darwin app compatibility or end-user product is ready.
> **Status of evidence:** written design and negative-test requirements only; **zero compiled images, zero QEMU boot attempts, zero executed model/checker runs, no measured performance, no verified ISA handler or secure production boot**.
> **No implementation authorization:** The design/research-only project instruction remains binding. The user's prior "keep going" and "tell me when implementation-ready" **do not grant an implementation Go**. Only a later explicit scoped approval may authorize editing code/build/test artifacts.
> **Controlling technical contract:** [L1 Fixed-Link ELF, CR3 Bridge & Entry-State Closure v0.1](../contracts/unios-l1-fixed-link-elf-cr3-bridge-entry-state-closure-v0.1.md), [L1 UEFI Handoff Microcontract](../contracts/unios-l1-uefi-handoff-kernel-entry-microcontract-v0.1.md), [Minimal Boot, Exception & Memory MVP](../contracts/unios-minimal-boot-exception-memory-mvp-contract-v0.1.md), [Implementation Entry Gate v0.1](unios-implementation-entry-readiness-gate-v0.1.md), [Architecture v0.2](../architecture/unios-architecture-v0.2.md).

## 0. Decision: what "ready" actually means

**A limited experimental implementation can now be responsibly proposed for authorization, with an explicit stop-on-failed-assumption workflow.** "Ready" here means the **requirements, platform, initial machine state, memory ownership, output oracle, allowed failure behavior and tests to attempt** are sufficiently bounded to start writing a diagnostic prototype *after user approval*. It does **not** imply that these requirements have already been implemented or their physical feasibility proven.

**Design-only adjudication:** H-O01 (linker/image constraints), H-O02 (CR3/bridge mappings), H-O03 (CPU/IDT/TSS/IST/ABI) have *reviewable LAB-H1 choices* in the fixed-link microcontract. The remaining unknowns are not waved away: LC-O01…08 describe expected **first-code verification points**, with hard stop/review when assumptions fail. Engineering discovered during implementation can invalidate LAB-H1 and require another design review.

**Why this threshold is technically defensible:** L1 excludes untrusted Tasks, pageable receive/usercopy, general PMM, native CapGrant/K-S/K-I, DMA and any real-user-data storage. Thus unresolved higher-level CAPIPC issuer/VM publication races do not enter this one-CPU restricted boot test. A runtime boot result is logically impossible before some authorized prototype source exists; making "successful QEMU boot" an implementation entry prerequisite would be circular.

**Explicit exception:** If the user expects the *entire independent secure OS* rather than this limited lab probe to be ready, the verdict is **NO-GO**. The READY label applies solely to L1.

## 1. B01–B10 re-adjudication

| Gate | Design evidence | Gate for **requesting L1 user approval** | Runtime proof not yet done |
|---|---|---|---|
| **B01** controlled platform | QEMU Q35/OVMF, x86_64 one-vCPU, LA57 off/PCIDE off, NX, COM1 and read-only media; LAB-H1 | **READY FOR REVIEW** | selected QEMU/OVMF binary versions and CPUID/old-map facts |
| **B02** ownership/ExitBS | bounded ELF staging, map stride+current MapKey, no forbidden firmware after first attempt, at most 2 lab ExitBS calls, post-exit fail-stop | **READY FOR REVIEW** | real OVMF firmware callbacks/map retry |
| **B03** early mapping/stack | 4-level 4KiB mappings, high-half fixed kernel base, dual-linear bridge, NX/WP, no RX writable aliases, separate main and DF IST stacks | **READY FOR REVIEW** | produced CR3/bridge map audit and guard faults |
| **B04** CPU/exception | IF=0, no SMP/scheduler, SysV kernel entry, DF clear, bounded resident IDT/GDT/TSS/IST + host failure oracle | **READY FOR REVIEW** | compiled selectors/stubs, x86 exception tests |
| **B05** object universe/exclusions | strictly no Task/CapGrant/IPC/Pager/DeviceCaps/DMA/PMM | **READY FOR REVIEW** | implementation audit guards scope |
| **B06** serial/debug | COM1 polled bounded record, expected final marker, no UEFI console after exit, host reset/timeout failure rule | **READY FOR REVIEW** | actual serial/QEMU observation |
| **B07** build/provenance plan | UEFI PE32+ vs ELF ET_EXEC, Rust targets, no dynamic relocation, embedded bounded program-header loader; exact tools/hashes pinned before first build | **READY FOR REVIEW** | linker-produced ELF PHDR and toolchain compatibility |
| **B08** negative plan | L1-X01…18, H-X01…20 and LC-X01…22 paper case suites, PASS/FAIL/expected-fault/reset classification | **READY FOR REVIEW** | **NONE EXECUTED**; only after explicit user Go |
| **B09** safety/governance | research docs in main, proposed future isolated experimental source branch, disposable VM/read-only media, no secrets/physical disks, source/firmware license audit | **READY FOR REVIEW** | branch and CI created only after approval |
| **B10** user-scoped Go card | this review gives scope, deliverables, stop gates, exclusions and a separate explicit approval action | **READY TO ASK / GO NOT GIVEN** | **user has not approved implementation** |

**Interpretation of B10:** Asking for permission requires a complete *Go decision card*, not already having received a Go. The state should be **READY FOR USER GO**, followed by **USER GO GIVEN** only after explicit user action. Earlier gate phrasing that seemed to require approval *before being ready to ask for approval* was ambiguous; this review disambiguates it without modifying the eight accepted architecture directions.

## 2. Exact proposed first implementation permit (if explicitly approved later)

**Permission would cover only:**

1. An **isolated experimental code branch** rooted at the then-current vetted repo state; no automatic production merges or stable API announcement.
2. A Rust UEFI PE32+ loader with a fixed **embedded limited ELF64 ET_EXEC** kernel payload built for the separate \`x86_64-unknown-none\` target; document firmware/compiler/linker dependencies and all third-party licenses.
3. Bounded ELF program-header loader, checked memory-map/BootContext and ExitBootServices retry; no EFI service calls after success.
4. One BSP-only protected page table/CR3 transition using a mapped bridge stub, aligned dedicated stack, early IDT/GDT/TSS/IST, kernel W^X/guard probes and serial fail-stop.
5. **QEMU/OVMF-only** validation harness (serial capture, expected good result and negative case resets/timeouts), checked in the same isolated branch. No writes to physical disks.
6. Produce a first-test evidence report: toolchain versions, firmware provenance/hash, ELF/PE headers, CPU feature state, memory map ownership, CR3/bridge guard, valid boot/negative cases, actual failures and tradeoffs.

**It would NOT authorize:** real physical hardware, native Task/Capability/IPC/Pager/driver/DMA, filesystem write, GUI/SDK, dynamic linking, Windows/Linux/Darwin emulation, hardened security/product launch, generalized Rust unsafe patterns, or expanding source changes/CI/branch rules beyond the scoped permit.

**Implementation stop conditions (require return to design review):**

- Initial compiler/linker cannot provide the specified ET_EXEC/high-half no-reloc kernel or the Rust target changes incompatible ABI/code model.
- Chosen OVMF/VM does not permit an auditable old-linear→physical bridge mapping stable into the new CR3; do **not** assume identity mapping or weaken the bridge guarantee.
- Prepared 512KiB page-table cap, 64MiB VA region, 16MiB loaded kernel/64KiB map buffer or finite stacks prove infeasible and need material re-scoping.
- A proposed exception/IST bridge requires unreviewed firmware calls, user-mode execution, an unbounded page allocator, or a forbidden writable alias.
- Negative tests expose a false PASS, nonterminating diagnostics or unsafe media writes; halt experiment and document.
- A request appears to extend the lab image into real hardware, capability IPC, general Pager, devices or public ABI.

## 3. Explicit first go/no-go conditions

| Category | Can be decided without executable code? | Present result |
|---|---|---|
| Product-level scope and exclusions | yes | **PASS (paper)** |
| Clear image/VM/CPU candidate contract | yes | **PASS (paper)** |
| Defined no-firmware-after-ExitBS and stop behavior | yes | **PASS (paper)** |
| Host/guest diagnostic failure oracle and negative test plan | yes | **PASS (paper)** |
| Protected source/target branch, secrets, media policy | yes as policy | **PASS (paper)** |
| Real ELFs/old CR3/exception handler behavior | no, needs an approved experiment | **NOT TESTED, POST-GO** |
| Native capability/IPC/VM/Pager assurance | no, outside L1 | **NOT READY / OUT OF SCOPE** |
| User's explicit permission to write code | yes, separate action | **NOT GIVEN** |

**Conclusion:** **L1 MINIMAL KERNEL DESIGN READY — IMPLEMENTATION CAN BE AUTHORIZED BY THE USER.** **No implementation is authorized yet.** L2 native microkernel and L3 general desktop OS stay in Proposed/NO-GO research status.

## 4. What the user should be told

> **UniOS 최소 L1 커널 구현 착수 설계 준비 완료.** 승인할 수 있는 범위는 **QEMU/OVMF 단일 CPU 부팅·보호된 CR3/메모리·예외·시리얼 진단 커널**뿐입니다. Capability/IPC, Pager, GUI, Linux/Windows/macOS 호환성과 실물 PC는 제외됩니다. 코드/테스트는 아직 하나도 만들지 않았고 실제 부팅도 검증하지 않았습니다. **명시적으로 구현을 승인하면** 그 제한된 실험에만 착수할 수 있습니다.

The user may simply approve or decline. No approval is inferred from earlier "continue" turns.

## 5. Sources and validity

- [L1 Fixed-Link ELF, CR3 Bridge & Entry State](../contracts/unios-l1-fixed-link-elf-cr3-bridge-entry-state-closure-v0.1.md) is the immediate reviewed lab technical design.
- [Prior boot/UEFI microcontract](../contracts/unios-l1-uefi-handoff-kernel-entry-microcontract-v0.1.md) records why alternative handoffs were rejected or deferred.
- [L1 preliminary MVP](../contracts/unios-minimal-boot-exception-memory-mvp-contract-v0.1.md) records expected future negative tests.
- [Architecture v0.2](../architecture/unios-architecture-v0.2.md) governs implementation R6 Go/No-Go and independent kernel direction.
- [UEFI 2.11 ExitBootServices](https://uefi.org/specs/UEFI/2.11/07_Services_Boot_Services.html), [Rust x86_64-unknown-none](https://doc.rust-lang.org/rustc/platform-support/x86_64-unknown-none.html), [Rust unknown-uefi](https://doc.rust-lang.org/rustc/platform-support/unknown-uefi.html), and [Intel SDM](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html) are external platform descriptions; they do not constitute actual UniOS software validation.

**No tests/proofs/run performed.** This is a judgment that the **experiment is sufficiently specified to begin engineering after approval**, not that the design is proven safe or the operating system is implementation-complete.

---

**History:** 2026-10-26 first scoped L1 design Go/No-Go review. B01–B09 paper-level REVIEWABLE, B10 ready to request permission / not granted. **L1 READY FOR USER GO, L2/L3 NOT READY**. No branch/code/test/ABI created.

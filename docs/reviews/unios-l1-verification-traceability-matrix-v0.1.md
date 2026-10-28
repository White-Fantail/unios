# UniOS — L1 Verification Case Traceability and Host-Oracle Matrix v0.1

> **Prepared:** 2026-10-29 NZDT. **Status: PAPER MATRIX — 0/60 CASES EXECUTED.**
> **Scope decision:** [L1 scoped user-Go](unios-l1-scoped-user-go-2026-10-26.md). **Execution runbook:** [L1 execution readiness v0.1](../contracts/unios-l1-execution-readiness-runbook-v0.1.md).
> **Coverage:** 18 \`L1-X\` first-slice cases + 20 \`H-X\` handoff cases + 22 \`LC-X\` fixed-link/CR3 cases = **60 uniquely indexed paper cases**. Overlap is intentional; count case IDs, not independent vulnerabilities. **One conditional DevSigned-only test (L1-X14) is deferred/out-of-profile for DevUnsigned L1 and is NOT a completed test.**
> **No compiled kernel, VM run, actual result, measured time, crash dump, formal proof or security guarantee exists in this matrix.**

## 1. Evidence semantics and test-level rules

- **Canonical source order:** [minimal MVP](../contracts/unios-minimal-boot-exception-memory-mvp-contract-v0.1.md) for \`L1-X\`, [handoff microcontract](../contracts/unios-l1-uefi-handoff-kernel-entry-microcontract-v0.1.md) for \`H-X\`, [fixed-link closure](../contracts/unios-l1-fixed-link-elf-cr3-bridge-entry-state-closure-v0.1.md) for \`LC-X\`. Preserve their IDs to avoid silently erasing untested promises.
- **Every row status = \`NOT_RUN\` as of 2026-10-29.** Future evidence must be case-ID-addressable and carry A00–A06 artifacts, expected result and actual host/guest trace. The paper acceptance predicate in a row is **not** evidence it was met.
- **Method** is a proposed way to verify, not a generated tool or script. **ELF parser** means controlled negative fixtures in a disposable environment; **VM injection** means intentional emulator perturbation; **HOST injection** means exercising the host result classifier. Do not manipulate real disks or claim fabricated firmware-map corruption equals a genuine OVMF integration result.
- Where safe source/artifact review is stronger than executing an illegal post-ExitBS firmware call, prefer a **static prohibited-call audit plus controlled simulation**. Real invalid firmware operations are never required to claim a safe negative-case observation.
- For a future positive test only, \`CLEAN_PASS\` requires declared stage ordering, exactly one \`HALTED_PASS\` and independent host evidence of the expected final non-return/no reset. A negative test succeeds only when its **expected rejection or fault** is confirmed, but its kernel is *not* a clean-boot PASS.
- Host reset/triple-fault/timeout, missing serial or a post-PASS crash are always **non-clean** results. Absence of evidence is \`UNRUN\` or \`INVALID_RUN\`, never implicit PASS.
- Test coverage is not proof of universal security. L2 capability, pageable receive, hardware DMA, real PC and app compatibility are outside this matrix.

## 2. E0–E5 reference gates, artifacts and test ownership

| Gate | Primary reviewable artifacts | Test families / cuts |
|---|---|---|
| **E0** lab profile and provenance | A00 scoped decision; A01 pinned dependencies, firmware/VM/CPU and read-only media | CPU/vCPU, DevUnsigned, watchdog and unsupported hardware |
| **E1** PE/ELF and bounded ownership | A02 PE/ELF/linker audit; A03 frame ledger | malformed payloads, permissions, relocation/caps |
| **E2** GetMemoryMap / ExitBootServices | A03 descriptor MapKey/stride and exit trace; A05 serial/host | stale key, retry, buffer overflow, post-exit illegal calls |
| **E3** CR3, ABI, W^X and stack safety | A04 PTE audit, entry disassembly and descriptors | bridge, old/new RIP/RSP, guards, CPU mode |
| **E4** exceptions, COM1 and fail-stop | A04 exception frame; A05 complete trace | #PF/#GP/#UD/#BP/#DF, panic, serial absent, false PASS |
| **E5** independent outcome adjudication | A06 full indexed matrix with reviewer and decision | exact expected/observed outcomes and no scope creep |

## 3. Original L1-X01…L1-X18 case coverage

| Case | Gate | Method (future) | Observable acceptance/rejection criterion |
|---|---|---|---|
| `L1-X01` | E2–E4 | VM+HOST | Valid image and map: ordered handoff, post-ExitBS protected entry and host-confirmed CLEAN_PASS |
| `L1-X02` | E2 | VM injection | Stale MapKey: at most one constrained retry; no illegal post-first-call services |
| `L1-X03` | E2 | Parser+VM | Too-small map buffer before first ExitBS: resize before cut, refresh key, no truncation |
| `L1-X04` | E2 | Parser | Invalid descriptor stride/version/size: reject before unsafe map iteration |
| `L1-X05` | E1–E3 | Map audit | Overlap with stacks/kernel/UEFI reserved: reject without unowned write |
| `L1-X06` | E1 | ELF parser | Zero/overflow segment, entry or relocation: reject before transfer |
| `L1-X07` | E0–E3 | CPUID VM | Absent required NX/other features: reject profile before protected entry |
| `L1-X08` | E4 | VM fault | Unmapped guard #PF: expected code/location and non-PASS termination |
| `L1-X09` | E3–E4 | VM fault | Write to RX text: protection #PF, no write accepted, no HALTED_PASS |
| `L1-X10` | E4 | VM fault | #UD/#GP/#BP: correct vector/error-code convention and bounded terminal result |
| `L1-X11` | E4 | VM fault | Main-stack failure/#DF: IST or host-classified failure; no false success |
| `L1-X12` | E4 | VM+HOST | COM1 absent or stalled: bounded polling and host-observable non-PASS |
| `L1-X13` | E2 | Static audit | Post-success ExitBS Boot Services use forbidden: reject image/source path |
| `L1-X14` | E0–E1 | Design-only deferred | DevSigned signature/hash failure test **NOT APPLICABLE** to DevUnsigned L1; cannot count as PASS |
| `L1-X15` | E0 | VM manifest | More than one vCPU: reject unsupported run, never claim SMP |
| `L1-X16` | E4–E5 | HOST | Reset or triple fault on positive path: host FAIL even if serial text looked complete |
| `L1-X17` | E4 | Panic injection | Malformed panic/log payload: finite no-heap/no-unwind/no raw RAM disclosure |
| `L1-X18` | E3 | Map audit+fault | Writable alias of text or mapped guard through identity mapping: fail protection audit |

## 4. Handoff H-X01…H-X20 case coverage

| Case | Gate | Method (future) | Observable acceptance/rejection criterion |
|---|---|---|---|
| `H-X01` | E2–E4 | VM+HOST | Valid embedded kernel and maps: ownership/context correct, ExitBS successful, kernel success observed |
| `H-X02` | E1 | ELF parser | filesz>memsz rejected before copying |
| `H-X03` | E1 | ELF parser | ELF/PHDR offset arithmetic overflow rejected |
| `H-X04` | E1–E3 | ELF+map audit | Conflicting PT_LOAD page rights rejected; no RW alias of RX |
| `H-X05` | E1 | ELF parser | Entry outside executable PT_LOAD rejected |
| `H-X06` | E1 | Artifact audit | Wrong code-model/high-half VA rejected before VM success claim |
| `H-X07` | E2 | VM injection | ExitBS stale key path obeys bounded GetMemoryMap retry |
| `H-X08` | E2 | Static+VM control | Partial Boot Services shutdown does not lead to EFI file/console misuse |
| `H-X09` | E2 | VM injection | Buffer insufficient during restricted retry: fail-stop; no late allocate |
| `H-X10` | E3 | Stack audit | Old firmware stack never relied upon after CR3 switch |
| `H-X11` | E3–E4 | VM injection | Unmapped next bridge instruction: host FAIL/reset; never HALTED_PASS |
| `H-X12` | E3–E4 | Descriptor audit+VM | Unmapped GDT/IDT/TSS on new CR3: reject setup or host failure |
| `H-X13` | E3 | Alias audit+VM | Low writable alias to high-half RX detected and rejected |
| `H-X14` | E1–E3 | Pointer audit | Noncanonical RSP, e_entry or BootContext pointer rejected before use |
| `H-X15` | E1–E3 | Ownership audit | BSS/BootContext cannot point into unreserved Boot Services memory |
| `H-X16` | E4 | VM fault | Expected #PF/#GP/#UD/#DF classified without positive PASS |
| `H-X17` | E4 | VM+HOST | Absent COM1/stuck transmit has finite budget and non-PASS host result |
| `H-X18` | E0–E4 | HOST audit | Host timeout/reset oracle independent from firmware watchdog |
| `H-X19` | E2–E4 | Static audit | Post-exit EFI Runtime Services or allocations are explicitly disallowed by L1 |
| `H-X20` | E4–E5 | HOST injection | Premature PASS then reset/triple fault is host FAIL |

## 5. Fixed-link / entry LC-X01…LC-X22 case coverage

| Case | Gate | Method (future) | Observable acceptance/rejection criterion |
|---|---|---|---|
| `LC-X01` | E1 | ELF parser | Wrong ELF class/endian/machine/type rejected |
| `LC-X02` | E1 | ELF parser | filesz>memsz, offset+size overflow rejected before write |
| `LC-X03` | E1 | ELF parser | Invalid p_align or offset/VA congruence rejected |
| `LC-X04` | E1 | ELF parser | Load exceeds 64 MiB VA or 16 MiB loaded cap: reject |
| `LC-X05` | E1–E3 | ELF+map audit | W+X PT_LOAD or conflicting page permissions rejected |
| `LC-X06` | E1 | ELF parser | Non-RX e_entry rejected |
| `LC-X07` | E1 | Artifact audit | PT_DYNAMIC or runtime-required relocations rejected; no auto-expansion |
| `LC-X08` | E3 | Old-map audit | Unknown physical backing of old bridge linear VA: STOP pre-ExitBS |
| `LC-X09` | E3–E4 | VM injection | Missing same-linear executable bridge mapping: host failure, never PASS |
| `LC-X10` | E3–E4 | VM injection | Bridge stack linear VA absent in either CR3: host failure, never PASS |
| `LC-X11` | E3 | Alias audit | Low writable alias to RX physical frame fails W^X audit |
| `LC-X12` | E3–E4 | Map audit+fault | Virtual guard mapped through bridge/direct alias: FAIL |
| `LC-X13` | E0–E3 | CPUID audit | LA57/PCIDE/WP/NXE mismatch: reject run or fail preflight |
| `LC-X14` | E3–E4 | Descriptor audit+VM | Missing emergency GDT/IDT/TSS/IST after CR3: host FAIL |
| `LC-X15` | E1–E3 | ABI disassembly | EFI ABI used for SysV kernel call: reject artifact, never allow clean PASS |
| `LC-X16` | E2 | VM injection | Repeated changed MapKey: no more than 2 ExitBS calls, fail-stop |
| `LC-X17` | E2 | Parser+VM | Insufficient map buffer or malformed stride: reject without overflow |
| `LC-X18` | E2 | Static audit | EFI console/file/AllocatePages after first ExitBS attempt forbidden by lab policy |
| `LC-X19` | E4 | VM fault | #PF/#GP/#UD/#DF: bounded or externally detected failure; no recursive panic PASS |
| `LC-X20` | E4 | VM+HOST | Missing COM1 or missing terminal marker: bounded timeout and FAIL |
| `LC-X21` | E4–E5 | HOST injection | HALTED_PASS followed by reset must classify FAIL |
| `LC-X22` | E0 | Scope/manifest audit | 2+ vCPUs, physical hardware or arbitrary media: refuse unsupported run |

## 6. Blocker-to-evidence closure matrix — LC-O01…LC-O08

| Question | Target gate | Required artifact / decision |
|---|---|---|
| LC-O01 fixed ET_EXEC and linker output | E1 | A02 real ELF/PE, PHDR, relocation and disassembly audit; STOP on incompatibility |
| LC-O02 old-linear/physical bridge provenance | E3 | A04 trace of actual old/new VA->PA and CR3 cut; STOP on unprovable mapping |
| LC-O03 fixed table/image/stack caps | E1/E3 | A03+A04 bounds and frame totals; STOP on cap exhaustion |
| LC-O04 real descriptor/IST exception safety | E3/E4 | A04+A05 separate normal/#DF paths and host failure classification |
| LC-O05 pinned tools and OVMF provenance | E0 | A01 versions, hashes, licenses, CPU/VM/device profile |
| LC-O06 host oracle truthfulness | E4/E5 | A05+A06 prove absence of false PASS on reset/timeout/serial failure |
| LC-O07 actual negative controls | E2/E3/E4 | A03+A04+A05 prove ExitBS/W^X/guards exercised, not just asserted |
| LC-O08 L2 authorization boundary | E5 | user-scoped decision; return to C01–C09, no automatic L2 implementation |

**Historical H-O01–H-O08 and L1-O01–L1-O10:** earlier paper gaps are not deleted; the [fixed-link contract](../contracts/unios-l1-fixed-link-elf-cr3-bridge-entry-state-closure-v0.1.md) gives the selected **LAB-H1 paper baseline**, with the real-machine unknowns now in LC-O01–LC-O08. None of the unknowns are described as tested/resolved in execution.

## 7. Proposed result row schema for future evidence

| Field | Meaning / constraint |
|---|---|
| \`case_id\` | one exact \`L1-Xnn\`, \`H-Xnn\`, or \`LC-Xnn\`; never inferred from test label |
| \`gate\` and \`method\` | one of E0–E5; precise injected artifact/fixture and preconditions |
| \`scope\` | expected VM profile, image+firmware version and test-mode flag |
| \`evidence_id\` | A01–A06 files/records proving environment, guest and host observations |
| \`expected\` | predeclared CLEAN_PASS, EXPECTED_REJECT, EXPECTED_FAULT, FAIL_STOP or HOST_FAIL |
| \`observed\` | actual outcome + serial stage records + host process/reset/timeout report |
| \`case_result\` | PASS_NEGATIVE, PASS_POSITIVE, FAIL, INVALID_RUN, UNRUN, DEFERRED; **PASS_NEGATIVE is not CLEAN_PASS** |
| \`review\` | reviewer, source commit, date and final open/closed explanation |

**Release gate:** after real implementation is enabled and attempted, E5 may declare **only narrowly validated L1** when all in-scope mandatory rows have supported outcomes and no critical stop condition remains. L1-X14 cannot be silently counted complete: either a separately scoped DevSigned study is authorized or E5 must explicitly record it as **DEFERRED (outside current signed-boot scope)**. If a controlled VM test cannot be meaningfully constructed, mark it \`UNRUN/NOT VERIFIED\` and do not claim full negative coverage.

## 8. Final current status

| Measure | Status |
|---|---|
| Explicit scoped user Go | **YES — 2026-10-26** |
| L1 paper gate definition | **DEFINED — this runbook + matrix** |
| Indexed test coverage | **60 / 60 identified on paper** |
| In-scope executed case results | **0** |
| L1-X14 DevSigned negative case | **DEFERRED / OUT OF PROFILE, NOT EXECUTED** |
| Compiled firmware, kernel or QEMU run | **NONE** |
| L1 implemented/validated | **NO** |
| L2 / general OS implementation approval | **NO** |

**Next action remains design review and traceability audit until the project's design-only operating constraint changes.** A later engineering record must **not** backfill \`PASS\` from these paper expectations.

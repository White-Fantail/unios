# LAB-H1 E2q — genuine post-ExitBootServices CR3 switch and third QMP CPU witness

**Date:** 2026-10-30 NZDT
**Last tested executable/source SHA:** `43d64d4612f6c7420963a8e0f37dd7fe2ab2a93b`
**Actual GitHub Actions + QEMU/OVMF:** https://github.com/White-Fantail/unios/actions/runs/40942425936
**Scope:** approved L1 Experimental Kernel. Disposable QEMU only. **Not a freestanding high-half kernel boot.**

## Implemented execution path

1. The immutable E2j **old firmware CR3 (J)** host checkpoint and E2o **dormant private PML4 (O)** host checkpoint remain mandatory. E2q expands the second QMP manifest from 19 to **21 exact VA/PA/rights targets**: an additional **actual executable terminal CR3-switch assembly start and end-1**. Each page intersecting the entire finite assembly span is independently proved old-CR3 EFI_LOADER_CODE supervisor executable and equal-physical new-private-PML4 supervisor RX. A verified two-page cut is allowed if the PE linker places the finite span at a 4KiB page boundary.
2. An independent **linked PE32+ machine-byte audit** requires exactly one nonwritable-executable `CLI; CLD; MOV RSP,RSI; MOV CR3,RDI` sequence (`fa fc 48 89 f4 0f 22 df`) and rejects mutated or duplicate examples. The old E2k linked emergency-handler signatures remain audited separately.
3. UEFI uses its default x86_64 Microsoft C ABI. The handwritten assembly requires SysV argument registers RDI (PML4 physical root) and RSI (owned bridge-stack top); it is explicitly declared **`extern "sysv64"`**. An earlier experiment using `extern "C"` was wrong (CPU loaded CR3=0x30 and RSP=0x17a0), and was fixed before the verified run.
4. The normal E2q **preflight build does not call ExitBootServices and does not write CR3**; it returns EFI_UNSUPPORTED and terminates QEMU with diagnostic host exit **125**.
5. An **explicit feature `lab-e2q-actual-cr3`** restricts irreversible hardware actions to the isolated disposable VM. After both paused QMP authorization gates, the loader obtains a fresh validated final GetMemoryMap/key and calls real ExitBootServices with bounded stale-key retry (E2p). On success, it invokes the reviewed finite assembly using the explicit SysV ABI.
6. Under old firmware CR3, interrupts are disabled and RSP moves to the **owned identity-mapped two-page bridge stack**. The bridge executes actual `MOV CR3,RDI` with the independently proved private PML4 root. All subsequent executed instructions are constrained to the mapped low EFI RX assembly span. The bridge sends an exact seven-byte **`\x7fUNI2Q\x7f`** COM2 marker using polled UART transmitter readiness.
7. The host **pauses QEMU a third time** and reads the **actual live vCPU0 registers** via QMP HMP: CR3 must equal the second-host-proved private root, RIP must lie inside the precise assembly range, and RSP must be 16-byte-aligned inside the exact two-page owned bridge stack. It independently walks the actual current RIP's new-page-table mapping using **bounded QMP physical PTE reads** and demands same-physical supervisor RX.
8. **The new PML4 intentionally does not identity-map its own physical table frames**. QEMU's pre-cut `qmp_read_table` function checks both physical and virtual aliases; using it after the CR3 switch was incorrect. The post-switch walker instead uses **physical-only QMP `xp` 4KiB reads**, bounded to the private table-frame allocation. QEMU HMP `gva2gpa` reports `Unmapped` after the cut; that command is **NOT counted as a successful independent virtual translation witness**. CPU CR3/RIP/RSP registers, execution of the post-CR3 COM2 marker and independent physical table bytes form the accepted evidence.
9. Only after successful third-host verification does the host send `Q`. The terminal code then emits ISA-debug-exit byte `0x3c` (host exit **121**) without calling firmware or invoking the freestanding high-half kernel. The fail-stop path sends a distinct code `0x3d` (host **123**).
10. The normal nonfatal E2q EFI is rebuilt, SHA/byte compared to the preflight artifact, and restored after opt-in fatal variants. Disposable VM GPT, FAT and pflash VARS are deleted.

## Actually observed in completed real QEMU/OVMF CI

| Gate | Result |
|---|---|
| Python host/negative/oracle unit tests | **157/157 PASS** |
| Rust no_std support unit tests | **134/134 PASS** |
| Total host + Rust checks | **291/291 PASS** |
| Default E2q final-map and bounded-code preflight | QEMU exit **125** |
| Linked E2q executable CR3 opcode inspection and mutated negative control | **PASS** |
| E2j paused-QMP old firmware CR3 | **`0xf801000`**, 14 reservation boundary pages, 3 upper table frames |
| E2o paused-QMP new dormant PML4 | **`0xdf5d000`**, 8 physical 4KiB frames, **21** targets, 2 unmapped guards |
| Actual QEMU E2p ExitBootServices regression | host exit **115** |
| Actual third QMP post-MOV-CR3 live CPU CR3 | **`0xdf5d000`** |
| Actual third QMP CPU RIP / RSP | **`0xdfe3172` / `0xe000000`** |
| Actual terminal RIP independent bounded physical-PTE RX audit | **PASS** |
| Post-CR3 host COM2 marker and Q authorization | **PASS** |
| **Real CR3 terminal QEMU run** | **host exit 121** |
| Real CPU #UD / software vector8 IST1 / genuine nested-GP #DF regressions | exits **103 / 105 / 109** |
| Independent QEMU real #DF exception-origin trace | nested GP=1, DF error0=1 |
| Standard preflight EFI image restored after all runs | **PASS** |
| Production post-cut GDT/IDT/TSS, high-half main RSP, high-half kernel entry | **NOT RUN** |
| True clean freestanding kernel success oracle | **exit 65 NOT RUN** |

The earlier E2q QEMU failures are important negative evidence: the default EFI C ABI passed the wrong assembly registers, the physical-table reader initially assumed absent low aliases, and the linked assembly could span a 4KiB boundary. These were isolated, corrected and subjected to the **final full CI test**, not silently ignored.

## Hard operator gates

- **Real new CR3 execution is now verified in a disposable QEMU terminal test**, but the processor remains in a small low EFI-code bridge. **The approved high-half kernel entry has NOT executed**.
- The terminal assembly uses the pre-existing firmware GDT and currently installed IDT, with interrupts disabled; the dormant E2f high-half metadata has not been activated. Production post-switch exception safety and IST1 provenance under new CR3 remain unresolved.
- No code has established the freestanding kernel ABI, switched to the high-half main stack, installed approved high-half GDT/IDT/TSS, or produced complete L1 serial stage markers and clean exit **65**.
- No physical device test, loader shipping image, L2 subsystem or general-purpose OS functionality was created.
- **Recommended E2r:** independently audit the exact high-half jump/stack/descriptor ABI and fail-stop QEMU kernel-entry handoff with post-switch QMP/serial success oracle. Explicitly separate QEMU-lab stage readiness from hardware testing.

**User-run real UniOS QEMU kernel BOOT: NO-GO. Physical machine: NO-GO. L2+: NOT AUTHORIZED.**

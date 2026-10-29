# UniOS L1 E2h — independent paused-QEMU host proof for firmware CR3 root

**Date:** 2026-10-29 NZDT.
**Branch:** `experiment/l1-e2h-qemu-root-corroboration`.
**Tested source SHA:** `9a0d76a0f8a995da5a37159c3920e53731c000b3`.
**Verified CI:** https://github.com/White-Fantail/unios/actions/runs/40802120893

## The gap closed — on this *one fixed QEMU/OVMF profile*

Earlier E2b–E2g experiments treated the physical address in the current
firmware CR3 as an EFI linear pointer before proving a mapping existed.
E2g validated the frame's UEFI RAM descriptor, but a descriptor does
not establish linear identity. In E2h a **separate QEMU host observer**
checks the current CPU CR3 **before any guest volatile dereference of
the firmware PML4 root**, in this order:

1. Guest obtains its old CR3 metadata and validates UEFI owner/range, then
   emits the unique `0x7f UNI2H 0x7f` checkpoint on isolated COM2
   and blocks awaiting a single host authorization token.
2. Host connects to QMP, receives that marker (ignoring earlier OVMF
   COM2 output), and invokes `stop`; `query-status` must show
   the vCPU stopped. No physical/linear root address supplied by
   the guest is trusted.
3. Host asks QEMU HMP `info registers` for vCPU0's CR3, independently
   extracts the exact physical root, and asks `gva2gpa <root>`.
   It demands that the returned **guest physical** address equals
   the **guest virtual** root address; QEMU 8.x replies `gpa: 0x...`.
4. Host reads the first **eight 64-bit root PML4 words** through
   `xp /8gx <root>` (physical read) and `x /8gx <root>`
   (guest virtual read), rejects empty or differing content,
   and re-reads `info registers` to reject any CR3 change.
5. Only then does the host send `A` to COM2 while vCPU is paused and
   `cont` the CPU. Guest resumes E2b–E2g staging/descriptor checks.
   Wrong, missing or timed-out evidence leaves guest unapproved.
6. The actual QEMU host process must independently exit **83** from
   E2h's `isa-debug-exit` byte 0x29. Exit **65** reserved for
   genuine kernel boot is never generated. No ExitBootServices or
   new CR3 write occurs here.

## Actual verification

| Gate | Observed |
|---|---|
| Python host oracle + QMP parser negative tests | **28/28** |
| Rust `unios_bootmap` | **11/11** |
| Rust `unios_bridgecheck` | **16/16** |
| Rust `unios_cpugate` | **9/9** |
| Rust `unios_descriptors` | **14/14** |
| Rust `unios_elf` | **9/9** |
| Rust `unios_paging` | **13/13** |
| Rust `unios_rootgate` | **17/17** |
| Rust `unios_staging` | **8/8** |
| **Total host and unit tests** | **125/125** |
| Independent previously configured host PTE, descriptor and RAM fixture checks | **PASS** |
| Real QEMU/OVMF root handshake with paused vCPU0 | **PASS** |
| Actual root tested (run-specific) | `0x0f801000` |
| `gva2gpa`, `xp`, `x` evidence + recheck CR3 | **PASS (8 PML4 qwords)** |
| Independent host diagnostic exit | **83** |
| New CR3, ExitBootServices, freestanding kernel entry | **NOT ATTEMPTED** |

See `out/e2h-host-root-proof.json`,
`out/e2h-host-hmp-transcript.txt`, and
`out/preflight-host-exit.txt` in the successful CI artifact.

Reproducibility SHA256 in actual successful CI:

- Freestanding kernel ELF: `8f008488fbca78baaba084aa70fe2168abf79c7922c98bd7edfe42b8e5dd80f5`
- UEFI loader PE32+: `7cb4ad038a6bd4588475d35d4f522a3f2ac8d9c04a6dc37712a589b06d763cc8`

## Constraints still OPEN

- Root validation is **QEMU-only and run-specific**. It is not a
  guarantee for UEFI implementations, hardware, hypervisors, or any
  different memory map; never enable it outside disposable LAB-H1.
- The host sampled only the **PML4 root page** and eight qwords. It did
  **not independently validate every lower-level page table frame**
  dereferenced by the guest walker. The initial root bootstrap
  ambiguity is now independently corroborated **for this QEMU run**
  but nested physical-as-linear walker assumptions remain P0.
- No executable real CR3 transition trampoline, actual GDT/IDT/TSS
  activation and emergency handler dynamic tests, final BootContext,
  ExitBootServices handoff or high-half kernel execution exists yet.
- HMP through QMP is version-dependent. A changed QEMU response,
  inaccessible COM2, missing root mapping, reset, timeout, guest error
  or physical/virtual mismatch must **fail closed**, not be accepted.
- Actual kernel entry with the ordered COM1 stage oracle and exit 65 is
  the minimum **가상환경 테스트 단계 진입** condition. Real
  physical-machine tests remain separate and unapproved.

**Status:** L1 E2h pre-ExitBootServices independent host check;
E2 partial; E3–E5 not passed. No L2+.

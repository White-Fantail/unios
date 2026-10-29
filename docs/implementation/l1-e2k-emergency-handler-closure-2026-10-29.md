# LAB-H1 E2k — exact fatal handler code closure and isolated direct-call smoke

**Date:** 2026-10-29 (NZDT)
**Validated code SHA:** `50b20f4079e0d5fc1d916373a2582a9c635d3004`
**Final actual GitHub Actions/QEMU:** https://github.com/White-Fantail/unios/actions/runs/40816236577
**Scope:** approved L1 Experimental Kernel only; EFI *preflight* with disposable QEMU/OVMF, NOT a real kernel boot.

## Reviewed implementation

1. Two existing EFI-resident assembly fail-stop handlers are now
   independently bounded by explicit begin/end linker symbols. The
   generic handler targets debug exit byte `0x31` and the dedicated
   double-fault handler targets `0x32`. Both are exactly **11 bytes**:
   CLI; MOV DX,0xF4; MOV AL,signal; OUT DX,AL; HLT; short JMP back to HLT.
   Neither has an IRETQ, returns to firmware, or emits full-boot
   success byte `0x20`.
2. Each containing code page is checked against EFI_LOADER_CODE under
   **old firmware CR3** and mapped at the same linear address and
   physical address under the **built but not activated** private CR3.
   New mappings must be supervisor **RX**; existing ones must be
   identical RX. Bounded 1–2 page span, disjoint code span ordering,
   CPU WP/NXE gates, and byte-accurate code signature are all required.
3. An **independent host PE32+ parser** locates exactly one occurrence
   of each 11-byte instruction sequence in a bounded, executable,
   nonwritable section of the linked EFI image. A deliberately
   modified HLT instruction must be rejected. No inference from
   unlinked assembly source is accepted as sufficient.
4. Standard E2k QEMU performs only preflight and returns debug byte
   `0x2c` => host **89**. The E2j paused-host COM2/QMP old-CR3
   reservation boundary witness continues to run.
5. Two independently compiled, **explicitly opt-in QEMU smoke**
   variants directly CALL the generic and DF-handler symbols *after*
   all preflight checks have passed. Expected host results are
   **99** (generic: `0x31`) and **101** (DF handler: `0x32`).
   These are **direct subroutine calls**, not CPU-delivered
   exceptions, not tests of LGDT/LIDT/LTR, and not tests of IST1.
6. Smoke variants cannot become the final default image: the script
   preserves the preflight PE, recompiles it after smoke, requires
   byte equality using `cmp`, restores the default ESP EFI,
   preserves separate text/QMP evidence, and discards the disposable
   GPT/FAT/pflash images containing fatal smoke variants.

## Verified evidence

| Gate | Verified outcome |
|---|---|
| Python host/oracle, linked PE and smoke negative controls | **77 / 77 pass** |
| Rust no_std-support library tests including new handler bytes | **103 / 103 pass** |
| Total host + unit tests | **180 / 180 pass** |
| Independently parsed linked UEFI PE signatures | **PASS**, unique RX code, 11 bytes each |
| Mutated HLT PE negative control | **REJECTED** |
| Actual standard QEMU/OVMF E2k preflight | **89** |
| Actual QEMU generic-handler direct CALL | **99** |
| Actual QEMU DF-handler direct CALL | **101** |
| QEMU witnessed reserved boundary addresses | **14**, all 2MiB mappings |
| QEMU host page-table frames fully compared | **3**, no live 4KiB PT leaf |
| Normal preflight executable restored byte-identical | **PASS** |
| Real CPU exception through installed IDT | **NOT RUN** |
| Real hardware #DF IST stack switch | **NOT RUN** |
| ExitBootServices, new CR3, high-half kernel entry | **NOT RUN** |

- Standard E2k EFI PE32+ SHA256:
  `9f7d8d4d2fe429027dd037d80751d17add45e279a23b1436a255a4231027ec90`
- Linked PE host offsets (standard preflight artifact):
  generic `0x3cd0`, DF `0x3ce0`.
- The two separately compiled smoke EFI variants may have different
  linker offsets. Their exact PE signatures are independently audited.
- Standard E2k path intentionally uses host **89**, not clean-boot
  success **65**. The non-PASS signal distinction is enforced.

## P0 blockers before active exception handling / CR3 transition

- Do not install GDT/IDT/TSS while firmware remains active unless
  a separately approved, fail-closed **disposable QEMU** runtime
  injection protocol is explicitly covered by new verified tests.
  Direct handler CALLs are NOT real IDT exception injection.
- The previous E2f TSS RSP0/IST1 descriptors and stack guards are
  built and verified but **remain dormant**; live `ltr` and
  fault delivery/IST correctness are unknown.
- Old-CR3 identity evidence is tied to pinned QEMU and sampled
  reservation boundaries, not universally proved for every page or
  for real physical firmware. E2e's four-byte canary is **not** a
  real CR3 transition stub.
- Full final GetMemoryMap/ExitBootServices choreography, immutable
  BootContext, post-cut no-firmware-call enforcement, real new CR3
  execution, SysV high-half kernel entry, complete COM1 oracle and
  independent true boot exit **65** have **not** occurred.

**Operator-level genuine QEMU kernel TEST STAGE = NO-GO.**
**Physical PC test = NO-GO. No L2 or higher functions.**

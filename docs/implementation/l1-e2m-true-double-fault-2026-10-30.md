# LAB-H1 E2m — genuine CPU double-fault and IST1 QEMU witness

**Date:** 2026-10-30 (NZDT)
**Actual executable code SHA:** `aa608e7b1f147312829ae674bfe1637c1c5fbd91`
**Verified Actions + real QEMU:** https://github.com/White-Fantail/unios/actions/runs/40930426640
**Scope:** approved L1 Experimental Kernel, strictly pinned *disposable* QEMU/OVMF.
This is a terminal EFI preflight fault test, **NOT a freestanding kernel boot**.

## Why the E2l software INT8 proof was insufficient

E2l installed physical, EFI-owned GDT/IDT/TSS under old CR3 and
demonstrated a genuine CPU #UD and hardware TSS IST1 delivery via a
**software** INT8. Software vector 8 does not prove the CPU actually
classified and delivered a *double fault*. E2m supplies a separate
fail-closed genuine nested-contributory-fault path and an independent
QEMU native `-d int` exception-origin witness.

## E2m restricted execution

1. Retain E2j's pre-dereference COM2 paused-QMP old-CR3 root and
   reserved-boundary identity witness, and E2k's independent EFI PE
   error-handler-byte audit. Every new executable test handler is
   checked as EFI_LOADER_CODE, with matching new-table supervisor RX
   identity mapping. New page tables **are built but never activated**.
2. Use an **explicit opt-in compile-time QEMU-only feature** to
   build a separate physical GDT/IDT/TSS at a validated EFI-owned
   descriptor page. RSP0 and IST1 point into reserved, identity-mapped
   physical stacks. This is NOT the dormant high-half production image.
3. Only for the actual DF smoke variant, deliberately corrupt the
   *#GP gate's segment selector* to `0x28` (beyond GDTR limit 39).
   The vector 8 #DF gate remains present, uses valid CS=0x08, and
   selects IST1. The physical descriptor page is independently
   verified, and strict builder negative tests prove no unexpected
   GDT/TSS/IDT changes. The ordinary preflight has **no corrupted gate**.
4. After LGDT/LIDT/LTR, `mov ds,0x28` generates CPU #GP. Delivery of
   that #GP attempts to use invalid #GP gate CS, causing a second
   contributory CPU #GP during exception delivery. x86 delivery then
   creates genuine **#DF**. No `int 8` or direct CALL is used by
   this variant.
5. The dedicated terminal #DF handler checks that RSP resides in
   the **reserved IST1 extent** and that the exception error code
   at the top of the CPU-generated frame is exactly `0`.
   Success output `0x36` yields QEMU host exit **109**;
   wrong stack or nonzero code outputs `0x37` / **111**, which
   fails the host oracle. No IRETQ or return to UEFI occurs.
6. A second independent **host-owned QEMU native exception trace**
   (`-d int`) must observe a **#GP during #GP** and subsequent
   **vector 8 with zero exception error code** in that order.
   A mutated trace control that destroys the nested-GP proof must
   be rejected. A guest-supplied success code **alone is insufficient**.
7. The experimental EFI fault variants are segregated. The default
   preflight PE is rebuilt, checked `cmp` byte-identical, restored,
   and the disposable guest GPT/FAT/pflash images containing the
   deliberate fault variants are deleted after testing.

## Actual observed results (not simulated)

| Gate | Actual result |
|---|---|
| Python host/oracle/native QEMU trace negative tests | **102 / 102 PASS** |
| Rust no_std support unit tests including malformed #GP gate isolation | **110 / 110 PASS** |
| Total unit/host checks | **212 / 212 PASS** |
| Linked EFI PE32+ emergency-handler signature and mutation controls | **PASS** |
| Standard QEMU preflight | **exit 93** |
| CPU-generated #UD regression | **exit 103** |
| Software INT8 IST1 regression | **exit 105** |
| **CPU-generated true #DF with IST1 stack and error code 0** | **exit 109** |
| **Independent QEMU native nested-fault evidence** | **1 #GP-during-#GP**, **1 vector8 error=0** |
| QEMU native trace negative mutation test | **REJECTED** |
| E2j independent reserved boundaries | **14 addresses, all 2MiB leaves**, 3 full table frames |
| Standard executable restored byte-identical | **PASS** |
| Actual ExitBootServices/new CR3/freestanding entry | **NOT RUN** |

Standard normal preflight EFI PE SHA256:
`0688c12cacf6192fbad405050ac85b4b852965110f7eda8eadf212a6fc85c46a`.

## Hard bounds and next gate

- Real #DF and IST1 **only** in pinned QEMU before ExitBootServices
  and while firmware's old CR3 remains current. This does not prove
  future post-ExitBootServices protections or a production
  high-half exception stack, and intentionally must NOT be run on
  any physical device.
- No handler recovery, scheduler, system call, L2 feature, hardware
  device access, or customer-facing OS boot functionality was added.
- Initial ELF segments are staged and private page tables are built,
  but safe transition still needs all required live RIP and RSP
  page addresses under both CR3 roots, validated W^X provenance,
  final memory map/key retry strategy, machine-state snapshot and
  no post-cut EFI interactions.
- Real ExitBootServices, active private CR3, bounded transition
  trampoline, main/IST high-half stack activation and genuine
  freestanding kernel entry **have not happened**.
- Complete serial stage oracle and true clean kernel exit **65**
  remain **unverified**. E2m exit 109 is *fault-injection evidence*,
  emphatically **NOT** success 65.

**Operator QEMU kernel boot test stage: NO-GO.**
**Physical hardware test: NO-GO.** L1 scope only.

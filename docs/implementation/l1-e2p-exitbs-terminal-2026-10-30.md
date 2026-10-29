# LAB-H1 E2p — bounded final memory-map key and terminal ExitBootServices in disposable QEMU

**Date:** 2026-10-30 (NZDT)
**Verified executable SHA:** `2618f0c60422d928161c303e4a6d250f108ec061`
**Actual end-to-end QEMU/OVMF CI:** https://github.com/White-Fantail/unios/actions/runs/40937395202
**Scope:** approved L1 Experimental Kernel, pinned isolated QEMU only. NOT a kernel boot.

## What E2p adds

1. E2o's two independent paused-QMP gates remain mandatory:
   original firmware **old CR3** / EFI reservation ownership, then
   **all eight used private PML4 4KiB frames** and 19 mapped/unmapped
   address/rights targets. The new PML4 remains BUILT BUT DORMANT.
2. The E2p no_std `unios_exitbs` support crate defines an explicit
   strict final GetMemoryMap/ExitBootServices transaction contract:
   descriptor version 1, aligned stride 40–256, nonzero key,
   64KiB fixed buffer, nonempty aligned map, bounded **two attempts**,
   and retry **ONLY** for `EFI_INVALID_PARAMETER` (stale map key).
   Other errors, exhausted retry, malformed descriptors or ownership
   changes are never treated as successful.
3. Before any first ExitBootServices attempt, the loader obtains a
   **NEW final EFI GetMemoryMap snapshot**, distinct from E2a's
   earlier staging proof. It validates EFI LoaderData ownership for
   all 16-or-fewer allocated reservation extents. Both successful
   first-try and stale-key-then-success models have deterministic mock
   tests, including the exact absence of any further firmware
   callbacks after success.
4. The **default** E2p build only checks that final map / key and
   then emits a distinct diagnostic byte `0x38` (QEMU host **113**),
   returning to UEFI while Boot Services are still alive. This is
   NOT proof of real ExitBootServices.
5. An **explicitly compiled QEMU-only opt-in feature**
   `lab-e2p-actual-exitbs` executes the real UEFI
   ExitBootServices(image_handle,final_key) service at the proper
   EFI_BOOT_SERVICES x86_64 table offset, after both host gates and
   the fresh map verification. If a stale key is reported, it performs
   at most ONE new GetMemoryMap/validate/ExitBootServices attempt,
   without allocations, logging, console access or unrelated EFI
   callbacks between GetMemoryMap and ExitBootServices.
6. On real EFI ExitBootServices SUCCESS, the test takes an
   **irreversible terminal path**: no firmware calls, no return to
   the UEFI entry routine; only direct ISA debug-exit I/O byte
   `0x39` (host **115**) and a HLT loop remain. Failure of this
   protocol produces a distinct non-PASS code `0x3a` (host 117)
   or prior-preflight rejection `0x3b` (host 119).
   None are the true kernel success exit **65**.
7. Existing QEMU #UD, software INT8 IST1 and independently QEMU-traced
   genuine #DF are unchanged. The terminal ExitBS test EFI is
   segregated, then the default nonfatal E2p image is rebuilt,
   compared byte-for-byte, restored, and disposable guest disks
   deleted.

## Observed verification

| Gate | Actual outcome |
|---|---|
| Python unit/host/oracle/negative tests | **139/139 PASS** |
| Rust no_std crate tests including ExitBS mock matrix | **134/134 PASS** |
| Total | **273/273 PASS** |
| Standard QEMU final-map preflight | **exit 113** |
| Genuine opt-in QEMU/OVMF ExitBootServices status==success terminal path | **exit 115** |
| CPU #UD regression | **exit 103** |
| Software INT8 IST1 regression | **exit 105**, not genuine #DF |
| Real CPU #GP-during-#GP → #DF regression | **exit 109**, native QEMU trace PASS |
| Independent paused-QMP old firmware CR3 root | `0xf801000` |
| Old firmware reservation boundary samples | **14**, 3 fully-compared old table frames |
| New inactive PML4 (standard build) | `0xdf5e000` |
| Host independently compared every used new private PTE frame | **8** |
| Host sampled new mapped and unmapped targets | **19**, including 2 guards |
| Default EFI artifact rebuilt byte-identical after disposable variants | **PASS** |

Default E2p EFI SHA256 (pinned CI):
`bbe9be2f027b9ef3866879281c48252924c615601733b13c92f5b813c4356098`.

## Correct scope and unresolved blockers

- Unlike earlier E2o, E2p truly calls UEFI ExitBootServices **in a
  separately compiled terminal QEMU variant**, and sees the
  success-branch ISA debug-exit result. This does not independently
  instrument OVMF's internal memory-map-key logic or prove general
  hardware portability.
- The terminal QEMU test still runs under the **old EFI CR3** and
  directly debug-exits. It does NOT activate new private CR3, switch
  RSP to the high-half kernel stack, switch to the high-half GDT/IDT,
  establish post-cut exceptions, execute freestanding kernel entry,
  or use the actual full boot serial-stage oracle.
- The current bounded bridge is an inert EFI canary, **not an
  executable CR3-switch trampoline**. Post-cut code/stack lifetime
  and complete RIP/RSP liveness at the first `mov cr3` are open.
- E2p's default build **never calls ExitBootServices**. The opt-in
  feature's success code 115 must not be mistaken for the real
  clean kernel boot code 65. Failing QEMU tests do not justify
  entering real hardware.
- **E2q recommendation:** independently bound and validate the
  exact live CR3-switch assembly bytes, same-linear instruction
  addresses and bridge-stack RSP for *both* roots; use a separate,
  fail-closed disposable QEMU transition matrix before any physical
  hardware testing. No L2+ development.

**Operator QEMU real kernel boot testing: NO-GO.**
**Physical computer testing: NO-GO.**

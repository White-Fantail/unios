# LAB-H1 E2j — pinned-QEMU targeted firmware leaf witness

**Date:** 2026-10-29 NZDT
**Last tested source:** `273f9561c03298b88ff82fef8a8411485a61c65a`
**Actual CI / QEMU:** https://github.com/White-Fantail/unios/actions/runs/40809399671
**Scope:** approved L1 Experimental Kernel only. Strict UEFI **preflight**, not a booted kernel.

## Implemented

- Extends the E2h/E2i QEMU-only COM2 pause-and-authorize scheme.
- The guest first validates its EFI memory descriptor snapshot and
  root ownership without dereferencing old physical page-table pointers.
  It then sends a bounded COM2 `UNI2J` marker with **up to 16**
  allocated reservations, each encoded as `id:u8`, `pages:u32`,
  `physical_base:u64`, before its first old-root table read.
- The host rejects malformed records, oversized or overlapping ranges,
  invalid seven auxiliary allocations, unsupported physical extent,
  unknown record ordering, and excessive target counts. The guest
  supplies **candidate addresses, not proof**; QMP supplies CR3,
  translation and memory contents independently.
- With the vCPU **stopped**, host independently reads QMP CR3 and
  PML4/PDPT upper tables, then walks page-table chains for only
  **first and last page of each EFI reservation**.
- The host demands identity mapping and effective writable permissions
  for each sampled reserved page, corroborates independent QMP
  `gva2gpa`, and compares all 512 words of each observed
  page-table frame using QMP physical `xp` and virtual `x`.
  The targeted walker recognizes 1GiB and 2MiB huge-page leaves,
  and supports 4KiB terminal leaves in synthetic negative tests.
- QEMU may continue only after host sends the unique authorization
  byte `J`; otherwise the preflight is rejected. Host exit **87**
  (`isa-debug-exit` byte `0x2b`) is not the full-boot PASS **65**.

## Actually observed (final run)

| Gate | Observed |
|---|---|
| Python host/oracle tests, including malformed target controls | **64 / 64 pass** |
| Rust pure/loader-support library tests | **97 / 97 pass** |
| Total host / unit test count | **161 / 161 pass** |
| Actual disposable QEMU + OVMF preflight | **PASS** |
| Independently traced EFI reservation boundary VAs | **14** |
| Physical/virtual 4KiB page-table frames fully compared | **3** |
| Observed terminal leaf sizes | **14 × 2MiB**, 0 × 1GiB, 0 × 4KiB |
| Guest outcome | **87**, distinct from clean kernel exit 65 |
| ExitBootServices / live new CR3 / high-half kernel entry | **NOT RUN** |

- UEFI PE SHA256: `5d0b07055cf91d130c4e56acd298e49e94160d2ee64b694a8b43efda4b3c702e`.
- The observed 2MiB leaves prove the **selected address
  translations through the PD level** in this pinned QEMU profile.
  No 4KiB PT frame was actually traversed on these guest paths.
  A pure host unit test covering a 4KiB terminal entry is **not**
  runtime evidence of a live QEMU 4KiB entry.
- The 512-word PML4/PDPT audit from E2i also remains in the path;
  counts in this report for targeted tracing refer to its own
  independent read cache, not all possible firmware tables.

## Explicit limitations / next safety gates

1. Only the **first and last page** of each EFI-reserved run was
   independently sampled; intermediate pages and all possible
   firmware addresses are not independently covered.
2. Host pause/translation is QEMU- and profile-specific; it does
   not provide portable proof on physical UEFI systems, nor guarantee
   that firmware mapping tables never change after guest resumes.
3. EFI bridge / emergency **code address** ownership is separately
   checked by E2d/E2f guest audits; E2j host manifest samples
   **reserved data ranges**, not these executable code pages.
4. Real CR3 transition trampoline, bounded RIP and RSP cut,
   active GDT/IDT/TSS, installed #DF handler, final
   GetMemoryMap/ExitBootServices, immutable BootContext, and
   post-cut freestanding kernel entry are **not implemented or
   independently verified**.
5. Full L1 stage serial trace plus independent clean exit **65**
   and the accepted L1 matrix are **not passed**.

**User QEMU kernel test: NO-GO. Physical machine test: NO-GO.**
Retain experiment-only isolation. No L2 or later functions.

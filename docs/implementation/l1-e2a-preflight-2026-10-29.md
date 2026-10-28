# UniOS — E2a UEFI physical reservations & memory-map preflight

**Evidence date:** 2026-10-29 NZDT. **Approved scope:** LAB-H1 L1 only.
**Branch:** `experiment/l1-e2-memory-map-preflight`.
**Actual source commit tested:** `6ca63a67106ef6b2703e37f82079894344421ac0`.
**QEMU/CI run:** https://github.com/White-Fantail/unios/actions/runs/40742771454

## Implemented

- A separate `no_std` `unios_bootmap` verifier checks descriptor
  version 1, variable 8-byte-aligned descriptor stride (40..256), size
  within the 64 KiB reserved loader-image map buffer, aligned physical
  descriptor ranges, physical arithmetic overflow and overlaps.
- UEFI `AllocatePages(AllocateAnyPages, EfiLoaderData)` allocates
  physically owned pages for every <=8 PT_LOAD segment and 7 boot
  reservations: 512 KiB table reserve (128 pages), 8 KiB bridge,
  16 KiB main stack, 16 KiB #DF IST, 4 KiB each for descriptor
  storage, BootContext metadata and diagnostics.
- After allocations, `GetMemoryMap` returns descriptor size/version/key;
  the verifier requires **every allocated reservation** be accounted for
  within LoaderData descriptors, including contiguous descriptor splits.
  A malformed, truncated, overlapping or uncovered range is rejected.
- The EFI image uses its preallocated, 4 KiB-aligned, 64 KiB static
  buffer and never allocates while validating its snapshot.
- Dedicated QEMU `isa-debug-exit` byte `0x22` signals only
  `E2A_MAP_PREFLIGHT_ONLY` (host exit 69). Full protected kernel boot
  exit `65` remains **disabled**. The host rejects false success.

## Actual observed evidence

| Observation | Result |
|---|---|
| `unios_bootmap` unit tests (positive/negative) | 11/11 PASS |
| existing `unios_elf` unit tests | 9/9 PASS |
| Python host success oracle simulator | 9/9 PASS in tested commit |
| kernel ELF64 ET_EXEC high-half, 2 load segments / 8 KiB | built/audited |
| loader PE32+ EFI application | built/audited |
| QEMU 8.2.2 Q35 / OVMF / 1 vCPU / NX / no network | **exit 69** E2a preflight observed |
| `ExitBootServices`, kernel entry, CR3 switch, IDT/TSS/IST | **NOT IMPLEMENTED / NOT RUN** |

**Firmware provenance (SHA-256):**
- OVMF CODE: `ad23261d43116abc3747d59a7837fbf578c9586a8993f4eeba064cd07eccb083`
- Initial OVMF VARS: `5d2ac383371b408398accee7ec27c8c09ea5b74a0de0ceea6513388b15be5d1e`
- QEMU binary: `8a35ccba41582fc6c38b9df85fc9e35fa1d42f414d2d7d8090ee9b2f5e7c0854`
- Kernel ELF: `22df44074b18d1995ec8b7472f8e63eacdc68d175f48335759e4b92d7024b4d5`
- EFI PE: `2978e717b4f902d030fad11fc6f7ecd52e5f80145c0e26edbea73e3d8c0f2a78`

## Engineering interpretation

E2a verifies **reservation ownership and a real firmware memory-map snapshot**,
not that the returned physical allocations can safely be dereferenced as Rust
pointers. Physical-to-linear provenance is still an **LC-O02 blocker**.

There is **no PT_LOAD copy, BSS initialization in staged pages, BootContext
publication, private page tables, ExitBootServices, stale MapKey retry, same-
linear bridge, SYSV transfer or HALTED_PASS** yet. No physical address is
cast to an executable virtual address; the stored preflight MapKey is invalid
for a future exit attempt after any console operation.

**Gate statuses:** E0 partial; E1 actual artifacts + preflight, staged-copy
incomplete; E2 **partial (E2a only)**; E3/E4/E5 not run. 59 applicable L1
traceability rows remain individually unvalidated; L1-X14 DevSigned deferred.
No L2 work and no claim of complete kernel boot.

**Next controlled E2b:** prove current firmware-CR3 physical backing and
approved linear access before copying ELF segments/BSS; prepare exact page
tables and emergency descriptors, verify post-ExitBS bridge at the final
cut, and only thereafter enable bounded ExitBootServices retry. Do not
weaken LAB-H1 assumptions or auto-merge to main on a preflight-only run.

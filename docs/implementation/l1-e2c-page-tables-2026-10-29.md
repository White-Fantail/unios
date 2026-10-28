# UniOS LAB-H1 E2c: 4-level private page tables — verified pre-cut

**Date:** 2026-10-29 NZDT.
**Branch:** `experiment/l1-e2c-private-page-tables`.
**Latest tested code:** `fb444e91e999109f5e89daa119d0d6b1509681d4`.
**GitHub Actions / QEMU:** https://github.com/White-Fantail/unios/actions/runs/40751402236

## What was implemented

1. The `no_std` `unios_paging` crate writes exactly 4-level x86_64
   supervisor-only PML4/PDPT/PD/PT and 4 KiB leaves to an **EFI-allocated,
   old-CR3-audited 512 KiB private table reservation** (hard limit 128 tables).
   No table page can be mapped as a leaf. Arithmetic and alignment are checked.
2. RX (P, !W, !NX), RO (P, !W, NX), and RW (P, W, NX) leaf rights are selected
   from the embedded ELF's audited `PT_LOAD` flags. Upper directory entries
   are supervisor present+writable to permit the leaf effective policy.
   No user bit, global bit, large page or writable alternative alias to RX.
3. Stable L1 experiment-only high-half slots host a 16 KiB main stack with
   one unmapped 4 KiB guard, separate 16 KiB #DF IST stack with guard,
   4 KiB metadata descriptor page and 4 KiB read-only BootContext page.
   The EFI-owned 8 KiB bridge stack is additionally identity-mapped
   under the newly constructed table using the old-CR3-checked physical
   allocation. **Bridge executable code is NOT mapped/proven.**
4. General public mapping is bounded by the kernel's 64 MiB high-half
   envelope; a dedicated API alone creates the two low bridge stack
   identity mappings. The builder audits expected rights, physical frames,
   double guard absence and table capacity.
5. An independent Python page walker inspects a **host-generated deterministic
   512 KiB PML4 fixture** against the actual linked ELF's PT_LOAD flags.
   The independent negative control corrupts the PML4 user bit and requires
   rejection. This host fixture is **NOT** the live QEMU page table dump.

## Real, reproducible evidence

| Result | Observed |
|---|---|
| Python host outcome oracle (simulated negative controls) | 12/12 PASS |
| `unios_bootmap` map/ownership unit tests | 11/11 PASS |
| `unios_elf` bounded parser unit tests | 9/9 PASS |
| `unios_staging` staging/BSS unit tests | 8/8 PASS |
| **New `unios_paging` unit tests** | **12/12 PASS** |
| Host-generated actual ELF-based PTE fixture | 8 tables / 15 leaves |
| Python **independent** host PTE walker | **PASS**, both guards absent |
| Python user-bit tamper negative control | **PASS**, corrupted root rejected |
| Real ELF ET_EXEC | 3 PT_LOADs / 12 KiB, one 4 KiB BSS |
| Real QEMU 8.2.2 / OVMF on pinned LAB-H1 | exit **73**, E2c preflight only |
| Kernel CR3 switch / ExitBootServices / full L1 success (65) | **NOT RUN** |

**CI and artifact provenance:** Actions run 40751402236 includes test logs,
actual ELF/EFI images and staged PTE fixture + independent audit logs. No
live guest PTE dump or runtime fault test is claimed from the host fixture.

## Still-blocking, no shortcuts

- **LC-O02 / old firmware CR3 root bootstrap** still uses experimental
  physical-as-linear first-table read. Allocated pages are then checked, but
  independent proof of that initial dereference and same-linear executable
  bridge instruction provenance remains absent.
- The 8 KiB **bridge stack** is mapped in both domains, but **EFI-resident
  bridge machine code / verified RIP**, emergency IDT/GDT/TSS and #DF IST
  entries and live fault handlers have not been constructed or audited.
- No EFER.NXE / CR0.WP / CPUID NX pre-cut gate has been proven for the *new*
  tables yet. Their NX bits are stored but not executed.
- BootContext RO page is mapped but its actual schema/contents and handoff
  immutability are not established. No real ExitBootServices attempt, MapKey
  constrained retry or post-exit firmware-call prohibition is executed.
- No kernel instruction has executed under the new CR3. A VM exit **73**
  must never be classified as clean kernel boot exit **65**.
- E0/E1/E2 remain partial as prescribed by prior gates. E3–E5 / 59
  applicable traceability cases remain unproven as a whole.
  DevSigned-only L1-X14 is still deferred.
- No L2 or higher functions have been implemented.

**Next gated increment:** independently prove current old-CR3 root access
and bridge executable same-linear translation; verify the exact resulting
PTEs and guard rights before constructing functional emergency descriptors.
Only after that may a separately reviewed experiment implement the bounded
GetMemoryMap/ExitBootServices retry and new CR3 transfer.

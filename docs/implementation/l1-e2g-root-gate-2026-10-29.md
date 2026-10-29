# UniOS L1 E2g — firmware CR3 root metadata gate

**Date:** 2026-10-29 (NZDT). **Scope:** approved L1 Experimental Kernel, pre-ExitBootServices only.

- Code SHA actually tested: `ccbb5e8e4e5cc652f342b96a59282feff30aeef4`
- Real CI/QEMU: https://github.com/White-Fantail/unios/actions/runs/40793407968
- Experimental branch: `experiment/l1-e2g-old-cr3-root-audit`

## Change and trust boundary

The former E2b–E2f old-firmware CR3 walker dereferenced the firmware
root physical frame as a linear pointer *before* checking its UEFI owner.

E2g introduces `unios_rootgate`, a `no_std` pure metadata verifier,
and makes the EFI loader call it **before its first volatile PML4 read**.
The gate rejects CR4.LA57 / CR4.PCIDE, missing CR4.PAE, invalid CR3
alignment/profile bits or root physical range, an invalid variable-stride
UEFI memory map, root frames not entirely inside exactly one
`EfiBootServicesData` or `EfiLoaderData` descriptor, conventional
(unallocated) pages, MMIO/runtime regions, duplicate owner descriptors
and collisions with any of the already-owned L1 reservations.

It binds the precise 64-bit CR3 value. The old-CR3 staging page walk,
EFI code-anchor audit, and executable bounded-stub audit all re-read
CR3 and fail closed if the register has changed since the original gate.

**Critical distinction:** A UEFI memory descriptor proves that a physical
page is in a permitted allocated RAM type; it **does not** prove that
the firmware maps this page at the identical linear address. E2g reduces
an arbitrary-pointer dereference risk but does not eliminate it or
provide an independent root virtual-translation proof. The first
PML4 physical-as-linear dereference therefore remains a QEMU-only
**P0 blocker for a CR3 transition and any real-machine experiment**.

## Observed evidence (final code)

| Gate | Actual result |
|---|---|
| Host oracle negative controls | **16/16** |
| Boot memory-map parser | **11/11** |
| Bounded executable bridge checker | **16/16** |
| CPU feature gate | **9/9** |
| GDT/IDT/TSS descriptor checker | **14/14** |
| ELF parser | **9/9** |
| Private PML4 builder | **13/13** |
| New rootgate parser and negative tests | **17/17** |
| Staging/BSS tests | **8/8** |
| **Total host and unit tests** | **113/113** |
| Host synthetic firmware-root fixture, independent Python parser | **PASS** |
| Host independent negative control (changed owner to conventional RAM) | **REJECTED** |
| Prior host PTE and descriptor fixture audits | **PASS** |
| Actual QEMU/OVMF EFI preflight | **PASS: host exit 81** |
| Actual ExitBootServices, CR3 change, kernel entry | **NOT IMPLEMENTED / NOT RUN** |

The independent root fixture is **synthetic host data**, not a live
QEMU physical memory dump or proof of the guest firmware's root
linear-to-physical translation. QEMU's diagnostic `isa-debug-exit`
value `0x28` yields observed host status **81**, not true kernel
clean-success **65**.

Hashes from the successful run:

- Kernel ELF SHA256: `8f008488fbca78baaba084aa70fe2168abf79c7922c98bd7edfe42b8e5dd80f5`
- EFI loader SHA256: `4cfd17ac7cf50cf92ad61ebf6b60971078a1d092d1acac2eb65abbb2bb658100`

## Next hard requirements

1. Capture **independent host-side** OVMF/QEMU root mapping evidence
   at a guest-coordinated checkpoint **before** dereferencing any
   old-root physical address as an EFI linear pointer. The paused guest's
   CR3, physical page-table bytes and virtual root mapping must be
   checked by a separate host agent. A UEFI descriptor alone is insufficient.
2. Implement and bound an actual CR3-switching assembly trampoline:
   instruction-span provenance, live pre-/post-switch RIP and stack,
   preserved state and no accidental post-cut firmware calls.
3. Independently review exception-handler instruction bytes, IDT/GDT/TSS
   installation, #DF fault tests and page-guard fault behavior before
   enabling the new address space.
4. Complete BootContext, final GetMemoryMap/MapKey and bounded
   ExitBootServices retry; then verify SysV high-half kernel entry
   and ordered COM1 stages with independent host exit 65.
5. Only after a successful actual kernel entry: announce
   **가상환경 테스트 단계 진입** with verified disposable QEMU commands.
   Physical PC testing is separately gated.

**Status:** L1 E2g pre-ExitBS preflight; E2 partial, E3–E5
unverified, real-user QEMU/PC boot **NO-GO**. No L2+.

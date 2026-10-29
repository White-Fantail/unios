# LAB-H1 E2i — paused-QEMU upper firmware table witness

**Date:** 2026-10-29 NZDT
**Code tested:** `40b1accd40803c403b61b6ac9e36c1599939bc78`
**CI:** https://github.com/White-Fantail/unios/actions/runs/40806487902
**Status:** bounded pre-ExitBootServices, no CR3 write, no real kernel boot.

## Actual implementation

The preflight UEFI application validates its current firmware CR3 root
physical ownership under the EFI memory map, then emits a unique COM2
`UNI2I` barrier **before its first old-root physical-as-linear
dereference**. The external host pauses QEMU vCPU0, queries CR3 from
QMP independently, and verifies root identity using `gva2gpa` and
physical/virtual `xp`/`x` data.

E2i then independently walks the **entire PML4 page and all reachable
PDPT pages**. For each visited 4KiB table frame, the host demands:

- same physical and virtual address under QEMU's actual guest state;
- all **512** consecutive 64-bit words from the physical dump;
- all **512** consecutive words from the virtual dump;
- byte-for-byte equality of physical and virtual representations;
- 4KiB alignment and a bounded, nonzero physical frame address;
- no silent skipping, truncation, or exceeding a bounded table-frame cap.

The PML4/PDPT walk is breadth bounded and stops at PDPT: **PD and PT
are unverified**. Any present pointer into the unverified lower levels
is accounted for in the witness as a deferred link rather than incorrectly
reported as proved. The general pure host walker also has a synthetic
4-level test path, but this **does not** imply a full live 4-level audit.

An independent pause/ack handshake sends the distinct byte `I` before
continuing the VM. The observed lab status is **85** (debug byte
`0x2a`), completely distinct from true full boot exit **65**.
No writable user disk, network or real PC is involved.

## Verified code and evidence

| Gate | Result |
|---|---|
| Python host oracle and witness negative tests | **46/46** |
| Rust crate unit tests (bootmap, bridge, CPU, descriptors, ELF, paging, rootgate, staging) | **97/97** |
| **Combined tests** | **143/143** |
| Independently parsed synthetic PTE / GDT / root ownership fixtures | **PASS** |
| Actual QEMU paused vCPU root + upper tables QMP audit | **PASS** |
| Actual QEMU table frames fully compared | **3 pages, 1,536 qwords** |
| Actual QEMU host preflight exit | **85** |
| `ExitBootServices` / `mov cr3` / loaded IDT / freestanding kernel entry | **NOT RUN** |

- EFI PE32+ SHA256:
  `e6f35f1239ec61f31ec7b434e4845585a437bfd77b835b457cef77da42126623`
- All probes are tied to the disposable LAB-H1 pinned QEMU
  (`qemu64,+nx,-la57,-pcid`, 1 vCPU, 256MiB, OVMF).
- Failed earlier E2i trials are retained in GitHub Actions. They first
  exposed a bare-hex HMP address format, then a **256-page proof bound**
  on an attempted full walk. Neither failed trial was counted as success.
  The final verified E2i deliberately narrows the scope to PML4/PDPT.

## Unclosed high-priority gates

1. Lower-level PD/PT table frames and terminal leaf provenance remain
   outside the independent paused-QEMU witness. Guest firmware page-table
   reads below PDPT still rely on the lab-specific assumptions.
2. Paused-QEMU evidence is **not** a portable firmware guarantee, a
   runtime temporal immutability guarantee, or a production hardware path.
3. A real bounded CR3 transition stub and associated RIP/RSP continuation
   proof is not implemented.
4. Emergency GDT/IDT/TSS are dormant, not installed or dynamically
   fault-injected; #DF IST runtime still untested.
5. Immutable BootContext, final GetMemoryMap/ExitBootServices choreography,
   post-cut firmware-call prohibition, new CR3 high-half SysV kernel
   entry and independent serial/host exit **65** remain unverified.

**Operator QEMU kernel test stage: NO-GO. Physical machine: NO-GO.**
Only L1 implementation approved; no L2 or above.

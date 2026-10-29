# LAB-H1 E2o — two independent paused-QMP witnesses for old CR3 and uninstalled private PML4

**Date:** 2026-10-30 (NZDT)
**Validated executable/source SHA:** `a697366688a08587f77e5cab72ecdfb3aee10848`
**Real QEMU/OVMF and GitHub Actions:** https://github.com/White-Fantail/unios/actions/runs/40935673773
**Scope:** approved L1 Experimental Kernel only, pinned disposable VM, not a true kernel boot.

## Changed implementation

1. Continue existing E2j **first host-only checkpoint** BEFORE any physical firmware-CR3 root dereference. Guest writes a bounded EFI allocation ledger to COM2, then waits for **J**. QEMU stops vCPU0 and independently reads old CR3, physical/virtual root bytes, full upper old table frames, and boundary old-map leaves.
2. Guest then stages the ELF, constructs an **uninstalled** high-half private PML4, verifies the reserved pages against the old firmware root, and independently audits the raw new PTE table from guest memory (E2n).
3. E2o introduces an entirely **separate second unique checkpoint** `\x7fUNI2O\x7f`. Guest writes the claimed new root physical address, bounded used table count, and exactly 19 target VA/PA/permissions records. Only the separately verified reserved private table pool is allowed as the new-root source. The guest blocks until the second independent host **O** authorization.
4. While guest waits, QEMU host pauses the vCPU again, requires CPU CR3 to **still equal the firmware old CR3**, and independently reads **every used 4KiB private page-table frame** via QMP physical reads corroborated against existing old-CR3 identity aliases. This is actual RAM evidence, **not just guest claims or Builder::lookup**.
5. The new Python host walker independently decodes all four page-table levels, rejects unexpected large pages, missing tables, out-of-reservation child pointers, any user bit, writable+executable leaves, NX executable paths, wrong physical page or rights, and incorrectly mapped guards. All used private frames and every listed target are checked, not merely the PML4's first few entries.
6. The 19 targets are exact **1 staged high-half ELF entry**, **4 low EFI executable points** (bridge begin/end-1 and two emergency handlers), **2 identity bridge-stack pages**, **4 high-half main-stack pages**, **4 high-half IST1 pages**, **1 RW descriptor**, **1 RO BootContext**, and **2 strictly absent high-half guard pages**.
7. Root address is required to equal the EFI reservation ID2 base. Other target claims are bound to EFI reservation IDs 1/3/4/5/6/7, fixed high-half virtual addresses and code old-root supervisor-executable checks. New CR3 remains dormant, and the guest cannot proceed without the QEMU host granting **O**.
8. Native QEMU exception regressions run only after both barriers: genuine #UD, software INT8 IST1, and genuine CPU-generated #DF. They retain their distinct nonboot exits. Each run stores its own new-PML4 proof JSON; the standard preflight JSON and executable are restored, compared byte-identically, and disposable virtual disks containing fatal variants are deleted.
9. The host writes separate **provisional old-root** and **full successful two-root** evidence files. A complete new-private proof is only published after the second QMP proof succeeds. The full QEMU failure exit and independent negative tests reject spoofed/invalid manifests and PTE corruption.

## Real independent evidence

| Gate | Observed |
|---|---|
| Python host/native QMP/oracle and negative controls | **131 / 131 passed** |
| Rust no_std library, descriptor and paging unit tests | **121 / 121 passed** |
| Total host and unit tests | **252 / 252 passed** |
| Actual paused-QMP old firmware root | `0xf801000` |
| QMP old reserved boundary samples | **14**, 2MiB leaves |
| Old upper table pages fully compared | **3** |
| Actual independently QMP-verified private root (standard E2o run) | `0xdf5e000` |
| **ALL** used new private table pages read from actual QEMU memory | **8** |
| New root targeted VA/PA/rights/guard records | **19**, including **2 unmapped guards** |
| Standard QEMU E2o two-host checkpoint success | **exit 97** |
| CPU #UD regression after private-host O authorization | **exit 103** |
| Software vector8 IST1 regression | **exit 105** (not a real #DF) |
| Genuine nested CPU #GP -> #DF / IST1 regression | **exit 109** |
| Native QEMU source-of-#DF trace | **1 nested GP and 1 DF error=0**, PASS |
| Corrupted/new-root host synthetic negative tests | **REJECTED** |
| Standard EFI restored byte-identical after fatal variants | **PASS** |
| `ExitBootServices`, new CR3 write, high-half kernel entry | **NOT RUN** |
| True clean kernel boot oracle exit 65 | **NOT RUN** |

Normal standard preflight PE32+ SHA256:
`195228d1a5e39c63ba1ca03977bd4f2fffd5b9e2e7cfa76e14d36823d88d043e`.

## Explicit evidence limits

- This is **the first independent host observation of the guest-built private PML4 bytes**. It confirms these bytes at the second paused point **while still executing on old CR3**. It does **not** prove processor use of the new page tables or the real CR3-switch instruction/stack transition.
- EFI loader executable ownership in the guest is checked through UEFI descriptors, and the host verifies the corresponding **old-root** PTE executable/supervisor status and exact new PTE physical mapping. Host does not independently authenticate every UEFI EFI_LOADER_CODE descriptor byte.
- The code anchors include a bounded E2e *canary stub*, **not an executed CR3 write trampoline**. Only selected code addresses are compared with the private PML4; arbitrary EFI code after ExitBS will not necessarily be mapped.
- The QEMU/OVMF proof is specific to the pinned single-vCPU lab machine and 4-level page tables. It is not a portable guarantee for real computers, LA57, PCID, firmware variability, or multiprocessor behavior.
- Future work **E2p**: plan a bounded, auditable final memory-map-key/ExitBootServices transition; no post-cut firmware calls; exact RIP/RSP/IST and code-span handoff proof; real new-CR3 activation only after a separate safety-reviewed QEMU failure matrix.
- The operator's first real QEMU kernel boot test remains **NO-GO** until independently witnessed high-half kernel entry and clean exit **65**.

**User-run real QEMU kernel BOOT tests: NO-GO. Physical PC: NO-GO. No L2+ changes.**

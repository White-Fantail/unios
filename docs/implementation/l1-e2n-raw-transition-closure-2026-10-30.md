# LAB-H1 E2n — independent raw PTE transition-cut closure (CR3 remains DORMANT)

**Date:** 2026-10-30 NZDT
**Last executable code SHA:** `6619275ba775ad68be3388b4017b8494c7082126`
**Actual QEMU and CI run:** https://github.com/White-Fantail/unios/actions/runs/40933567132
**Scope:** approved L1 Experimental Kernel; disposable QEMU/OVMF preflight, **NOT live boot or CR3 transition**.

## Implemented

- After EFI allocation, memory-map ownership checks and E2j paused-QEMU
  old-CR3 witness, a **new private four-level PML4 is built but not loaded**.
- E2n introduces a **separate, immutable raw-PTE walker**
  (`crates/paging/src/transition_closure.rs`) that intentionally
  does NOT reuse the builder's original `lookup` or `verify` path.
  It decodes actual 512-word frames and verifies strict four-level
  4KiB mappings: supervisor only, nonlarge pages, bounded descendant
  table frame offsets, RW implies NX, and exact expected physical
  address / leaf rights.
- The witness covers the actual staged kernel ELF's **entry RIP**
  (`RX`), all boundary addresses of the E2e linker-defined bridge
  canary span, generic and DF fail-stop EFI code addresses
  (four executable anchor samples), and 13 precisely enumerated
  data pages: **2 physical identity-mapped bridge stack pages**,
  **4 high-half main stack pages**, **4 high-half DF IST1 pages**,
  **1 RW high-half descriptor page**, **1 RO high-half BootContext
  mapping**, and **1 executable high-half kernel entry page**.
- It checks the last byte of **each** stack page as well as the first
  byte, the exact top of all three stacks, and that both high-half
  guard virtual pages are **unmapped**. Code anchors are separately
  re-audited for old-firmware-CR3 RX, EFI_LOADER_CODE owner and
  same-physical new-mapping RX. An unknown or altered mapping fails
  closed, never becoming the success exit.
- Dedicated independent unit negative controls corrupt leaf rights,
  code physical address, kernel entry, stack physical address,
  missing intermediate stack page, page-table parent pointer,
  supervisor bit, huge-page link, descriptor presence, and
  noncanonical entry; all must be rejected.
- The E2n audit happens **before** the opt-in E2l/E2m terminal
  QEMU hardware exception probes. Regression cases continue to
  cover genuine CPU #UD, software INT8 IST1, and genuine CPU
  nested-GP double fault using native `-d int` origin verification.
- Standard preflight signals debug byte `0x2f`, QEMU host **95**.
  This is NOT true clean kernel exit **65**.
- Fault variants are disposable; standard preflight EFI is rebuilt,
  byte-compared and restored, and VM guest disks discarded.

## Verified CI/QEMU evidence

| Gate | Actual |
|---|---|
| Python oracle/host tests | **109/109 passed** |
| Rust no_std support/descriptor/paging tests | **121/121 passed** |
| Total unit/host tests | **230/230 passed** |
| E2n guest independent raw private-PTE closure | **PASS**, standard QEMU host exit **95** |
| Real CPU #UD regression | host exit **103** |
| Software INT8 IST1 regression | host exit **105** (NOT real #DF) |
| CPU generated real #DF regression | host exit **109**, native QEMU nested #GP trace |
| Fatal-variant EFI image teardown and default image restoration | **PASS** |
| E2j independent QEMU old CR3 root + reservation boundaries | **14 VA, 3 table frames, all 2MiB leaves** |
| New CR3 actually loaded in CPU | **NEVER** |
| ExitBootServices, high-half stack switch, kernel entry | **NEVER** |

The host tool label and JSON now explicitly distinguish these two
independent—but **different**—witnesses:

1. **Host QMP old CR3:** guest paused before any old-CR3 root table
   dereference; independent hardware CR3 and selected EFI
   reservation boundary table comparisons. This is external
   QEMU-host corroboration.
2. **Guest raw PTE private table:** read-only verification of the
   guest-built, currently **uninstalled** new private table. The
   host infers that this gate ran from the distinct guest exit code;
   the host did **NOT** independently dump or validate the new
   private CR3's physical PTE pages. Do not describe it as such.

## Blockers and next E2o work

1. **E2o:** add a second bounded paused-QMP witness **after** the
   guest has built its private table to independently compare the
   **new PML4** and carefully selected old/new mapping paths.
   The current E2n host only observes the old CR3.
2. The four bridge/handler anchors are linker-defined code
   canaries, not a proven active CR3-switch instruction span.
   No executed bridge RIP-after-write or RSP landing has been
   verified. The low physical bridge stack exists for a future
   transition, not an already switched stack.
3. The high-half GDT/IDT/TSS image is built but dormant.
   E2l/E2m's active physical descriptor is a disposable, terminal
   **old-CR3** QEMU fault test, not a post-CR3 production handler.
4. Final GetMemoryMap -> ExitBootServices with bounded retry and
   no post-cut firmware calls, genuine CR3 transition, high-half
   freestanding kernel entry, complete serial stages, and
   independent clean exit **65** have not been executed.

**Operator real QEMU kernel BOOT stage: NO-GO.**
**Physical machine test: NO-GO.** No L2+ changes.

# LAB-H1 E2l — QEMU-only active IDT #UD and IST1 software-vector smoke

**Date:** 2026-10-30 (NZDT)
**Tested executable source SHA:** `0b469e2dec739f722e340190e72f306b0a4b218b`
**Full actual CI / QEMU:** https://github.com/White-Fantail/unios/actions/runs/40923788959
**Scope:** approved L1 Experimental Kernel, disposable QEMU/OVMF only.
**NOT an exited-firmware kernel or physical-hardware proof.**

## Purpose and method

E2k proved the two terminal emergency handler machine-code instruction
sequences by inspecting linked PE bytes and by direct CALLs. E2l is the
first **CPU-delivered #UD exception** and **software-vector IST1 stack
switch** laboratory exercise. The stage intentionally does not call
ExitBootServices or load the new private PML4.

The E2f descriptor image at `DESCRIPTORS_VA` is high-half and **not
accessible under the old firmware CR3**. E2l therefore rebuilds a
separate **terminal lab-only** GDT/IDT/TSS at the **EFI-allocated and
independently old-CR3-identity-verified physical address** of the
metadata page, and uses physical RSP0 and IST1 top addresses. The
approved high-half private page tables continue to be built/audited
but remain **dormant and never activated**.

The new no_std descriptor-builder contract retains the reviewed
32-vector E2f IDT except for a dedicated gate for vector 6, with
negative tests for its selector/target integrity. Vector 8 uses
IST1 in the actual TSS.

The QEMU-only opt-in assembly sequence uses `cli; lgdt; lidt; ltr`,
then **`ud2`**, causing a real CPU-delivered invalid-opcode #UD
(vector 6) through the active IDT. The special terminal #UD handler
writes debug exit byte `0x33` (QEMU host **103**); a mistaken
generic exception would instead exit with a distinct error code.
The other opt-in sequence sets `r10=IST_lower` and `r11=IST_upper`
then executes `int 8`. The vector 8 gate specifies IST1; the
terminal handler compares the hardware-loaded RSP against those
bounds before outputting `0x34` (host **105**). An out-of-range
stack would output `0x35` (host **107**), which is rejected.

**Important:** `int 8` is a *software interrupt targeting vector 8*,
not an authentic nested CPU-generated double-fault #DF. The test
supports IST1 delivery and RSP-window checks, but does **not** prove
the CPU's double-fault generation or exception classification logic.
The #UD test does exercise a genuine CPU-generated fault.

The tested LAB-H1 QEMU profile remains single-vCPU x86_64 q35,
OVMF, TCG, NX, no LA57/PCID, 256MiB, no network, disposable
GPT/FAT and pflash; no real device or personal disk.

## Real verification

| Gate | Observed |
|---|---|
| Python oracle, QMP and negative controls | **85/85 PASS** |
| Rust no_std-support unit tests, including vector 6 override | **106/106 PASS** |
| Total host + Rust unit tests | **191/191 PASS** |
| EFI PE32+ linked emergency handler audit | **PASS** |
| Corrupt PE HLT negative control | **REJECTED** |
| QEMU new physical descriptor preflight, no actual install | **exit 91** |
| QEMU after physical LGDT/LIDT/LTR then genuine UD2 fault | **exit 103** |
| QEMU after physical LGDT/LIDT/LTR then software INT8 IST1 | **exit 105** |
| Test-only fatal images segregated and default preflight image restored | **PASS** |
| Existing E2j paused-QEMU root + reservation boundary evidence | **14 boundary pages**, 3 table pages, 2MiB leaves |
| True CPU-generated #DF, active new PML4, ExitBootServices, freestanding kernel | **NOT RUN** |

The fail-stop handlers intentionally do not IRETQ or return to the
UEFI application. The normal build does **not** issue LGDT/LIDT/LTR.
The smoke variants are available only with explicit compile-time
features and intentionally terminate the disposable VM; they are not
safe for production hardware.

## Next blockers and operator status

1. **True #DF fault generation and IST1 runtime provenance** still
   need controlled, QEMU-only negative tests; software INT8 is not
   equivalent to a CPU-generated double fault.
2. The active lab descriptor image is at low physical addresses
   under **old CR3**. The actual production transition requires a
   separately audited high-half descriptor installation and RIP/RSP
   handoff after a verified new CR3 switch.
3. The E2j independent host witness samples only reservation
   **boundary** frames, and QEMU used 2MiB pages; this is not a
   portable all-page firmware guarantee. Other EFI runtime
   executable code/provenance and intermediate translations are not
   yet proven for a full transition.
4. Final GetMemoryMap/ExitBootServices with bounded retries and
   no post-cut Boot Services, immutable BootContext, real CR3-switch
   bridge and freestanding high-half kernel entry have not occurred.
5. The independent clean kernel boot success marker/exit **65**
   and complete accepted L1 matrix are **not verified**.

**Real operator QEMU kernel boot stage: NO-GO.**
**Physical-machine test: NO-GO. No L2 or higher changes.**

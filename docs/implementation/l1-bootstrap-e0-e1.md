# UniOS L1 implementation log — LAB-H1 E0/E1 (experimental)

Date: 2026-10-29 NZDT. Branch: `experiment/l1-lab-h1-bootstrap`.
Authority: scoped L1 Go; NOT permission to implement L2.

## Exactly what exists

- `no_std` standalone ELF64 `x86_64-unknown-none` Rust kernel with a high-half
  linker script and non-returning SysV entry. It emits only `KERNEL_ENTRY_UNVERIFIED`
  if invoked; **there is deliberately no `HALTED_PASS` path** in this first stage.
- `no_std` UEFI `x86_64-unknown-uefi` Rust loader with `efiapi` entry and an embedded
  separately built kernel ELF. It checks ELF before any memory mapping/staging and
  returns `EFI_UNSUPPORTED` on valid preflight; no claimed boot.
- `unios_elf`: independent bounded parser for <=8 PT_LOAD, <=16 MiB staged
  pages, 64 MiB fixed VA envelope, ET_EXEC/EM_X86_64, no remaining relocation
  sections, no dynamic program headers, no shared pages and no W+X.
- Host-side ELF audit and simulated terminal-oracle negative controls.
- Rust 1.90.0 build recipe and GitHub Actions artifact checks.

## Current status: do not predeclare any E0–E5 gate as green

| Gate | Evidence state |
|---|---|
| E0 | **PARTIAL** — experimental branch, compiler pin, oracle policy; actual runner/firmware/VM hashes required |
| E1 | **ARTIFACT BUILT / PARTIALLY VERIFIED** — ET_EXEC, PE32+, no relocation and parser audit passed in GitHub Actions; full staging/ownership checks remain |
| E2 | **NOT IMPLEMENTED / NOT RUN** — GetMemoryMap and ExitBootServices absent |
| E3 | **NOT IMPLEMENTED / NOT RUN** — CR3, bridge, guard, GDT/TSS/IST absent |
| E4 | **NOT IMPLEMENTED / NOT RUN** — QEMU fault probes / success oracle absent |
| E5 | **NOT RUN** — 59 DevUnsigned cases outstanding; L1-X14 DEFERRED |

## Explicit stop boundaries

The initial E1 artifact validation passed in GitHub Actions run 40737489885.\nQEMU/OVMF E1 preflight smoke also passed at source commit d3fe104c24ad45175ee5f94fed95ba6f8c543076 (Actions run 40738064621, host exit 67). This is NOT an E2+ handoff.\n\nNo kernel entry from EFI before proven old/new same-linear bridge RIP/stack/
descriptors after final ExitBootServices. Do not assume identity paging.
All memory/stacks/table capacities stay at approved LAB-H1 limits. Never
award CLEAN_PASS based on text or a simulated oracle. No main merge
before engineering review. No userspace, capability/IPC, pager, scheduler,
networking or application compatibility layers.

## Reproduce E1 locally with pinned toolchain and binutils

```sh
bash scripts/build-l1.sh
```

Inspect out/kernel-elf-{header,programs,relocs,audit}.txt,
out/uefi-pe-header.txt, out/image-digests.txt and the GitHub Actions log.
There has NOT yet been an actual L1 QEMU boot or memory-protection proof.

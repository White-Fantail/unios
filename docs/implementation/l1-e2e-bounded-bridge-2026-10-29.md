# UniOS LAB-H1 — E2e bounded executable stub provenance preflight

Date: 2026-10-29 NZDT. Scope: approved L1 Experimental Kernel only.
Code tested: `4680471e3cc39b30fa6d08b71e6ca58fe1e9fda7`.
Real CI/QEMU: https://github.com/White-Fantail/unios/actions/runs/40766272049

## Actual implementation

- An explicitly linker-labelled UEFI x86_64 **four-byte assembly probe**
  `cli; cld; nop; ret` has exact start/end symbols. It is deliberately
  **not executed** and contains **no CR3 write**, stack switch or far jump.
- `unios_bridgecheck` rejects zero/reversed/out-of-range or >1 KiB spans,
  >2 page spans, page reorder, changed physical frames, old/new no-execute
  mismatches, user-accessible pages, non-EfiLoaderCode ownership, duplicate
  physical code backing, and disabled CR0.WP / EFER.NXE.
- The live OVMF loader obtains the emitted symbol bounds, audits each
  containing page under the **current firmware CR3**, demands both
  start/end-of-page translation, effective executable/supervisor permissions,
  and a matching `EfiLoaderCode` memory descriptor.
- It inserts these observed physical code frames as supervisor RX at
  their **same linear address** into the new private PML4 without creating
  writable aliases; it verifies the resulting leaves.
- It reads back and checks the **exact four machine-code bytes** only
  after these preflight address checks. This is an executable range canary,
  not a real bridge function or instruction-level live-switch proof.
- The CPU-state probe confirms current WP and NXE flags are set before the
  preflight accepts success. No register is changed or set on this path.
- The previous BSS, owned allocation, ELF, page-table and host-negative
  checks remain mandatory; no raw UEFI calls after ExitBootServices
  can occur because ExitBootServices is never called in this experiment.

## Reproducible evidence

| Gate | Actual result |
|---|---|
| Python host oracle negative controls | **14/14** |
| `unios_bootmap` | **11/11** |
| `unios_bridgecheck` | **16/16** |
| `unios_cpugate` | **9/9** |
| `unios_elf` | **9/9** |
| `unios_paging` | **13/13** |
| `unios_staging` | **8/8** |
| **Total unit/host tests** | **80/80** |
| Actual kernel ELF | 3 PT_LOADs; 12 KiB staged; 4 KiB BSS |
| Independent host PTE fixture audit | 8 tables/15 leaf mappings; 2 guards absent |
| Actual QEMU 8.2.2 + OVMF UEFI E2e | **host exit 77** |
| ExitBootServices / new CR3 write / SysV kernel entry | **NOT ATTEMPTED** |

Note the independent Python PTE audit consumes a **synthetic host-produced
fixture**, not a dump of the real firmware's page table. The QEMU status
77 is a preflight-only byte `0x26`, not clean kernel success byte
`0x20` / host exit 65. A timeout, reset or alternate status is not
a passing outcome.

SHA256 provenance recorded in run:
- kernel ELF: `8f008488fbca78baaba084aa70fe2168abf79c7922c98bd7edfe42b8e5dd80f5`
- UEFI PE32+: `feba57254efbbcaf21da57d95e533e64ea5040d65180a538703cc683b302c5b7`
- QEMU binary: `8a35ccba41582fc6c38b9df85fc9e35fa1d42f414d2d7d8090ee9b2f5e7c0854`
- OVMF CODE: `ad23261d43116abc3747d59a7837fbf578c9586a8993f4eeba064cd07eccb083`
- OVMF VARS template: `5d2ac383371b408398accee7ec27c8c09ea5b74a0de0ceea6513388b15be5d1e`

## P0 blockers before actual bridge / user QEMU kernel test

1. **Old firmware CR3 root bootstrap** is still probed through an uncorroborated
   physical-address-equals-linear-address assumption. The later per-page walk
   is fail-closed for mismatches, but is not independently secured at its root.
2. The four-byte `cli; cld; nop; ret` stub is **not the approved transition
   trampoline**. Actual contiguous CR3-switch instructions, stack switch,
   controlled post-cut RIP, SysV entry and return-impossibility proof are absent.
3. GDT/IDT/TSS/#DF IST, emergency exception stubs and guard fault tests have
   not been installed or run. A raw CR3 switch now would risk a triple fault.
4. Final memory map and MapKey / bounded ExitBootServices retry, immutable
   BootContext content, and post-exit firmware-call prohibition remain pending.
5. CPU features were checked **before** a hypothetical cut, but must be
   rechecked/maintained at the actual switch.
6. Full L1 gates and all applicable independent traceability cases are not
   passed. L2+ remains outside approved scope.

**Gate status:** E2e **pre-ExitBS preflight observed**; E2 overall partial;
E3/E4/E5 not demonstrated. User-facing “가상환경 테스트 단계 진입”
remains **NO-GO** until a *real* new-CR3 kernel entry and independent
host exit 65 + correct serial stages have been verified. Physical machine
testing is even later and requires separate authorization.

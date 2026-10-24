# UniOS

**UniOS** is an independent operating system project currently in the **architecture design and feasibility research** stage. Its long-term goal is a general-purpose desktop OS, starting with a limited developer-focused environment.

## Vision

Design and evaluate an operating system with its **own kernel, native ABI, system services, and desktop experience**—not a Linux distribution or a fork of an existing operating system.

Interoperability with applications originally built for **Linux, Windows, and macOS/Darwin** is a staged research goal, **not an implemented or guaranteed feature**.

## Architecture direction (accepted at design level)

Eight high-level project decisions were accepted on 2026-10-24:

| Area | Direction |
|---|---|
| Product | General-purpose desktop ambition; developer-first delivery |
| Initial hardware | x86_64 UEFI; QEMU/OVMF/VirtIO first, with early validation on a selected physical PC |
| Application compatibility | Native SDK first, POSIX source ports and selected Linux CLI binaries next; investigate Wine/Windows in parallel |
| Language policy | Rust-first hybrid; carefully bounded Assembly, unsafe Rust, and C/C++ interoperability |
| Kernel | Practical capability microkernel, with narrowly reviewed hybrid exceptions if supported by evidence |
| Licensing | Permissive core; conditional, compliant copyleft use in user space |
| Security | Design for eventual general-user threats from the start; verify and deliver protections in stages |
| Darwin/macOS | Prioritize limited Mach-O/Darwin CLI research; GUI/AppKit/Metal are long-term, high-risk research topics |

These are **accepted design directions**, **not validated implementations**. Kernel interfaces, driver support, compatibility behavior, delivery milestones, and performance/security claims require further specifications and evidence.

## Research priorities

- **Kernel and TCB:** objects and capabilities, threads/scheduling, virtual memory, IPC, privilege boundaries, recovery.
- **Native developer platform:** language-neutral C ABI, SDK, libc/POSIX source ports, Rust runtime/toolchain feasibility.
- **Hardware:** reference virtual machines and early reference-PC boot/driver/DMA/IOMMU validation.
- **Compatibility:** Linux syscall-entry semantics and user-space personality, Wine port dependencies, Darwin/Mach-O feasibility.
- **Desktop:** basic terminal, files, input, software 2D compositor, and measured interactive responsiveness.
- **Trust and release:** sandboxing, signed boot/updates, user data protection, licensing, and supply-chain review.

## Current status

**Design and feasibility research only.** No UniOS kernel, device drivers, application runtime, or compatibility layer has been implemented. No Windows, Linux, or macOS applications are currently verified to run on UniOS.

There is no approved implementation schedule or supported-hardware catalog. Progression from architecture work to implementation requires a separate go/no-go decision.

## Architecture documents

- **[Architecture & Research Design v0.2 — Official Draft](docs/architecture/unios-architecture-v0.2.md)** — current design baseline; eight accepted directions, system boundaries, unresolved risks, normalized ADR index, RFC research registry, security and validation gates, and research roadmap.
- **[RFC-001 Kernel Boundary & Trusted Computing Base v0.2 — Proposed Specification](docs/rfcs/rfc-001-kernel-boundary-and-tcb.md)** — independently reviewed kernel/user-space responsibility split, asset-specific TCB, Linux guest execution alternatives, DMA/IOMMU group boundaries, and verification gates; not yet accepted or implemented.
- **[RFC-002 Object, Capability, Lifetime, Delegation & Revocation v0.2 — Proposed Specification](docs/rfcs/rfc-002-object-capability-lifetime-delegation-revocation.md)** — critically reviewed capability model covering grant lineage anchors, IPC commit/result ambiguity, scope attenuation, resource accounting, and separate family-quiescence vs object-reclamation guarantees; not accepted or implemented.
- **[RFC-003 IPC Wire Protocol & Synchronization v0.1 — Proposed Specification](docs/rfcs/rfc-003-ipc-wire-protocol-and-synchronization.md)** — bounded Channel, one-shot ReplyToken, Event/WaitSet, kernel/service wire separation, transactional handle MOVE, commit/result uncertainty, timeouts, backpressure, recovery, threat invariants and research gates; not accepted or implemented.
- [Five-Kernel Comparison & Feasibility v0.1 — Historical Research Draft](docs/architecture/kernel-architecture-and-feasibility-v0.1.md) — Linux, Windows NT, Apple XNU, FreeBSD, and Fuchsia Zircon comparison and earlier proposed designs. Preserved as research history; its “all decisions pending” status has been superseded by v0.2.

## Guiding principles

1. Preserve a genuinely independent kernel and native architecture.
2. Use public specifications and legally compatible third-party components without assuming they provide automatic binary compatibility.
3. Make security boundaries, failure semantics, and user-data integrity explicit before promising production use.
4. Report compatibility **per tested application, version, CPU architecture, runtime, hardware, and feature set**.
5. Separate **accepted direction**, **proposed specification**, **validated evidence**, and **implemented software**.
6. Keep architecture decisions traceable and revisit them only with documented technical or product evidence.

## Repository

This repository is currently for **research and design documentation**. Implementation must be authorized separately.

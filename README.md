# UniOS

**UniOS** is an early-stage research and design project exploring a new, independent operating system.

## Vision

Design an operating system with its own kernel and system architecture—not a Linux distribution or a fork of an existing operating system—while investigating ways to run applications originally built for **Linux, Windows, and macOS**.

Cross-platform application compatibility is a research goal, **not an implemented or guaranteed feature**.

## Initial research areas

- **Kernel and architecture:** boot process, memory management, processes, scheduling, filesystems, drivers, and security.
- **Application compatibility:** executable formats, system calls, APIs/ABIs, graphics, and possible compatibility or virtualization approaches for Linux, Windows, and macOS software.
- **Hardware support:** feasibility of starting with a limited hardware target before expanding device and driver support.
- **User experience:** a coherent desktop environment and developer platform independent of existing OS designs.
- **Feasibility and licensing:** performance, maintenance effort, security boundaries, and legal constraints around third-party technologies.

## Current status

**Design and feasibility research only.** No kernel, compatibility layer, or application runtime has been implemented. Architectural choices, target hardware, programming languages, and delivery milestones remain open for evaluation.

## Guiding principles

1. Prioritize a genuinely independent architecture over rebranding an existing OS.
2. Study proven ideas from Linux, Windows, macOS, and other systems without assuming their designs must be copied.
3. Validate compatibility claims through technical research and experiments before making promises.
4. Make incremental, evidence-based design decisions, with clear attention to maintainability, security, and licensing.

## Repository

This repository currently contains project documentation. Design notes, architecture proposals, and feasibility findings will be added as decisions are made.

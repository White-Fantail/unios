# UniOS RFC-010 — Linux Personality & Binary Compatibility

> **Status:** Proposed — Specification v0.1 (design/research ONLY; NOT Accepted, implemented, Linux-compatible, performance-validated or formally verified)
> **Date:** 2026-10-24 NZDT
> **Architecture:** [Architecture v0.2](../architecture/unios-architecture-v0.2.md) D3 Native SDK → POSIX source ports → limited Linux x86_64 CLI binary, an accepted *direction*, not demonstrated compatibility.
> **Prior RFCs:** [001 Kernel](rfc-001-kernel-boundary-and-tcb.md), [002 Capabilities](rfc-002-object-capability-lifetime-delegation-revocation.md), [003 IPC](rfc-003-ipc-wire-protocol-and-synchronization.md), [004 VM/Pager](rfc-004-virtual-memory-pager-cache-and-oom.md), [005 Scheduler](rfc-005-scheduling-preemption-and-cpu-budget.md), [006 Boot](rfc-006-boot-trust-chain-service-startup-and-recovery.md), [007 Device/DMA](rfc-007-user-space-drivers-dma-and-iommu.md), [008 Storage](rfc-008-storage-filesystems-and-crash-durability.md), [009 Native ABI v0.2](rfc-009-native-abi-and-system-call-contract.md), [017 Trust](rfc-017-verified-boot-updates-and-rollback-protection.md).
> **Strict restriction:** No ELF loader, libc, syscall interception, process runtime, native/guest code, repository source files, QEMU tests, Linux application execution, firmware, hardware or implementation is authorized by this proposed design.

## 0. Executive recommendation — LINUX-A conditional research only

**LINUX-A:** a restricted x86_64 Linux ELF GuestDomain in a kernel-tagged Task/AddressSpace, with bounded kernel trap capture and a separately protected privileged LinuxPersonality broker. All guest calls for File/Directory/Memory/Network/Time are mapped to attenuated native capabilities via versioned SABI. Native kernel ABI is independent of Linux syscall numbers, errno and POSIX namespaces. **Per-Thread native/guest label inside one shared Task is not enough to isolate a trusted broker**, as established in RFC-009 v0.2. Without a proved kernel/hardware trap forwarding scheme, a userspace personality alone cannot intercept unmodified Linux SYSCALL instructions.

**State exact scope:** POSIX source port means rebuild with native SDK; *binary compatibility* means execution of unchanged ELF and compatible Linux userspace runtime, with an explicit tested semantic coverage matrix; full Linux guest OS in a VM means another kernel, not the same project milestone. None is working in UniOS.

**Alternative study families:**
- **LINUX-A:** kernel-owned restricted GuestDomain, minimal trap forwarding, separate protected native broker; leading hypothesis, not accepted.
- **LINUX-B:** userspace CPU emulator or translator with Linux syscall emulation; fewer native entry concerns but instruction, signal, JIT and performance challenges.
- **LINUX-C:** implement Linux system calls directly in kernel; conflicts with default Ring-0 TCB boundary and broadens audit surface.
- **LINUX-D:** virtualize the entire Linux OS; separate hardware/hypervisor, memory, virtual device and license path, not native binary compatibility.
- **LINUX-E:** source-port POSIX apps only; most practical initial native route, but not unchanged binary support.

**P0 gates:** guest trap identity and native authority isolation; ELF and interpreter/auxv/TLS/vDSO; shared-memory futex atomic compare-and-block; Linux thread/process/exec/signal; FD/open-file-description path and namespace; file durability/effect ambiguity; kernel and broker CPU/Pager budget; unknown syscall/feature honest refusal and legal runtime sourcing.

## 1. Research goals, scope and adversaries

Goals: limited but repeatably measured Linux x86_64 CLI binaries, original files unchanged; explicit static/dynamic loader levels; bounded host grants and guest UID0 containment; precise ABI register layouts and Linux errno mappings; defined guest PID/TID/FD tables, exception/signal and thread behavior; staged VFS/Network/Time; no direct host kernel/TPM/IOMMU/boot authority; separate failure/restart and resource accounting.

**Not goals:** unmodified general Linux distro, systemd, Docker/container engine, privileged Linux namespaces, Linux kernel modules/BPF/ptrace, broad hardware DRM/GPU/Wi-Fi, Android apps, every glibc distribution, all POSIX signals/threads, Linux x32/i386/arm64, or production compatibility certification. Unsupported app requirements are reported rather than returning forged success.

Adversaries: malicious ELF directly calls native privilege opcode, forges GuestDomain in TLS, jumps to same-process broker, writes protected loader address, races guest pointer/signal frame with broker copyin, guesses PID/TID/FD, uses old FD to reach new FileCap, drives futex lost-wakeup, namespace root escape via symlink/magiclink, spoofs guest root/setuid, sends malformed net cmsg/SCM_RIGHTS, exhausts Thread/IPC budgets, kills broker after an already committed filesystem rename or network send, or abuses pagefault recursion to starve M0. Firmware/SMM, pre-IOMMU DMA and dishonest disk are stronger hardware assumptions (RFC-006/007/008/017), not fixed by Linux syscall decoding.

**Security precedence:** kernel/native Cap boundary and user data isolation > effect/error truth > consistency/forward progress > source/binary compatibility > performance. Calling a guest API does not mint authority.

## 2. Source-based feasibility comparison

| Source | Published behavior | UniOS constraint |
|---|---|---|
| Linux syscall(2) [S1] | x86_64 RAX syscall number, args RDI/RSI/RDX/R10/R8/R9 | guest machine ABI separate from native KABI and C ABI |
| Linux execve(2) [S2] | replaces address space and reinitializes certain process attributes | exec is transactional process-image replacement, not only new Task |
| Linux ld.so(8) [S3] | PT_INTERP dynamic loader, DT_NEEDED, RUNPATH/RPATH and secure env policy | selected guest glibc/musl+versioned symbols and library tree needed |
| Linux getauxval(3) [S4] | aux vector communicates runtime/platform facts | initial guest stack and real HWCAP/AT_SECURE need correct state |
| Linux vDSO(7) [S5] | versioned mapped ELF functions can avoid kernel syscall path | trap-only broker does not cover full libc-visible ABI |
| Linux futex(2) [S6] | atomic compare-and-block on 32-bit shared word, even on x86_64 | two separate read and wait RPC calls lose wakeup |
| Linux clone(2) [S7] | CLONE_VM/SIGHAND/THREAD coupling, TID/TGID, process and thread signals | rich guest ThreadGroup/FD/VM model required |
| Linux signal(7) [S8] | thread signal mask, group dispositions and SA_RESTART/EINTR semantics | cancellation is not automatically correct restart |
| Linux rt_sigreturn(2) [S9] | user signal frame restores register/stack/mask context | malicious sigframe must never restore privileged UniOS state |
| Linux epoll(7) [S10] | edge/level, ready-list starvation, FD/open description lifetime | native WaitSet not automatically epoll-compatible |
| Linux openat2(2) [S11] | RESOLVE_IN_ROOT/BENEATH/NO_MAGICLINKS, race EAGAIN | native DirectoryCap and race-safe path traversal |
| Linux user_namespaces(7) [S12] | guest UID0 may be root only inside namespace | no native host RootCap from guest creds |
| Linux no_new_privs [S13] | exec privilege suppression, irreversible inherited setting | guest privilege emulation explicit; not complete sandbox |
| Linux seccomp documentation [S14] | syscall filter is not sandbox; pointer-deref restrictions | filtering analogies don't secure native kernel privilege |
| Linux rseq [S15] | per-thread scheduler ABI, migration/signal abort, newer V2 | don't return successful rseq unless guarantees supplied |
| Linux fsync(2) [S16] | file sync doesn't necessarily sync directory entry | RFC-008 durable receipt and namespace-specific errors |
| Linux proc(5) [S17] | procfs may expose process and magiclink state | synthetic scoped /proc, no host ObjectID exposure |
| Linux mmap(2) [S18] | MAP_PRIVATE COW vs MAP_SHARED and flags | RFC-004/008 backing/writeback proof gates |
| Linux capabilities(7) [S19] | Linux UID/capability sets/exec changes | guest metadata only, never host issuer authority |
| Linux pidfd_open(2) [S20] | fd referencing a process avoids numeric PID reuse | guest process-ref generation tied to authorized Task |
| Linux socket(7) [S21] | socket and ancillary payload semantics | guest descriptor and native NetworkCap separation |
| Linux ELF loader conventions [S22] | ELF program headers and interpreter metadata are target ABI input | safe parser/format/version/CPU selection, never cap grant |
| Linux epoll_ctl(2) [S23] | watches FD/open-file-description and flags | event subscriptions need separate lifecycle contract |

These describe Linux behavior, **not** a completed UniOS compatibility capability. Other OS source imports, runtime redistributions and licenses require independent review.

## 3. Architecture, host TCB and syscall entry

### 3.1 Candidate trust graph

~~~text
Unchanged Linux ELF program (untrusted Guest Task + Guest AddressSpace)
       | Linux x86_64 syscall / fault / optional vDSO
       v
UniOS Kernel: trusted GuestDomain tag, bounded trap+VM+Thread transitions
       | authorized small GuestRequest, copied scalar args
       v
LinuxPersonality broker in SEPARATE protected native Task/AddressSpace
       | guest PID/TID, FD, namespace, credentials, signal and syscall policy
       +--> Native VFS/DirectoryCaps + DataVolume service
       +--> Native Clock/CPU/Timer + MemoryObject/Pager
       +--> Native Network/Socket scoped services
       +--> Native permission portal/launcher
       v
kernel validates guest return register/context and effect phase
~~~

**TCB-K**: GuestDomain on native Task/Thread, trap capture, MMU/caps, ABI return and any atomic futex assist. **TCB-P**: trusted LinuxPersonality syscall service and process namespace/FD rights. **TCB-S**: delegated VFS, Network and Pager correctness. **TCB-L**: ELF/ld.so parsing and launch under narrow grants. Hardware boot/firmware/DMA remains separate. If TCB-P compromised with write access to all DataVolume, all user files are at risk; separation alone is not proof of least privilege.

Raw guest registers, ELF notes, process names and Guest UID0 never select Native syscall mode. Shared MemoryObject to broker must be data-only with bounded copy/pinning and no native privileged code entry or shared cap table. G2 mixed Task/native broker is rejected as default (RFC-009 v0.2). Kernel assistance needed to intercept SYSCALL before broker; plain IPC userspace process cannot do so unaided.

### 3.2 Capture/return and unknown effects

~~~text
Guest SYSCALL -> Kernel checks protected GuestDomain
 -> snapshot guest machine registers + valid scalar args / bounded pointers
 -> send restricted request to native broker via RFC-003 E1 channel
 -> broker validates guest FS/Thread/FD/credentials and resource charges
 -> native VFS/Network/VM work may commit a durable or external effect
 -> broker maps outcome to supported Linux status/result
 -> kernel validates guest Thread generation, signal and return context
 -> guest sees result, EINTR/restart, error or unknown effect
~~~

An enqueue-committed request does not mean broker executed it; timeout or peer death after commit does not undo server write or network send. Error choice must not claim no effect if work committed then copyout lost. Guest errno requires per-operation semantics; not just NativeError→LinuxError bit-cast. A malicious guest may alter mapped pointer bytes during validation; use bounded snapshots or proven pinned object generation, not a raw pointer broker dereference.

## 4. ELF, static and dynamic runtime profiles

**L0: image recognition/validation only.** Check x86_64 class, endianness, ELF header/program header lengths and integer overflow, PT_LOAD mapping/overlap/permissions/BSS, page alignment, W^X, PT_INTERP bounded string, PIE/ET_DYN, entrypoint, ASLR, stack and mapped executable bytes. Read-only validation doesn't prove running a binary. Unsupported format never falls back to privileged native execution.

**L1: selected static unmodified single-thread CLI candidate.** Initial argc/argv/envp/auxv and x86_64 initial stack alignment, verified AT_PHDR, AT_ENTRY, AT_BASE, AT_RANDOM, AT_HWCAP/HWCAP2, AT_PAGESZ and AT_SECURE; minimal pregranted stdout/readonly file/time. **Even supposedly static libc may perform unexpected brk/mmap/TLS/arch_prctl/rseq/futex/getrandom/exit_group**, so an actual sample plus distro/libc version must be measured before declaring it works. No app has been executed now.

**L2: selected dynamic CLI.** PT_INTERP identifies the guest Linux loader, not the UniOS native SDK. ld.so loads DT_NEEDED with ELF relocations/symbol-version/SONAME/DT_RPATH vs DT_RUNPATH/LD_LIBRARY_PATH rules, and uses AT_SECURE on privilege-sensitive launches [S3]. Loader/libc versions are part of support profile. Guest dynamic library root is DirectoryCap-constrained; cannot use arbitrary host /lib or special files. Runtime may expect TLS, DSO constructors, auxv and vDSO. A Linux ELF dynamically linked against glibc is not satisfied by a mere musl binary of the same architecture.

**L3:** fork/clone/exec, process tree, POSIX signals and threading/futex only after atomicity proof. **L4:** sockets/epoll, file watchers, timerfd/procfs and more complex CLI. **L5:** broader distro/containers/GUI outside current research target. Stage numbers denote verification tiers, **not implementation dates**.

**vDSO limitation:** a Linux vDSO can execute functions without traps; its symbol versions and data pages are ABI [S5]. It may be omitted only if targeted runtime has a correct fallback. CPU HWCAP and getcpu must correspond to real processor and scheduler guarantees, not invented values. 32-bit x86, x32 and arm64 Linux ABI are not inherited from 64-bit x86 coverage.

## 5. Process, threads, credentials, signals

Linux Process/ThreadGroup (TGID), Thread (TID), FDTable, OpenFileDescription, FSContext (cwd/root/umask), shared SignalDisposition, per-Thread mask, Credentials (UID/GID and Linux capabilities), Parent/Child/zombie/wait relation, guest Namespace and native ResourceBudget are different states. One UniOS Task/Thread is not automatically one Linux process or one complete Linux syscall context.

**fork:** COW mappings plus inherited open FD entries sharing OpenFileDescriptions, parent/child and signals. **clone/clone3:** flags CLONE_VM, CLONE_FILES, CLONE_SIGHAND, CLONE_THREAD, TID and TLS determine shared scope; CLONE_THREAD requires SIGHAND and SIGHAND requires VM [S7]. **execve:** program address space replaced, CLOEXEC descriptors dropped, signal state reset/retained according to Linux, thread group changes, PID/wait semantics persist in specified ways [S2]. **wait/kill:** numeric PID reuse needs generational process reference; guest namespace authority not native TaskCap.

**Signals:** process-directed and thread-directed delivery, masks, default dispositions, real-time queue, SA_RESTART versus EINTR, SIGCHLD, SIGPIPE, synchronous SIGSEGV/SIGBUS and handler alt stacks [S8]. On rt_sigreturn the guest-controlled signal frame cannot set kernel privilege, NativeGuestDomain, protected syscall return flags, forbidden FS/GS or arbitrary system kernel stack: full return context validation [S9]. Native Pager failure may yield guest SIGBUS or SIGSEGV according to actual reason; don't fabricate successful page supply.

**Guest privilege:** setuid, no_new_privs, user namespaces, Linux capabilities are modeled in guest only. A Linux namespace UID0 may be privileged relative to guest children but never gains host TPM/PCI/UEFI/BootGood/File outside launch grants. If setuid or file caps can't be accurately supported, require refusal/narrow model rather than simulating a privilege-increasing success.

## 6. Futex, rseq, memory and scheduling (P0)

**Futex** is a 32-bit user memory word. Linux futex WAIT compares and blocks atomically relative to concurrent writes/wake; one read RPC then a sleep RPC is wrong [S6]. Private futex key and shared MemoryObject+offset futex key differ; aliases can refer to same physical word. Native MAP_PRIVATE COW, page unmap, Task kill, timeout, signal and waiter cancellation interleave. Choices: F-A narrow kernel-backed compare-and-block/wake contract (TCB cost), F-B carefully justified emulator with equivalent atomicity, or F-C explicit exclusion of multithreaded Linux guest. Robust owner death, PI futex and requeue are further gates, not implied by basic futex.

**rseq** shares a per-thread registered region and requires CPU ID updates and critical section abort on preemption/migration/signals; recent Linux rseq V2 has advertised auxv feature size [S15]. An unsupported target may return an error and libc may fall back, but **not all binaries handle failure**. Claim only tested target behavior; never lie that rseq registered successfully without actual scheduler hooks. Scheduling priority inside Linux personality cannot mint native real-time or M0 reserved CPU rights (RFC-005).

**Memory:** mmap MAP_PRIVATE COW vs MAP_SHARED dirty, mprotect, brk, TLS/FS base, mremap, msync and file-backed Pager. RFC-004 M0/M1/M2/M3 gates: writable mmap/fsync not assumed until M3 is proven. A copy-only mmap fake breaks shared-memory futex and file writeback. Guest processes cannot map privileged native PFN or raw DeviceGroup/IOVA, and need correct fault/Signal memory error mapping.

## 7. Guest FD, VFS, path, namespace and disk truth

An FD is a guest table index, **not** a native Handle. FDEntry can carry FD_CLOEXEC, OpenFileDescription carries shared offset/status flags and underlying native FileCap/DirectoryCap/SocketCap. dup/fork may share file description/offset; open twice usually produces distinct descriptions. Recycled FD number with outstanding epoll callbacks must not authorize unrelated objects.

Guest path root, cwd, chroot, mount namespace, symbolic and proc magic links resolve through attenuated native DirectoryCaps. openat2 RESOLVE_BENEATH/IN_ROOT and NO_MAGICLINKS can fail EAGAIN on rename race [S11]. Guest root cannot reopen host EFI partition or inspect arbitrary native users through /proc/self/fd. /proc and /sys are separately synthetic, privacy-scoped guest service protocols, not raw host kernel object tables.

**Write durability:** buffered write returned count is not fsync success. RFC-008 FileSyncData/DirectorySync and durable receipts depend on file metadata, journal and hardware Flush. fsync alone doesn't necessarily sync the containing directory [S16]. Guest ENOSPC, EIO, EFAULT and EINTR mappings must reflect actual stage; a VFS rename may have committed before broker died, so blindly resubmitting the call isn't safe. O_APPEND, partial writes and atomic pipe writes require independent proof.


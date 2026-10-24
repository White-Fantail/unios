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


## 8. Socket, epoll, terminal and pseudo-filesystem compatibility

### 8.1 Network is a scoped service, not a syscall number

Guest socket(domain,type,protocol), bind, connect, listen, accept, send/recv, sendmsg/recvmsg, shutdown, socket options, nonblocking, DNS libc behavior and poll/epoll may map to Native Network SABI only through an authorized NetworkCap/address policy. The broker validates address family, lengths, SCM_RIGHTS and ancillary metadata. Guest AF_UNIX path sockets need DirectoryCap and namespace policy; SCM_RIGHTS passes guest FD/description rights through a typed guest-only transfer. Native object HANDLE values must never be copied into a guest cmsg or accepted from it as real kernel credentials.

TCP/UDP behavior, half-close, socket buffers, async close, SO_REUSEADDR, SIGPIPE and network failure aren't guaranteed by a Channel object. Linux POSIX socket state and host network service policy differ. A guest SOCK_RAW, packet socket, BPF device, or privileged port request doesn't gain raw NIC or network admin grant simply by guest UID0.

### 8.2 epoll cannot be mapped to generic Event one-to-one

An epoll instance watches guest FD/open-file-description identity, holds event registration and returns readiness with EPOLLET, EPOLLONESHOT and possibly EPOLLEXCLUSIVE semantics [S10,S23]. Edge-triggered consumers must drain until EAGAIN; Linux epoll can retain ready list and duplicate registration consequences under file description sharing. Native RFC-003 Event/WaitSet remains an object model whose semantics are not assumed equivalent.

**Case:** guest registers an FD on epoll, closes it, reuses the numeric FD for a different file, and receives an old ready event. Broker must not apply event to new FileDescription; registration identity/generation is protected. Duplicate FD aliases, hup/error, service restart and peer shutdown require exact behavior, not a generic "Event fired".

### 8.3 Pipes, terminals, pseudo-filesystems

stdin/stdout/stderr as pregranted streams suffice for a narrow static test only if host terminal/pipe semantics match those binary expectations. Interactive CLI apps may require termios/ioctl, PTY, job control, process groups, TTY foreground signals, ANSI terminal output, pipe capacity/PIPE_BUF atomic writes and blocking semantics. Returning successful unsupported ioctl can corrupt application behavior, so keep explicit feature tiers.

Synthetic guest /proc and /sys are optional, scoped and versioned. Files like /proc/self/maps, /proc/cpuinfo, /proc/self/fd/* and /proc/[pid]/status can expose CPU features, mapped executable addresses, process inventory and link escape paths. Values must describe the **guest** environment accurately enough for supported libc/tools without exposing host TaskCaps, physical IOMMU topology, arbitrary user processes or root boot metadata. Lack of a proc entry must be declared rather than fabricated as proof of ordinary Linux kernel presence.

## 9. Error translation, syscall restart, remote effects and services

**Distinct outcome layers:** guest SYSCALL register return, kernel Channel enqueue (COMMITTED/NOT_COMMITTED), broker request decode and native cap admission, native VFS/Network execution, file durability and guest reply copyout are different moments. A native peer death does not necessarily mean a Linux write was never accepted; a Linux EINTR is not evidence a canceled RPC did no work. When effects uncertain, broker needs persistent transaction identity or explicit guest-visible error with documented uncertainty; a generic retry after timeout is unsafe for rename, file create, socket send and any external operation.

| Guest situation | Native candidate evidence | Valid conservative policy |
|---|---|---|
| invalid syscall number | guest-domain dispatch doesn't recognize opcode | ENOSYS; don't dispatch native ABI opcode |
| invalid guest FD | FDTable generation/rights check failed before side effect | EBADF, no native capability minted |
| denied path/host resource | Directory/NetworkCap scope denies before commit | EPERM/EACCES as correct operation demands |
| page fault on output buffer | copyout may fail after service effect | EFAULT does not imply that remote effect was absent; require per-operation reconcile |
| blocked syscall interrupted | local signal arrived, maybe remote work already started | exact SA_RESTART/EINTR rule and side-effect history |
| unavailable broker/reincarnation | stale Endpoint/ReplyToken | error/UNKNOWN, not automatic completed success |
| data buffered, not flushed | native VFS accepted bytes but no DurableReceipt | guest write can return N; guest fsync still pending |
| file sync successful | correct file+metadata+hardware support | fsync result does not promise parent directory durability |
| unsupported privileged ioctl | guest asks raw physical command | safe denied error/ENOTTY/ENOSYS according to syscall |
| runtime expects rseq/futex | kernel lacks required atomic/scheduler semantics | explicit reject or compatible fallback only when target tested |

**Restart policy:** for each operation, identify if Linux SA_RESTART mandates automatic restart, EINTR is returned, partial bytes can be returned, or an operation has irrevocable side effects [S8]. Never replay a non-idempotent native service mutation merely because a guest signal handler exited. Committed request but undeliverable result must not produce a false NOT_COMMITTED return. Prevent stale broker generation or guest PID reuse from delivering a reply to the wrong process.

## 10. Resource budget, scheduling, fault isolation and performance feasibility

Each guest Task, syscall broker request, FD/open-file-description, pipe/socket buffer, process zombie, signal queue, futex waiter, epoll watch, mmap backing and file dirty page must be charged to its resource sponsor. Per-guest quotas prevent a single untrusted ELF from using all global TaskIDs, file handles, kernel IPC slots, memory or Pager resources. Broker priority cannot be raised to M0 merely by calling itself "compatibility critical"; separate essential CPU entitlement requires RFC-005 analysis.

Bounded kernel forwarding and separate broker protect the kernel from many syscall policy bugs but introduce **two-sided deadlocks**: guest waits on broker; broker faults on a page supplied by Pager; Pager needs disk service already starved by that guest; or the broker holds a high-value queue claim while guest has no CPU budget to consume result. Analyze graph edges at runtime, not just startup DAG. If broker crashes, old replies, FutexWait keys, SignalFrames, FileDescriptions and native service mutations may remain uncertain. Reincarnation needs fresh Endpoint and scoped caps; don't restore stale guest state as if prior transactions were undone.

Performance hypothesis only: simple syscall trap forwarding may require extra context switch/copy vs native kernel dispatch. Faster paths (shared memory/rings, vDSO, kernel futex assist) have larger lifetime/atomics/security obligations. Compare measured p50/p95/p99 syscalls/launch/fault, throughput, memory pinned, GUI responsiveness on **named QEMU and a real Reference PC** only after approved implementation and correctness evidence. No numbers are promised here.

## 11. Security policy mapping and capability boundaries

### 11.1 Guest path, process and native grants

Guest startup policy includes scoped NamespaceRoot/DirectoryCap, optional NetworkCap, stdin/out/err stream handles, memory budget, CPU quota and syscall-family support tiers. This policy is issued by UniOS launcher/portal, not by guest ELF metadata, signer name or Linux effective UID. Guest fork inherits no broader native grants than parent; guest exec/setuid may modify *guest* credential state where supported but may not mint host root. Native kernel remains the authority over real capability transfers and revocation.

Linux user namespaces can let UID0 have capabilities inside a child namespace but not in the host outside [S12]. UniOS can model an analogous bounded guest credential realm, but doesn't thereby implement actual Linux kernel user namespace semantics. A namespace filesystem mount may be denied if native DirectoryCap cannot represent its requested rights. Guest mount/chroot alone cannot secure the host; native FS enforces access regardless of guest tricks.

### 11.2 Signed code is not native authority

An immutable signed SystemImage loader verifies guest runtime bytes only to establish code origin under RFC-017 policy, not to grant DeviceGroup/IRQ/TPM/FirmwareRoot privileges. Code signed by a guest publisher may still be malicious; an unsigned ELF may be executable in explicitly permitted developer-only sandbox without implying a production verified boot. No guest syscall updates UEFI dbx or trusted floor.

### 11.3 Linux seccomp and anti-host-escape

Seccomp is not a complete sandbox on Linux itself [S14]. A future guest seccomp emulation may restrict the guest's syscall surface but **never substitutes for UniOS native Cap rights**. No assumption that a Linux user-notification service can intercept arbitrary raw guest SYSCALL without UniOS kernel/hardware mediation. If seccomp filter cannot be honored exactly and an app's policy relies on it, deny or report Unsupported rather than permit a less restrictive execution environment silently. Linux ptrace, CAP_SYS_ADMIN, CAP_SYS_PTRACE and keyring device APIs need scoped versions/denial, not direct host targets.

## 12. Linux compatibility coverage ladder and operational truth

| Level | Scope of evidence to target later | Explicit nonclaim |
|---|---|---|
| **L0** | safe parse, validate and map a chosen x86_64 ELF and its static CPU entry (not executed yet) | no binary compatibility |
| **L1** | 1+ chosen unmodified static single-threaded x86_64 CLI binaries with known syscall trace, exit/stdout/basic read | no glibc dynamic linker, threading or broad Linux |
| **L2** | selected dynamic ELF and exact ld.so/libc version, auxv/TLS/relocations, mmap and file operations | no arbitrary distro glibc parity |
| **L3** | process fork/clone/exec/wait, guest signals, one futex profile and documented scheduler behavior | no all Linux threading, robust PI/rseq automatically |
| **L4** | scoped sockets/epoll/pipe/pty, time, selected /proc and concurrent CLI programs | no Docker/systemd/GPU broad hardware |
| **L5** | additional distro/rootfs/app classes only after separate RFCs and tests | no universal Linux ABI parity |

A program may be dynamically linked but still single-threaded; another supposedly simple program may use clone/rseq at startup. The ladder is **feature evidence**, not a guaranteed linear ordering for every binary. Publish compatibility results as tuples:

- exact ELF binary hash, app/version and source provenance, target libc and interpreter, DSO dependency versions, CPU features/architecture;
- actual syscall+flag coverage, PageFault/Pager/writeback features and host storage/network policy;
- Linux guest process/thread/FD/signal/Namespace profile, supported kernel ABI expectation range;
- observed test counts and exact failures/limitations on named emulator/reference PC;
- no conflation of execution, functional correctness, data durability, full native hardware support and sandbox strength.

One command printing "hello" would establish only that specific executable behavior under the stated environment. No such command was executed for UniOS.

## 13. Comparative alternatives and feasible architecture choices

| Dimension | LINUX-A | LINUX-B | LINUX-C | LINUX-D | LINUX-E |
|---|---|---|---|---|---|
| Native kernel Linux ABI import | small trusted trap/atomic aids needed | emulator host operations, limited native dispatcher | **large** | guest kernel handles Linux | none |
| Guest raw syscall capture | kernel protected GuestDomain | emulator/translator traps itself | kernel Linux dispatch | Linux guest kernel | native app only |
| Linux semantic completeness | chosen personality services | chosen interpreter/system service | still needs broad kernel/POSIX semantics | best for supported real guest distributions | no Linux binary |
| Security major risk | broker authority/pointer/SignalFrame/futex | emulator/JIT escape, memory semantics | privileged giant parser+kernel TCB | VM boundary/virtual devices/passthrough | native source port bugs |
| Practical first research | strong alignment with RFC-009 | plausible fallback | generally conflicts D5 | separate optional product feature | already D3 first step |
| Unverified performance | IPC context switching | interpreter/recompiler cost | fewer crossings but huge code | hypervisor overhead | native execution |
| Key go/no-go | safe trap forwarding, atomic futex, coherent semantics | ISA/runtime fidelity and performance | TCB justification/proof | hypervisor/device stack and licenses | available source and toolchain |

**Provisional LINUX-A choice is *not* an engineering commitment.** A safe userspace emulator with lower verified TCB and acceptable measured performance could displace it; an inability to trap SYSCALL safely without expanding Ring 0 may justify deferring unchanged ELF entirely. No kernel port/compatibility implementation is approved.

## 14. Concrete attack, crash and concurrency counterexamples

| ID | Scenario | Unsafe inference and required research test |
|---|---|---|
| X10-01 | guest issues native CapFactory opcode in RAX | GuestDomain prevents native privilege, regardless of register value |
| X10-02 | guest jumps into broker code in shared Task | physically separate AddressSpaces or independently proved in-process compartment |
| X10-03 | broker dereferences guest pointer after mapping changed | bounded snapshot / backing generation, no TOCTOU |
| X10-04 | guest ELF PT_INTERP points outside guest DirectoryCap | deny sandbox escape, not host loader path |
| X10-05 | DT_RUNPATH and LD_LIBRARY_PATH resolve host arbitrary library | enforce guest root and secure-exec mode |
| X10-06 | signed ELF requests raw PCI device file | no native DeviceGroup from signature/path |
| X10-07 | guest attempts extra mmap W+X mapping under executable alias | RFC-004 W^X/alias and typed memory rights |
| X10-08 | vDSO reports impossible CPU/time data | real clock/CPU feature contract or correct libc fallback |
| X10-09 | ELF PT_LOAD arithmetic wraps, points to boot RAM | bounded parser and user address policy |
| X10-10 | static libc registers rseq not implemented; broker returns success | return explicit unsupported, validate libc fallback |
| X10-11 | two threads futex race read→wake→sleep | atomic compare-and-block proof, not RPC split |
| X10-12 | futex shared mapping aliases different virtual addresses | key by actual backing/offset where required, not numeric VA |
| X10-13 | robust mutex owner dies and waiters block forever | no full thread ABI claim until robust/PI model passes |
| X10-14 | clone(CLONE_THREAD) gets separate address spaces | reject unsupported flags or prove matching share semantics |
| X10-15 | guest child PID reused, old wait/kill targets new process | generational PID/PidFd references |
| X10-16 | exec closes wrong CLOEXEC description while dup shares offset | FDEntry vs OpenFileDescription lifetime |
| X10-17 | SIGSEGV fault maps to writable kernel fault waiter | private FaultWaiter and validated sigframe |
| X10-18 | rt_sigreturn frame requests native privileged thread mode | validate exact user GuestDomain/CPU return |
| X10-19 | SA_RESTART causes duplicate already committed rename | per-op restart/effect ledger and unknown status |
| X10-20 | interrupted write returns EINTR though bytes emitted | precise partial write and remote-effect behavior |
| X10-21 | guest root uses chroot/openat2 magiclink to access host ESP | DirectoryCap authority and race-safe path |
| X10-22 | /proc/self/fd exposes native Host CapIDs | synthetic guest-only proc information |
| X10-23 | guest CAP_SYS_ADMIN mounts raw disk | map guest caps within guest realm only |
| X10-24 | epoll wake on old FD after numeric reuse | registration bound to FileDescription+generation |
| X10-25 | epoll edge consumer starves forever | ET drain/ready fairness and repeat semantics |
| X10-26 | native WaitSet one-shot substituted for epoll LT | explicitly prove mapping or don't claim epoll |
| X10-27 | Socket SCM_RIGHTS sends native raw Handle bytes | guest FD authority/provenance mediation |
| X10-28 | guest sync succeeds but parent directory unsynced | RFC-008 file/directory durability scope |
| X10-29 | VFS commit then personality service dies | no false no-effect syscall result/retry |
| X10-30 | guest floods zombie child and signal queue | sponsor/quota and bounded cleanup |
| X10-31 | guest broker Pager requires itself | M0/wired pager and resource dependency fail bounded |
| X10-32 | broker restart gives old Thread stale successful result | service+Task generation checks |
| X10-33 | proposed PID -> native TaskCap by integer cast | native grant required, PID is only guest identifier |
| X10-34 | unimplemented seccomp returns successful sandbox setup | fail closed, no security downgrade |
| X10-35 | kernel exposes HWCAP without saving extended CPU state | refuse unsupported features, correct context |
| X10-36 | QEMU hello program used to advertise Linux distribution parity | claim only exact tested hash/supported syscalls |
| X10-37 | guest POSIX thread fills all CPU and M0 has no budget | RFC-005 scheduling reservation and quotas |
| X10-38 | MAP_SHARED writable file backing replaced while futex pending | generation/fault/writeback and proper waiter wake/error |
| X10-39 | process dies after cap transit but before error reply | RFC-003 MOVE/ReplyToken, sponsor and unknown effect |
| X10-40 | host network denies privileged bind but guest UID0 succeeds | explicit native NetworkCap policy, not guest root |

## 15. Proposed invariants (LX-01…LX-45) — NONE proved

| ID | Proposed invariant (not implemented) |
|---|---|
| LX-01 | Native KABI and Linux x86_64 raw syscall ABI remain distinct; guest opcode cannot route privileged native operation. |
| LX-02 | GuestDomain is kernel-owned, validated at every trap, never selected by guest ELF note/TLS/registers. |
| LX-03 | Default privileged Linux broker is a separate Task/AddressSpace/cap table, protected from direct guest instruction jumps. |
| LX-04 | Guest launch, process, UID0 and ELF signature cannot issue independent native CapIssuer, DeviceGroup, TPM or BootGood authority. |
| LX-05 | ELF header, PT_LOAD/PT_INTERP, sizes, overlap, alignment and user-space permissions are validated before mapping. |
| LX-06 | Unsupported loader or runtime is refused; Linux executable never auto-starts in native privileged ABI mode. |
| LX-07 | Initial guest stack, auxv/HWCAP, architecture/TLS/entry and CPU context facts match declared guest ABI profile. |
| LX-08 | Dynamic loader/libc versions and library path resolution stay within granted guest DirectoryCap root. |
| LX-09 | Guest vDSO behavior and versioned symbols are supported accurately or omitted only under tested fallback. |
| LX-10 | Guest raw syscall registers/bytes and potentially mutable pointers are bounded and revalidated before native service action. |
| LX-11 | Kernel Channel COMMITTED, broker accepted, native effect done and guest reply delivered have different status/ownership rules. |
| LX-12 | An unknown remote effect cannot be assumed undone merely from guest timeout, signal or broker death. |
| LX-13 | Guest errno is chosen from actual per-syscall semantic preconditions and effect state, not numerical native error casts. |
| LX-14 | Clone flags express accurate ThreadGroup, VM, SignalDisposition, FD and TID relationships or are explicitly denied. |
| LX-15 | Exec replacement and CLOEXEC/handler/ThreadGroup transitions occur at coherent declared commit boundaries. |
| LX-16 | Guest PID/TID reuse, PidFd, wait and kill cannot reach a new unrelated Task via stale integer generation. |
| LX-17 | Guest descriptor FDEntry and OpenFileDescription are distinct; dup/fork offset sharing and close lifetime obey declared policy. |
| LX-18 | Native FileCaps and NetworkCaps only reachable via broker-approved, attenuated guest descriptors; no direct host IDs. |
| LX-19 | Guest namespace root/openat2 symlink/magiclink traversal cannot escape native DirectoryCap scope. |
| LX-20 | Guest /proc and /sys synthetic data does not expose arbitrary native Task/Device/Boot identifiers or grant rights. |
| LX-21 | Linux signal mask/disposition/delivery and SA_RESTART/EINTR are scoped to supported syscall effects. |
| LX-22 | Malicious guest rt_sigreturn frame cannot change privilege, protected GuestDomain, kernel stack or unauthorized CPU state. |
| LX-23 | Native Pager fault/error maps to guest SIGSEGV/SIGBUS/errno only after proper cause and generation validation. |
| LX-24 | Futex compare-and-block is atomic relative to memory state+wake or advertised Linux threaded profile is unsupported. |
| LX-25 | Shared futex keying respects backing MemoryObject+offset and private mapping identity; never raw address equivalence alone. |
| LX-26 | Futex waiter, robust/PI capabilities and owner death are declared separately; basic wait/wake implies no full PI support. |
| LX-27 | Successful rseq registration implies correct per-thread CPU migration/signal abort and advertised feature-size semantics. |
| LX-28 | Guest scheduling preferences cannot mint protected M0 or real-time native CPU budget. |
| LX-29 | MAP_PRIVATE/COW and MAP_SHARED/dirty writeback respect RFC-004/008; no fake shared mapping for futex. |
| LX-30 | File write buffer acceptance, FileSyncData, DirectorySync and crash-durable scope are not conflated in guest status. |
| LX-31 | Guest epoll ET/LT/ONESHOT/readiness and registration lifetime aren't presumed identical to native Event/WaitSet. |
| LX-32 | Closing/reusing guest FD cannot redirect pending epoll/signal/IO callback to unrelated new FileDescription. |
| LX-33 | Socket rights, SCM_RIGHTS and network namespace cannot carry native host cap integers as credentials. |
| LX-34 | Guest privileged ioctl/ptrace/seccomp/mount/BPF either conform under scoped authority or refuse with safe error. |
| LX-35 | No unsupported security-sensitive syscall succeeds merely to satisfy Linux ABI probing. |
| LX-36 | New guest Thread/FD/signal/futex/broker queue/pager memory remains sponsor-bounded across failure/restart. |
| LX-37 | Broker service restart does not make old ReplyToken, signal, guest syscall request or file effect automatically valid. |
| LX-38 | M0 recovery and kernel bootstrap do not depend on a pageable LinuxPersonality process or guest libc. |
| LX-39 | Native Kernel guest trap path and syscall return respect CPU privilege, FS/GS, flags and canonical user addresses. |
| LX-40 | Time/vDSO/rseq CPU ID values correspond to supported actual scheduler/clock capabilities, not invented compatibility promises. |
| LX-41 | Linux x86_64 support implies no support for x32/i386/arm64 or every kernel/libc release without independent profile. |
| LX-42 | Published binary support is bounded by tested executable hash, runtime dependencies, semantic syscall coverage and device policy. |
| LX-43 | Source port, native Linux binary personality and whole Linux VM virtualization are reported as different results. |
| LX-44 | External licenses, source provenance and compatibility redistribution obligations are reviewed before runtime code adoption. |
| LX-45 | No LX proposal is accepted as working Linux binary compatibility, and no code/hardware test is authorized in this RFC. |


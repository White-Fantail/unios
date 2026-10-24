# UniOS RFC-010 — Linux Personality & Binary Compatibility

> **Status:** Proposed — Specification v0.2 (independent adversarial review; NOT Accepted, implemented, Linux-compatible, performance-validated or formally verified)
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

**v0.2 critical corrections:** (1) execve can pass a point of no return on rare failure and on success kills other Threads/unshares CLONE_FILES, so its error/commit cut is not always reversible [S2,S28]; (2) Linux close may free FD before a later error and retry can close someone else's newly reused FD [S24]; (3) dup2/dup3 replacement and O_CLOEXEC at creation require atomic FD-table operations against concurrent exec [S25,S26]; (4) epoll key is FD-number+open-file-description pair, and a closed FD may still produce old ready events while duplicated underlying OFD remains alive [S10]; (5) SA_RESTART and EINTR depend on exact syscall/device/timeout and partial transfers [S8]; (6) openat2 path resolution requires per-flag, per-component policy, not merely a DirectoryCap label [S11]; (7) getrandom entropy and small-request guarantees are distinct from returning arbitrary bytes [S27].

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
| Linux close(2) [S24] | FD is released before later errors; retry risks closing reused FD; IO may retain OFD after close | guest close, async errors and FD identity require two separate lifetimes |
| Linux dup2/dup3 [S25] | atomic target FD replacement under concurrent signal/Thread | never implement guest dup2 as close+dup RPC pair |
| Linux open(2) O_CLOEXEC [S26] | atomic close-on-exec flag at creation avoids fork/exec descriptor leak | no separate open→fcntl security-sensitive window |
| Linux getrandom(2) [S27] | entropy initialization, blocking/nonblock and length/EINTR conditional semantics | no fake predictable bytes or generic success at guest libc startup |
| Linux execve(2) [S28] | successful exec kills sibling Threads/unshares CLONE_FILES; rare past-no-return failure | distinguish precommit exec error from post-commit terminal guest failure |

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

**L1: selected static unmodified single-thread CLI candidate.** The libc/runtime startup contract may also call getrandom; this is a real entropy-security dependency. Linux getrandom blocks until urandom state is initialized by default, EAGAIN with GRND_NONBLOCK, and has request-size-dependent partial/EINTR behavior [S27]. Do not return predictable bytes or fake successful entropy just to make a guest binary start. Initial argc/argv/envp/auxv and x86_64 initial stack alignment, verified AT_PHDR, AT_ENTRY, AT_BASE, AT_RANDOM, AT_HWCAP/HWCAP2, AT_PAGESZ and AT_SECURE; minimal pregranted stdout/readonly file/time. **Even supposedly static libc may perform unexpected brk/mmap/TLS/arch_prctl/rseq/futex/getrandom/exit_group**, so an actual sample plus distro/libc version must be measured before declaring it works. No app has been executed now.

**L2: selected dynamic CLI.** PT_INTERP identifies the guest Linux loader, not the UniOS native SDK. ld.so loads DT_NEEDED with ELF relocations/symbol-version/SONAME/DT_RPATH vs DT_RUNPATH/LD_LIBRARY_PATH rules, and uses AT_SECURE on privilege-sensitive launches [S3]. Loader/libc versions are part of support profile. Guest dynamic library root is DirectoryCap-constrained; cannot use arbitrary host /lib or special files. Runtime may expect TLS, DSO constructors, auxv and vDSO. A Linux ELF dynamically linked against glibc is not satisfied by a mere musl binary of the same architecture.

**L3:** fork/clone/exec, process tree, POSIX signals and threading/futex only after atomicity proof. **L4:** sockets/epoll, file watchers, timerfd/procfs and more complex CLI. **L5:** broader distro/containers/GUI outside current research target. Stage numbers denote verification tiers, **not implementation dates**.

**vDSO limitation:** a Linux vDSO can execute functions without traps; its symbol versions and data pages are ABI [S5]. It may be omitted only if targeted runtime has a correct fallback. CPU HWCAP and getcpu must correspond to real processor and scheduler guarantees, not invented values. 32-bit x86, x32 and arm64 Linux ABI are not inherited from 64-bit x86 coverage.

## 5. Process, threads, credentials, signals

Linux Process/ThreadGroup (TGID), Thread (TID), FDTable, OpenFileDescription, FSContext (cwd/root/umask), shared SignalDisposition, per-Thread mask, Credentials (UID/GID and Linux capabilities), Parent/Child/zombie/wait relation, guest Namespace and native ResourceBudget are different states. One UniOS Task/Thread is not automatically one Linux process or one complete Linux syscall context.

**fork:** COW mappings plus inherited open FD entries sharing OpenFileDescriptions, parent/child and signals. **clone/clone3:** flags CLONE_VM, CLONE_FILES, CLONE_SIGHAND, CLONE_THREAD, TID and TLS determine shared scope; CLONE_THREAD requires SIGHAND and SIGHAND requires VM [S7]. **execve:** normally replaces an existing process image while preserving PID, kills the other Threads, and **unshares a previously CLONE_FILES-shared FD table**, applying CLOEXEC and specific signal/credential resets [S2,S28]. A failed execve **usually** returns to old image, but after the rare point-of-no-return failure the old image can already have been destroyed; Linux kills the process rather than restoring it. Thus UniOS guest execution transition must have a designated irreversible cut and a post-cut fatal failure path rather than promise universal transactional rollback. **wait/kill:** numeric PID reuse needs generational process reference; guest namespace authority not native TaskCap.

**Signals:** process-directed and thread-directed delivery, masks, default dispositions, real-time queue, SA_RESTART versus EINTR, SIGCHLD, SIGPIPE, synchronous SIGSEGV/SIGBUS and handler alt stacks [S8]. SA_RESTART is **per syscall and blocking mode**: epoll_wait/poll/nanosleep never automatically restart merely due to SA_RESTART, some slow-device read/write can restart, while I/O that already transferred bytes generally returns count rather than EINTR. Native RPC timeout/cancel cannot be assumed to reproduce this guest-visible behavior. On rt_sigreturn the guest-controlled signal frame cannot set kernel privilege, NativeGuestDomain, protected syscall return flags, forbidden FS/GS or arbitrary system kernel stack: full return context validation [S9]. Native Pager failure may yield guest SIGBUS or SIGSEGV according to actual reason; don't fabricate successful page supply.

**Guest privilege:** setuid, no_new_privs, user namespaces, Linux capabilities are modeled in guest only. A Linux namespace UID0 may be privileged relative to guest children but never gains host TPM/PCI/UEFI/BootGood/File outside launch grants. If setuid or file caps can't be accurately supported, require refusal/narrow model rather than simulating a privilege-increasing success.

## 6. Futex, rseq, memory and scheduling (P0)

**Futex** is a 32-bit user memory word. Linux futex WAIT compares and blocks atomically relative to concurrent writes/wake; one read RPC then a sleep RPC is wrong [S6]. Private futex key and shared MemoryObject+offset futex key differ; aliases can refer to same physical word. Native MAP_PRIVATE COW, page unmap, Task kill, timeout, signal and waiter cancellation interleave. Choices: F-A narrow kernel-backed compare-and-block/wake contract (TCB cost), F-B carefully justified emulator with equivalent atomicity, or F-C explicit exclusion of multithreaded Linux guest. Robust owner death, PI futex and requeue are further gates, not implied by basic futex.

**rseq** shares a per-thread registered region and requires CPU ID updates and critical section abort on preemption/migration/signals; recent Linux rseq V2 has advertised auxv feature size [S15]. An unsupported target may return an error and libc may fall back, but **not all binaries handle failure**. Claim only tested target behavior; never lie that rseq registered successfully without actual scheduler hooks. Scheduling priority inside Linux personality cannot mint native real-time or M0 reserved CPU rights (RFC-005).

**Memory:** mmap MAP_PRIVATE COW vs MAP_SHARED dirty, mprotect, brk, TLS/FS base, mremap, msync and file-backed Pager. RFC-004 M0/M1/M2/M3 gates: writable mmap/fsync not assumed until M3 is proven. A copy-only mmap fake breaks shared-memory futex and file writeback. Guest processes cannot map privileged native PFN or raw DeviceGroup/IOVA, and need correct fault/Signal memory error mapping.

## 7. Guest FD, VFS, path, namespace and disk truth

An FD is a guest table index, **not** a native Handle. FDEntry can carry FD_CLOEXEC, OpenFileDescription carries shared offset/status flags and underlying native FileCap/DirectoryCap/SocketCap. dup/fork may share file description/offset; open twice usually produces distinct descriptions. **Atomic guest FD-table transitions:** dup2/dup3 closes/replaces target FD as one indivisible action (not close then dup) [S25]; O_CLOEXEC/pipe2/dup3 flags must be assigned at creation to close fork/exec races [S26]. **Linux close** may release the integer FD early even when later close reports EIO/ENOSPC/EINTR; never blindly retry close on the same integer, which could now refer to a new FD [S24]. A blocked operation can retain its own OpenFileDescription while the numeric entry is closed. Recycled numeric FD and epoll/IPC callbacks must never be resolved by looking up the *current* FD entry alone.

Guest path root, cwd, chroot, mount namespace, symbolic and proc magic links resolve through attenuated native DirectoryCaps. openat2 RESOLVE_BENEATH forbids path escapes incl. absolute links, while RESOLVE_IN_ROOT interprets absolute paths/links relative to supplied dirfd and confines '..'; both require per-component race-safe lookup and EAGAIN when safe resolution cannot be proved [S11]. **Do not rely on their current incidental magic-link rejection**: RESOLVE_NO_MAGICLINKS must be separately enforced if requested. Unknown or incompatible resolve flags, size/mode fields are not silently discarded. All requested semantics are still intersected with native DirectoryCap. Guest root cannot reopen host EFI partition or inspect arbitrary native users through /proc/self/fd. /proc and /sys are separately synthetic, privacy-scoped guest service protocols, not raw host kernel object tables.

**Write durability:** buffered write returned count is not fsync success. RFC-008 FileSyncData/DirectorySync and durable receipts depend on file metadata, journal and hardware Flush. fsync alone doesn't necessarily sync the containing directory [S16]. Guest ENOSPC, EIO, EFAULT and EINTR mappings must reflect actual stage; a VFS rename may have committed before broker died, so blindly resubmitting the call isn't safe. Linux close errors may report **previous writeback failure after FD removal** and neither EIO nor EINTR licenses retrying close on a recycled FD [S24]; no `close` success proves FileSyncData or DirectorySync. O_APPEND, partial writes and atomic pipe writes require independent proof.


## 8. Socket, epoll, terminal and pseudo-filesystem compatibility

### 8.1 Network is a scoped service, not a syscall number

Guest socket(domain,type,protocol), bind, connect, listen, accept, send/recv, sendmsg/recvmsg, shutdown, socket options, nonblocking, DNS libc behavior and poll/epoll may map to Native Network SABI only through an authorized NetworkCap/address policy. The broker validates address family, lengths, SCM_RIGHTS and ancillary metadata. Guest AF_UNIX path sockets need DirectoryCap and namespace policy; SCM_RIGHTS passes guest FD/description rights through a typed guest-only transfer. Native object HANDLE values must never be copied into a guest cmsg or accepted from it as real kernel credentials.

TCP/UDP behavior, half-close, socket buffers, async close, SO_REUSEADDR, SIGPIPE and network failure aren't guaranteed by a Channel object. Linux POSIX socket state and host network service policy differ. A guest SOCK_RAW, packet socket, BPF device, or privileged port request doesn't gain raw NIC or network admin grant simply by guest UID0.

### 8.2 epoll cannot be mapped to generic Event one-to-one

An epoll instance's interest key is the **pair (registered FD number, underlying OpenFileDescription)**, not just one or the other [S10]. Duplicate numeric FDs referencing the same OFD can be registered separately, while re-registering the *same pair* is normally EEXIST. An epoll watch may continue returning events for a closed registered FD as long as **other FD references to the same OFD** remain; that does not authorize lookup of a newly reused FD with the same number. It holds event registration and returns readiness with EPOLLET, EPOLLONESHOT and possibly EPOLLEXCLUSIVE semantics [S10,S23]. Edge-triggered consumers must drain until EAGAIN; Linux epoll can retain ready list and duplicate registration consequences under file description sharing. Native RFC-003 Event/WaitSet remains an object model whose semantics are not assumed equivalent.

**Case:** guest registers fd=7 for OFD-A; dup(7) creates fd=8 referring to OFD-A; close(7); open() reuses fd=7 for OFD-B. epoll may still report old registered fd=7 events because fd=8 keeps OFD-A alive [S10]. A broker MUST preserve the watch's (registered FD,OFD-A,registration generation) and deliver the documented event without reinterpreting it as OFD-B authority. Explicit EPOLL_CTL_DEL or last OFD close has distinct semantics; duplicates, hup/error, multi-epoll watches, service restart and peer shutdown need model.

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

**Restart policy:** for each operation, identify if Linux SA_RESTART permits restart, EINTR is returned, partial bytes can be returned, or an operation has irrevocable side effect [S8]. For example epoll_wait, nanosleep and timeout-configured socket calls are not SA_RESTART restarts; a slow-device write that transferred bytes returns the transferred count, not a fabricated EINTR. Replaying a side-effecting operation from scratch just because a handler returned is unsafe. Never replay a non-idempotent native service mutation merely because a guest signal handler exited. Committed request but undeliverable result must not produce a false NOT_COMMITTED return. Prevent stale broker generation or guest PID reuse from delivering a reply to the wrong process.

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

## 15. Proposed invariants (LX-01…LX-57) — NONE proved

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
| LX-46 | execve has a defined precommit failure versus post-point-of-no-return fatal outcome; no promise of universal process-image rollback. |
| LX-47 | Successful execve keeps Linux process identity as required, terminates sibling Threads and unshares previously CLONE_FILES-shared FD table before applying exec descriptor semantics. |
| LX-48 | Guest close may retire numeric FD even when deferred writeback/error follows; blindly retrying close cannot affect newly reused FD. |
| LX-49 | Guest dup2/dup3 replacement of target FD is one atomic FD-table transition, not a close and duplicate pair subject to signal/Thread races. |
| LX-50 | Guest O_CLOEXEC and equivalent create flags are applied atomically with FD publication, not as later fcntl susceptible to fork/exec leaks. |
| LX-51 | Epoll registration identity is the pair (registered FD number, OpenFileDescription); reusing a number cannot retarget old subscription to a different OFD. |
| LX-52 | An epoll watch may remain and emit old-FD events after close while another reference to the original OFD persists; close is not always watch removal. |
| LX-53 | SA_RESTART/EINTR behavior is per syscall, I/O device kind and timeout configuration, with partially transferred bytes reported correctly. |
| LX-54 | Native VFS close error reporting and guest FD release state are independent of FileSync/DirectorySync success, under RFC-008 durable receipt semantics. |
| LX-55 | Guest openat2 RESOLVE_IN_ROOT and RESOLVE_BENEATH semantics differ; explicit NO_MAGICLINKS and per-component race handling cannot be replaced by relying on incidental current flags. |
| LX-56 | Guest getrandom never claims cryptographic entropy success without suitable trusted entropy state; nonblock, EAGAIN and short/EINTR returns match declared Linux profile. |
| LX-57 | rseq optimized V2 is offered only for CPU/platform feature profile with real scheduler event semantics; a legacy registration and V2 feature-size announcement are not interchangeable. |


## 16. Open research blockers (O10-01…O10-53)

| ID | Priority | Unresolved question / exact proof gap | Co-review |
|---|---|---|---|
| O10-01 | P0 | Real x86_64 GuestDomain SYSCALL capture, return and native opcode non-bypass with small kernel TCB? | 001/009/010 |
| O10-02 | P0 | Guest Task vs native broker separate AddressSpace/cap table and untrusted pointer snapshot? | 002/003/009/010 |
| O10-03 | P0 | LINUX-A kernel trap forwarding vs LINUX-B userspace emulator technical feasibility and TCB/latency? | 001/009/010 |
| O10-04 | P0 | Selected x86_64 ELF PT_LOAD/PIE/PT_INTERP image parser, W^X, relocations and signature? | 004/010/017 |
| O10-05 | P0 | Exact known static ELF and linker/libc startup syscall footprint; no unknown boot dependency? | 009/010/015 |
| O10-06 | P0 | Real initial stack, auxv/HWCAP, FS/TLS, entry stack, CPU features and OS ABI? | 005/009/010 |
| O10-07 | P0 | Kernel↔broker guest request state and effect/copyout failure with source pointer mutation? | 003/004/009/010 |
| O10-08 | P0 | Linux errno/restart/EINTR vs committed native file or network external effects? | 003/008/010 |
| O10-09 | P0 | Basic 32-bit futex atomic compare-block and wait-key identity over shared MemoryObject/aliasing? | 004/005/009/010 |
| O10-10 | P0 | Guest MAP_PRIVATE COW, shared mmap/dirty backing, mprotect, unmap, and fault-to-signal? | 004/008/010 |
| O10-11 | P0 | Guest ThreadGroup/TID/TGID PID lifetimes and safe stale PID, wait, signals? | 002/009/010 |
| O10-12 | P0 | Clone shared VM/FD/Sighand/TLS flags, TID clear, thread/group leader exit? | 004/005/009/010 |
| O10-13 | P0 | Guest exec and ELF interpreter replacement preserving process ID/FDT and runtime trust? | 004/009/010 |
| O10-14 | P0 | Linux signal frame/rt_sigreturn CPU state privilege validation and nested signal/alt stack? | 001/004/009/010 |
| O10-15 | P0 | Native FileCaps mapped to FDEntry/OpenFileDescription with dup/fork/close and offset atomics? | 002/008/010 |
| O10-16 | P0 | Guest cwd/mount namespace/openat2 symlink/magiclink escape vs native DirectoryCap? | 002/008/010 |
| O10-17 | P0 | Linux credential UID/GID/root/setuid/no_new_privs/capabilities scope and non-escalation? | 002/009/010/016 |
| O10-18 | P0 | Shared dependent Pager/CPU/IPC budgets so Linux broker never blocks M0 recovery? | 004/005/006/010 |
| O10-19 | P0 | Guest support-tier reporting cannot label unsupported seccomp/futex/rseq as success? | 009/010/015 |
| O10-20 | P0 | Linux ABI/version/host feature matrix and exact behavioral test oracle/later proof? | 009/010/015/018 |
| O10-21 | P1 | glibc/musl selected interpreter and relocation/ld.so/ABI symbol-version and license? | 009/010/018 |
| O10-22 | P1 | Correct AT_SECURE, LD_* filtering, auxv version and vDSO contract for selected libc? | 009/010/017 |
| O10-23 | P1 | rseq original vs V2 feature size, CPU migration and signal abort; libc fallback? | 005/009/010 |
| O10-24 | P1 | Robust futex/PI/requeue/FUTEX_WAITV owner death and scheduling donation? | 003/005/009/010 |
| O10-25 | P1 | Guest scheduler policy and POSIX clocks vs native CPU/time permissions? | 005/009/010 |
| O10-26 | P1 | Signal realtime queue, signalfd, SIGCHLD, SA_RESTART matrix vs broker restart? | 003/005/009/010 |
| O10-27 | P1 | Guest epoll edge/level, EPOLLONESHOT/EXCLUSIVE, FD reuse and open-file-description reachability? | 003/010 |
| O10-28 | P1 | Pipe PTY/termios/ioctl, process group/job control and SIGPIPE/PIPE_BUF semantics? | 003/010 |
| O10-29 | P1 | Sockets/SCM_RIGHTS AF_UNIX, network namespace and actual native NetworkCap routes? | 002/010 |
| O10-30 | P1 | Synthetic /proc/self/fd/maps/cpuinfo and /sys data minimizing leakage and magiclink? | 002/004/010/016 |
| O10-31 | P1 | Precise guest fsync/fdatasync/msync, O_APPEND and data/dir durability mapping? | 008/009/010 |
| O10-32 | P1 | Linux clone/exec fork under broker death or parent exit, zombie reaping and PID namespace? | 003/009/010 |
| O10-33 | P1 | Shared memory futex key correctness through mremap, COW and partial revoke? | 002/004/010 |
| O10-34 | P1 | Guest seccomp/ptrace/capability checks treated as secure or explicitly unsupported? | 002/010/015 |
| O10-35 | P1 | Legal copy/redistribution of glibc, musl, ld.so and transitive DSO dependencies? | 010/018 |
| O10-36 | P1 | Debug, compatibility traces, crash reports and privacy-safe guest/native state separation? | 009/010/015/016 |
| O10-37 | P2 | Broader static/musl/glibc/PIE CLI performance and memory overhead on reference PC? | 005/010/015 |
| O10-38 | P2 | Native wine/Windows and Darwin personalities share services without cross-guest authority? | 009/010/012/014 |
| O10-39 | P2 | Linux virtualization alternative VM hardware/virtio/PCI passthrough/security feasibility? | 007/010/015 |
| O10-40 | P2 | Multi-arch x32/i386/arm64 guest future legal and semantic support boundaries? | 009/010/018 |
| O10-41 | P0 | Model execve precommit recoverable error vs after teardown no-return fatal, retained PID, sibling Thread teardown? | 004/009/010 |
| O10-42 | P0 | Correct CLONE_FILES FD table unshare on exec and close-on-exec atomic under concurrent other Threads? | 002/009/010 |
| O10-43 | P0 | Linux close error after early FD release vs async IO still pinning OFD, no retry closing reused descriptor? | 008/009/010 |
| O10-44 | P0 | dup2/dup3 atomic replace and target close vs guest signal/FD allocator race, shared CLONE_FILES? | 002/009/010 |
| O10-45 | P0 | Atomic FD_CLOEXEC at open/socket/pipe/create, old/new image transition and leaked delegated native FileCaps? | 002/009/010 |
| O10-46 | P0 | epoll key (FD number,OFD) with duplicate aliases, close last-reference removal, numeric FD reuse, event user-data semantics? | 003/009/010 |
| O10-47 | P0 | epoll reports old closed FD while duplicated OFD alive; no unauthorized lookup of newly reused FD? | 002/003/010 |
| O10-48 | P0 | Linux signal SA_RESTART exceptions by interface and socket timeout, partial transfer precedence over EINTR? | 003/008/009/010 |
| O10-49 | P1 | Linux/POSIX 2024 close EINTR behavior differs; target Linux version and close/retry/error contract explicit? | 008/010 |
| O10-50 | P0 | openat2 RESOLVE_IN_ROOT vs BENEATH, separate NO_MAGICLINKS, rename EAGAIN, flags/size/mode validation? | 008/010 |
| O10-51 | P0 | Native entropy provider supports Linux getrandom initialized/blocking/nonblock and <=256-byte rules without fake success? | 005/009/010/017 |
| O10-52 | P1 | Optimized rseq V2 features only on supported CPU/kernel entry architecture; auxv length and legacy 32-byte fallback exact? | 005/009/010 |
| O10-53 | P1 | Distinguish actual Linux syscall success/errno from host service result after postcommit close, copyout and replay? | 003/008/009/010 |

## 17. Proposed decision register (LX-D01…LX-D33) — NONE Accepted

- **LX-D01:** LINUX-A protected guest Task + separate native LinuxPersonality broker is preferred conditional design hypothesis, not selected implementation.
- **LX-D02:** POSIX source port, Linux binary personality, and Linux guest OS virtualization are separately described and measured.
- **LX-D03:** GuestDomain is kernel-owned and dispatches raw Linux syscall separately from native KABI; no guest register/TLS/ELF switch can promote privileges.
- **LX-D04:** G1 guest Task and privileged broker must not share unprotected code/data/cap execution domain; G2 same-address-space policy remains unadmitted.
- **LX-D05:** LINUX-B emulator remains a serious alternative if safe direct syscall interception is too costly or invasive.
- **LX-D06:** Start x86_64 only with L0 validation, L1 bounded static CLI, L2 selected dynamic libc, L3 threads/signals, L4 scoped network/epoll, each as unproved stage.
- **LX-D07:** Guest ELF/PT_INTERP/auxv/TLS/vDSO/runtime metadata follow their selected Linux ABI, never native SDK struct identity.
- **LX-D08:** Linux loader and runtime are confined to an authorized guest DirectoryCap tree and CPU feature/trust policy.
- **LX-D09:** Native Task/Thread mapping, Linux ThreadGroup/TGID/TID, FD and signal state are distinct; no naive single TaskCreate abstraction.
- **LX-D10:** Guest UID0, setuid, namespace caps and package signature do not mint UniOS CapIssuer, DMA, IRQ or BootFloor authority.
- **LX-D11:** Linux futex compare-and-block needs proven MemoryObject-aware atomicity, else multithreaded guest scope denied; robust/PI separately gated.
- **LX-D12:** rseq must have real scheduler/migration/signal guarantees or explicit unsupported behavior tested against chosen libc.
- **LX-D13 [clarified]:** Guest signal frame/return and SA_RESTART/EINTR are per-operation, and exec includes post-point-of-no-return fatal states; no privilege confusion or dishonest effect rollback.
- **LX-D14 [clarified]:** Linux FDEntry/OpenFileDescription shared-offset and lifetime semantics map through native FileCaps; guest numeric FD may retire early despite close error or still-referenced OFD.
- **LX-D15:** Guest openat2, cwd, mount, symlinks and proc magiclinks never traverse beyond issued DirectoryCaps.
- **LX-D16 [clarified]:** epoll (registered FD,OFD) key, duplicate aliases, old FD events after close, ET/LT/ONESHOT and removal must be proved over native WaitSet, not inferred from notification existence.
- **LX-D17:** Socket/SCM_RIGHTS and network rights remain delegated and constrained, no raw host handles or network admin from guest credential.
- **LX-D18:** Guest errno and syscall restart are mapped from exact native effect phase; no automatic retry after committed rename/send.
- **LX-D19:** Guest mmap/private vs shared writable/pager and fsync/file durability respect RFC-004 M-stages and RFC-008 receipts.
- **LX-D20:** Guest broker/FD/futex/Thread and process queues are resource sponsor bounded and may fail safely when hardware/scheduler unavailable.
- **LX-D21:** Unsupported Linux security-sensitive syscalls cannot claim success merely to avoid libc/program errors.
- **LX-D22:** Every published Linux compatibility claim names executable hash, library versions, CPU, syscall/flag/feature tier and observed tests.
- **LX-D23:** Compiler/runtime third-party licenses and upstream provenance reviewed before any code import or redistribution.
- **LX-D24:** No native syscall number, Linux ABI freeze, loader implementation, compatibility test, source import or product guarantee authorized until P0 gates reviewed.
- **LX-D25 [new]:** Treat successful execve as process-image replacement with sibling Thread teardown and CLONE_FILES unshare; rare post-teardown exec failure is fatal, not automatically recoverable.
- **LX-D26 [new]:** Guest close retires FD under Linux semantics even when later writeback errors are reported; a failed close must not be blindly retried against same reusable integer.
- **LX-D27 [new]:** Implementability of dup2/dup3 and O_CLOEXEC atomic FD-table transitions is a hard compatibility and host capability-leak proof gate, not an optional optimization.
- **LX-D28 [new]:** Epoll watch identity is registered FD plus original OpenFileDescription; old readiness may survive numeric FD close when OFD still referenced by dup/fork.
- **LX-D29 [new]:** SA_RESTART/EINTR, partial returns, per-device blocking and timeouts are explicit guest semantic contracts; generic restart of remote request is denied.
- **LX-D30 [new]:** Guest path security must enforce explicit openat2 flag semantics, including RESOLVE_IN_ROOT/BENEATH and NO_MAGICLINKS; native DirectoryCap remains final authority.
- **LX-D31 [new]:** Guest getrandom must derive bytes from separately trusted entropy and expose blocking/EAGAIN/interruption semantics; no deterministic fake randomness to pass tests.
- **LX-D32 [new]:** Advanced rseq V2 optimization is a feature-profile gate requiring scheduler/auxv correctness on the selected architecture; legacy fallback requires evidence.
- **LX-D33 [new]:** v0.2 contract hardening remains Proposed and does not grant production Linux ABI/ELF compatibility or implementation permission.

## 18. Cross-RFC integration, contradictions and ownership

| RFC | Dependency or possible contradiction |
|---|---|
| RFC-001 | extra Linux syscall handling must not turn small independent microkernel into privileged POSIX parser without explicit ADR |
| RFC-002 | broker grants from native issuer, FD/PID numbers not caps, no ABA, sponsor inheritance not infinite |
| RFC-003 | E1 Channel bytes+MOVE, ReceiveEscrow, ReplyToken one-shot, UNKNOWN effects, service incarnation |
| RFC-004 | COW, guest mmap/PagerFault, private FaultWaiter, shared-memory futex keys, M3 writable backing |
| RFC-005 | Linux rseq scheduler event, guest nice/real-time policy vs protected M0 budgets, futex PI |
| RFC-006 | M0 startup and service recovery must work independent of guest runtime; no guest process needed for rescue |
| RFC-007 | guest direct MMIO/IOMMU/driver/ioctl not allowed, hardware group/DMA fence needed for genuine device work |
| RFC-008 | native FileCaps, durability receipt, dirty mmap, directory fsync, writeback error loss limitations |
| RFC-009 | ABI-A separation, G1 distinct guest and privileged broker, native KABI independent of Linux registers/errno |
| RFC-010 | L0–L4 explicit coverage, guest PID/FD/syscall ABI and resourcing; no Linux claims beyond evidence |
| RFC-011 | install/package and guest runtime/library management not code signing grant issuer |
| RFC-012 | Windows/Wine remains separate ABI, shared native services must not leak guest privilege |
| RFC-013 | desktop terminal/GUI/GPU capabilities separately researched |
| RFC-014 | Darwin/Mach-O remains a separate guest execution and licensing scope |
| RFC-015 | CPU trap, syscalls, malicious ELF, futex/signal clone races and fuzz suite/proof gates |
| RFC-016 | user file/network/device permission grants and consent; guest credential not identity proof |
| RFC-017 | verified signed BootBundle/SystemImage, untrusted executable signing vs real native grants |
| RFC-018 | syscall version ID, guest ABI structs/errno, typed SABI handle roles and compatibility matrix |

**Joint P0 groups:** (001/009/010) actual raw Linux SYSCALL interception + native privilege boundary; (002/003/004/009/010) argument snapshot and cap transfer; (004/005/010) futex atomic compare/block and rseq migration; (008/009/010) FD/path and durable effect errors; (006/010/017) signed launch vs permission and recovery. This RFC cannot resolve them by simply naming a broker.

## 19. Proposed bounded model and future validation plan

### 19.1 Finite model sketch

Two Guest Tasks plus one Native Broker Task and one Native VFS; two guest Threads in one ThreadGroup, a shared MemoryObject with two different VAs referencing same futex word, a private COW mapping, GuestFD 3 with reused number, two FileDescriptions, one guest PID reused after child exit, one signal mask/disposition, one pending rt_sigreturn frame, one broker request and ReplyToken, one service restart, a Thread with CPU budget zero, a Pager that may die and one native DeviceCap that only broker holds.

Transitions: guest raw syscall native opcode injection; ELF launch with forged HWCAP; clone flags; fork/exec wait; guest pointer modification after trap; futex value check+wake+block; signal before/after blocked syscall; broker forwards FS rename; VFS commits then broker dies; guest retries; mmap COW and shared aliases; FD close+reuse during epoll wake; path rename/openat2 magiclink race; signed/unsigned guest launch; resource sponsor exhaustion.

**Safety properties:**
- no guest native privilege amplification or cross-Task broker instruction jump;
- no host cap from UID0, FD, PID, ELF signature or raw syscall opcode;
- futex no lost wake under declared atomic contract; no wrongly shared/private wait key;
- guest signal frame cannot restore privileged native context;
- FD/PID reused number cannot redirect stale operation;
- file write/rename committed but lost response never treated as proven undone;
- path resolution stays within issued DirectoryCap and data durability reports actual scope;
- no capability or resource leak under broker crash/guest death.

**Conditional liveness:** only under working CPU, timer, Pager, memory and broker resources. Successful Linux emulation cannot be promised when storage firmware lies, DMA hardware is uncontained, or the broker CPU budget is exhausted.

### 19.2 Future experiments (NONE performed)

1. Static ELF malformed-header/overflow fuzz with trusted native loader sandbox and read-only sacrificial test image.
2. Controlled CPU guest trap/return register tests: syscall opcodes, kernel-owned domain, sign extension, noncanonical RIP/RSP and FS/GS.
3. Basic static unmodified CLI candidate with exact syscall trace, version hash and x86_64 CPU/hardware profile.
4. Chosen glibc/musl dynamic loader/auxv/DSO/TLS/IFUNC and vDSO probes with versioned symbols and secure-exec cases.
5. Shared futex compare-and-block adversarial interleavings (lost wake, alias, owner death) and scheduler/rseq migration/signal abort.
6. Fork/clone/exec/wait ThreadGroup and PID reuse with signal mask, handler, SA_RESTART and malformed rt_sigreturn frame.
7. Guest FD dup/fork shared offset, CLOEXEC, namespace path escapes, proc magiclinks and openat2 RESOLVE_* races.
8. epoll ET/LT, ONESHOT, duplicate FD/file description and close/reuse race, pipe/terminal/SCM_RIGHTS.
9. Service death after native VFS/Network already committed; effect status, EINTR/restart and reconciliations.
10. CPU/IPC/Pager sponsor quota exhaustion, broker crash and M0 independence.
11. Security comparison with userspace emulator and raw in-kernel Linux ABI on same threat model and workloads.
12. Actual-reference-PC vs QEMU hardware-specific performance and verified compatibility reports, once later authorized.

**None of these are run now.** A test plan is not a test report or proof; no binary may be called supported until actually run on a named target and judged against the required behavior and security boundary.

## 20. Exit gates / Go-No-Go for design approval

**RFC-010 R1 research exit: OPEN / NOT PASSED.** Before considering detailed architecture Accepted, let alone code:
1. Demonstrate a coherent kernel-owned guest SYSCALL entry and *separate privileged broker* under direct jump/injection/pointer faults, or select an emulator/defer unchanged binaries.
2. Name one target Linux x86_64 ELF/libc/ld.so and obtain a complete expected initial-stack/auxv/TLS/ELF loader schema and test oracle.
3. Define guest process/ThreadGroup/FD/OpenFileDescription/PID/Namespace objects with exact race/close/restart ownership and sponsor.
4. Either prove basic futex atomic compare-and-block/shared alias behavior or exclude Linux threaded apps in published tier.
5. Model rseq registration and scheduler migration/signals, or prove chosen libc fallback under explicit unsupported.
6. Model clone/fork/exec and Linux signal frame/rt_sigreturn/SA_RESTART with actual guest machine ABI and interruptibility.
7. Specify FD/path/openat2 and epoll/socket/terminal features with native restricted cap policies and unavailable-feature error semantics.
8. Define exact VFS/Network effect commit, guest errno, EINTR/restart, copyout error and dirty file durability mapping with RFC-003/008.
9. Define CPU/IPC/VM/Pager resource and M0 recovery independence across broker failure and malicious quota exhaustion.
10. Compare LINUX-A/B/C/D with equivalent security assumptions and hardware/test numbers after explicit separate implementation Go.
11. Audit any imported C runtime, ELF parser, loader and guest root filesystem content license before reuse.
12. Publish compatibility tier/report format with explicit unimplemented features and revocation/upgrade behavior.
13. Review RFC-010 v0.2's §21 adversarial counterexamples and align RFC-009/018 IDL ABI before freezing any native or guest syscall representation.
14. **v0.2 P0:** model execve's image-destroyed/no-return failure and CLONE_FILES unshare, plus FD_CLOEXEC atomics and delegated cap leakage across concurrent fork/exec.
15. **v0.2 P0:** prove close/error/reuse, dup2/dup3 atomic replacement and epoll (FD,OFD) alias/close/reuse races at the guest–native broker boundary.
16. **v0.2 P0:** build per-syscall SA_RESTART/EINTR/partial-progress/timeout table and cover cases where native RPC already has an external effect.
17. **v0.2 P0:** demonstrate openat2 BENEATH vs IN_ROOT/magiclink/race error, and getrandom entropy/read-length/startup semantics against a real guest libc trace.

**Verdict:** LINUX-A is a good direction to research, but raw guest interception, futex atomics and process/signal semantics remain unproven. No source, compiler, runtime or guest implementation is permitted by this document.

**Next proposed research:** RFC-010 v0.1→v0.2 critical review is recorded in §21, but all P0 design exit criteria remain OPEN. Next study RFC-018 versioned IDL/status and Native/Guest ABI compatibility mapping, with joint RFC-008/009/010 FD/effect review. This design only, and D1–D8 remain the only broad Accepted directions.

## 22. Official primary sources, verified scope (rechecked 2026-10-24)

- **[S1]** Linux syscall(2) architecture calling conventions: https://man7.org/linux/man-pages/man2/syscall.2.html
- **[S2]** Linux execve(2), process image and credentials: https://man7.org/linux/man-pages/man2/execve.2.html
- **[S3]** Linux ld.so(8), dynamic ELF loader and search order: https://man7.org/linux/man-pages/man8/ld.so.8.html
- **[S4]** Linux getauxval(3) auxiliary vector: https://man7.org/linux/man-pages/man3/getauxval.3.html
- **[S5]** Linux vDSO(7) architecture-specific mapped ELF: https://man7.org/linux/man-pages/man7/vdso.7.html
- **[S6]** Linux futex(2), atomic compare-and-block: https://man7.org/linux/man-pages/man2/futex.2.html
- **[S7]** Linux clone(2)/clone3(2), shared VM and ThreadGroup: https://man7.org/linux/man-pages/man2/clone.2.html
- **[S8]** Linux signal(7), SA_RESTART, per-thread masks: https://man7.org/linux/man-pages/man7/signal.7.html
- **[S9]** Linux rt_sigreturn(2) saved user signal frame: https://man7.org/linux/man-pages/man2/sigreturn.2.html
- **[S10]** Linux epoll(7) ET/LT and queue behavior: https://man7.org/linux/man-pages/man7/epoll.7.html
- **[S11]** Linux openat2(2), RESOLVE_BENEATH/IN_ROOT/NO_MAGICLINKS: https://man7.org/linux/man-pages/man2/openat2.2.html
- **[S12]** Linux user_namespaces(7), UID0 scoping: https://man7.org/linux/man-pages/man7/user_namespaces.7.html
- **[S13]** Linux no_new_privs and exec security: https://docs.kernel.org/userspace-api/no_new_privs.html
- **[S14]** Linux seccomp BPF and non-sandbox warning: https://docs.kernel.org/userspace-api/seccomp_filter.html
- **[S15]** Linux restartable sequences (rseq) and CPU migration: https://docs.kernel.org/userspace-api/rseq.html
- **[S16]** Linux fsync(2) file versus directory durability: https://man7.org/linux/man-pages/man2/fsync.2.html
- **[S17]** Linux proc(5) pseudo-filesystem behavior: https://man7.org/linux/man-pages/man5/proc.5.html
- **[S18]** Linux mmap(2) shared/private mapping: https://man7.org/linux/man-pages/man2/mmap.2.html
- **[S19]** Linux capabilities(7) Linux credential rights: https://man7.org/linux/man-pages/man7/capabilities.7.html
- **[S20]** Linux pidfd_open(2), PID reuse-resistant process reference: https://man7.org/linux/man-pages/man2/pidfd_open.2.html
- **[S21]** Linux socket(7), socket behavior: https://man7.org/linux/man-pages/man7/socket.7.html
- **[S22]** Linux ELF(5), executable format: https://man7.org/linux/man-pages/man5/elf.5.html
- **[S23]** Linux epoll_ctl(2), registration and flags: https://man7.org/linux/man-pages/man2/epoll_ctl.2.html
- **[S24]** Linux close(2), FD release before later errors and retry warning: https://man7.org/linux/man-pages/man2/close.2.html
- **[S25]** Linux dup2/dup3 atomic FD replacement: https://man7.org/linux/man-pages/man2/dup.2.html
- **[S26]** Linux open(2), O_CLOEXEC descriptor inheritance race: https://man7.org/linux/man-pages/man2/open.2.html
- **[S27]** Linux getrandom(2) initialized entropy/blocking/nonblocking and partial returns: https://man7.org/linux/man-pages/man2/getrandom.2.html
- **[S28]** Linux execve(2) process teardown, CLONE_FILES and rare point-of-no-return failure: https://man7.org/linux/man-pages/man2/execve.2.html

Technical references describe existing Linux interfaces and not UniOS implementation or legal authorization to redistribute components. Implementation source, runtime versions, exact compatibility/ABI profile and compiler target are all future decisions.

---

**Document history:** 2026-10-24 v0.1 original initial research draft remains in Git. 2026-10-24 v0.2 adversarial review: execve post-PONR / sibling Thread and FD-table unshare, Linux close early FD retirement and retry risk, dup2/O_CLOEXEC atomicity, epoll (FD,OFD) with retained events after close, signal restart/partial transfers, openat2 flag precision and secure entropy getrandom. Design only; no kernel/ELF/personality/libc implementation, tests, performance, model proof or hardware work.

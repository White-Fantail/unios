# UniOS RFC-009 — Native ABI, System Call & User-Service Contract

> **Status:** Proposed — Specification v0.2 (critical adversarial re-review; NOT Accepted, implemented, frozen as stable ABI, benchmarked or formally verified)
> **Date:** 2026-10-24 (Pacific/Auckland)
> **Architecture:** [UniOS Architecture v0.2](../architecture/unios-architecture-v0.2.md); Accepted D1–D8 apply as broad direction only.
> **Primary detailed dependencies:** [RFC-001 Kernel/TCB](rfc-001-kernel-boundary-and-tcb.md), [RFC-002 Capabilities](rfc-002-object-capability-lifetime-delegation-revocation.md), [RFC-003 IPC](rfc-003-ipc-wire-protocol-and-synchronization.md), [RFC-004 VM/Pager](rfc-004-virtual-memory-pager-cache-and-oom.md), [RFC-005 CPU](rfc-005-scheduling-preemption-and-cpu-budget.md), [RFC-006 Boot](rfc-006-boot-trust-chain-service-startup-and-recovery.md), [RFC-007 Drivers](rfc-007-user-space-drivers-dma-and-iommu.md), [RFC-008 Storage](rfc-008-storage-filesystems-and-crash-durability.md), [RFC-017 Trust](rfc-017-verified-boot-updates-and-rollback-protection.md).
> **Nonimplementation order:** No kernel syscalls, syscall numbers, C headers, Rust ABI, ELF loader, libc, SDK, compiler target, POSIX guest, Windows/Wine layer, Darwin layer, tests or benchmarks are authorized. This is documentation and feasibility research only.

## 0. Executive recommendation — ABI-A conditional research profile

**ABI-A: a deliberately narrow native kernel ABI (KABI) with capability-enforced operations and minimal user-to-kernel entry conventions, versioned user-space service ABI/protocols (SABI), and a stable developer-facing C-compatible SDK ABI when evidence permits.** Linux, Windows/NT and Darwin execution ABIs live behind separately mediated userspace personalities and are not adopted as UniOS native syscall semantics.

Kernel enforces the boundary; userspace can choose convenience:
1. **KABI:** enter kernel as a uniquely identified native or restricted guest execution domain; validate typed Task-local handles, memory access, IPC transfer/commit, waits, faults, scheduling and privileged hardware objects. Kernel knows neither POSIX pathnames nor Win32/Mach service methods.
2. **SABI:** versioned user-service RPC protocols over bounded Channels, events and MemoryObjects. VFS, file sync, networking, UI, identity portals, package manager and recovery reporting belong here, subject to narrowly trusted service asset TCB.
3. **SDK ABI:** C-compatible stable convention candidate plus Rust wrappers, C++ bindings and a custom libc once separately proven. The Rust compiler's internal layout, panic strategy or unstable native Rust ABI is not a public cross-language contract.
4. **Guest ABI:** Linux x86_64 ELF machine ABI/syscalls, Windows PE/Win32/NT calls, Darwin Mach-O/Mach traps/library frameworks are **three distinct research tracks**, each with its own loader/runtime, emulation semantics and permission mapping. Native privileged KABI remains independent.
5. **No false compatibility:** POSIX **source port**, unmodified **binary guest compatibility**, and **guest OS virtualization** are different outcomes. An ELF file that starts does not mean Linux POSIX/process/thread/signal semantics work; Wine port does not imply every Windows app, and Darwin CLI experiments do not imply AppKit/Metal GUI support.

**Foundational distinction:** A process can see a syscall return SUCCESS and still not know whether a remote service processed a request, whether physical DMA ended, or whether a file survived power loss. RFC-002/003's NOT_COMMITTED / COMMITTED / UNKNOWN and RFC-008's scoped DurableReceipt constrain ABI labels and exception/timeout semantics. **No exact numerical syscall opcode, register allocation, wire header, public status enum value, handle bit layout or stable v1 ABI is chosen here.**

Compare:
- **ABI-A — direct typed KABI + versioned SABI + native SDK:** recommended research target; small Ring 0 semantics but requires reliable service discovery, ABI layering and robust compatibility bridging.
- **ABI-B — message-centric KABI only with pure user-space servers:** perhaps smallest trap surface and uniform tracing; hardware/VM/thread/fault objects still require secure privileged operations and performance/boot proof.
- **ABI-C — expanded kernel POSIX-like ABI:** simpler initial POSIX source port and raw syscall bridging, but enlarges kernel TCB and contradicts D5 without a strong measured exception.
- **ABI-D — Linux syscall ABI as native KABI:** quick limited CLI compatibility on one architecture but makes Linux semantics a permanent kernel-facing policy dependency; not recommended absent compelling proof.

**v0.2 critical corrections:** a per-Thread kernel-owned guest tag is insufficient when guest code can jump into trusted broker executable memory or reach native-capable thread state in the same Task; basic Fuchsia zx_channel_write **consumes transferred handles even on failure**, unlike UniOS candidate NOT_COMMITTED; receiver byte copyout and cap install need a protected claim/escrow proof; identical recycled handle integers defeat purely internal generation checks; Linux futex compare-and-block and rseq need atomic scheduler/memory semantics; capability-bearing SABI version negotiation must fail closed.

**P0 blockers:** raw x86_64 native vs Linux guest entry identification cannot rely on userspace-controllable registers or one library shim; input/output copy and handle transfer must have clear atomicity; pending reply, timeout and cancel cannot invent rollback of remote effects; 64-bit data layouts and future 32-bit guests must not get confused; service/IDL version updates cannot grant new capabilities; bootstrap/rescue must run with the minimal supported ABI even when ordinary services fail.

## 1. Goals, threat model and policy ordering

### 1.1 Goals

- Minimize and precisely bound stable kernel-entry mechanisms while allowing the desktop SDK and services to evolve.
- Specify what can be trusted from raw user registers, addresses, bytes, handle integers, CPU context and registered service endpoints.
- Separate kernel object identification, authority, allocation sponsor, revocation lineage and physical resource reclamation.
- Give IPC/Wait/VM/Thread/IRQ syscall families a clear no-partial-privilege and no-lying-success contract.
- Support language-neutral developer tooling without treating Rust internal structures as the ABI or exposing privileged untyped pointers.
- Support optional Linux CLI personality first without turning its syscall set into the kernel's own ABI; Windows/Darwin compatibility remains separately gated.
- Specify monotonic clock/deadline, interrupts, exceptions, service reincarnation and uncertain remote side effects across reboot/restart.
- Enable versioned extension/feature negotiation and documented compatibility commitments after selected implementation/hardware proofs.
- Keep early BootBundle and M0 wired environment independent of late-bound file/libc/dynamic loader.

### 1.2 Threats

A malicious Task emits raw SYSCALL/INT, edits syscall argument buffers while kernel reads them, forges/stales a handle number, guesses privileged object IDs, races COPYIN with VM unmap, passes alignment/length overflow, causes user page fault while kernel locks held, races revocation with handle MOVE, sends oversized/unknown SABI fields, floods waiters, crashes after IPC commit, fabricates a sender badge, spoof service registry entry, causes use-after-free through descriptor ABA, asks guest Linux root for native hardware rights, and toggles user FS/GS/stack flags to bypass guest-entry policy.

A compromised M0/loader, firmware/SMM, real IOMMU, DMA before verified boot, PCIe rogue device and malicious disk firmware are separate trusted-hardware and TCB assumptions; KABI permission checks alone cannot repair damage they already performed.

**Priority:** authority integrity and memory safety > correct failure/commit semantics and data recovery > TCB simplicity and testability > developer ergonomics > IPC latency > external syscall convenience. Low syscall count alone does not prove small or safe kernel.

## 2. Official architectural precedents

| Primary source | External technical fact | UniOS design consequence |
|---|---|---|
| seL4 IPC tutorial [S1] | IPC uses endpoints, labels/badges, receive/send, grants and optimized fastpath with precise constraints | distinguish kernel-asserted badge from user payload; don't promise fastpath with arbitrary cap transfers |
| seL4 Manual [S2] | capability/CSpace, Thread, Endpoint, Reply and fault handling have explicit kernel operations | small capability primitive family plausible; seL4 API/ABI not copied automatically |
| Fuchsia Zircon handles [S3] | process-local typed handles have rights; duplicate/replace/close, transfer in channels | model Task-local handle integer as locator, not globally meaningful transferable credential |
| Fuchsia Channels [S4,S19–S21] | basic zx_channel_write consumes all supplied handles **on success or failure**; write_etc has disposition-specific operations; read_etc exposes type/rights and size/discard options | UniOS must choose own NOT_COMMITTED sender-ownership rule and bounded receive escrow, never assume Fuchsia failure contract equals it |
| Fuchsia async waits [S5] | objects may register async wait packets; multiple subscriptions can exist and consume resources | WaitSet quota, registration generation, edge/level/cancel semantics need precise promises |
| Fuchsia FIDL flexible types [S6] | strict/flexible unknown fields, methods and resource/handle wire metadata support evolving clients | separate SABI IDL unknown-field handling from KABI small envelope, strict unknown handles cannot leak |
| Linux syscall ABI (arch-specific) [S7] | syscall interface differs by architecture; Linux stable syscall set is distinct from POSIX library API | Linux x86_64 guest dispatch belongs to compatibility personality, not UniOS-native number table |
| Linux syscall(2) x86-64 [S8] | x86-64 Linux passes args in rdi,rsi,rdx,r10,r8,r9; calling convention not identical to ordinary C function | separate syscall machine convention from C SDK ABI and define guest register snapshot |
| Linux seccomp user notification [S9,S22] | filtering is not a sandbox; user notification has argument memory TOCTOU, ptrace and tracer lifecycle hazards | use only as interception analogy, not as native hardware syscall trap or capability authorization |
| FreeBSD Capsicum [S10] | descriptors become rights-bearing capabilities in hybrid POSIX environment | a guest FD/proxy can carry attenuated native FileCaps; UID0 remains not native root |
| Windows NT Ntdll API docs [S11] | user applications typically call Ntdll Nt entries; undocumented Native ABI isn't stable Win32 API contract | Windows compatibility should target named documented APIs/loader semantics before native NT syscall numbers |
| Linux ABI stability docs [S12] | kernel userspace ABI has explicit stability tiers, syscalls generally append-only | define future stable vs experimental UniOS ABI claims independently, not assume all prototypes stable |
| Fuchsia wait/handle docs [S13] | close invalidates handle and can cancel associated waits, peer closure has separate signaling | choose explicit wait invalidation and peer death race semantics |
| Fuchsia FIDL wire format [S14] | a transferred handle's on-wire placeholder is not the sender's integer handle value | never treat serialized handle number as access credential |
| Linux io_uring [S15] | asynchronous ring submission/completion carries complex buffer and lifecycle rules | optional batch/ring transport only after simple syscall+Channel safety model proven |
| Linux futex(2) [S23] | atomically compare guest-shared memory value and block against concurrent writer/wake | naive two-RPC read-then-sleep loses wakeups |
| Linux rseq [S24] | per-thread CPU/scheduler and userspace memory ABI across migration/signals | guest compatibility is not only syscall forwarding |
| Linux clone(2) [S25] | thread group, signal state, shared VM and TID relationships are coupled | can't substitute TaskCreate once for clone/fork |
| POSIX/Linux fsync and errors [S16] | completed write and successful fsync require distinct semantics and directory persist for namespace | SABI FS error maps to native receipt and errno personality, not a kernel fsync syscall |

**External source limits:** seL4's formal proofs, Zircon's ABI, Linux compatibility promise, FreeBSD's Capsicum and Microsoft API guarantees do NOT automatically become UniOS achievements. FIDL version-evolution techniques may be relevant but importing FIDL schemas or Zircon syscalls would require independent technical/licensing decision. No implementation or numeric performance evidence.

## 3. Three separate binary interfaces and one guest boundary

### 3.1 Separation diagram

~~~text
Native Rust / C / C++ application
      -> Native SDK stable C-compatible ABI candidate
      -> Userspace support runtime
          |               |
          | KABI trap     | SABI over KABI Channel
          v               v
   UniOS Kernel       VFS/Net/GUI/Portal/Recovery/etc. service
    cap/MMU/CPU           policy + file/path/sockets/SDK semantics
    IPC/Wait              managed by own versioned protocol
          ^
          | Native KABI dispatch after kernel-owned domain check
          |
  Linux ELF guest -> Guest Entry/Rewriter/Trap Control -> Linux Personality
  Windows PE/Wine -> Win32/NT userspace -> Native services
  Darwin Mach-O   -> experimental Darwin userspace/personality
~~~

The diagram does NOT assume Linux raw SYSCALL becomes a user-level message automatically: guest trap interception/routing and register/return ABI are themselves unimplemented P0. A Windows or Darwin executable may need a loader, dynamic linker, C/C++/Objective-C/Swift runtime, OS services, exception and entitlement compatibility as well as syscall mapping.

### 3.2 KABI scope

KABI owns only trusted kernel objects and transitions: Task/Thread/AddressSpace, MemoryObject/PagerBinding, CapabilityTable/RevocationController/ResourceBudget, bounded Endpoint/ReplyToken/Event/WaitSet, timer/IRQ/DeviceGroup/IOMMUDomain under typed grants, Task control and fault/exception primitive. A kernel object type set may evolve before stability; unknown object IDs/feature flags never mean "kernel silently cast type."

KABI explicitly does NOT parse pathnames, JSON/FIDL application RPC, POSIX inode/data format, Win32 handles or NT object names, Linux signals/errno, Mach port policy, GUI window trees, user-facing permission dialogs, package indexes, update repository policy, firmware key management or full device command queues.

### 3.3 SABI scope

SABI handles service methods and their versioned user-space schemas, transaction/request IDs, service incarnation, semantic error families, durable FileSync/DirectorySync, sockets/net addressing, app permission portal, update status and UI compositor. Kernel transports opaque bytes and typed cap vectors with length/rights accounting; service is obligated to validate its own methods, unsupported versions/fields, unknown cap roles and authorization. Service name discovery alone is not grant issuance.

### 3.4 SDK scope

Initial native SDK should offer a C-compatible ABI candidate (plain fixed-width scalars, declared alignment, opaque handles and error type at boundary; no C++ objects or Rust repr-default types), with Rust-safe wrappers and optionally C++ RAII binding and libc/POSIX source port later. Public source API need not expose raw syscall operation names; stable SDK version and stable KABI are separate promises. Rust panic unwind across C ABI boundary is forbidden unless an explicitly supported safe convention and runtime contract is proven.

### 3.5 GABI/guest boundary

A Linux ELF's libc expects Linux syscall numbers, thread-local errno, signals, TLS, fork/exec, futex, blocking and restart behavior, /proc, dynamic linker and host file and network semantics. Linux futex's **compare-and-block is atomic** against concurrent memory updates and wakeups [S23], not a read-RPC then wait-RPC; Linux rseq registers a per-Thread user/kernel state coupled to migration and signal delivery [S24]; clone flags couple shared address space, signal handlers and TIDs [S25]. Translating one syscall number does not confer Linux binary compatibility. Windows PE needs Windows loader DLL dependencies, Win32/NT APIs, SEH/TLS, handle ownership, registry, GUI, Wine feasibility. Darwin Mach-O needs Mach traps, dyld, Objective-C/Swift ABI, Apple framework/licensing gates. None should automatically grant native DeviceGroup, BootPolicy, TPM floor or guest UID0 global caps.

## 4. Entry/exit x86_64 and architectural domain separation

### 4.1 Entry gate, not fixed numeric syscall ABI

Native Ring 3→Ring 0 likely uses x86_64 SYSCALL or architecture-controlled exception vector; IRQ, page fault, exception and service IPC are distinct paths. Research whether the design should use SYSCALL with versioned opcode and pointer to bounded descriptor, or a small fixed register set, while managing CPU GS/TLS bases, kernel stack switching, entry state, ASID/PCID/CR3, preemption, RFLAGS masking, speculation mitigations, XSAVE/FPU context and canonical user address validation.

SYSRET has canonical return-address pitfalls and architecture-specific flags; an IRETQ or safe alternate return may be necessary. Never accept user-provided kernel stack, caller privilege, guest/native selector or raw object address merely because it appears in RAX/RDI or SDK wrapper. Raw syscall can bypass library checks; the kernel validates every invocation.

**Machine ABI ≠ C ABI:** exact syscall instruction, opcode register, arguments, return/status registers and clobbers must be specified per arch after proof. The Linux x86-64 syscall ABI uses a fourth argument in r10 rather than ordinary C function's rcx [S8]; copying Linux's calling convention into UniOS is not required. Do not publish fixed numeric register assignments, syscall IDs, struct offsets, vDSO structure or user mode flags as Stable v1 yet.

### 4.2 Native vs restricted guest P0 execution-domain problem

At each kernel entry the authoritative execution domain is **kernel-owned per-Thread/per-Task state** established by a privileged loader/control transition; it must not be inferred solely from registers, ELF note, a service header, a writable TLS flag, a CPU syscall instruction or an untrusted compatibility library. **v0.2:** kernel-owned metadata cannot protect a trusted broker from a malicious jump into its code when both share executable memory and native-capable cap tables. Default restricted Linux guest and native privileged broker must occupy **different Task/AddressSpace trust boundaries**; G2 co-location remains unadmitted unless in-process instruction/memory/cap isolation is independently proved.

Candidate designs to compare:
- **G1 preferred baseline, per-Task fixed GuestDomain:** guest syscall entries route through immutable kernel-owned GuestDomain, while the native-capable broker lives in a **separate protected Task and address space**, holding only typed grants needed to act for a request. No guest-controlled executable jump can become an authorized broker entry.
- **G2 conditional countermodel, per-Thread Native/GuestEntry state:** kernel-owned state alone fails if guest bytes can run native broker code. Requires distinct enforced in-process protected region/cap compartment, non-jumpable privileged transition, reentrant signal and context proof; **not an eligible default** merely because the kernel stores a flag.
- **G3 restrict guest to a userspace interpreter/emulator:** guest instructions/syscalls go through explicit host service, perhaps slower yet simpler kernel; cannot claim native hardware-speed Linux binaries.
- **G4 direct Linux syscall dispatch in kernel:** ABI compatibility easier superficially, but requires Linux-specific semantics/TCB and native/guest collision rules; not default.

**Mandatory attack example:** a guest sets its RAX to a UniOS privileged syscall number or jumps into a native syscall shim, attempts privileged native CapFactory or DeviceMap. More strongly, with G2 sharing an AddressSpace, guest code can jump directly to native broker instructions and use that privileged Thread/CapTable unless actual execution and authority isolation exists. Default G1 segregates the privileged broker into another Task. Kernel must still dispatch according to its trusted guest-domain label and deny native-only operations; if native broker runs on same Thread, it requires a validated state-switch authorization not a plain writable flag.

**Entry-time TOCTOU:** guest state cannot change between validating current execution domain and the actual dispatch / user-pointer address-space selection. Guest-to-native return must restore intended FS/GS, exception ABI and caller caps without allowing an untrusted guest frame to overwrite privileged continuity.

### 4.3 New architectures and 32-bit guests

x86_64 UEFI first; possible AArch64/RISC-V later need separate trap conventions. Do not encode x86 canonical-pointer/address-size rules as universal C SDK contract. 32-bit Linux guest may require pointer extension/sign rules, alternate calling convention, x32 distinct syscall encoding and time64 structures. Do not accept a guest length field that wraps at 32 bits and authorize 64-bit kernel mapping.

## 5. Type/size/copyin/copyout ABI discipline

### 5.1 Untrusted memory

Every syscall argument from userspace is untrusted even from an authenticated application binary. Validate pointer canonical form and mapped user address/range, alignment, bitfield/reserved values, length and count multiplication overflow, overlap, straddling page boundaries, array count, type tag and required version/size fields. Never dereference a raw pointer while holding a global kernel scheduling/object/IRQ lock that cannot tolerate a page fault. Copyin may itself fault or block on Pager, so design a bounded unwinding/error path or pre-pin/snapshot as appropriate; no unbounded recursive Pager dependency under M0.

A user can concurrently modify memory through another Thread, mapping alias or DMA even if the first length check passed. Snapshot structures and cap descriptors to bounded kernel memory before validating semantics, or use justified pin+generation with no assumed immutability of writable pages. A pointer validation pass is not a second-use guarantee. Buffer lifetime during async work cannot rely on a raw Task VA after the Task unmapped or died.

### 5.2 Plain C-compatible record candidates

Only as concepts: fixed-width unsigned/signed integer fields; explicit endianness and alignment per arch; opaque handle scalar type (not pointer or public object ID); version/size/tags; reserved fields zero and future extension discipline. No unchecked nested pointer graphs; explicit data-len/handle-len with upper bound and checked sums; no Rust Vec, trait objects, native enums with unspecified discriminant, compiler-specific bitfields or untagged pointers in public binary structs. Use simple per-call input descriptor with copied scalar fields and separately validated buffers.

Structures may use size+version in early experimental APIs; this is not an implicit promise that any future struct tail can be ignored. **Unknown required fields and rights must be rejected**, unknown optional fields only if a defined negotiation rule permits safely skipping them. Return actual processed lengths/feature masks only with clear size/version semantics.

### 5.3 Copyout, effect and error atomicity

A kernel syscall may commit an effect **before** its result can be copied to a user output pointer. If copyout fails after commit, "INVALID_BUFFER means nothing happened" is false. Three candidate ways to prevent lying:
1. Fully validate/prepare output and reserve handle slots before irreversible commit; still consider concurrent user unmap/copyout faults after preparation.
2. Deliver a small status and stable opaque resulting handle through a carefully specified atomic register/handle-install point, but x86 and concurrency constraints need proof.
3. Expose a distinct **COMMITTED_BUT_RESULT_UNAVAILABLE / UNKNOWN** completion/report/receipt and a bounded query/reconciliation mechanism where supported.

**Safety requirement:** no kernel privilege may be minted twice and no foreign handle leaked regardless of copyout fault. An object may be created with charges even if user fails to receive its handle; ownership, cleanup and status must be defined without pretending it never existed.

### 5.4 Numeric/error namespace

KABI logical families: INVALID_HANDLE, WRONG_TYPE, ACCESS_DENIED, REVOKED, STALE_INSTANCE, INVALID_USER_ADDRESS, INVALID_SIZE/ALIGNMENT, QUOTA_EXCEEDED, WOULD_BLOCK, DEADLINE_EXPIRED, CANCELLED_LOCAL, PEER_CLOSED, BUFFER_TOO_SMALL, OUT_OF_MEMORY, UNSUPPORTED_FEATURE, INTERRUPTED, NOT_COMMITTED, COMMITTED, UNKNOWN_EFFECT, HARDWARE_UNAVAILABLE and FATAL_PROTOCOL_VIOLATION.

An error number must distinguish caller-side invalid input from external side effects and denied authority; timeout does not mean remote cancellation. Linux errno, NTSTATUS and Mach error numbers are **not** direct kernel-native status values; user-space guest layer must translate preserving semantics or report Unsupported. Any public integer values and bitmask mappings are deferred to RFC-018.

## 6. Capability/object handling and lifetime

### 6.1 Kernel handle table

Task-local opaque Handle is a lookup token validated with kernel type, rights, lineage, sponsor and object generation (RFC-002). Duplicating numeric handle bits to another Task never transfers authority. Copy/attenuation requires a typed CREATE/DUP/DERIVE grant, not simple integer arithmetic; owner/sponsor/revocation controller aren't implied equal. **Handle close R0** only drops one reference. It does not guarantee subtree revoke B, CPU TLB cutoff R1, DMA quiescence R2-F or final object-wide R2-O reclamation.

### 6.2 Cap operations (logical inventory, not function signatures)

| Family | Candidate operation class | Distinct safety contract |
|---|---|---|
| Handle | CLOSE, DUPLICATE_ATTENUATED, MOVE, QUERY_TYPE_RIGHTS | rights cannot amplify; Task-local values/ABA prevented |
| Capability | DERIVE_SCOPED, REVOKE_B, WAIT_FENCE_R1/R2_F/R2_O | no claimed synchronous complete revoke if hardware pending |
| Creation | CREATE_OBJECT_AUTHORIZED, RESOURCE_ACCOUNT_QUERY | creation permission and sponsor budget separate from holding object |
| Task/Thread | TASK_CREATE, THREAD_CREATE/START/STOP, DOMAIN_ENTER_AUTHORIZED | no ambient exec privilege; controlled initial cap envelope |
| AddressSpace | MAP/UNMAP/PROTECT; MemoryObject create and share | aligned bounded range, W^X, page fault, CPU TLB R1 |
| Pager/Fault | BIND_PAGER, SUPPLY_PAGE, FAIL_RANGE, FAULT_WAIT | no foreign waiter wake or Pager spoof; kernel-private FaultWaiter |
| IPC | CREATE_ENDPOINT, SEND/MOVE, RECEIVE, CALL, REPLY | queue commit, ReplyToken, credits and peer generation |
| Wait/Time | WATCH/UNWATCH, WAIT_EVENT, READ_MONO_CLOCK | finite registrations, clock ID and deadline semantics |
| Scheduling | QUERY_CHARGE, SET_POLICY_AUTHORIZED | cannot mint CPU reservation by renaming Task/handle |
| Device/IRQ | IRQ_BIND/MASK/ACK, MMIO_MAP, IOMMU/DMA_MAP_AUTH | physically verified group; no raw BDF/PFN authority |
| Bootstrap/trust | GET_BOOT_FACTS_ATTENUATED, OBSERVE_TRUST_STATUS | ordinary app cannot set floor, Good or firmware dbx |

All names are identifiers for design discussion, not syscall names/numbers. A small opcode-dispatch kernel is not safe merely because the operations are grouped under one multiplexed syscall; parser complexity and rights validation matter more than counted trap instructions.

### 6.3 Capability transport, transfer and ABA

For MOVE, kernel must atomically move live typed authority from sender table to in-transit kernel queue or receiver table under exactly one defined commit. UniOS E1 candidate: **provable NOT_COMMITTED preserves all source handles and rights; COMMITTED removes them into transit**. This is not Fuchsia basic zx_channel_write's failure behavior [S19]. Raw 32/64-bit user-side handle integer is Task-relative; SABI on-wire payload reserves a role/slot and **separate kernel handle-vector transfer**. Validate duplicated handles, derivative rights and destination receiver quota before commit. If delivery is impossible, define whether caps remain enqueued, move back or drop under verified contract; never silently duplicate source+destination authority by retrying.

A closed handle's **same externally visible integer must not reauthorize a different object during the same Task lifetime when an old copy can still be used**. Checking an internal table-generation only after a reused identical integer is presented is insufficient. Require nonwrapping public-generation token or non-reissue during Task lifetime with fail-closed exhaustion; this aligns RFC-002 §7 without freezing handle bit format. Kernel generation/lineage and server incarnation are independent; FIDL/Zircon semantics can inform tests but do not define UniOS default atomics [S3,S4,S14].

## 7. IPC and RPC transition semantics

### 7.1 Kernel E1 bounded Channel choice

Per RFC-003 IPC-A: small bounded asynchronous Channel, E1 single-owner endpoint study, one-shot ReplyToken, Event/WaitSet W1/W2 study and optional shared MemoryObject for bulk data. Sender data plus MOVE grants either atomically commit into queue or do not; receiver delivery/copyout/install are later separately failure-prone stages. The SABI method/version header remains opaque bytes to kernel. Not all Channels behave as a general synchronous POSIX FD; no universal read/write semantics are promised.

### 7.2 Four independent phases

~~~text
T0 sender: validate type/rights/budget; snapshot bytes+move descriptors
T1 kernel: E1 queue ENQUEUE + transit handles COMMITTED or NOT_COMMITTED
T2 receiver: dequeue + message copyout/receive handle-install (own commit)
T3 server: semantic work (file/network/GUI/etc.) maybe has remote effects
T4 reply: one-shot ReplyToken result delivered OR timeout/death/revoke
~~~

At T1 COMMITTED does not imply T2 delivery, T3 execution, T4 reply, durable file or hardware result. A local deadline at T4 may expire while server still processes T3. Canceling the Wait/ReplyToken is not rollback; peer death is not "never did work." If kernel cannot establish which phase happened from a lost fault/IPC reply, report UNKNOWN_EFFECT and require app/service-specific idempotent reconciliation, not an automatic retry of external operations.

### 7.3 ReplyToken and nested calls

Kernel-minted ReplyToken binds initiating Call, caller, service endpoint incarnation/generation and one valid reply grant. It must resist forging from request ID bytes, duplicate replies, stale generation or teardown. **Caller timeout** may release local wait and reply authority, but cannot revoke already committed remote storage work. Nested synchronous call chains can deadlock if the target Pager or server is throttled/blocked; a reply capability is neither a CPU scheduling budget donation nor progress guarantee (RFC-005).

### 7.4 Buffer-too-small and receive copyout

**Receive linearization correction:** user buffer preflight cannot make data bytes copyout and handle-table installation one atomic user-visible transaction. Partial user memory writes may be observed even if the cap install/message removal later fails. RFC-003 §6's bounded **exclusive ReceiveClaim/escrow** is the preferred unproved countermodel, with precise claim/abort/commit and cleanup; register-only delivery is an alternative for small messages. Never call copy-before-commit secure when an unauthorized receiver may glimpse secret bytes.

An incoming message may have data+cap vector larger than receiver's buffers. Define exactly whether message stays queued, is explicitly discarded, or moves to a terminal error with returned handles; do not claim successful receipt then lose security-relevant caps. Need a preflight query/read-size or bounded reservation to avoid repeated malformed input queue pinning. A malicious receiver may intentionally refuse to install cap handles; its resource charges and transit cleanup must remain bounded (RFC-003).

## 8. Event, WaitSet, timeout and asynchronous wake

- Event notifications differ from queued messages: counts may coalesce or be lost depending level/edge contract. No generic Event=semaphore guarantee.
- WaitSet subscriptions are individually owned kernel registrations with rights, quotas, object/registration generations, bounded cancellation; duplicate registration is not automatically deduplicated [S5].
- A Wait completion must describe selected clock (Monotonic vs BootTime, if any), absolute deadline or relative duration with overflow behavior, allowed timer slack and exact local completion ordering. Guest Linux clocks map at personality boundary, not by casually assigning clock IDs.
- Cancellation/death/unmap/timeout/peer-close/IRQ can race. Define a linearization point for Wait cancellation and event selection; no stale wake/foreign endpoint generation.
- Signal one Event can wake one, many or coalesce according to documented per-type policy; userspace must not accidentally infer an OS-wide condition variable or POSIX futex semantics.
- A triggered Wait does not grant authority to operate on the signaled object; actual handle rights checked on subsequent invocation.
- Ordinary Event/WaitSet cannot consume wired M0 reserved queue/IPC/CPU resources without a deliberate authenticated grant.

## 9. Memory/Pager and fault interface contract

The kernel fault path uses a **kernel-private per-Thread FaultWaiter** and scoped PagerBinding to authorized MemoryObject and VA range, not a general user-produced resume pointer. Fault event to Pager is tied to Task/Thread, VA, access class, backing identity and generation. Pager can provide only authorized PageSupply or fail range; it cannot wake a foreign FaultWaiter, map arbitrary physical frames, or gain kernel grants via a user-selected FaultID.

KABI Map/Protect/Unmap must define R1 CPU TLB fence result, including concurrent CPUs/guest domain. It cannot return full DMA R2-F merely because CPU PTEs changed. Page pin lifetime and IOVA after unmap require RFC-007 typed grants and hardware fence. File mapped pages, dirty generations, FileSync and storage DurableReceipt are SABI + RFC-008, not kernel inode syscalls.

A kernel copyin which faults can recurse into user Pager; never demand-page the wired recovery executor/Pager's own stack, locks, queues or essential backing. Allocation/copyout failures must be distinguishable from already committed effects and protect all referenced frames/sponsors during teardown.

## 10. Scheduling/Thread and exception boundary

- Native Thread scheduling charge, CPU runtime and group cap are independent from wall-clock deadlines and priority weight (RFC-005); syscall dispatch/retry does not mint new CPU budget.
- A syscall blocked awaiting Pager/WaitSet/IPC is not runnable; its quota may be depleted simultaneously. Thread wait state, scheduling eligibility and CPU owner are orthogonal and race-proof.
- Trusted privileged config controls CPU reservation, not app's preferred numeric Priority or guest process nice value.
- Kernel faults, page fault, illegal instruction and privileged exception have typed fault event and safe Task/Thread death handling. POSIX signals, Windows SEH/VEH and Darwin Mach exceptions are guest personality semantics; no kernel reinterpretation of untrusted signal structures as native privilege.
- Thread entry/exit, TLS FS/GS, stack alignment/red-zone, vector/SIMD lazy save, FPU/XSAVE/CET and CPU feature mapping need reference-CPU evidence before ABI freeze; no unverified assumption that all x86_64 binaries use the same runtime/stack conventions.
- Native app program start is distinct from kernel Thread start: executable format parsing/relocation/DSO linking, runtime initialization and process policy generally in a trusted user loader, with signed BootBundle loader boundary separate.

## 11. Version/feature negotiation strategy

**Version separately:**
- KABI machine convention and kernel object operation revision (slowly evolved).
- SABI protocol ID/method ordinal, payload version, strict/flexible unknown semantics, handle role and service incarnation.
- Native SDK C ABI symbol/version/headers and Rust wrapper crate interface.
- Executable/loader ABI, ELF/Mach-O/PE personality and runtime features.
- BootPolicyManifest/BootstrapDescriptor, filesystem DataVolume schema and local security floor (RFC-017).
- Architecture CPU feature set and optional device/IRQ capability support.

Recommend future feature discovery via bounded kernel query returning interface generation and explicit supported operation flags; unknown mandatory features return UNSUPPORTED rather than silently being ignored. New optional SABI fields must not widen authority or transfer an unknown capability into a service accidentally; strict/flexible evolution is per protocol/field, with handle resource semantics and unknown enum checks. Multi-version compatibility cannot be achieved by assuming every struct's future appended tail is safe to ignore.

**No frozen v1 promise now:** conceptually use Experimental ABI generation and SDK compatibility gates in design proofs; changing syscall numbers/layout before acceptance is permitted and expected. When Stable ABI is declared, specify compatibility window, architecture coverage, deprecation and security-necessary exception process. Never call an untested snapshot of the current doc "binary-compatible".


## 12. Native userspace runtime, executable formats and toolchain feasibility

### 12.1 Native C ABI research profile

The candidate SDK boundary is a plain C-compatible function calling convention per target architecture and toolchain: fixed-width scalars, explicit sized records, opaque handles with owned/borrowed/duplicated rights, no allocator cross-boundary free without contract, no exception or panic unwinding across FFI without separate runtime policy. Define which DLL/shared library and symbol/visibility/version mechanism will exist on UniOS, not pretend Linux .so, Mach-O dyld or Windows PE export structure is magically available.

**ABI features to prove before stable native SDK:**
- C calling convention, stack alignment and red zone, register save rules and variadic calls; x86-64 SysV is candidate only after deliberate choice, not assumed synonym for Linux KABI.
- TLS access and per-Thread data, errno-equivalent SDK state, thread creation/exit and signal/event dispatch.
- unwind tables, setjmp/longjmp, C++ exceptions, Rust panics, async executor stack/future state; no cross-domain escalation or arbitrary kernel unwind.
- compatible symbol mangling rules and C++ ABI (Itanium vs alternatives) only under selected compiler/toolchain; Rust extern C is not a promise of Rust repr or crate binary stability.
- usable static ELF first; dynamic loader, relocation, RELRO, ASLR, W^X, ELF TLS/IFUNC and library dependency resolution are independent technical gates.
- libc/sysroot/compiler builtins/start files, linker scripts, trusted target metadata and signed releases/cap manifest; do not assume a stock Linux target triple provides UniOS behavior.
- standard native filesystem/network/window APIs delivered through SABI, not direct privileged KABI shortcuts.

### 12.2 Loader authority separation

Boot M0 verified loader from RFC-006/017 may have more initial grants than an ordinary userspace application loader; signing an ELF image is not authorization to mint a new Task's privileged caps. App creation needs authorized executable content identity, controlled AddressSpace/Thread creation, verified SDK/runtime if policy demands, and a scoped capability bootstrap list that cannot include a guessed issuer/root handle.

A native static program can be compiled and launched before a full dynamic linker/libc exists *if* necessary sysroot, entry point, TLS and syscall support is deliberately supplied. This is feasibility, not proof that any compiler target has been created. Compatibility loader selection must be a trusted policy decision, not arbitrary magic bytes that switch kernel dispatch security domain.

### 12.3 Compat apps do not inherit native trust

A Linux guest's effective UID0 may control its guest namespace but doesn't imply native root. Windows Administrator token and Darwin entitlement likewise are translated guest constructs. Guest launch must be given precise File/Directory/Network/Display/Process grants with brokered access. A guest can possess authorized native capabilities via a host broker only when independent native policy issues them; it cannot create them by translating a numeric FD, HANDLE, Mach port name or guessed PID.

## 13. Linux x86_64 CLI compatibility as a separate GABI research program

### 13.1 Launch versus syscall

A restricted Linux ELF may include interpreter PT_INTERP, Linux auxiliary vector, ELF TLS, environment and initial stack, glibc/musl dynamic loader dependencies and vdso/vsyscall expectations. First study a constrained static binary with documented dependency set, then a dynamically linked CLI, signals/threading/mmap/futex/epoll, then complex apps. No unmodified Linux binary claim until actual guest ABI behavior is tested.

Linux x86-64 SYSCALL number in rax and arguments in rdi/rsi/rdx/r10/r8/r9 are the Linux ABI (not UniOS native ABI). A plain CPU trap doesn't carry "Linux personality" automatically. Native kernel must use kernel-owned guest-domain entry and route required calls through a restricted handler, with no privilege escalation and explicit return/cancellation behavior.

### 13.2 Personality workflow candidate

~~~text
Guest ELF loader authenticates/authorizes restricted guest Task
 -> Kernel tags Thread/Task GuestDomain and allowed NativeBroker cap ceiling
 -> Guest issues Linux x86-64 syscall ABI
 -> Kernel intercepts based on trusted GuestDomain, saves full guest context
 -> Restricted syscall forwarding/trap broker in a SEPARATE protected Task/AddressSpace
 -> Linux Personality service translates Linux syscalls to native scoped KABI/SABI
 -> complete guest return registers/errno, signal/restart and side effects
 -> kernel restores guest domain, denying any unauthorized native entry
~~~

**Not a statement this can be implemented cheaply:** user-mode intermediary needs a privileged trap redirection, safe guest register/TLS/signal and futex context, possible architecture emulation, and handling of arbitrary guest syscall instructions. A safe solution may require either a kernel-contained small shim (larger KABI TCB), hardware virtualization, binary rewriting, user-level emulator or independent control gate. RFC-001 G1/G2 approaches need joint proof. A userspace broker alone cannot intercept x86 SYSCALL from an uncontrolled guest without kernel/hardware participation.

### 13.3 Kernel/user semantics trap

Linux fork/clone/execve/wait, process IDs, namespaces, signal delivery/restart, futex atomic wait/wake with mapped addresses, io_uring, epoll, fsync/fdatasync, socket flags, ptrace/procfs/mount and thread credentials cannot be represented merely as one RPC each. Race and atomicity between guest address space, guest FDs, signal handlers, shared-memory fast paths and kernel scheduling must be checked. Linux seccomp filter is not a complete sandbox, and untrusted guest user-notif request may already have side effects; never infer Unix privilege model substitutes for UniOS native cap enforcement [S9].

**Compatibility ladder (research gates, not promises):**
- L0 ELF parse/validate static byte layout and CPU entry without guest syscall behavior.
- L1 constrained static CLI: exit, write to scoped stream, time, basic read from brokered file with explicit syscall table.
- L2 loaded ELF program, dynamic loader, auxv, TLS, libc startup, mmap and file IO.
- L3 threading/futex/signals/fork/exec/wait and /proc assumptions.
- L4 sockets/epoll/io_uring/namespace/host tools and broader distributions.
Each level requires actual executed tests and a published supported syscall/semantic list on the selected hardware; none has yet been performed.

### 13.4 Security and perf counterexamples

Guest process passes rax=native-CapFactory index, points argument to kernel-sensitive memory, or maps a writable SDK stub: trusted domain routes/rejects; raw user pointer never creates grant. If a guest signal arrives while blocked in syscall broker and handler calls another Linux syscall, guest ABI restart/stack/domain must be preserved without leaked native Thread-control cap. If native broker sleeps waiting on guest Pager whose pages require the broker itself, deadlock occurs; M0/critical broker code must use separately admitted resident path or fail boundedly. If guest timeout triggers while VFS already committed rename, return semantics cannot fabricate NOT_COMMITTED.

## 14. Windows, Darwin and virtualization — nonnative guest interfaces

### 14.1 Windows/Wine

A PE binary expects loader sections/import tables, Windows DLL routines, TLS and SEH/VEH, Win32 handles, synchronization objects, native NT status/Windows messages, registry, filesystem case rules, process security token semantics, COM and graphics/DirectX on a selected implementation. Wine or a comparable userspace compatibility project could be ported to UniOS-native services in stages, subject to dependency, license and graphics-driver feasibility. **Do not** attempt to make UniOS KABI equal to undocumented raw NT syscall numbers. Microsoft documents Ntdll user-mode entry and warns against relying on certain undocumented Nt/Zw surfaces [S11]. The detailed Windows compatibility study belongs to RFC-011 or dedicated RFC-012 as architecture index assigns.

### 14.2 macOS/Darwin

Mach-O and Darwin/BSD syscalls, Mach IPC/ports, dyld, Objective-C/Swift runtime and frameworks like AppKit/Metal are distinct technical/legal layers. Even limited CLI compatibility requires a service/loader/ABI study; GUI app compatibility remains long-term unproven research. Mapping Mach port integer to UniOS native Handle without an authorized broker breaks capability semantics. Apple binaries/framework redistribution and platform licenses must be evaluated independently; this RFC grants no Apple SDK use or copying rights.

### 14.3 Full guest virtualization

Running Linux/Windows/macOS as a separate guest OS inside a VM is not native unmodified binary compatibility; it requires CPU virtualization, virtual devices, memory/IOMMU integration, correct guest ABI and licensing. PCI passthrough is an RFC-007 hardware DeviceGroup security issue. Do not attribute a guest OS's own successful filesystem or Win32 stack to UniOS compatibility layer.

## 15. SABI schema versioning, unknown data and handle safety

### 15.1 Version negotiation

Services advertise protocol identity, supported major/minor/feature ranges and their *current service instance* through an authorized channel. Clients use capability-granted Endpoint/Channel, not an untrusted registry string alone. A version can change interpretation of data and rights; select an agreed version before treating payload as authenticated method arguments. A process may have two old/new service instances concurrently during rollout; method request includes target service incarnation/IDL contract as needed, not an assumption that name is stable.

### 15.2 Strict vs flexible fields

- **Required unknown method/field/union/enum:** reject or close according to protocol contract, never default to a broadly permissive service method.
- **Optional unknown value field:** may be skipped if length-bounded and absence has safe defaults.
- **Unknown handle/resource field:** not just ignorable bytes; the kernel may have transferred an actual capability. Service must either account and close/drop the unexpected handle, preserve only under explicitly compatible proxy semantics, or reject transaction without hidden leaked right.
- **Unknown status:** preserve semantic distinction if safe; a newer stronger success mode cannot be mapped silently to old generic SUCCESS.
- **Reserved bits:** are zero or explicitly negotiated; arbitrary unknown rights never granted.
- **Deprecation:** old/new clients must have a tested overlap window; version negotiation cannot create grants beyond caller's cap.

### 15.3 Wire layout independent from code layout

A kernel Channel moves bytes and a separate vector of capabilities. It does not transport raw Rust structs, process pointers or local numerical handle IDs; any tagged handle presence/role belongs to SABI and must match the actual kernel vector and declared rights. Fuchsia's FIDL wire format distinguishes on-wire handle placeholder from out-of-band handle transfer [S14]; this is a security precedent, not a selected UniOS encoding.

## 16. Error taxonomy, idempotency and return rights

| Layer/status | What it establishes | What it must NOT be taken to mean |
|---|---|---|
| KABI INVALID_HANDLE/WRONG_TYPE | local cap token invalid/type mismatch | remote object was not operating |
| KABI ACCESS_DENIED/REVOKED | no new authorization for this invocation/grant lineage | all earlier effects undone / all DMA stopped |
| KABI NOT_COMMITTED | particular enqueue/MOVE operation did not commit | prior RPC/sends on another queue did not run |
| KABI COMMITTED | particular kernel enqueue/MOVE committed | receiver processed, file durable, peer didn't die |
| KABI UNKNOWN_EFFECT | caller cannot establish a commit/effect state | safe blind retry |
| SABI PEER_CLOSED/STALE_INSTANCE | endpoint/service generation invalid | remote external work has not occurred |
| Wait DEADLINE_EXPIRED | local deadline expired | remote operation canceled successfully |
| Wait CANCELLED_LOCAL | local pending wait/reply withdrawn | storage/phone/network write undone |
| SABI VFS BUFFERED_VISIBLE | data visible in current file cache | persisted after power failure |
| SABI FS SYNC_COMPLETED/Receipt | documented file+directory+journal+Flush dependencies under hardware assumptions | SSD guaranteed never to fail / disk not lying |
| Device R1 / R2-F / R2-O | progressively stronger CPU cutoff, DMA physical fence, object-wide reclamation | one handle-close is identical to all three |
| Guest errno / NTSTATUS / Mach status | guest-specific translated status of a supported semantic | any native kernel numeric enum always identical |

**Error mapping safety rule:** translate a native result to guest success only if all guest-visible preconditions have truly been met. When unsupported, prefer a real error over false success. A guest expects EINTR/SA_RESTART or SEH exception codes; translating generic DEADLINE_EXPIRED to guest EINTR without matching interruption rules can cause data corruption. Exact mappings remain RFC-018/010 research.

## 17. Representative ABI failure/race matrix

| Attack or race | Dangerous shortcut | Expected research safety contract |
|---|---|---|
| Guest rax equals native privileged opcode | dispatch solely on numeric value | kernel-owned execution domain before dispatch |
| Guest changes TLS "native" flag | trust userspace domain label | kernel tag and authorized guest↔native transitions only |
| Kernel checks pointer once then rereads | TOCTOU through another Thread/DMA | snapshot or pinned stable generation |
| User passes length*count overflow | unchecked allocation or copy | checked bounds/type/finite quotas |
| Kernel faults on Pager while global lock | self-deadlock | no faultable copyin in critical lock; bounded wired path |
| Copyout fails after handle transfer | report NOT_COMMITTED / leak moved handle | explicit commit/copyout distinction and conservation |
| IPC carries same handle twice | double-move or duplicated rights | detect duplicate descriptors; single transfer transaction |
| Receiver buffer too small for message cap vector | bytes arrive but caps disappear | bounded preflight, retained/discard semantics defined |
| Task dies after creating object but before output | invisible resource leak/refund | sponsor/object cleanup and reconcilable state |
| Stale handle integer reused | old wait completes for new object | kernel generation/lineage+registration validation |
| Revocation B completed, DMA still active | all memory reclaimed | R1/R2-F/R2-O separate hardware gating |
| Client RPC times out after VFS commit | remote effect rolled back | UNKNOWN_REMOTE_EFFECT/transaction recovery |
| Cancel Wait/ReplyToken | client assumes server aborted | cancel local only, remote separate |
| Server restarts with same ServiceID | old reply/IRQ appears valid | service instance generation and fresh caps |
| Two Wait subscriptions with same key | automatic dedup assumed | account each or explicitly reject duplicate |
| Level-trigger event treated as edge-count | missed wake/deadlock | contract specifies coalescing/arming |
| Infinite timeout overflow | wraps into immediate expiry | bounded monotonic arithmetic and explicit clock |
| Thread fault + kill + PageSupply | old Pager wakes a dead/new Thread | private FaultWaiter and generation |
| Scheduler throttled essential Pager | "higher priority client" makes progress | independent M0/essential budget and admitted dependency |
| Guest/Linux syscall raw pointer contains host cap token | guest UID0 creates native cap | no unmediated guest authority |
| Windows HANDLE reused as native handle | bitwise identity assumed | guest handle table and brokered mapping |
| Mach port integer copied to native cap | code bypasses entitlements | translation restricted by host granted rights |
| Unknown SABI field contains transferred handle | skip data leaving live cap | close/reject/explicit proxy under role rights |
| New status value unknown to older client | interpret as success | strict status/version negotiation |
| Native SDK Rust struct default repr | compiler version changes layout | extern-C sized records only |
| ARM64 port reuses x86_64 syscall register ABI | corrupt registers/args | separate per-architecture entry conventions |
| 32-bit guest length sign-extends negative | 64-bit memory overwrite | checked guest-width conversions |
| User service claims SIGPIPE/EINTR parity from IPC | incorrect process semantics | guest personality tests |
| File rename reported success before DirectorySync | data loss on power cut | scoped FS DurableReceipt |
| Untrusted service registry returns root handle | discovery equals authority | issuer-issued attenuated cap only |
| Bootstrap M0 depends on late dynamic SDK loader | recovery recursive Pager cycle | wired minimal runtime and ABI |
| Linux seccomp filter allows syscall | complete sandbox promised | native cap check always authoritative |
| Client downgrades protocol version to gain cap | old schema loses required rights field | signed/authorized policy + strict required field |
| Fastpath differs from slowpath for cap semantics | security depends on performance path | model equivalence and linearization proof |
| Thread Sysret noncanonical return IP | user controls kernel fault path | canonical return/alternate IRET-safe transition |
| Device interrupt claims USER IRQ vector | raw integer becomes privilege | typed IRQSource+device group |
| OSSigned new kernel but old SDK status enum | unknown privilege accepted | version/feature refusal and maintenance contract |

## 18. Proposed correctness/security invariants (ABI-01…ABI-58) — NOT proved

| ID | Candidate obligation, no implementation/proof |
|---|---|
| ABI-01 | Native KABI, versioned userspace SABI, SDK C ABI and guest Linux/Windows/Darwin ABIs are independently specified and versioned. |
| ABI-02 | KABI dispatch checks kernel-owned Thread/Task execution domain, but this tag is NOT by itself a security boundary if guest code shares trusted broker executable/cap state; privileged broker needs protected memory/execution/cap compartment. |
| ABI-03 | Guest raw syscall/opcode cannot invoke privileged native object operations without independent native authority. |
| ABI-04 | Native syscall input pointers/lengths/versions/flags are validated with overflow and cross-page copy/fault race handling. |
| ABI-05 | A userspace wrapper/vDSO never substitutes for kernel-side rights and raw-entry validation. |
| ABI-06 | Bounded kernel descriptor snapshot or justified stable pin prevents argument TOCTOU use after authorization. |
| ABI-07 | Copyout/cancel/Task death after commit cannot be misreported as no kernel effect or leak acquired authority; any protected result retention must be bounded, charged and eventually reclaimed. |
| ABI-08 | Task-local handle token must not reuse identical public value for a new unrelated authority during one Task lifetime while stale copies could be invoked; internal generation without public token difference is insufficient. |
| ABI-09 | Dup/derive attenuates rights and CREATE authority/sponsor cannot be minted from a held ordinary handle. |
| ABI-10 | R0 close, B deny, R1 CPU cutoff, R2-F physical DMA fence and R2-O object reclaim remain distinct results. |
| ABI-11 | Data+cap MOVE on bounded Channel has one defined atomic enqueue commit; UniOS NOT_COMMITTED preserves original sender handles, while COMMITTED consumes them into transit. This differs from Fuchsia basic channel_write. |
| ABI-12 | Receiver byte publication, exclusive message claim/escrow, cap install and queue removal have separately proven commit/publication semantics; no abstract byte+cap atomicity without a real isolation mechanism. |
| ABI-13 | ReplyToken is one-shot, kernel-minted and scoped to caller/request/service incarnation. |
| ABI-14 | Deadline expiry/CancelLocal/PeerClosed never claim remote operation rollback without separate evidence. |
| ABI-15 | IPC queue bytes, handles, Wait and Reply registrations are admitted and charged to scoped ResourceBudget. |
| ABI-16 | A SABI method name, ServiceID, numeric PID, UID0 or registry URL cannot authorize kernel object access alone. |
| ABI-17 | SABI unknown required methods/fields/handle types are safely refused; optional extensible fields need explicit authorization-safe semantics. |
| ABI-18 | On-wire handle placeholder/role never carries an actual transferable Task-local integer credential by itself. |
| ABI-19 | SABI negotiation and capability transit cannot widen the sender's kernel rights or revocation lineage. |
| ABI-20 | A returned native status names the exact phase committed, not a general remote-processing or durable-file result. |
| ABI-21 | FileSyncData/DirectorySync and user-storage DurableReceipt are service semantics and hardware-conditioned, not implicit kernel syscall success. |
| ABI-22 | User/guest libc errno, NTSTATUS and Mach error values map via personality, not assumed equal to native KABI status. |
| ABI-23 | CPU mapped R1 does not imply DMA R2-F or reliable storage persistence, irrespective of syscall used. |
| ABI-24 | FaultWaiter identity is kernel-private, scoped to PagerBinding/Task generation; user Pager cannot resume foreign thread. |
| ABI-25 | Kernel copyin page fault never silently demands normal Pager while holding essential nonfaultable kernel/M0 resources. |
| ABI-26 | Thread state wait, quota eligibility and CPU ownership remain orthogonal under timeout/fault/kill races. |
| ABI-27 | Scheduling weight, CPU cap and guaranteed reservation remain independent with hardware/scheduler preconditions. |
| ABI-28 | Critical boot/rescue KABI works from wired M0 without requiring late dynamic loader/VFS/network services. |
| ABI-29 | Guest Linux syscall dispatch/return uses x86 ABI and kernel-owned domain; default native privileged broker is a separate protected Task/AddressSpace, not just a same-address-space syscall shim. |
| ABI-30 | GABI 32/64-bit pointer/size conversion cannot extend guest authority or overflow host range. |
| ABI-31 | Guest Thread/exception/signal/futex/FD and blocking behavior is explicitly modeled before claiming Linux binary compatibility. |
| ABI-32 | Windows/Darwin guest HANDLE/Mach port identity and privileges cannot become native CapHandle by bitwise translation. |
| ABI-33 | C-compatible SDK boundary uses declared scalar/struct calling conventions, not default Rust or compiler-private layouts. |
| ABI-34 | Public SDK ABI controls panic/exceptions, allocator/ownership and lifetime across FFI before stable release. |
| ABI-35 | Unknown KABI mandatory feature/opcode or incompatible record version fails closed, never silently grants additional rights. |
| ABI-36 | Optional SABI fields must not allow old clients to ignore security-critical new requirements. |
| ABI-37 | KABI/SABI/SDK/guest ABI versions are independently negotiated without conflating signed BootPolicy and security floor. |
| ABI-38 | User-mode fastpaths and optional batch/ring transports preserve ordinary syscall authorization/commit semantics. |
| ABI-39 | Event/WaitSet registration generations, duplicates, quotas, cancellations and coalescing are defined with no stale wake. |
| ABI-40 | IRQ/DeviceGroup/MMIO/DMA operation requires typed native grants and actual hardware ownership, not numerical BDF/IOVA. |
| ABI-41 | Native/guest context transitions preserve FS/GS/TLS/return RIP/RFLAGS and failure state without privilege confusion. |
| ABI-42 | Runtime service reincarnation does not make stale Endpoint/ReplyToken/Wait/IOVA/file receipt valid in a new instance. |
| ABI-43 | Native object creation/outcome can be recovered or safely cleaned if Task dies during user output delivery. |
| ABI-44 | A compatibility claim is tied to an explicitly tested target set of binaries, SDK/ABI versions and hardware, not extrapolated from one app. |
| ABI-45 | Proposed operation families, machine registers and status labels are NOT Accepted ABI v1 or implementation authorization. |
| ABI-46 | Guest and trusted native broker in same address space are NOT isolated by a kernel-owned per-Thread mode flag alone: direct jumps, shared caps and injected control flow must be prevented by proven hardware/code isolation or separate Tasks. |
| ABI-47 | Default Linux guest broker runs in a separate Task/AddressSpace with attenuated Endpoint grants and no executable cross-jump into native privileged code. |
| ABI-48 | Public handle-token value never reauthorizes a different object for stale Task-held integer copies; generation exhaustion fails closed, not wraps. |
| ABI-49 | A provable NOT_COMMITTED for UniOS Channel MOVE preserves all sender source handles, unlike Fuchsia zx_channel_write; no failure is allowed to silently duplicate or consume unrelated authority. |
| ABI-50 | Partial receive user-byte disclosure before exclusive claim cannot be called abort-atomic; escrow claim, byte publication and handle-table commit have individually named safety predicates. |
| ABI-51 | ReceiveEscrow/claim cannot be held without finite quotas, cancellation, receiver-death cleanup and bounded progress/fail-stop. |
| ABI-52 | Post-effect result copyout loss must leave a bounded reconciliable kernel object/result or a defined UNKNOWN outcome; successful handle creation never leaks without owner/sponsor charge. |
| ABI-53 | Linux futex compare-value-and-sleep preserves atomicity against concurrent wake/write, or the advertised guest binary ABI explicitly excludes that operation. |
| ABI-54 | Linux rseq guest support requires kernel scheduler/migration and signal interaction model or explicit unsupported status, not a userspace-only syscall number translator. |
| ABI-55 | Linux clone/CLONE_VM/THREAD/SIGHAND/TID and signal relationships cannot be approximated by one generic native TaskCreate while claiming full guest parity. |
| ABI-56 | Unknown transferred SABI resource/handle cannot be silently discarded as if it were a harmless unknown value field without accounted close/reject policy. |
| ABI-57 | Feature negotiation cannot downgrade mandatory authorization, handle-rights, status or safety semantics merely to preserve backward binary compatibility. |
| ABI-58 | An accepted kernel entry domain and C ABI layout do not independently prove guest binary application coverage, future architecture support or trustworthiness of raw native broker code. |


## 19. Trade-off and feasibility comparison

| Approach | Benefit | Security/correctness cost | Candidate decision |
|---|---|---|---|
| ABI-A small typed KABI + versioned SABI + C-compatible SDK | preserves D5 microkernel, compartmentalized protocol evolution, native cross-language tooling | RPC lifetime, Pager recursion, service trust and compatibility broker are nontrivial | first conditional research target |
| ABI-B IPC-minimal everything through servers | simple kernel surface, few direct traps | privileged VM/thread/IRQ still require kernel object semantics and boot self-dependencies; not necessarily faster/smaller proven | compare on equal full boot/security graph |
| ABI-C kernel POSIX syscall support | apparent ease for source ports, conventional UNIX programs | filesystem/path/process/network semantics enlarge kernel TCB and legacy ABI surface | defer unless measured compelling case |
| ABI-D Linux syscall ABI native | static Linux CLI easy in narrow cases | exposes Linux semantics to every native task and freezes guest policy into Ring 0 | not preferred |
| G1 fixed kernel GuestDomain | simple non-bypass domain check | mixed native broker and guest code in same address space difficult | first guest dispatch comparison |
| G2 controlled per-Thread native↔guest switch | flexible broker/thunks | reentrancy/exception/TLS/return state broadens privileged TCB | research only |
| G3 user-mode emulator/binary translator | no direct raw guest trap to native dispatch | performance/coverage and self-modifying/instruction corner cases | fallback research |
| G4 raw guest syscall multiplexing in kernel | close to guest entry | grows untrusted Linux ABI parser in kernel and still needs services | last resort |
| Static native C ABI candidate | portable SDK baseline | toolchain/FFI, lifecycle and binary stability obligations | good staged research |
| Immediate stable ABI v1 | convenient platform branding | changes become difficult; undocumented security semantics could become permanent | **reject before evidence** |

**System-call count and early benchmarks are not objective security proofs.** Compare object handles, syscall entry, page-fault costs, queue/wait fairness, copy/zero-copy cost, guest syscall forwarding, native static ELF and failure/cap races under the same work. No performance threshold or ABI promise is selected from external published numbers.

## 20. Open research blockers (O9-01…O9-49)

| ID | Priority | Unresolved question / needed evidence | Co-review |
|---|---|---|---|
| O9-01 | P0 | KABI min object operation set and implementation-side TCB accounting, ABI-A vs ABI-B/C/D? | 001/002/009 |
| O9-02 | P0 | x86_64 SYSCALL vs alternate entry/return, Sysret canonical RIP/RFLAGS, KPTI and kernel stack safety? | 001/004/009 |
| O9-03 | P0 | Trusted Native/GuestDomain tag and G1/G2/G3/G4 handoff, malicious direct raw syscall bypass? | 001/009/010 |
| O9-04 | P0 | Guest↔native transition reentrant signals/faults/TLS/stack and separate kernel typed issuer rights? | 002/009/010 |
| O9-05 | P0 | User pointer copyin faults/TOCTOU, separate input descriptor snapshot and no recursive critical Pager path? | 003/004/006/009 |
| O9-06 | P0 | Post-commit copyout fault, Task death, handle creation and NOT_COMMITTED/UNKNOWN status linearization? | 002/003/009 |
| O9-07 | P0 | IPC MOVE+bytes atomic queue commit versus receive copyout/install, duplicated handles and quota failure? | 002/003/009 |
| O9-08 | P0 | Handle numeric ABA, Task-local generation, reuse, revocation lineage and detached sponsor charge? | 002/009 |
| O9-09 | P0 | Final return semantics for B/R1/R2-F/R2-O, status when DMA cannot quiesce and frame stays pinned? | 002/004/007/009 |
| O9-10 | P0 | ReplyToken timeout, cancellation, client crash and already committed external storage/device effects? | 003/008/009 |
| O9-11 | P0 | Private FaultWaiter and PagerReply integrity under VM unmap/Task kill/revoke concurrent event? | 003/004/009 |
| O9-12 | P0 | M0 independently resident ABI/runtime, boot library and kernel parser/initial cap proof? | 004/005/006/009/017 |
| O9-13 | P0 | Native SDK language-neutral C ABI/FFI, compiler/toolchain target, panic/unwind/TLS/stack exception rules? | 009/018 |
| O9-14 | P0 | Native/SABI/SDK/Guest ABI versioning and strict unknown mandatory fields/status/cap roles? | 003/009/018 |
| O9-15 | P0 | Linux x86_64 guest hardware trap and execution context routing without importing full Linux syscall handling? | 001/009/010 |
| O9-16 | P0 | Kernel-owned feature query cannot be spoofed by SABI or downgraded to grant guest higher rights? | 002/006/009 |
| O9-17 | P0 | User-facing service registry discovery versus native attenuated cap issuance and rights? | 001/002/003/009 |
| O9-18 | P0 | Event/WaitSet edge-level/coalescing/cancel/peer death and wait registration memory/CPU sponsor caps? | 003/005/009 |
| O9-19 | P0 | Monotonic clock/absolute deadline unit, overflow, suspend/resume semantics and timeout causality? | 005/006/009 |
| O9-20 | P1 | 32-bit guest pointer conversion, x32 Linux overlap and future ARM/RISC-V syscall differences? | 009/010/018 |
| O9-21 | P1 | Native static ELF loader, process startup vector, TLS, sysroot and build-chain license feasibility? | 006/009/011 |
| O9-22 | P1 | Native dynamic linker, ELF relocations/RELRO, signed image content and library compatibility? | 004/009/011/017 |
| O9-23 | P1 | POSIX source-port libc, Linux errno/signals/futex/clone/fork/exec/epoll and guest ABI coverage? | 009/010 |
| O9-24 | P1 | Windows PE/Wine host service mapping, SEH/TLS, object semantics, documented vs NT native ABI? | 009/012 |
| O9-25 | P1 | Darwin Mach-O/dyld/ObjC/Swift CLI and legal GUI limits under native cap enforcement? | 009/014 |
| O9-26 | P1 | Multiple versions of SABI method/handle role/unknown-flexible fields and service restart coexistence? | 003/009/018 |
| O9-27 | P1 | KABI stable-v1 policy, deprecation security fixes, supported arch/version matrices and test corpus? | 009/018 |
| O9-28 | P1 | Optional zero-copy/batch/ring transport cap/IO generation and cancellation safety? | 003/004/008/009 |
| O9-29 | P1 | Shared memory producer/consumer barrier and per-core status/ownership for asynchronous IO? | 003/004/009 |
| O9-30 | P1 | Distinct FileSync/DirectorySync/commit receipt and guest POSIX errno/Windows/Mach translations? | 008/009/010 |
| O9-31 | P1 | x86_64 CET/XSAVE/FS/GS/stack flags and selected reference PC feature/version contract? | 005/009 |
| O9-32 | P1 | Guest vDSO/syscall restart and signals with blocked broker, native page-fault concurrency? | 004/009/010 |
| O9-33 | P1 | SABI server compromise and signed service overgrant effect on file/boot security? | 001/002/009/017 |
| O9-34 | P1 | Native status unknown newer enum and caller legacy error handling safe downgrade? | 009/018 |
| O9-35 | P2 | Toolchain/build reproducibility, debug/unwind info, observability and SDK distribution provenance? | 009/015/018 |
| O9-36 | P2 | Perf feasibility: IPC/copyin/guest broker/timer overhead p50/p99 and UI responsiveness vs expanded kernel? | 005/009/015 |
| O9-37 | P0 | Prove G1 Linux guest and native privileged broker are separate protected Task/AddressSpace with no dangerous broker cap sharing? | 001/002/009/010 |
| O9-38 | P0 | Can G2 mixed same-address-space broker EVER be safe with authorized transition, CFI, protected code/data and no jump into native privilege? | 001/002/009/015 |
| O9-39 | P0 | Fuchsia channel_write failure consumes MOVE handles: what exact UniOS NOT_COMMITTED sender ownership and duplicate-handle error policy? | 002/003/009 |
| O9-40 | P0 | RCV-A ReceiveEscrow/claim exclusivity vs visible partial copyout, cap install, terminal errors and confidentiality? | 002/003/004/009 |
| O9-41 | P0 | Bounded escrow retention/charges/revoke/death and forward-progress policy when hostile receiver holds claim forever? | 002/003/005/009 |
| O9-42 | P0 | Kernel post-commit copyout cannot return receipt: protected result escrow or UNKNOWN with bounded sponsor/accounting? | 002/003/004/009 |
| O9-43 | P0 | Nonreused full public handle token vs bounded width/generation wrap and Task lifetime; align with RFC-002 §7? | 002/009/018 |
| O9-44 | P0 | Linux futex atomic compare-and-block and PI/robust mutex under MemoryObject aliases, Task migration and broker? | 004/005/009/010 |
| O9-45 | P1 | Linux rseq per-thread fields/CPU migration/signal/critical-section abort policy and support level? | 005/009/010 |
| O9-46 | P1 | Linux clone CLONE_VM/THREAD/SIGHAND, TIDs, process vs thread signal and exit/wait race compatibility scope? | 004/009/010 |
| O9-47 | P0 | Evolving SABI unknown handles and strict/flexible mandatory security fields: deny/close without leaked authority? | 002/003/009/018 |
| O9-48 | P1 | Fuchsia zx_channel_write vs write_etc MOVE/DUP failure semantics: ensure external analogies aren't treated as UniOS contract? | 003/009 |
| O9-49 | P0 | Guest ABI direct syscall trap with seccomp-like broker: pointer snapshot, privilege domain, retry/cancel and signal path? | 001/003/009/010 |

## 21. Proposed decision register (ABI-D01…ABI-D33) — NONE Accepted

- **ABI-D01:** ABI-A minimal typed KABI, versioned SABI and independent language-neutral SDK ABI is the first conditional research architecture.
- **ABI-D02:** KABI owns kernel objects/rights, not POSIX paths, NT objects, Mach frameworks, filesystem parser or GUI methods.
- **ABI-D03:** Linux/Windows/Darwin ABI are restricted compatibility personalities, not aliases of UniOS native syscall numbers.
- **ABI-D04 [clarified]:** Kernel-owned GuestDomain is required before dispatch but not sufficient to isolate mixed guest/native broker code. Default broker has separate Task/AddressSpace/cap table.
- **ABI-D05 [clarified]:** First study G1 fixed restricted GuestDomain with separate native broker; G2 same-address-space privileged switch is rejected as a default unless code/data/cap execution separation is independently proven; compare G3 emulator and G4 enlarged kernel on equal threats.
- **ABI-D06:** Keep x86_64 syscall register convention, IDs, raw return status, handle width and descriptor layout **unfrozen** until formal/experimental review.
- **ABI-D07:** Raw user pointers, descriptor lengths, alignments and versions are checked with safe copyin/copyout under concurrent mapping/fault rules.
- **ABI-D08:** Post-commit copyout failure cannot be reported as no effect; object/handle lifetime conserved under process death.
- **ABI-D09 [clarified]:** Handles are Task-local opaque public tokens whose **exact bit pattern must not be reissued for unrelated authority within the Task lifetime while stale bits remain usable**; no internal generation-only ABA fiction. Rights/sponsor/lineage maintained in kernel.
- **ABI-D10:** R0/B/R1/R2-F/R2-O and remote side-effect commit remain separate status/evidence levels.
- **ABI-D11 [clarified]:** IPC E1 bounded Channel/ReplyToken follows RFC-003; candidate NOT_COMMITTED preserves moved source handles, COMMITTED takes them into transit, and RCV-A claimed receive needs separate proof. This is NOT automatically Fuchsia zx_channel_write semantics.
- **ABI-D12:** Wait/Event/timeout has named clock/cancel/registration-generation/coalescing semantics and finite resource accounting.
- **ABI-D13:** Pager fault uses private FaultWaiter+PageSupply rights and wired M0 to avoid self-deadlock.
- **ABI-D14:** Kernel errors and guest errno/NTSTATUS/Mach result codes remain independent semantic namespaces.
- **ABI-D15:** Service discoverability is separate from authorized Endpoint cap acquisition; user SABI payload cannot mint kernel privileges.
- **ABI-D16:** SABI schema strict/flexible rules are versioned; unknown transferred handle resources require explicit close/reject/proxy policy.
- **ABI-D17:** Native SDK research prefers a C-compatible FFI boundary and Rust/C/C++ bindings; Rust default ABI/layout never freezes OS contract.
- **ABI-D18:** Native static ELF and minimal runtime before full libc/dynamic linker are research stages, not working app claims.
- **ABI-D19:** Linux binary compatibility must be reported at executed semantic-coverage level; full Linux/POSIX parity and Wine/Mach GUI are unproven.
- **ABI-D20:** Privileged device/boot/trust operations require independent typed capabilities, not guest administrator, UID0 or executable signature.
- **ABI-D21:** FileSync/DirectorySync conditional durability and remote unknown effects follow RFC-008; no blanket "saved" syscall result.
- **ABI-D22:** Feature/version negotiation distinguishes KABI, service SABI, SDK, guest ABI and immutable signed BootPolicy descriptors.
- **ABI-D23:** Optional fastpath/zero-copy/batched ring operations only after they can be shown equivalent in security/commit and sponsor accounting.
- **ABI-D24:** Stable v1 ABI, syscall numbers, compiler target, code implementation and any compatibility certification remain unauthorized while R1 gates open.
- **ABI-D25 [new]:** Baseline guest broker isolation uses independent protected Task/AddressSpace and only validated attenuated IPC rights; guest/native co-location is not presumed safe with a kernel flag.
- **ABI-D26 [new]:** Prefer RFC-003 RCV-A bounded exclusive ReceiveClaim/escrow research for message bytes+cap install, explicitly denying magical single atomicity of user memory copyout.
- **ABI-D27 [new]:** UniOS E1 NOT_COMMITTED leaves sender handles intact, while COMMITTED removes into transit; do not import Fuchsia basic write() failure consumption.
- **ABI-D28 [new]:** Public handle token nonreuse/generation exhaustion and callback ABA protection follow RFC-002 §7, without freezing numeric bits.
- **ABI-D29 [new]:** A completed effect followed by failed result copyout needs a protected reconcilable receipt/escrow or honest UNKNOWN outcome with bounded owner/sponsor cleanup.
- **ABI-D30 [new]:** Linux futex, rseq and clone require their real shared-memory, scheduler, thread/signal atomic contracts or are excluded explicitly from supported guest feature tiers.
- **ABI-D31 [new]:** SABI evolving unknown resource fields must reject/close/explicitly proxy under bounded capability accounting, never silently grant or leak a transferred cap.
- **ABI-D32 [new]:** Even a secure kernel-owned guest tag does not authorize a shared native privileged broker thread to execute untrusted arbitrary guest code; in-process G2 requires stronger independent memory/execution proof.
- **ABI-D33 [new]:** No Stable ABI/SDK/guest binary certification until side-effect, error, version, syscall entry and reference-PC feasibility gates are independently passed.

## 22. Cross-RFC interface ownership and contradiction register

| RFC | ABI boundary it owns/constrains | Potential conflict to resolve |
|---|---|---|
| RFC-001 | kernel object/security TCB and execution domain | adding POSIX or broad device policy to KABI violates default boundary |
| RFC-002 | Task-local typed handles, creator/sponsor/lineage, R0/B/R1/R2-F/R2-O | closing handle vs global revoke and stale numeric ABI token |
| RFC-003 | E1 bounded Channel and ReplyToken/Wait semantics | SABI wire version/role must not be mistakenly interpreted by kernel |
| RFC-004 | PagerBinding/FaultWaiter, page faults, copyin and R1 | kernel copyin fault may deadlock on ordinary Pager or invalidate copyout status |
| RFC-005 | CPU budget/weight/reservation and work charging | async broker or user driver must run when client is blocked; no automatic donation |
| RFC-006 | signed/wired BootBundle M0 startup and restart | bootstrap ABI cannot require VFS/late libc runtime; Good status not app self-report |
| RFC-007 | typed IRQ/MMIO/IOMMU DeviceGroup and R2-F | naive native DeviceMap without physical topology grants unsafe DMA |
| RFC-008 | File/DirectoryCap, FileSync, writeback errors/UNKNOWN | kernel IPC COMMITTED must not become synchronous persisted FileWrite claim |
| RFC-009 | per-arch KABI entry ABI, SDK FFI and guest separation | these need RFC-018 version/type/status stable schema decisions |
| RFC-010 | Linux ELF/POSIX guest ABI and syscall forwarding | syscall number collisions cannot bypass trusted GuestDomain |
| RFC-011 | native packaging/runtime/service manifests | service install/signer is not unconditional kernel cap |
| RFC-012 | Windows PE/Wine/Win32/NT semantic coverage | NTSTATUS and Windows HANDLE aren't native KABI |
| RFC-013 | desktop compositor and GPU buffer sharing | service protocol GPU/Window handles require native rights and DMA fences |
| RFC-014 | Darwin/Mach-O/Mach/Objective-C/Swift feasibility | no assumption Apple GUI/Metal works from native C SDK |
| RFC-015 | security test/fuzz/crash failure and source custody | model and fuzz KABI descriptors, IDL unknown fields and boot grant ceilings |
| RFC-016 | permission portal, app grants, UI consent | user consent broker cannot bypass typed native capability checks |
| RFC-017 | authenticated BootPolicyManifest, trust mode, floor | runtime ABI feature query cannot rewrite signed security policy |
| RFC-018 | versioned IDL, status codes/ABI evolution | external numeric table cannot freeze before failure semantics verified |

**Important status:** RFC-010 through RFC-016 and RFC-018 are future or separate research allocations in the original architecture index; this RFC does not assert any of their design/implementation has been completed.

## 23. Research/model checking and design exit gates

### 23.1 Minimal bounded model before any ABI acceptance

Model two Tasks, two Threads and one restricted Linux guest; one legitimate and one malicious native client; one capability issuer, 2 handles with reuse across generations, one MemoryObject and PagerBinding, bounded Channel of capacity 1, one transferred Handle, one ReplyToken, one app-visible RPC to a simulated VFS server, one peer death/restart, one resource sponsor, one CPU running context and one zero-budget Pager. Include one physical DMA frame with R1/R2-F distinct.

Transitions to analyze: preflight copyin, user modifies payload, sender queue commit, receiver copyout fault, handle install, client death, server death, ReplyToken timeout, cancel, invalid PagerSupply, handle close/recycle, transfer cap revoke B, CPU TLB R1, failed R2-F/quarantine, guest raw syscall with native opcode, guest→native transition spoof, SABI major mismatch, unknown handle role, double send, and OOM during cleanup.

**Safety properties:** no kernel privilege amplification; no duplicate MOVE ownership; no stale reply/wake/handle after generation change; no false NOT_COMMITTED after irrevocable enqueue; no invented file durable receipt; no native cap from guest numeric ID; no leaked kernel raw pointers; no unbounded M0 resource drain by ordinary guest.

**Conditional liveness:** given valid timer/IRQ/CPU budget, wired M0 and healthy Pager, bounded clean failure may complete. Absent those hardware conditions, stop safely rather than assert deterministic response time. A filesystem service may fail with UNKNOWN despite correct KABI behavior.

### 23.2 Future ABI-specific tests (not performed)

1. Controlled x86_64 entry/return state testing: noncanonical user RIP/RSP, RFLAGS/GS/TLS and safe kernel stack under SYSCALL, traps and interrupts.
2. Pointer and layout property tests: mismatched size/version, zero/overflow length, 32-bit conversion, concurrent VA unmap/remap, double copyin mutation, faulty output pointer after commit.
3. Capability state interleaving: grant rights attenuation, MOVE with duplicate list, sender and receiver death, handle ABA, revoke B then R1/R2-F, false complete-Fence.
4. IPC/Wait operation semantics: E1 bounded message, ReplyToken one-shot/timeout, queue backpressure, duplicate registration/cancel/wake, peer restart and no unauthorized reset.
5. Native C/Rust/C++ SDK FFI static test candidates: struct sizes/alignments, endian, panic/exception and allocation boundaries, target builds with multiple compilers (only after separate code authorization).
6. Linux minimal ELF static CLI syscall smoke with explicit supported table; guest direct SYS call bypass tests, signal/futex/exec/dynamic linker as staged targets, no blanket parity.
7. SABI golden payload compatibility corpus across versions, unknown strict/flexible field/handle roles, revoked service incarnations and schema fuzz.
8. VFS FileSync/DirectorySync durability and IO uncertain result mapping to guest errno; never use production/user disk.
9. Kernel IPC fastpath versus baseline and optional ring/shared-memory path observational equivalence for rights, crash state and CPU charge.
10. Performance p50/p95/p99 under named QEMU/hardware configurations after correctness: syscall trap, copyin/out, endpoint ping-pong, Pager fault, guest forwarding, bandwidth and UX latency.
11. Reference PC CPU feature, timer/IRQ/IOMMU/firmware boot profile for native ABI; QEMU is distinct evidence.
12. Version change and old binaries: explicitly fail on unknown mandatory feature, no accidental old-client permission expansion, migration with two SABI server versions.

No model, fuzzing, build, real computer test, software implementation or performance measurement has been run by authoring this design document.

### 23.3 Exit criteria (R1 = OPEN)

Before calling ABI-A Accepted or Stable v1:
- **v0.2 required:** prove kernel-owned GuestDomain PLUS separate protected guest/native broker AddressSpaces and cap tables; G2 in-process transitions require strong independent executable/control-flow/cap isolation or are rejected. Defer direct Linux binary support if this cannot be established.
- **v0.2 required:** prove sender MOVE NOT_COMMITTED preserves source grants, exclusive bounded receive claim/escrow controls byte disclosure and cap install, and lost result copyout has honest committed/unknown cleanup.
- **v0.2 required:** prove no identical stale public Handle token reauthorizes another object under finite generation exhaustion; align RFC-002 §7.
- **v0.2 required:** either provide Linux futex atomic compare-block (plus specified later rseq/clone semantics) or explicitly exclude those guest ABI features from the support tier.
- Publish exact transition/linearization state model for handle move, syscall input/copyout, async send/receive, cancel/Reply and Task death.
- Prove that SABI/SDK/version handling does not expand capability grants under unknown fields or forged ServiceID.
- Bound critical copyin/Pager/IRQ/CPU/OOM paths and distinguish actual R2-F vs CPU mapping.
- Select machine calling conventions and static native ABI layout through multi-compiler/toolchain validation and hardware proof; no Rust private ABI leakage.
- Define FileSync/DirectorySync/unknown remote effect and guest error mapping without false success.
- Name supported architectures, CPU features, stable vs experimental ABI versions and clear compatibility window.
- Complete comparative architecture TCB/latency trade-offs based on **actual** measurements/verification, not external performance claims.
- Review compatibility feasibility/legal constraints for native libc, POSIX, Wine and Darwin separately.
- Record ADR approval independently of high-level D1–D8 accepted directions. Until then all ABI-D proposals remain **Proposed**, no implementation permission.

**Future research recommendation:** RFC-009 v0.1→v0.2 independent adversarial review is now recorded in §25. Next RFC-010 Linux Personality should resolve separated guest syscall broker and the futex/clone/rseq scope; then RFC-018 should formalize versioned IDL and status/cap-transfer compatibility. All P0 proof gates remain OPEN.

## 24. Official primary technical references (rechecked 2026-10-24)

- **[S1] seL4 IPC, endpoint, badges and fastpath:** https://docs.sel4.systems/Tutorials/ipc
- **[S2] seL4 Reference Manual:** https://docs.sel4.systems/projects/sel4/manual.html
- **[S3] Fuchsia Zircon process-local handles:** https://fuchsia.dev/fuchsia-src/concepts/kernel/handles
- **[S4] Fuchsia Zircon Channel object contract:** https://fuchsia.googlesource.com/fuchsia/+/refs/heads/main/zircon/docs/objects/channel.md
- **[S5] Fuchsia zx_object_wait_async:** https://fuchsia.dev/reference/syscalls/object_wait_async
- **[S6] Fuchsia FIDL strict/flexible unknown fields and method compatibility:** https://fuchsia.dev/fuchsia-src/reference/fidl/language/language
- **[S7] Linux userspace syscall stable ABI (architecture-specific):** https://github.com/torvalds/linux/blob/master/Documentation/ABI/stable/syscalls
- **[S8] Linux syscall(2) machine register conventions:** https://man7.org/linux/man-pages/man2/syscall.2.html
- **[S9] Linux seccomp BPF and USER_NOTIF limits:** https://docs.kernel.org/userspace-api/seccomp_filter.html
- **[S10] FreeBSD Capsicum capabilities and sandbox:** https://docs.freebsd.org/en/books/handbook/security/#capsicum
- **[S11] Microsoft Libraries and Headers — Ntdll Nt/Zw user calls and documentation limits:** https://learn.microsoft.com/en-us/windows-hardware/drivers/kernel/libraries-and-headers
- **[S12] Linux ABI stability levels:** https://docs.kernel.org/admin-guide/abi.html
- **[S13] Fuchsia zx_handle_close and wait invalidation:** https://fuchsia.dev/reference/syscalls/handle_close
- **[S14] Fuchsia FIDL wire format, handle placeholders and out-of-band transport:** https://fuchsia.dev/fuchsia-src/reference/fidl/language/wire-format
- **[S15] Linux io_uring_enter(2), split submit/completion and cancel:** https://man7.org/linux/man-pages/man2/io_uring_enter.2.html
- **[S16] Linux fsync(2), directory entry persistence and fdatasync:** https://man7.org/linux/man-pages/man2/fsync.2.html
- **[S17] seL4 faults and handler/resume:** https://docs.sel4.systems/Tutorials/fault-handlers.html
- **[S18] Fuchsia FIDL language version/evolution rubric:** https://fuchsia.dev/fuchsia-src/development/api/fidl
- **[S19] Fuchsia zx_channel_write() basic handle consumption even on failure:** https://fuchsia.dev/reference/syscalls/channel_write
- **[S20] Fuchsia zx_channel_write_etc() MOVE/DUP per-handle dispositions:** https://fuchsia.dev/reference/syscalls/channel_write_etc
- **[S21] Fuchsia zx_channel_read_etc() buffer size/discard and handle type/rights:** https://fuchsia.dev/reference/syscalls/channel_read_etc
- **[S22] Linux seccomp BPF and user-notification security limitations:** https://docs.kernel.org/userspace-api/seccomp_filter.html
- **[S23] Linux futex(2) atomic compare-and-block:** https://man7.org/linux/man-pages/man2/futex.2.html
- **[S24] Linux restartable sequences (rseq) scheduler and ABI:** https://docs.kernel.org/userspace-api/rseq.html
- **[S25] Linux clone(2) and clone3(2) CLONE_VM/THREAD/SIGHAND/TID:** https://man7.org/linux/man-pages/man2/clone.2.html

**Use limits:** These sources describe actual other OS kernel/user contracts and are informative only. No seL4 proof transfers to UniOS, no FIDL/Zircon compatibility is claimed, no Windows syscall stability beyond Microsoft's documentation, no Linux ELF program was run, no new license/source import authorized. External user-facing formats and interface implementations require separate version, license and security evidence.

---

**Document history:** 2026-10-24 v0.1 initial Proposed draft remains in Git history. 2026-10-24 v0.2 independent adversarial review revises co-located native/guest broker unsafety, Fuchsia vs UniOS MOVE-on-failure differences, receive escrow disclosure, public handle token ABA, kernel post-commit copyout, Linux futex/rseq/clone semantic gates and SABI unknown-resource compatibility. Documentation only; no kernel/loader/guest/SDK implementation, tests, formal proof, reference hardware work or ABI freeze.

## 25. v0.2 Adversarial re-review — guest privilege, IPC disclosure and ABI correctness

**Reviewed 2026-10-24.** RFC-009 v0.1 challenged against RFC-001/002/003/004/005/006/007/008/017, seL4 IPC, Fuchsia Channel/FIDL specifications [S19–S21], Linux seccomp [S22], futex [S23], rseq [S24] and clone [S25]. All findings are **document-based**. No executable code, formal model, proof, physical tests, source imports or ABI freeze were performed.

### 25.1 Counterexample matrix

| Finding | Concrete adversarial scenario | Revised design boundary | Priority |
|---|---|---|---|
| GUEST-1 per-Thread flag is insufficient | Guest code shares Task memory with privileged native broker. Although kernel maintains a read-only GuestDomain tag, the guest jumps into native broker instructions or corrupts broker-writable data, gaining privileged native operation. | Default G1: guest and native broker are **separate protected Tasks/AddressSpaces and cap tables**. G2 in-process mixed-code privilege boundary is unsupported until strong code and cap isolation proven. | P0 |
| GUEST-2 raw syscall trap | Linux guest calls a native opcode by putting its number in RAX or invokes a native userspace stub. | Kernel dispatch requires its own Task domain first; guest cannot self-promote via registers, shims, TLS or ELF note. Broker receives attenuated IPC, never a raw privileged jump. | P0 |
| GUEST-3 mutable argument pointer | Kernel forwards guest user address to broker; another guest Thread changes mapping/data after broker validates path/size. | Snapshot validated args or use proven stable pinned backing, bounded lifetime and per-MemoryObject generation; remote pointer is not authority. | P0 |
| SEND-1 mistaken Zircon handle semantics | Fuchsia basic channel_write fails but caller handles were consumed; UniOS sender code assumes failure preserved them, or vice versa. | **UniOS E1 NOT_COMMITTED leaves all sender handles intact; COMMITTED transfers entire byte+cap vector**. This is a UniOS proposal, not Fuchsia behavior [S19,S20]. | P0 |
| RECV-1 copyout exposes unowned bytes | User buffer receives secret message bytes, then copyout faults before queue/cap commit. Message is subsequently delivered to another receiver. | Require exclusive bounded ReceiveClaim/escrow and independent publication/commit rules; no magical atomic user-memory bytes+handle transaction [S21]. | P0 |
| RECV-2 hostile escrow hold | Receiver claims a message and never completes receive, consuming handles/queue quota and starving essential recovery IPC. | Bounded claim time/cost, quota sponsor, cleanup on Task death/revoke and declared discard/requeue policy, with protected M0 reserve. | P0 |
| COPY-1 handle created, output lost | Kernel creates charged cap then caller output buffer is unmapped; caller gets invalid-buffer despite committed grant or object leaks forever. | Protected bounded result/receipt or honest UNKNOWN and deterministic owner/sponsor cleanup; never report NOT_COMMITTED for a completed effect. | P0 |
| COPY-2 partial user writes observable | First half of syscall output copied; second half faults. Concurrent Task observed first half; kernel says it rolled back output. | User-memory writes are not generally transactional; cap authority and queue commit must be conserved, partial byte exposure reported where relevant. | P0 |
| CAP-1 identical handle token ABA | Task closes handle 0x42, gets new unrelated object also 0x42; stale copied old integer now invokes new privilege even if internal slot generation changed. | Public token must not reissue same value while stale copies usable; nonwrapping generation or Task-lifetime nonreuse, fail-closed exhaustion [RFC-002 §7]. | P0 |
| LINUX-1 futex lost wake | Broker reads shared futex word, another Thread changes/wakes, broker then sleeps forever after wake happened. | Linux futex compare-and-block must be atomic against shared-memory updates/wakes [S23]; study native kernel scoped primitive or explicitly exclude guest threads. | P0 |
| LINUX-2 rseq false support | Runtime registers restartable-sequence area; CPU migrates during per-CPU critical sequence without abort. | Require scheduler/migration/signal hooks or return unsupported rather than pretend successful Linux rseq [S24]. | P1 |
| LINUX-3 clone false parity | CLONE_THREAD/VM/SIGHAND translated into independent Task with wrong TID, signals or memory. | Model entire Linux process/thread group semantics or limit support tier; do not equate generic TaskCreate and clone [S25]. | P1 |
| SABI-1 optional unknown cap | Old protocol ignores unknown field, but that field carries an actual transferred capability, leaking grant or silently discarding a resource. | Unknown handle must be explicitly closed/rejected/authorized proxy with accounting; normal flexible-value skip is unsafe [S6,S21]. | P0 |
| VERSION-1 compatibility downgrade | New service requires attenuated rights/durable reply field; client negotiates old version and obtains broad permission or false save success. | Required security semantics are enforced even on old wire version; refuse negotiation if cannot satisfy them. | P0 |
| SCHED-1 broker lacks resources | Restricted syscall forwards to privileged broker which waits on Pager, but Pager is CPU-starved or needs itself. | Wired critical M0 path and independently admitted CPU/IPC/VM budget [RFC-004/005/006]; ReplyToken isn't a CPU reservation. | P0 |
| ENTRY-1 context return | Guest controls FS/GS/return address; unsafe SYSRET or restored flags yield invalid kernel entry or state confusion. | Architecture-specific context discipline, canonical RIP validation, verified IRETQ fallback, dedicated kernel-owned stacks. | P0 |

### 25.2 Rejection of simple in-process G2

A kernel-owned per-Thread GuestDomain is a necessary *dispatch label*, not an instruction provenance proof. If untrusted guest bytes and privileged broker functions coexist in one address space with shared executable memory, the guest may branch to or corrupt broker behavior. Restricting access to a native syscall by checking Thread state does not fix a broker function executing in a native-authorized Thread/capability context.

**Preferred G1 research:** Guest Task has only its restricted interpreter/personality domain, bounded memory and granted channel endpoints. An independently protected native broker Task has exactly the narrow authority it needs and treats every guest message as hostile. No direct jump, native syscall reuse, writable code alias, shared privileged handle table or unvalidated user pointer can bridge that boundary.

**Unresolved G2 alternatives:** prove genuinely separate intra-process protected execution/metadata domains with code integrity, no writable/executable aliases, protected native cap entries, safe nested signal/exception transitions, reentry/return and control-flow provenance. A library wrapper, TLS flag, signed guest binary or nonwritable kernel flag alone does not satisfy this gate. G3 emulator and G4 native-kernel Linux dispatch remain comparisons, not implementations.

### 25.3 IPC ownership and receive-publication stages

| State | Distinct event | Authorized conclusion |
|---|---|---|
| SEND_PREPARE | Copy input bytes and inspect sender SEND/TRANSFER rights, reserve credits | no cap MOVE yet |
| SEND_NOT_COMMITTED | Precommit failure before queue/cap transition | sender source handles retained; no queued message |
| SEND_COMMITTED | One queue record installed and source handles moved into kernel transit | sender no longer owns moved caps; receiver not yet guaranteed |
| RECEIVE_CLAIMED | Exclusive bounded claim + handle-slot reservation | rival receiver cannot deliver same message; claim resources charged |
| RECEIVE_BYTES_PARTIAL | Some user bytes copied, potentially concurrently observed | no claim that these writes were rolled back |
| RECEIVE_CAP_COMMITTED | All caps installed with fresh Task-local tokens; queue record removed | exactly one authorized receiver owns new handles |
| RECEIVE_RESULT_LOST | Status copyout or client death after committed transition | receipt/UNKNOWN/cleanup, not fabricated NOT_COMMITTED |
| RPC_TIMEOUT | local wait deadline fired | remote server effect may continue |

RFC-003 §6 notes that receiving bytes into user memory and installing handles are distinct commit issues. **RCV-A ReceiveEscrow is the research candidate**; it must bound claimant lifetime and charges, forbid a second reader race, handle copyout disclosure and Task death, and explicitly specify drop/requeue/terminal-error behavior. Pre-validating a userspace pointer cannot make multi-page copies atomic under concurrent unmap. A protected result receipt is itself another bounded object with failure/garbage collection semantics.

Fuchsia comparison precisely: its basic channel_write discards handles on both success/failure [S19]; channel_write_etc uses per-handle MOVE (source closed) or DUPLICATE (source retained) dispositions and detailed error results [S20]; channel_read_etc provides type/rights and size/discard semantics [S21]. UniOS's proposed NOT_COMMITTED preserving source caps is a **different** and still unproved E1 design decision, not a summary of Fuchsia.

### 25.4 Handle ABA and effect reconciliation

An internal slot-generation guard cannot distinguish a stale public handle integer from a newly valid object when both invocations present exactly identical user bits. Avoid identical public token reassignment over the Task life, or prove equivalent anti-ABA via trusted per-invocation context unavailable to stale user copies. Finite token space implies an explicit exhaust/fail-closed transition. No spec freezes slot widths or generation bit positions.

An invocation pin before revoke B may finish under the scoped revoke model; a new invocation after B cannot become authorized simply by reusing a numeric handle. CPU R1, DMA R2-F and final R2-O are independent.

When output delivery fails **after** object creation, reply token use, IPC receive or storage request, distinguish committed authority transfer, partial user-visible bytes and unknown remote side effects. User-visible INVALID_BUFFER does not imply no effect. The kernel cannot guarantee the client observed its receipt before crash; safe cleanup and queryability are explicit separate design obligations.

### 25.5 Linux guest compatibility limit is a semantic contract

Linux futex compares shared user memory and sleeps **atomically** with respect to concurrent wake/update [S23]. A client RPC to read the value followed by another RPC to sleep has a lost-wakeup race. To claim Linux multithreaded guest support, prove a comparable kernel/shared-MemoryObject synchronization primitive and account for futex address alias, owner death and priority inheritance where relevant.

Linux rseq uses registered per-thread memory fields and aborts critical sections on migration/signal/context switch conditions [S24]. Claiming successful rseq without implementing these guarantees can corrupt programs. Linux clone flags couple AddressSpace, thread-group identity, signal dispositions and TIDs [S25]. These requirements must be included in the declared support tier, not hidden under "syscall forwarding." Linux vDSO operations can avoid traps; they require a separately correct runtime mapping/context policy. Restrict initial L1 compatibility claims if these are unavailable.

### 25.6 Exit-gate changes

1. **P0 guest isolation:** default separated Guest Task and native privileged Broker Task with adversarial jump/cap/shared-memory tests designed; only later consider G2 with independent execution/cap isolation proof.
2. **P0 sender MOVE:** one defined NOT_COMMITTED and COMMITTED boundary plus sender ownership/charges under all failure reasons, including duplicate/malformed handles.
3. **P0 receive escrow:** exclusive claim before sensitive byte disclosure, bounded resource lifetime, all-cap install, copyout partial failure and receiver death correctness.
4. **P0 public token:** nonreissued handle values across possible stale references and a precise exhaustion rule.
5. **P0 postcommit status:** object created / side effect committed but result lost must be reconciliable or honestly UNKNOWN, never reported undone.
6. **P0 guest semantics:** futex compare-block atomicity or explicit unsupported, and rseq/clone scope clearly gated.
7. **P0 SABI safety:** unknown resource handles and security-critical field/version negotiation fail closed with quota cleanups.
8. **P0 M0 and CPU:** first-stage native ABI and broker path must not recursively depend on broken Pager or starved services.
9. **P0 x86/SDK:** register calling convention, user return path, FFI structs/status remain unfrozen pending concrete proof and authorized compiler/hardware tests.

**Verdict:** ABI-A remains a conditional research target. G1 separate protected broker is the preferred security boundary, G2 same-address-space switch is unadmitted, and RCV-A/handle ABA/guest atomicity remain unresolved cross-RFC P0 proofs. RFC-009 v0.2 is still Proposed, without implementation permission.

## 26. 2026-10-25 Native Handle and operation Receipt boundary audit — Proposed

**Joint research reference:** [CAPIPC Task Handle Namespace & Kernel Receive Receipts v0.1](../models/capipc-task-handle-namespace-and-receipt-v0.1.md) with RFC-002 Handle ABA, RFC-003 SEND/RECEIVE K-S/K-I, RFC-018 typed SABI and RFC-010 guest FD/OFD. This documents constraints, **not** an ABI freeze or selected syscall signatures/status numbers.

**Distinct native namespaces:** Task-generation-scoped public CapHandle identity must be kernel checked and not rebind stale values to unrelated grants within the same live Task under current Proposed requirement. HandleSlot allocation can still be recycled if external generation compares correctly. A separate Task-scoped ReceiptSlot+preidentified operation nonce tracks a kernel transition; this nonce is neither a capability nor a transferable original Handle. The caller must know or securely discover the identity **before** an irrevocable cut, including when initial registration/ClaimID output itself is lost. Receipt-query authority must survive source Handle MOVE *without* granting new native operation rights.

**Kernel truth vs observer return:** K-S/K-I/K-U, K-Q protected query and receipt ACK/expiry need precise ordering. PENDING is not FINAL_NOT_COMMITTED; COMMITTED_KS differs from COMMITTED_KI and downstream EFFECT_APPLIED or SCOPED_DURABLE; an expired or absent record is not proof no effect occurred. A historical receive receipt does not mint a second CapGrant or prove a now-closed public token remains invokable. Bounded receipt storage must be reserved before the kernel commit it claims to describe.

**Guest separation:** Linux guest FD values **are intentionally reusable** and can share OpenFileDescriptions; a Guest FD, Native Handle token, ReplyToken, ReceiptSlot, SABI RequestId and DurableReceipt may never be converted by a raw integer cast or identical-key assumption. Exact errno/EINTR/close/epoll/restart semantics need per-operation translation, not a generic native UNKNOWN → clean no-effect map.

**P0 still open:** native public token width/overflow, receipt bootstrap and nonce collisions/GC, copyout/alias/SMP barriers, Task restart and post-Task Supervisor query rights, M0 fixed budget and independent device/storage effects. This document stays **Proposed v0.2**, with no compiled ABI, test result or implementation authorized.

## 27. 2026-10-25 Operation identity replay fence versus result evidence — Proposed

**Source:** [CAPIPC Finite Handle Namespace, Receipt Ledger & GC v0.1](../models/capipc-finite-handle-namespace-receipt-ledger-gc-v0.1.md). This addendum remains **non-normative**; no numeric ID width, syscall layout, SDK interface, kernel ring buffer or ABI status value is approved.

The separate proposed KR-M1 Task receipt identity is a **preknown strictly ordered operation sequence**, with kernel-protected high-water-mark admission and bounded records. After receipt GC, an old sequence q≤H is blocked from readmission but its actual committed outcome may be UNKNOWN. A historical completion reference does not reissue closed/invalid CapHandles; a TaskGeneration restart, a Linux guest FD or io_uring user_data value cannot be used to infer the same receipt identity or cross-reboot idempotency.

**Proof/feasibility gate:** compare strict-entry ordering versus bounded sliding sequence window and unordered nonce history; require admission+PENDING storage atomicity, commit/result coherence, quota fail-before-cut, query rights after source MOVE, expiry truth and multi-Thread liveness. Existing RFC-009 Proposed v0.2 status is unchanged.

## 28. 2026-10-25 Multi-Thread receipt ordering and SKIP — Proposed ABI research

**Reference:** [CAPIPC Ordered vs Sliding-Window Receipt Admission v0.1](../models/capipc-ordered-vs-sliding-window-receipt-admission-v0.1.md). No syscalls, public bit layout, SDK type or errno value is defined. A caller preidentifies TaskGeneration-scoped q **before** a nonreversible kernel cut, but raw q does not authorize. KR-M2 candidates use protected C/B admission and distinct R[q] for result evidence: q≤C or bit=used must not re-execute; q>C+W must fail **without** consuming q; an authorized SKIP only fences q that was never admitted. Upon final receipt GC, old q remains rejected but historical result may be EXPIRED_EVIDENCE/UNKNOWN.

**P0 OPEN:** atomic A(q)/X(q) and K-S/K-I result cuts, actual Task receipt-query and SKIP authority, protected quota-before-cut and truthful expiry/error mapping. A frontend restart with a new TaskGeneration is not proof a previous service effect didn't occur. Linux guest FD/OFD identities and SABI RequestId remain distinct; no cross-reboot exactly-once guarantee is implied.

## 29. 2026-10-25 KR-M2 cross-RFC safety-case and ABI truth boundary — Proposed

**Design-only:** [CAPIPC Cross-RFC Safety Case v0.1](../reviews/capipc-cross-rfc-safety-case-v0.1.md). Proposed q/A/X/C/B and receipt R[q] remain logical, *not* syscall fields or KABI numbers. A q≤C prevents re-admission, not proof of final COMMITTED/NOT_COMMITTED after result GC; PENDING, historical COMMITTED_KS/KI/KU and EXPIRED_EVIDENCE are distinct from SABI effect, guest Linux errno/FD and hardware durability. Query must use actual Task/scoped rights after source MOVE; no public Handle grant may be reissued from history. Commit/revoke/endpoints, faultable copyout, skip-status loss and M0 quota remain proof blockers. This addendum does not freeze any ABI or authorize code.

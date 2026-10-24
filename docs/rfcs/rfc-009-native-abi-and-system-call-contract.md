# UniOS RFC-009 — Native ABI, System Call & User-Service Contract

> **Status:** Proposed — Specification v0.1 (research/design only; NOT Accepted, implemented, frozen as stable ABI, benchmarked, or formally verified)
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
| Fuchsia Channels [S4] | messages carry bytes and handle ownership; atomic send/receive phases; explicit ownership on errors | define UniOS separate E1 queue-commit, delivery and handle install outcomes rather than borrow incompatible subtle failure semantics |
| Fuchsia async waits [S5] | objects may register async wait packets; multiple subscriptions can exist and consume resources | WaitSet quota, registration generation, edge/level/cancel semantics need precise promises |
| Fuchsia FIDL flexible types [S6] | strict/flexible unknown fields, methods and resource/handle wire metadata support evolving clients | separate SABI IDL unknown-field handling from KABI small envelope, strict unknown handles cannot leak |
| Linux syscall ABI (arch-specific) [S7] | syscall interface differs by architecture; Linux stable syscall set is distinct from POSIX library API | Linux x86_64 guest dispatch belongs to compatibility personality, not UniOS-native number table |
| Linux syscall(2) x86-64 [S8] | x86-64 Linux passes args in rdi,rsi,rdx,r10,r8,r9; calling convention not identical to ordinary C function | separate syscall machine convention from C SDK ABI and define guest register snapshot |
| Linux seccomp user notification [S9] | filters/notifier broker syscalls in user space but seccomp explicitly is not a complete sandbox | use as comparison for guest dispatch, never treat userspace filter/interception as native cap authorization |
| FreeBSD Capsicum [S10] | descriptors become rights-bearing capabilities in hybrid POSIX environment | a guest FD/proxy can carry attenuated native FileCaps; UID0 remains not native root |
| Windows NT Ntdll API docs [S11] | user applications typically call Ntdll Nt entries; undocumented Native ABI isn't stable Win32 API contract | Windows compatibility should target named documented APIs/loader semantics before native NT syscall numbers |
| Linux ABI stability docs [S12] | kernel userspace ABI has explicit stability tiers, syscalls generally append-only | define future stable vs experimental UniOS ABI claims independently, not assume all prototypes stable |
| Fuchsia wait/handle docs [S13] | close invalidates handle and can cancel associated waits, peer closure has separate signaling | choose explicit wait invalidation and peer death race semantics |
| Fuchsia FIDL wire format [S14] | a transferred handle's on-wire placeholder is not the sender's integer handle value | never treat serialized handle number as access credential |
| Linux io_uring [S15] | asynchronous ring submission/completion carries complex buffer and lifecycle rules | optional batch/ring transport only after simple syscall+Channel safety model proven |
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

A Linux ELF's libc expects Linux syscall numbers, thread-local errno, signals, TLS, fork/exec, futex, blocking and restart behavior, /proc, dynamic linker and host file and network semantics. Translating one syscall number does not confer Linux binary compatibility. Windows PE needs Windows loader DLL dependencies, Win32/NT APIs, SEH/TLS, handle ownership, registry, GUI, Wine feasibility. Darwin Mach-O needs Mach traps, dyld, Objective-C/Swift ABI, Apple framework/licensing gates. None should automatically grant native DeviceGroup, BootPolicy, TPM floor or guest UID0 global caps.

## 4. Entry/exit x86_64 and architectural domain separation

### 4.1 Entry gate, not fixed numeric syscall ABI

Native Ring 3→Ring 0 likely uses x86_64 SYSCALL or architecture-controlled exception vector; IRQ, page fault, exception and service IPC are distinct paths. Research whether the design should use SYSCALL with versioned opcode and pointer to bounded descriptor, or a small fixed register set, while managing CPU GS/TLS bases, kernel stack switching, entry state, ASID/PCID/CR3, preemption, RFLAGS masking, speculation mitigations, XSAVE/FPU context and canonical user address validation.

SYSRET has canonical return-address pitfalls and architecture-specific flags; an IRETQ or safe alternate return may be necessary. Never accept user-provided kernel stack, caller privilege, guest/native selector or raw object address merely because it appears in RAX/RDI or SDK wrapper. Raw syscall can bypass library checks; the kernel validates every invocation.

**Machine ABI ≠ C ABI:** exact syscall instruction, opcode register, arguments, return/status registers and clobbers must be specified per arch after proof. The Linux x86-64 syscall ABI uses a fourth argument in r10 rather than ordinary C function's rcx [S8]; copying Linux's calling convention into UniOS is not required. Do not publish fixed numeric register assignments, syscall IDs, struct offsets, vDSO structure or user mode flags as Stable v1 yet.

### 4.2 Native vs restricted guest P0 execution-domain problem

At each kernel entry the authoritative execution domain is **kernel-owned per-Thread/per-Task state** established by a privileged loader/control transition; it must not be inferred solely from registers, ELF note, a service header, a writable TLS flag, a CPU syscall instruction or an untrusted compatibility library.

Candidate designs to compare:
- **G1 per-Task fixed GuestDomain:** all syscall entries from guest Task are trapped and interpreted in guest mode; simpler invariants, difficult when a trusted native compat broker shares Task and wants native KABI.
- **G2 kernel-controlled per-Thread Native/GuestEntry state:** controlled privilege-checked transitions and separately mapped entry gates, careful signal/fault/return and same-thread reentrancy; more kernel TCB and state races.
- **G3 restrict guest to a userspace interpreter/emulator:** guest instructions/syscalls go through explicit host service, perhaps slower yet simpler kernel; cannot claim native hardware-speed Linux binaries.
- **G4 direct Linux syscall dispatch in kernel:** ABI compatibility easier superficially, but requires Linux-specific semantics/TCB and native/guest collision rules; not default.

**Mandatory attack example:** a guest sets its RAX to a UniOS privileged syscall number or jumps into a native syscall shim, attempts privileged native CapFactory or DeviceMap. Kernel must still dispatch according to its trusted guest-domain label and deny native-only operations; if native broker runs on same Thread, it requires a validated state-switch authorization not a plain writable flag.

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

For MOVE, kernel must atomically move live typed authority from sender table to in-transit kernel queue or receiver table under exactly one defined commit. Raw 32/64-bit user-side handle integer is Task-relative; SABI on-wire payload reserves a role/slot and **separate kernel handle-vector transfer**. Validate duplicated handles, derivative rights and destination receiver quota before commit. If delivery is impossible, define whether caps remain enqueued, move back or drop under verified contract; never silently duplicate source+destination authority by retrying.

A closed handle's integer can be reused with new object and must not make a stale outstanding IPC/Wait/Reply/driver callback act on the new object. Kernel generation/lineage and server incarnation are independent; FIDL/Zircon semantics can inform tests but do not define UniOS default atomics [S3,S4,S14].

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


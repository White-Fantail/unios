# UniOS RFC-018 — Versioned IDL, ABI Evolution & Compatibility Contracts

> **Status:** Proposed — Specification v0.1 (design research only; NOT Accepted, implemented, stable ABI, wire-format frozen, compiler-generated, benchmarked or formally verified)
> **Date:** 2026-10-25 NZDT
> **Architecture direction:** [UniOS v0.2](../architecture/unios-architecture-v0.2.md); accepted D1–D8 direction is NOT detailed acceptance of this protocol.
> **Required RFC coordination:** [RFC-002 Capabilities](rfc-002-object-capability-lifetime-delegation-revocation.md), [RFC-003 IPC](rfc-003-ipc-wire-protocol-and-synchronization.md), [RFC-004 Pager](rfc-004-virtual-memory-pager-cache-and-oom.md), [RFC-005 Scheduling](rfc-005-scheduling-preemption-and-cpu-budget.md), [RFC-006 Boot](rfc-006-boot-trust-chain-service-startup-and-recovery.md), [RFC-008 Storage](rfc-008-storage-filesystems-and-crash-durability.md), [RFC-009 KABI/SABI v0.2](rfc-009-native-abi-and-system-call-contract.md), [RFC-010 Linux Personality v0.2](rfc-010-linux-personality-and-binary-compatibility.md), [RFC-017 Verified Boot v0.2](rfc-017-verified-boot-updates-and-rollback-protection.md).
> **Hard stop:** No IDL compiler, generator, binary codec, RPC handler, new syscall, kernel wire header, guest bridge, SDK, signing key, BootPolicyManifest, software implementation, test or benchmark authorized. Only Markdown design/research is in scope.

## 0. Executive recommendation: IDL-A — bounded typed SABI with separate capability transfer

**IDL-A provisional research baseline:** Kernel IPC transports bounded opaque message bytes plus a **separate kernel-authenticated vector of transferred capabilities**, respecting RFC-003 E1 SEND ownership and ReceiveClaim. Every userspace service defines a versioned IDL contract for methods, scalar payloads, transfer slot roles, errors, effect phases and optional features. Discovery/protocol negotiation uses the already issued scoped Endpoint grant; method/version names are **data, not authority**. Native SDK wrappers evolve independently of KABI, SABI, guest Linux errno and the signed boot/recovery schema.

Core recommendation consists of seven separations:

1. **KABI trap and kernel IPC envelope**: extremely small typed transport and capability/queue accounting in Ring 0; no FIDL/Protobuf/Cap'n Proto application schema parser in kernel.
2. **SABI protocol and IDL**: method/ordinal, request identity, resource roles, bounds, strict/flexible evolution and service incarnation in bounded userspace messages.
3. **Capability transfer plane**: actual live type/rights/scope/lineage/sponsor follow the kernel cap vector, NOT an encoded integer claimed by user bytes.
4. **Native SDK ABI**: C-compatible boundary candidate + language bindings only after proof; method evolution is separate from compiler ABI.
5. **Linux/Windows/Darwin guest ABI translation**: guest status/FD/HANDLE/port metadata not native credential, guest binary feature availability independent of SABI wire version.
6. **Boot/Trust manifest**: signed, deliberately small, canonical/verified bootstrap descriptor under independent Root authority, NOT a general lenient user service packet.
7. **Storage DataVolume metadata**: persistent crash-durable schema/version and migration graph under RFC-008, not an ephemeral RPC wire version.

**No selection of exact encoding, wire word endian, field-number space, 64-bit IDs, status numeric values, C structure offsets, generated compiler or TUF/canonical manifest signature bytes in v0.1.** The prose includes candidate logical fields, **not** implementation signatures.

**Decisive safety rule:** Ability to parse a new message does NOT imply permission to act on a new grant; an unknown field may carry a capability, an authorization restriction, a durability requirement or a new result state. Ignoring it can leak authority or falsify success. Flexible evolution must be opt-in, bounded and only where semantics remain safe for older clients.

### 0.1 Competing alternatives

| Family | Benefit | Main correctness/TCB cost | Position |
|---|---|---|---|
| IDL-A custom minimal SABI schema, strict resource-aware evolution | tightly matches RFC-002/003, resource accounting and tiny kernel parser | inventing wire format/IDL/compiler is expensive; requires formal interop tests | **conditional research leader**, format unchosen |
| IDL-B adopt/port Fuchsia FIDL userspace schema/codec | explicit handle resource semantics and versioned unknown interactions | FIDL transport assumptions/Zircon rights and toolkit licensing/portability must be adapted; no drop-in Zircon | strong reference/port feasibility competitor |
| IDL-C Protobuf/Thrift style data-only messages + separate capability envelope | mature schema tooling, broad multi-language runtime | data-only unknown fields miss resource roles and parse-safe-but-policy-unsafe changes; canonical signature unsuited by default | evaluate for pure data, not automatic SABI resource transport |
| IDL-D Cap'n Proto-style schema/data and optional capability RPC | fast direct-access data model and evolution rules | pointer/segment bounds, cap RPC semantics/lifetimes and import TCB larger than zero-copy marketing | evaluate with equal security/cost model |
| IDL-E fixed non-extensible C structs | simple bootstrap, low runtime parsing | ABI padding/compiler/layout issues and locked evolution, pointer bugs | only for tiny kernel bootstrap with audited encoding |
| IDL-F CBOR deterministic restricted profile | schema-free transport flexibility; deterministic profile available per RFC 8949 | complex generic parser, duplicate keys, canonical specifics, unknown-critical fields and resource transport unsolved | candidate signed policy profile only after strict subset proof |

**Important:** this is a design comparison, not selection of a codec. Source licenses, dependency versions, parser TCB and code-generation reproducibility remain separate gates.

## 1. Scope, threat model and source claims

### 1.1 Goals

- Keep the independent capability kernel uncommitted to userspace RPC schemas, language runtimes or Linux syscall ABI.
- Make incompatible method/schema/rights/error changes detectable *before* meaningful effects where feasible.
- Define wire parse contracts for malformed lengths, duplicate fields, unknown fields/handles/enums/methods, integer overflow, nested-object explosion, resource limits and decoding against hostile bytes.
- Define source/wire/behavior/security/ABI compatibility as separate properties with independently testable old/new clients and servers.
- Preserve cap MOVE ownership and receiver ReceiveClaim, including caller cancellation, copyout partial exposure, sender failure, server death and bounded escrow cleanup.
- Expose the exact phase completed: kernel COMMITTED, service accepted, effect applied, file synced under hardware assumptions or effect UNKNOWN; avoid promising exactly once.
- Protect old and new protocol versions against *security downgrade* and "legacy default admin" behavior.
- Isolate guest ABI translation from native SABI status numbers and grant issuance.
- Support versioned service reincarnation and compatibility across upgrade/recovery without requiring full VFS/loader in wired M0 bootstrap.
- Keep production boot signer/trust policy canonicalization separate from generic flexible RPC.

### 1.2 Threat actors and failure modes

Malicious native client sends forged protocol ID/ServiceID, huge length or deeply nested payload, unknown required field, duplicate security field, extra resource handle masked as optional unknown union, stale endpoint/request after service restart, unexpected capability type or excessive rights, revoked lineage, duplicate transferred handle, incomplete cap vector, zero/negative size confusion, cyclic object references or compressed oversized body. A server may return new SUCCESS variant older SDK misreads as ordinary success; a proxy may silently erase unknown mandatory restrictions. A compromised discovery service could offer an older protocol to gain broad rights. Guest Linux UID0 may forge a numeric native handle or request "admin" method by ordinal.

Power loss/service death may happen after a message COMMITTED or a file rename applied but before any result/receipt delivered. A client timeout may lead to a duplicate non-idempotent effect; a valid "transaction ID" in a packet is not proof the backend durably recorded/deduplicated it. An old rescue OS may read a schema version it recognizes syntactically but not safely migrate it.

**Trust assumptions:** kernel cap type/rights/scope/lineage, endpoint ownership, sender/receiver Task, resource sponsor and IPC commit phase are authoritative *only* when provided by the kernel and trusted services. User headers never mint these. A service parser is an asset-specific userspace TCB, not automatically isolated from the files/devices it has been granted. Storage media and EFI/TPM hardware truth remain external RFC-008/017 predicates.

## 2. Primary technical precedents and limitations

| Official reference | Technical evidence | Proposed UniOS lesson |
|---|---|---|
| Fuchsia FIDL language [S1] | protocols distinguish open/ajar/closed unknown interactions and strict/flexible methods | choose unknown method policy explicitly; security-sensitive changes must not be silently accepted |
| Fuchsia FIDL wire [S2] | envelopes include byte and handle counts allowing unknown-field skipping; handle payload not just integer bytes | unknown handle roles need precise consumption/rejection and lossless resource accounting |
| Fuchsia FIDL API rubric [S3] | resources affect flexible unions/evolution, strict to flexible takes coordinated API windows | resource-bearing schema extension is harder than plain optional value fields |
| Fuchsia Zircon Channels [S4] | kernel transfers actual handles independently from message bytes | maintain UniOS E1 NOT_COMMITTED and ReceiveClaim; DO NOT import Zircon failure-ownership contract |
| Protobuf proto3 language [S5] | field numbers and enum values should be reserved and not reused; unknown field preservation differs by conversion | reserve tombstones; unknown byte preservation does NOT equal semantic authorization |
| Protobuf non-canonical serialization [S6] | deterministic serialization is not generally canonical, schema/runtime versions can alter bytes | no blind signing/hash over generic proto output for BootPolicyManifest |
| Cap'n Proto schema evolution [S7] | ordinals and additive evolution rules; some compatible changes alter canonical encoding | separate parse compatibility from byte-for-byte signed identity |
| Cap'n Proto canonical encoding [S8] | canonical form exists but default encoder need not output it | canonical input validation and exact signed bytes are separate |
| Apache Thrift IDL [S9] | required fields hinder soft evolution; absent/default values change semantics | use explicit requiredness/field presence, avoid security defaults |
| gRPC statuses [S10] | DEADLINE_EXCEEDED can occur after state change completed | deadline not remote rollback; effect phase belongs to service semantics |
| gRPC cancellation [S11] | cancellation doesn't automatically interrupt all server handler computations | no blanket cancellation/undo or automatic replay |
| CBOR RFC 8949 [S12] | deterministic encoding possible with specific map ordering/duplicate validation rules | consider restricted signed policy subset, not generic "CBOR is canonical" |
| Linux ABI stability docs [S13] | several stability tiers and explicit userspace compatibility promises | UniOS Experimental/Supported/Stable must name scope and maintenance rules |
| Fuchsia FIDL language strict/flexible evolution [S14] | unknown union/member and method reaction differ from unknown data field | strict security envelope, flexible only safe cases |
| UniOS RFC-002/003/009/010/017 [local] | live cap transfer and lifetime, IPC receipt, separate ABI and boot chain | SABI cannot change kernel authority, guest semantics or signed boot floor |

**Caution:** FIDL's unknown-method, cap-transfer and channel failure behavior is not automatic UniOS behavior. Protobuf/Thrift data compatibility is not capability compatibility. CBOR deterministic restrictions are a starting point, not a complete signed manifest safety proof. No wire format imported, benchmarked or code generated.

## 3. Layer ownership and version axes

### 3.1 Explicit separation graph

~~~text
Native Application (SDK source API and C-compatible ABI)
    | user-space generated client binding / versioned IDL method
    |   (not trusted as an authorization check)
    v
SABI user-space message: protocol+method, version, values, resource-role map,
                       request/effect IDs, service incarnation, declared flags
    | separate binary bytes and typed kernel cap-transfer vector
    v
UniOS KABI (RFC-003): bounded bytes, cap transit, queue commit, receive claim,
                      ReplyToken and WaitSet; NO service-method schema parse
    v
Restricted Service Endpoint with kernel-issued capabilities
    | trusted parser and policy; intersect caller grants with method rights
    | Native VFS / Network / Pager / GUI / Update / Portal
    v
Result with explicit service/effect phase (not automatically file durability)

Separate:
Linux guest ELF -> LinuxPersonality broker -> SABI; guest errno/FD scoped
Bootloader -> canonical signed BootPolicyManifest -> bounded kernel descriptor
Disk DataVolume -> durable format migration protocol, NOT SABI wire evolution
~~~

### 3.2 Compatibility dimensions, NEVER one global version

| Axis | Who governs | Change type | What success means |
|---|---|---|---|
| KABI machine trap/object semantic generation | kernel | x86_64 per-arch entry and privileged typed objects | kernel accepts native entry with required rights |
| Kernel IPC transport envelope | kernel (RFC-003) | bytes+cap vector/Endpoint/ReplyToken linearization | transport record committed, not service work |
| SABI protocol family and method schema | service owner | RPC method/field/enum and resource role evolution | matching secure method can be decoded and authorized |
| Native SDK C ABI and Rust wrappers | SDK/toolchain | symbol/calling convention, ownership, layout | chosen binary can link/call as documented |
| Linux GABI/personality | separate guest broker | Linux arch registers/errno, ELF/libc/syscall semantics | exact tested unchanged Linux application behavior |
| Windows/Darwin GABI | separate personalities | PE/NT/Win32 or Mach/ObjC/Swift | no implications for Linux/native |
| BootPolicyManifest/BootstrapDescriptor | loader, Root policy and kernel | signed canonical schema, cap ceilings, security vector | currently eligible exact image admitted under independent floor |
| DataVolume filesystem/recipe/user schema | VFS/store/update | crash-durable persistent data migration | old/new slot can actually read/decrypt required data |
| Update TUF metadata version | release Root/Targets etc. | security freshness/rotation | signed delivery eligibility, not OS execution grant |

Changing a SABI method ordinal does not change a Linux syscall number. Raising the image security floor does not mean changing the SABI version. A successful handshaken client protocol does not imply it is safe to read the current disk schema or that its SDK ABI is stable.

## 4. Security and lifecycle properties of a versioned service

### 4.1 Service discovery is not permission

An app discovers a protocol name and receives an Endpoint capability only from an authorized launcher/portal/registry carrying kernel-issued scoped authority. Strings, publisher signatures, service badges in user bytes and guessed endpoint numeric IDs cannot create rights. A compromised registry might serve malicious metadata; the client must hold an authenticated Endpoint origin (or rely explicitly on the trusted issuer/portal provenance) when deciding whether to submit secrets or transferable handles.

A service incarnation/identity binds endpoint generation and optional request namespace to a concrete running process. A restart may preserve a *logical service name* but MUST NOT automatically accept old one-shot ReplyToken, cap escrow, stale WaitSet registration or unverified prior effect receipt as new; durable reconciliation requires separate evidence.

### 4.2 Negotiation: capability is a ceiling, protocol is a filter

Candidate negotiation happens on an already-issued authorized Endpoint. Each side expresses supported versions/features (major/minor/required feature set), selected method family and resource schema generation within finite bounds. Actual effective authority = kernel grant's rights and scope **intersected with** the trusted service's method policy and caller-specific authorization. **Never union grants from independently reported features.**

Client's required security properties (e.g. only scoped DirectoryCap, mandatory durability, no raw block access, strict unknown handles, user consent) cannot be silently omitted by selecting older minor version. Server's current security policy is an independent floor and cannot be overridden by old client's deprecated default. If client and server lack a common **secure** intersection, explicitly fail to connect or deny method. Choosing a version is not authentication of the peer, and a negotiation reply cannot install a native cap by naming its role.

### 4.3 Unknown field/method classes

- **Unknown optional value-only field:** may be skipped/retained if bounded and specified to be semantically irrelevant to the older consumer; forwarding unknown values must not create hidden policy changes.
- **Unknown required security field:** reject transaction or require compatible schema upgrade. Never silently default "allow", "not encrypted", "non-durable", "full access" or "not revoked".
- **Unknown method ordinal:** strict protocol/critical method refuses with declared error/close policy; flexible unknown interaction may invoke a bounded handler only if negotiated as safe (FIDL's closed/ajar/open research).
- **Unknown enum/union:** strict security values fail closed; flexible value unknown-case allowed only with safe unsupported/ignore branch explicitly implemented. Unknown SUCCESS/effect/rights enum MUST NOT become generic SUCCESS.
- **Unknown resource/handle role:** actual cap already moved in kernel transport and MUST be accounted for and closed/rejected, or intentionally retained by an authorized typed proxy; opaque user bytes cannot hide installed authority.
- **Duplicate field/role and malformed encoding:** reject when ambiguous; especially reject conflicting authorization, durability, sender identity, resource role and request IDs. Numeric overflow or nested-object quota is a decode failure, not a service operation.

### 4.4 Tombstones and ID reuse

Protocol IDs, method ordinals, field numbers, enum values, handle-role IDs and logical status discriminants, once published in a stable profile, cannot be recycled with incompatible semantics. Deprecate and reserve/tombstone IDs, document owner and last version, disallow alias reuse in a durable schema registry. A syntactically compatible type reinterpretation (signed int vs enum, optional vs required, all-zero default changing) can break authorization and must be treated as potentially incompatible.

Do not publish a single universal ID number space for KABI, SABI and Linux syscalls; their identifiers have separate authority and versioning.


## 5. Candidate SABI envelope — logical contract, not frozen bytes

| Logical field | Authoritative origin | Failure boundary |
|---|---|---|
| Kernel queue/commit and sender Task | kernel RFC-003 | E1 COMMITTED is queue+cap transfer only |
| Actual live transferred caps | kernel typed Task-local handle table | sender bytes cannot mint authority |
| Protocol family / method / schema version | sender SABI data | service validates against negotiated grant/policy |
| Request ID / client generation | scoped client claim | collision/replay not automatically prevented |
| Service incarnation | trusted endpoint issuer and generation | old replies and requests cannot bind new service |
| Resource-role declarations | IDL sender, checked against actual cap vector | count, type, scope, rights and slot must match |
| Payload fields / flags | sender | checked size, requiredness, unknown/security variants |
| Deadline clock and relative/absolute mode | documented caller + service policy | timeout doesn't undo external effects |
| Reconciliation/idempotency key | authenticated caller method and backend | actual durable ledger required for any dedupe claim |
| Reply effect status | authorized service issuer | distinct from transport and storage durability |

**No numeric wire header, field positions, register allocation, endian, schema/compiler runtime or public status values selected.** Kernel transport parses its own bounded envelope and cap transfer only, never user-service method-specific IDL.

### 5.1 Parser resource ceilings

For each service/method define maximum bytes, handles, nested depth, repeated entries, string bytes and UTF-8 rule, string normalization if needed, integer range, map key uniqueness, decoding CPU/stack/memory charges and per-Task sponsor. Reject overflow in offsets/counts and recursive allocation. Unknown valid data may still exceed permitted service limits; never accept a 1GiB optional blob solely because the wire parser can skip it. Input user pointers can change after preflight; validate copied bounded bytes or stable pinned backing, not a mutable raw guest VA. For kernel M0, no bulky generic parser or demand-paged bootstrap resources.

### 5.2 Ordering and method classification

SABI request/response methods, one-way events and asynchronous notification streams are different contracts. RFC-003 bounded Channel E1 queue ordering applies to the declared Endpoint direction only; does not imply ordering across endpoints, restarts or external storage. Separate per-session correlation and service generation prevent a duplicate request ID from incorrectly matching a different caller. One-way delivery is not work success. Flexible unknown two-way call must have bounded explicit Unsupported/close behavior, never hang awaiting an unimplemented reply.

## 6. Resource-aware IDL and cap conservation

### 6.1 Real authority is an out-of-band typed grant

Transferred authority is not the on-wire numeric handle, guest Linux FD, Windows HANDLE or Mach port name. Sender prepares a bounded kernel cap vector with MOVE-only candidate semantics. Kernel checks actual ownership, rights/lineage/scope and ResourceBudget. Service IDL declares roles (e.g. file-data cap or reply grant) whose actual received cap index/type/rights/scope must match the live kernel-transferred vector. To COPY, caller first DERIVEs attenuated grant then MOVEs it, following RFC-002/003; arbitrary IDL DUPLICATE keyword is not enough.

**Source capability conservation:**

| State | Kernel state | Valid assertion |
|---|---|---|
| SEND_PREPARED | sender still owns handles; preflight reserves budget | error before commit = no cap movement |
| SEND_NOT_COMMITTED | no queue record/cap move | source grants preserved; provisional charges released |
| SEND_COMMITTED | queue owns complete bytes and moved cap grants | sender no longer owns them, receiver need not have read |
| SEND_RESULT_LOST | caller not sure whether commit happened | UNKNOWN, unsafe to reissue same MOVE blindly |
| RECEIVE_CLAIMED | exclusive bounded ReceiveClaim/escrow, charged | second receiver cannot claim same message |
| RECEIVE_BYTES_PARTIAL | bytes may already have reached user memory | NOT provably rolled back or secret from receiver |
| RECEIVE_CAP_COMMITTED | receiver Task table owns newly minted local handle tokens | all roles installed once, no duplicate source ownership |
| RECEIVE_ABORT / TASK_DIES | claim ends/cleanup/requeue/discard per RFC-003 | transit caps must be charged/closed and no leaked grant |
| RPC_REMOTE_EFFECT | backend may mutate VFS/device/network | IPC commit != applied or durable |

Fuchsia basic ChannelWrite can consume source handles even on failure; UniOS's proposed NOT_COMMITTED preserves them (RFC-009 v0.2). This paper **does not import Fuchsia channel failure contracts**. Byte copyout to writable user memory and kernel handle install cannot magically be one atomic visible operation. Bounded exclusive escrow/claim, publication ordering and cleanup are open P0 proofs.

### 6.2 Unknown resource field trap

A flexible unknown scalar can be safely skipped only if guaranteed semantically optional. An unknown resource-bearing union can include an **actual cap moved by kernel**, so discarding bytes without consuming/closing/rejecting the cap leaks authority. Candidate secure alternatives: fail method with an accounted close/drop; or explicitly negotiated trusted proxy that can receive and forward opaque typed cap with unchanged/narrowed rights, updated lineage, charged resource and allowed destination. Default is **reject unknown cap roles**, not forward automatically.

A generated typed binding may prevent accidental double-close and distinguish moved vs borrowed handles; it is only developer assistance. Kernel rights checks and server-side live type validation still mandatory for raw malicious syscall/IPC callers.

### 6.3 Per-call cap requirements versus negotiating features

The resource-role map must include **minimum actual type and maximum permitted scope/rights** under the current server policy; an old client declaring wider access doesn't change kernel cap. A new method demanding a stronger right is incompatible if client cannot securely authorize it, not a reason to map unknown role to "global root." Declared order/index of cap vector, duplicate handle numbers, wrong handle count, unknown role IDs and revoked endpoints should yield predictable refusal and cleanup. Extra caps never installed and ignored as harmless unknown bytes.

## 7. Evolution: parse, behavior, authority and durable outcome are different

### 7.1 Orthogonal compatibility matrix

| Predicate | Example | Not established by it |
|---|---|---|
| Wire decode compatibility | old reader skips a bounded optional field | correct method security |
| Semantic compatibility | file read/rename and error behavior preserved | byte layout stability |
| Capability ownership compatibility | sender MOVE and receiver handle lifetime unchanged | field numbering alone |
| Security monotonicity | no new authority or removed mandatory constraint | negotiated feature presence |
| Effect/durability compatibility | BufferedVisible distinct from FileSync/DirectorySync | generic SUCCESS |
| ABI binary compatibility | C call conventions and struct layout unchanged | SABI server safety |
| Source SDK compatibility | old app recompiles unchanged | binary calling or behavior |
| Guest Linux/Windows/Darwin compatibility | exact syscall/FD/error semantics tested | native SABI ordinal identity |
| Persistent format compatibility | data schema readable after OS A/B trial/floor | RPC version handshake |
| Boot trust compatibility | verifier root/floor and signed manifest canonical bytes | successful SABI decoding |

### 7.2 Change classes

- **E0 Add value field:** optional bounded scalar with *security-neutral* absence/default; old client may ignore only if service can prove same authorized effect.
- **E1 Add resource field:** new Capability role/handle-bearing union; requires explicit resource negotiation and close/reject/proxy, usually a compatibility-impacting change.
- **E2 Add mandatory policy:** file permissions, encryption, durability, consent, revocation, authentication or safety require server enforcement across old methods or refuse outdated method/version.
- **E3 Reinterpret old field:** status value, units, deadline clock, open-file-description ownership, MOVE→COPY, durability tier or guest errno; **wire compatible is not behavior compatible**.
- **E4 Recycle identifiers:** reused ordinal/field/enum/role/status number with new meaning; forbidden for stable schemas—reserve/tombstone.
- **E5 Guest mapping:** native method evolves but guest Linux syscall still expects original errno/restart/FD semantics; bridge must handle explicit conversion or say Unsupported.
- **E6 Security patch:** old operation may be intentionally denied despite previously announced compatibility, documented as security-required refusal with migration path.

Protobuf/Thrift show field presence, default and reserved numeric values matter [S5,S9]; protobuf binary wire may keep unknown fields but can lose them on JSON transformation [S5]. Unknown preservation does not make authorization-preserving forwarding.

### 7.3 Version negotiation and downgrade

Candidate protocol family ID + major/minor + required feature set + negotiated handle-role set, all bounded and versioned under **one already-authorized Endpoint grant**. A server-side security minimum is not client negotiable: "I only support old protocol" cannot let a deprecated client ask for global DirectoryCap, legacy unsigned executable load or weakened data durability. If there's no common secure intersection, reject method or session.

Protocol identity/ServiceID in a payload is not authenticated by itself. A trusted launcher/portal grants actual Endpoint; on restart the endpoint's service instance/generation must be revalidated. Mixing client versions under rolling update needs tests for OldClient→NewServer and NewClient→OldServer, with mandatory feature rejection. An older SDK may connect only if it can represent required status and own every cap transferred.

### 7.4 Tombstones and default semantics

Stable field numbers, ordinals, enum values, resource roles and result codes never recycle for unrelated meaning. Registry must record deprecated/reserved IDs and previous requiredness/defaults. Optional→required and changed default values are security and binary compatibility risks. Unknown negative or large enum value is not mapped to SUCCESS. Unknown "yes" bits in rights/flags don't mean permissions granted. A newer protocol with required additional security cannot be presented as minor wire-compatible merely because the parser doesn't crash.

## 8. Explicit effect, errors, idempotency and guest mapping

### 8.1 Layers of effect

~~~text
T: kernel send COMMITTED and cap vector now in transit
R: server received/claimed and method validated
E: service completed/appended/deleted/sent remote effect
D: exact FileSync/DirectorySync/Flush-based durability receipt achieved
O: client actually observed the report for the named phase
~~~

No implication T→R→E→D→O without independent evidence. An ACK may be lost at O even after D; a deadline can expire when external operation already succeeded. RFC-008's disk durability is conditional on actual honest hardware and scoped metadata; signed receipt not media proof.

| Logical status (no numeric assignment) | Truthful scope |
|---|---|
| NOT_COMMITTED | this E1 send/MOVE did not commit; sender still owns caps |
| COMMITTED | this one transport queue/cap record committed, not the service effect |
| UNKNOWN_TRANSPORT | caller cannot know source handle MOVE result, no blind retry |
| BAD_SCHEMA / UNKNOWN_REQUIRED | service refuses bounded method decode, assuming no prior method side effect |
| BAD_RESOURCE / ACCESS_DENIED | live cap type/rights/scope/policy failed for this operation |
| STALE_SERVICE_INCARNATION | old endpoint/reply no longer valid for new server |
| DEADLINE_EXPIRED / CANCELLED_LOCAL | local waiting ended, remote effect not necessarily canceled |
| EFFECT_APPLIED / EFFECT_NO_EFFECT / EFFECT_UNKNOWN | service evidence of method outcome, not automatically disk sync |
| BUFFERED_VISIBLE | live cache readable, no crash survival claim |
| SYNC_COMPLETED_SCOPED | named data/journal/directory/device conditions satisfied |
| RECEIPT_UNAVAILABLE | reconciliation cannot establish result after failure; no false safe retry |
| UNSUPPORTED_FEATURE | no compatible secure method behavior |
| FATAL_PROTOCOL_ERROR | decoder/security invariant violated; contain/close with cap cleanup |

gRPC expressly documents that DEADLINE_EXCEEDED can happen after state-changing work finishes [S10]; cancel doesn't forcibly interrupt a handler [S11]. UniOS must not label a remote-effect UNKNOWN with a definite Linux errno that tells callers the effect did not happen. Map native statuses to Linux/NT/Mach on a per-operation semantic model (RFC-010), not by integer equality.

### 8.2 Retry-safe vs unsafe

| Operation kind | Retry when proved NOT_COMMITTED | After UNKNOWN |
|---|---|---|
| pure stat/read | generally if still authorized, different snapshot possible | may repeat, but snapshot/time can differ |
| MOVE of cap | yes when sources proven retained and policy unchanged | **NO** without ownership reconciliation |
| non-idempotent create/append/send/rename | only if no effect proved, not merely no transport | **NO** until durable transaction evidence |
| idempotent set under version guard | conditional safe on unchanged authenticated scope | only if durable state/key reconciliation proven |
| FileSync | request may be repeated with new scope, generation and deadline | must inspect exact write/receipt status |
| firmware trust/floor change | not generic RPC / guest interface | never use RPC retry semantics as key operation authorization |

An IDL request_id is only a correlation value; it is not exactly-once execution. Reliable dedupe requires authenticated request identity, exact method+argument digest, durable result log written with correct ordering to the external effect, bounded retention and boot/service-incarnation recovery. If external downstream write and local receipt are not atomically coordinated, exactly-once across both is not promised. Service-issued persistent transaction IDs cannot gain issuer authority from a client string.


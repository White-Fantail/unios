# UniOS RFC-018 — Versioned IDL, ABI Evolution & Compatibility Contracts

> **Status:** Proposed — Specification v0.2 (independent adversarial re-review; NOT Accepted, implemented, stable ABI, wire-format frozen, compiler-generated, benchmarked or formally verified)
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

**v0.2 corrections:** (a) a completed kernel MOVE cannot be undone by a later userspace decode failure; unknown transferred resources must be accounted for and settled; (b) a delegable Endpoint bearer cap doesn't authenticate the original human or actual caller, so identity-sensitive services need kernel/issuer-attested provenance or must treat every holder as equivalent; (c) negotiations are bound to Endpoint, service incarnation, server authorization policy epoch and agreed resource schema, not a mutable service name; (d) a flexible unknown value differs from an unknown resource and binary-to-JSON Protobuf conversion can lose unknown fields; (e) signed boot bytes must have consistent schema/parser/domain semantics; (f) unknown effects are not automatically safe Linux errno or success.

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
| Fuchsia FIDL language [S1,S17] | closed/ajar/open protocol and strict/flexible interactions differ; flexible unknown value types cannot safely admit unknown handles | unknown method/resource policy must distinguish data-only from resource and one-way from two-way |
| Fuchsia FIDL wire [S2,S18] | envelopes provide byte and handle counts; unknown content can be skipped only with handling of transferred resource handles | post-MOVE decoder rejection cannot restore a sender's already moved handles |
| Fuchsia FIDL API rubric [S3] | resources affect flexible unions/evolution, strict to flexible takes coordinated API windows | resource-bearing schema extension is harder than plain optional value fields |
| Fuchsia Zircon Channels [S4] | kernel transfers actual handles independently from message bytes | maintain UniOS E1 NOT_COMMITTED and ReceiveClaim; DO NOT import Zircon failure-ownership contract |
| Protobuf proto3 language [S5,S19] | unknown binary fields may survive binary decoding, but ProtoJSON does not preserve unknown fields; reserved numbers/names differ | no safe opaque critical-field forwarding through arbitrary JSON/field-copy bridge |
| Protobuf non-canonical serialization [S6] | deterministic serialization is not generally canonical, schema/runtime versions can alter bytes | no blind signing/hash over generic proto output for BootPolicyManifest |
| Cap'n Proto schema evolution [S7] | ordinals and additive evolution rules; some compatible changes alter canonical encoding | separate parse compatibility from byte-for-byte signed identity |
| Cap'n Proto canonical encoding [S8] | canonical form exists but default encoder need not output it | canonical input validation and exact signed bytes are separate |
| Apache Thrift IDL [S9] | required fields hinder soft evolution; absent/default values change semantics | use explicit requiredness/field presence, avoid security defaults |
| gRPC statuses [S10] | DEADLINE_EXCEEDED can occur after state change completed | deadline not remote rollback; effect phase belongs to service semantics |
| gRPC cancellation [S11] | cancellation doesn't automatically interrupt all server handler computations | no blanket cancellation/undo or automatic replay |
| CBOR RFC 8949 [S12] | core deterministic key order differs from legacy length-first ordering; duplicate keys and key equivalence need strict decoder policy | a signed profile must mandate duplicate rejection, ordering, allowed tag/float forms and parser equivalence |
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

**Delegation/confused deputy correction:** a bearer Endpoint proves a granted operation ceiling but does NOT automatically reveal original user identity, request origin or provenance through forwarding. If service policy needs identity, require independently trusted kernel/issuer per-call metadata or design bearer-only policy that treats all valid holders equivalently. Reject sender-authored caller ID, PID or purported issuer as authentication. Holder, sponsor, original grant creator, actual caller and issuer can differ.

A service incarnation/identity binds endpoint generation and optional request namespace to a concrete running process. A restart may preserve a *logical service name* but MUST NOT automatically accept old one-shot ReplyToken, cap escrow, stale WaitSet registration or unverified prior effect receipt as new; durable reconciliation requires separate evidence.

### 4.2 Negotiation: capability is a ceiling, protocol is a filter

**Session-binding correction:** negotiate with the concrete Endpoint grant and service incarnation plus server authorization-policy epoch, resource-role schema and required feature floor. Previously negotiated metadata cannot survive revocation, service restart or policy/issuer change automatically; verify current live grants and policy at each effectful method. Do not trust a cached handshake flag as lasting authorization.

Candidate negotiation happens on an already-issued authorized Endpoint. Each side expresses supported versions/features (major/minor/required feature set), selected method family and resource schema generation within finite bounds. Actual effective authority = kernel grant's rights and scope **intersected with** the trusted service's method policy and caller-specific authorization. **Never union grants from independently reported features.**

Client's required security properties (e.g. only scoped DirectoryCap, mandatory durability, no raw block access, strict unknown handles, user consent) cannot be silently omitted by selecting older minor version. Server's current security policy is an independent floor and cannot be overridden by old client's deprecated default. If client and server lack a common **secure** intersection, explicitly fail to connect or deny method. Choosing a version is not authentication of the peer, and a negotiation reply cannot install a native cap by naming its role.

### 4.3 Unknown field/method classes

- **Unknown optional value-only field:** may be skipped/retained only if bounded and security-neutral. Protobuf-to-JSON or manually rebuilt messages may drop unknown data [S19], so end-to-end preservation needs proof for every relay.
- **Unknown required security field:** reject transaction or require compatible schema upgrade. Never silently default "allow", "not encrypted", "non-durable", "full access" or "not revoked".
- **Unknown method ordinal:** strict protocol/critical method refuses with declared error/close policy; flexible unknown interaction may invoke a bounded handler only if negotiated as safe (FIDL's closed/ajar/open research).
- **Unknown enum/union:** strict security values fail closed; flexible value unknown-case allowed only with safe unsupported/ignore branch explicitly implemented. Unknown SUCCESS/effect/rights enum MUST NOT become generic SUCCESS.
- **Unknown resource/handle role:** kernel SEND/MOVE may have COMMITTED before userspace decoding fails. Live caps in transit/claim/receiver custody MUST be accounted for and closed/rejected or explicitly proxied. Decode rejection NEVER retroactively restores source ownership. Handle close does not imply actual DeviceGroup DMA R2-F fence.
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

A flexible unknown scalar can be safely skipped only if guaranteed semantically optional. An unknown resource-bearing union can include an **actual cap moved by kernel**, so discarding bytes without consuming/closing/rejecting the cap leaks authority. Candidate secure alternatives: fail method with an accounted close/drop; or explicitly negotiated trusted proxy that can receive and forward opaque typed cap with unchanged/narrowed rights, updated lineage, charged resource and allowed destination. Default is **reject unknown cap roles**, not forward automatically. After SEND_COMMITTED, rejection must settle already transferred authority in a bounded transit/receiver state; sender source caps are not automatically restored. Close/drop of a DeviceGroup handle does not establish R2-F quiescence.

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

Candidate protocol family ID + major/minor + required feature set + negotiated handle-role set, all bounded and versioned under **one already-authorized Endpoint grant**. A server-side security minimum is not client negotiable: "I only support old protocol" cannot let a deprecated client ask for global DirectoryCap, legacy unsigned executable load or weakened data durability. If there's no common secure intersection, reject method or session. Negotiation is bound to actual Endpoint grant, server incarnation, required features, policy epoch and resource-role schema; prior handshake cannot survive revocation, policy update, server restart or wider cap change without revalidation.

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

No implication T→R→E→D→O without independent evidence. **Effect and observation precision:** T/R/E/D/O are separately evidenced milestones, not mandatory steps for every operation. Every claimed effect or receipt needs trusted issuer, exact resource/generation and phase. If a guest SDK cannot represent EFFECT_UNKNOWN without implying success or no effect, deny compatibility/contain the session rather than fabricate a safe error.

An ACK may be lost at O even after D; a deadline can expire when external operation already succeeded. RFC-008's disk durability is conditional on actual honest hardware and scoped metadata; signed receipt not media proof.

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

An IDL request_id is only a correlation value; it is not exactly-once execution. Caller, method, exact effect digest, policy/service generation, dedupe retention and authenticated durable ledger matter independently; an untrusted proxy cannot reuse another caller's request ID as effect authorization. Reliable dedupe requires authenticated request identity, exact method+argument digest, durable result log written with correct ordering to the external effect, bounded retention and boot/service-incarnation recovery. If external downstream write and local receipt are not atomically coordinated, exactly-once across both is not promised. Service-issued persistent transaction IDs cannot gain issuer authority from a client string.


## 9. Signed boot manifest, canonical bytes and trust isolation

### 9.1 Boot policy is NOT a lenient runtime IDL packet

RFC-017 requires BootPolicyManifest independently signed/verified from pre-existing Root trust, hardware/security version-floor policy, exact kernel/M0/SystemImage digest and small typed BootstrapDescriptor. Signed bytes alone are insufficient if trusted readers disagree on parsed security meaning. Bind signing domain/role, exact canonical or accepted input bytes, parser-equivalence and key/critical-field semantics, signer epoch and hardware profile; refuse ambiguous duplicates or unknown policy-critical extensions. A runtime SABI codec chosen for convenience is not automatically suitable as a signed manifest; a manifest signature must be domain-separated from an app message or DataVolume record even when payload bytes match. generic Protobuf deterministic serialization is explicitly **not** generally canonical [S6]. CBOR RFC 8949 can specify a deterministic restricted profile, but duplicate map keys, canonical tag/number choices, invalid text, nesting and algorithm confusion require a strict custom format/profile [S12]. Cap'n Proto can canonicalize, but default encoders do not always emit canonical form [S8].

**Choose later:** signed exact canonical byte string (or exact verified raw bytes with policy-defined encoding), size and cryptographic digest, schema/revision and security-extension criticality, Root/key epoch, enforceable independent version floor and exact signer policy. A manifest cannot select arbitrary new signing key to validate itself; a boot service's protocol version cannot override trusted Root/signer policy or grant ceiling. Any unknown critical signer/rights/security policy field leads to rejected boot candidate or explicit recovery-only mode, not a default allow.

### 9.2 Parser and trust zone

The signing parser belongs to a minimal verified loader or narrowly trusted M0 under bounded startup resource budget, with the kernel consuming only a small fixed typed bootstrap record (RFC-006/017). Do not put generic user RPC Protobuf/FIDL/CBOR whole-parser, reflection metadata, dynamic language runtime and network certificate graph in Ring 0 solely to parse BootPolicyManifest. A self-described required field cannot be ignored if a newer signed policy requires it; old rescue loader can only boot under a separately signed explicitly compatible image with correct floor/key/data schema.

### 9.3 Update and data format coupling

- App and SABI version upgrade may happen while clients remain running and old services have in-flight ReplyTokens; not the same as OS image A/B update.
- DataVolume schema migration after B trial can prevent older but signed A reading user files. A successful old SABI decoder says nothing about data/key recovery.
- TPM-backed security index, signer/root epoch and locally trusted TUF metadata versions are distinct; IDL version cannot arbitrarily increase or lower them.
- A/B Good and FloorWriter authority are separate kernel/boot-policy grants. A client calling a method named MarkGood/FloorAdvance does not get that authority by name.
- Persistent idempotency ledger and rollback record must have independent crash-durability and policy version semantics. On loss, effects are UNKNOWN, not silently replayed.

**No proof of universal rollback:** even if binary wire is backwards readable, durable data, persistent signing policy and hardware rescue availability may not be compatible. The update protocol must state which dimensions are jointly required for a permitted floor/key transition.

## 10. SDK and guest ABI evolution contracts

### 10.1 Native C ABI candidate separate from IDL code generation

A generated Rust client may use internal structs, slices, future executors and enum layouts. None is automatically stable across compiler versions. Public Native SDK C ABI requires fixed calling convention, numeric width, struct size/alignment, outparam ownership, symbol version, stack/unwind and allocator semantics proven on selected architecture. IDL change producing new generated Rust code can be source-compatible and still be binary-incompatible if runtime wrappers or C signatures change. Bindings must never trust user-space static type checks as kernel authorization.

### 10.2 Linux guest semantics remain Linux

LinuxPersonality guest errno, signal restart, close/dup/epoll FD/OFD semantics, shared mmap/futex, exec and entropy behavior defined by RFC-010 are separate from SABI method and error changes. For example a new Native FS method might include a stronger DurableReceipt; the guest fsync result may only return after its correct Linux file/directory scope, not after generic IPC ACK. When Linux old binary does close(fd) and receives EIO, it may have lost FD already; a new SABI INVALID_HANDLE reply does not license repeating close on a recycled integer.

Windows and Darwin guest runtime method/property interfaces are separate compatibility studies. An opaque Linux FD or Windows HANDLE value cannot be passed into an IDL typed Capability field as if it were a native kernel token.

### 10.3 Version binding by session, not by mutable service name

A guest or native app holds an authorized Endpoint tied to protocol family, service generation and allowed feature set. An upgrade can replace service instance and schema but old pending handle cap transit/replies must settle under previous generation. Reconnection to newly discovered service requires new issuer-granted Endpoint, repeated secure feature negotiation and explicit reconciliation of previous effects. No old address/port name or nominal string is a shared globally trusted identity.

## 11. Critical adversarial cases (C18-01…C18-44)

| ID | Adversarial input or transition | What must be proved or refused |
|---|---|---|
| C18-01 | old client ignores new optional authorization-required field | server enforces restriction independently or refuses old version |
| C18-02 | resource field added to flexible union, unknown to old decoder | actual moved cap closed/rejected/proxied under explicit authorization |
| C18-03 | IDL payload contains fake native handle integer | cannot mint kernel cap by deserializing bytes |
| C18-04 | method ordinal recycled after delete | stable ID registry tombstones it and negotiation rejects misuse |
| C18-05 | old flexible enum receives new numeric SUCCESS_WITH_LIMITS and maps plain SUCCESS | unknown success/status fails closed |
| C18-06 | field default changes from needConsent=true to false | semantic/security incompatibility blocks rollout |
| C18-07 | duplicate permission field one deny and one allow | deterministic single interpretation or reject duplicate |
| C18-08 | nested list count/offset overflows size, decoder allocates gigabytes | bounded parser, checked arithmetic and sponsor quota |
| C18-09 | in-band Endpoint name claims BootFloorWrite capability | actual typed cap/issuer required |
| C18-10 | sender double-moves same handle in one message | one committed ownership event or no move, never duplicates |
| C18-11 | sender NOT_COMMITTED while other Task sees cap | prohibited; preserves source ownership/charge |
| C18-12 | IPC COMMITTED then sender loses return | caller UNKNOWN, cannot blindly repeat MOVE |
| C18-13 | receiver copies secret bytes then cap install fails | no false rollback/privacy guarantee, bounded exclusive claim |
| C18-14 | receiver holds ReceiveClaim, never ACK | quotas, timeout/abort/cleanup and protected M0 reserve |
| C18-15 | old server returns unknown resource role, new client not prepared | explicit close/reject and no leaked native authority |
| C18-16 | a proxy preserves unknown handle automatically and forwards to unrelated service | must not silently extend scope/lineage or destination authority |
| C18-17 | new ServiceInstance with same service name receives stale ReplyToken | reject due to generation and issuer |
| C18-18 | attacker reuses another caller's request ID / idempotency key | scope by authenticated principal, method, exact digest |
| C18-19 | external file rename committed, result lost, client deadline expired | server effect UNKNOWN or durable reconciliation, not safe retry |
| C18-20 | idempotency ledger stored after external effect, crash occurs between writes | duplicate effect remains possible; exactly-once claim denied |
| C18-21 | backend FileSync reports file data but unsynced parent directory | scoped receipt not automatic namespace durability |
| C18-22 | older SDK maps new result type to generic SUCCESS | reject unknown effect/durability classes |
| C18-23 | wrong monotonic deadline clock after suspend/resume | explicit clock identity, overflow rules, local cancellation only |
| C18-24 | negotiation downgrades to old method with broader default rights | security floor enforced by issuer/server, no rights union |
| C18-25 | signed manifest uses Protobuf deterministic bytes assumed canonical across schema versions | reject unreviewed signing encoding; separate signed profile |
| C18-26 | CBOR map contains duplicate RootKey and ambiguous parser interpretation | strict unique key validation / abort; no last-wins trust |
| C18-27 | loader reads unknown security-critical extension but treats it as optional | fail closed or independently signed compatible rescue |
| C18-28 | new Root/Update schema with older A still signed but unable to read DataVolume V2 | compatibility is data/key/signer/floor/hardware joint gate |
| C18-29 | Linux guest uid=0 chooses version supporting raw DeviceGroup method | guest ID not kernel-native capability |
| C18-30 | SABI byte message holds expired transferred IRQ cap after R2-F unresolved | no reuse/claim hardware quiescence without actual fence |
| C18-31 | old/new server overlap, old client reconnects and gets old stale Endpoint | issuance, service generation and rights revalidated |
| C18-32 | mixed SDK languages interpret endian/field presence/default differently | golden vectors, cross-language schema conformance gates |
| C18-33 | sender MOVE committed, receiver rejects unknown handle-bearing union, asks sender to resend original source handle | cap no longer at sender; must settle transit/receiver resources, no fake NOT_COMMITTED |
| C18-34 | delegated Endpoint and original user identity conflated by an identity-sensitive server | trusted per-call provenance or bearer-only policy, no confused-deputy escalation |
| C18-35 | service restarts under same name and old negotiated session claims more rights | bind handshake to exact Endpoint+service instance+policy epoch; reissue/revalidate |
| C18-36 | revoked grant is accepted due to cached feature agreement | per-effect live cap scope/lineage check and policy-epoch invalidation |
| C18-37 | Protobuf binary payload traverses ProtoJSON relay and loses future security-required field | refuse unsafe data-format hop or enforce constraint at final service |
| C18-38 | closed or ajar protocol receives unknown two-way method and never replies | explicit strict/closed refusal; flexible only where protocol and method openness allow |
| C18-39 | FIDL-like flexible value union contains unknown live resource | value/resource distinction enforces rejection/settlement, not scalar skip |
| C18-40 | version negotiation message itself sent with unknown DeviceCap role | no grants or rights from handshake; reject/close in bounded escrow |
| C18-41 | stored signed BootPolicy bytes accepted by two parser versions with different grants | strict grammar/parser equivalence and critical-field rejection, role/domain binding |
| C18-42 | CBOR decoder accepts duplicate root keys under first-wins, other decoder last-wins | fail signing/boot validation, strict duplicate rejection and canonical profile |
| C18-43 | signer uses one domain for BootPolicy and user RPC payload, cross-type signature replay | domain/role/schema separation of signature input |
| C18-44 | older Linux shim sees EFFECT_UNKNOWN and maps it to errno indicating no side effect | explicit unsupported/abort/reconciliation; never invent binary compatibility |

## 12. Proposed invariants (IDL-01…IDL-54) — NOT proved

| ID | Safety/compatibility obligation (proposed, not verified) |
|---|---|
| IDL-01 | KABI transport envelope, user-space SABI IDL, Native SDK ABI, guest ABI, signed manifest and DataVolume format have separately versioned trust boundaries. |
| IDL-02 | Kernel never parses arbitrary user method IDL nor elevates typed capabilities from user-supplied bytes/ServiceID. |
| IDL-03 | Protocol negotiation occurs only through an issuer-authorized Endpoint grant and cannot independently mint capabilities. |
| IDL-04 | Effective method authority is intersection of kernel cap rights/scope and current service authorization, never feature bit union. |
| IDL-05 | An old protocol version cannot bypass a newly mandatory authorization, consent, encryption, durability or safety condition. |
| IDL-06 | Missing secure feature intersection causes explicit Unsupported/deny, never silent fallback to insecure default. |
| IDL-07 | Message byte length, depth, element count, handle count and decoder work are checked and sponsor bounded. |
| IDL-08 | Arithmetic overflow, duplicate critical fields, ambiguous encodings and malformed mandatory values cause reject. |
| IDL-09 | Unknown optional scalar fields can be skipped only when semantically and resource safe for that method. |
| IDL-10 | Unknown required security fields/methods/enum result kinds fail closed under declared protocol policy. |
| IDL-11 | Unknown transferred Capability field must have explicit close/reject/authorized proxy and resource accounting, never plain ignore. |
| IDL-12 | Actual cap existence, type, rights, scope, lineage and sponsor derive from kernel transfer, not IDL declaration. |
| IDL-13 | SABI resource role indices/count and expected types align exactly with kernel cap-transfer vector. |
| IDL-14 | No raw sender Task-local handle integer, guest FD, Windows HANDLE or Mach port name is a native transferable grant. |
| IDL-15 | Candidate E1 NOT_COMMITTED preserves sender source grants; COMMITTED moves all authorized caps to kernel transit once. |
| IDL-16 | UNKNOWN send result never allows blind repeated MOVE or non-idempotent service invocation. |
| IDL-17 | ReceiveClaim, byte publication, cap-table install and queue removal are distinct, bounded states; partial user memory writes aren't magically rolled back. |
| IDL-18 | Unrecognized resource-bearing unions/fields do not widen authority during proxy/forward or schema evolution. |
| IDL-19 | Resource-bearing service operations are charged and reclaimed across failure, claimed messages and service death. |
| IDL-20 | One-shot ReplyToken cannot be forged/replayed as a bare user request ID or revived in a new service incarnation. |
| IDL-21 | Service name/protocol ID/version are metadata, not globally trusted issuer identities. |
| IDL-22 | KABI enqueue commit is not service execution, side-effect completion, user receipt or persistent data durability. |
| IDL-23 | Deadline and local cancellation do not assert server canceled/undid an external effect. |
| IDL-24 | Status values state the exact effect phase and cannot be translated unknown/new SUCCESS to older plain SUCCESS. |
| IDL-25 | Idempotency/reconciliation guarantees require an authenticated actual backend ledger and appropriate durable ordering, not solely client request_id. |
| IDL-26 | "Exactly once" is not promised for arbitrary cross-device/network transactions solely from a message schema. |
| IDL-27 | FileSync/DirectorySync receipts remain scoped by immutable write generation and honest RFC-008 device assumptions. |
| IDL-28 | Stable field/method/status/resource ordinals cannot be reassigned to incompatible semantics; tombstones remain reserved. |
| IDL-29 | A field default, type/presence, strictness or requirement change is examined for semantic and security compatibility, not only wire parser acceptance. |
| IDL-30 | Mixed old/new clients/servers use explicit feature and lifecycle limits; no silent unapproved security downgrade. |
| IDL-31 | A prior service reply/handle/wait/transaction result is not reattached to a different service incarnation by name reuse. |
| IDL-32 | Signed BootPolicyManifest uses independently justified canonical/exact byte authorization, not generic runtime SABI flexibility. |
| IDL-33 | Untrusted manifest cannot choose its own signing root, security floor, kernel grants or unknown-critical extension bypass. |
| IDL-34 | A valid SABI version handshake does not establish compatibility with current DataVolume schema, sealed key or signer/floor eligibility. |
| IDL-35 | Native SDK C ABI calling convention and Rust generated types are not equated or frozen without separate binary/layout proof. |
| IDL-36 | Linux errno/FD/futex/signal/exec and Windows/Mach guest semantics remain outside native SABI numeric status contract. |
| IDL-37 | A guest's UID0 or native policy string cannot promote Endpoint to FileRoot/IRQ/BootFloor authority. |
| IDL-38 | Kernel M0 boot/rescue does not depend on a pageable full userspace IDL reflection/runtime/registry service. |
| IDL-39 | A schema compatible on paper is not declared compatible for products until cross-version, cross-language and adversarial tests exist. |
| IDL-40 | Optional shared-memory/zero-copy serialization preserves validated immutable snapshot/ownership despite user pointer races. |
| IDL-41 | Security key rotation, update floor and user-data format transitions require independent recovery predicates even if RPC encoding stable. |
| IDL-42 | All IDL choices remain Proposed; neither numeric values nor executable codec/SDK compiler or stable v1 ABI is authorized. |
| IDL-43 | Completed E1 capability MOVE is not retroactively canceled by userspace IDL decode failure; receiver/escrow settles the committed grants. |
| IDL-44 | Rejecting an unknown live resource cannot be reported as sender retaining the pre-MOVE source handle. |
| IDL-45 | Endpoint bearer possession cannot by itself authenticate the original user or delegation chain; services requiring identity use trusted caller provenance or explicit bearer-only authority. |
| IDL-46 | Negotiated feature/resource roles are bound to an issued Endpoint, service incarnation, current issuer policy epoch and actual current grant. |
| IDL-47 | Old cached handshake results cease to authorize work after revocation, policy floor change or service reincarnation. |
| IDL-48 | Flexible unknown **value** and flexible unknown **resource** have different decoder and ownership requirements; unknown resource may not be silently skipped. |
| IDL-49 | Binary Protobuf unknown-field retention does not imply ProtoJSON/proxy/generator preservation or security-safe forwarding. |
| IDL-50 | Unknown strict/flexible calls follow chosen protocol openness and direction; no unsupported two-way request may hang as if a success reply were guaranteed. |
| IDL-51 | Signed policy verification binds exact bytes to unambiguous parser semantics, signer role/domain, required critical fields, hardware scope and bootstrap grant ceiling. |
| IDL-52 | Restricted CBOR deterministic profile and map key equivalence/duplicate rejection are separate from generic CBOR decoding, where differing parsers may disagree. |
| IDL-53 | SDK/guest outcome mapping cannot turn EFFECT_UNKNOWN into plain success or a false proven no-effect result solely to preserve legacy binary behavior. |
| IDL-54 | DeviceCap close, receiver resource rejection and generic cap destruction are not proof of R2-F hardware DMA quiescence or rollback of previously committed work. |


## 13. Open issues (O18-01…O18-49) — no automatic resolution

| ID | Priority | Required research or proof | Co-review |
|---|---|---|---|
| O18-01 | P0 | IDL-A custom schema versus FIDL/resource-adapted runtime and other formats: true parser/codegen TCB and lifecycle tradeoff? | 001/003/009/018 |
| O18-02 | P0 | Kernel IPC frame vs SABI envelope: which field is kernel trusted, copied and sponsor charged? | 002/003/018 |
| O18-03 | P0 | Exact cap role/count/order/type/rights/scope validation against kernel MOVE vector and unknown resource union? | 002/003/018 |
| O18-04 | P0 | E1 NOT_COMMITTED preserves sender caps, COMMITTED moves, result UNKNOWN after copyout: final observable operation contract? | 002/003/009/018 |
| O18-05 | P0 | ReceiveClaim and byte publication before cap install: partial disclosure, abort and receiver death proof? | 003/004/009/018 |
| O18-06 | P0 | Hostile receiver retains claim forever and starves M0: quota, lifetime, reclaim and future progress? | 002/003/005/018 |
| O18-07 | P0 | User pointer modification/shared-memory alias during decoding, cap-vector validation and service method action? | 004/009/018 |
| O18-08 | P0 | Protocol/features handshake cannot downgrade security policy/mandatory rights/durability? | 002/009/018 |
| O18-09 | P0 | Unknown required field, method, union, enum, status and transferred resource handling semantics on old binaries? | 003/009/018 |
| O18-10 | P0 | Data-only optional field addition whose absence changes consent, encryption or file safety? | 008/009/018 |
| O18-11 | P0 | Server method policy intersects actual cap rights and issuer authorizations without ambient registry identity? | 001/002/018 |
| O18-12 | P0 | Stable method/field/enum/role/status ID tombstones and signed registry governance? | 009/018 |
| O18-13 | P0 | Committed service effect then return lost: bounded receipt and accurate UNKNOWN from caller point of view? | 003/008/009/018 |
| O18-14 | P0 | Durable authenticated dedupe ledger + effects ordering if any method claims idempotency/exactly-once? | 003/008/018 |
| O18-15 | P0 | Old service incarnation ReplyToken/cap transfer result reattached incorrectly to new server? | 002/003/006/018 |
| O18-16 | P0 | How are transport, method admission, effect and scoped storage durability exposed without false SUCCESS? | 003/008/009/018 |
| O18-17 | P0 | BootPolicyManifest exact canonical signed encoding vs generic SABI serializer non-canonical behavior? | 006/017/018 |
| O18-18 | P0 | Signed manifest unknown security-critical extension/duplicate key/signer confusion and small kernel descriptor? | 001/006/017/018 |
| O18-19 | P0 | Signed Root/security floor, boot-good and data schema versioned separately from service ABI? | 006/008/017/018 |
| O18-20 | P0 | Linux guest errno/restart/status, FD close/epoll and fork/exec semantics survive SABI changes? | 009/010/018 |
| O18-21 | P0 | Old/New RPC peers negotiate security feature without elevating rights or losing mandatory status semantics? | 002/009/018 |
| O18-22 | P0 | Message parsing bounded for nested objects, overlong length, unknown encrypted blob and malicious UTF8? | 003/018 |
| O18-23 | P0 | Source/SDK C ABI ownership, panic/unwind, struct layout and borrowed/moved cap safety across language bindings? | 009/018 |
| O18-24 | P0 | Trusted service discovery plus Endpoint origin/authentication and controlled rebind on restart? | 002/003/006/018 |
| O18-25 | P1 | Exact field-number/method-ordinal format, endianness, width, magic, validation and parser simplicity? | 009/018 |
| O18-26 | P1 | Unit tests for malformed duplicate/unknown fields and transparent parser ambiguity in candidate codec? | 015/018 |
| O18-27 | P1 | FIDL compiler/transport port license and out-of-band cap semantics under UniOS E1 differences? | 003/018 |
| O18-28 | P1 | Protobuf/Thrift/Cap'n Proto binary compatibility vs resource semantics, parser allocation and license? | 018 |
| O18-29 | P1 | Flow control, fragmentation/shared-memory ring and zero-copy with handle+payload lifetimes? | 003/004/018 |
| O18-30 | P1 | Deadline monotonic/boottime conversion, suspension and multi-hop timeout propagation across services? | 005/018 |
| O18-31 | P1 | Cross-language generated C/Rust/C++ and possible Swift/JS bindings enforce same presence/enum/default semantics? | 009/018 |
| O18-32 | P1 | Per-method error-to-guest Linux/Windows/Darwin translation maps unknown newer status fail closed? | 009/010/018 |
| O18-33 | P1 | Version coexistence when upgrade/rollback changes Network/VFS/Pager yet old client requests remain queued? | 004/006/008/018 |
| O18-34 | P1 | Stable v1 ABI lifecycle: support window, deprecation, security exception and named reference hardware/toolchain? | 009/018 |
| O18-35 | P1 | Future signed protocol-schema repository/trust and release supply-chain provenance, avoid client-supplied schema root? | 015/017/018 |
| O18-36 | P1 | Privacy/provenance of diagnostic trace, request IDs and transferred cap metadata across sandbox boundaries? | 016/018 |
| O18-37 | P2 | Measured IPC/codec/binding memory/latency p50/p95/p99, bounded in worst-case decoder paths? | 005/015/018 |
| O18-38 | P2 | Later architecture/compatibility targets and remote RPC transports require separate threat model/licensing? | 009/012/014/018 |
| O18-39 | P0 | Post-MOVE unknown-resource decoder rejection ownership: kernel transit vs ReceiveClaim vs installed receiver; no fake sender rollback? | 002/003/009/018 |
| O18-40 | P0 | Actual principal when Endpoint is delegated: trusted per-call Task/issuer metadata or bearer-only policy, proxy confused-deputy boundary? | 001/002/016/018 |
| O18-41 | P0 | Handshake session binding to Endpoint issuance generation, revocation B, ServiceInstance and server policy epoch? | 002/003/006/018 |
| O18-42 | P0 | Security-feature floor/rights recheck between negotiation and committed method with concurrent revocation/policy update? | 002/009/018 |
| O18-43 | P0 | Unknown strict/flexible method and resource fields in closed/ajar/open protocol across one-way, two-way and event directions? | 003/009/018 |
| O18-44 | P0 | Binary-to-JSON/forwarding serialization drops unknown extension; how detect or refuse security-critical field loss? | 015/018 |
| O18-45 | P0 | Signed exact bytes but old/new decoder disagreement on duplicate, key equivalent, required extension, numbers and signer role? | 006/017/018 |
| O18-46 | P0 | CBOR core deterministic vs legacy length-first ordering, equivalence of map keys, tag/float and duplicate restrictions? | 017/018 |
| O18-47 | P0 | Native/guest EFFECT_UNKNOWN support gaps and lack of matching errno/NTSTATUS: safe refusal/reconciliation instead of false no-effect? | 009/010/018 |
| O18-48 | P0 | DeviceGroup cap close after unknown resource rejection while R2-F unresolved: quarantine and sponsorship? | 002/007/018 |
| O18-49 | P1 | Per-message source identity/rights proof in generated proxies and service policy epoch audit without privacy leakage? | 015/016/018 |

## 14. Proposed decision register (IDL-D01…IDL-D32) — NONE Accepted

- **IDL-D01:** IDL-A (bounded user-space versioned schema, kernel opaque bytes+real cap vector, Native SDK separate) is conditional first design target.
- **IDL-D02:** Keep complex SABI/IDL schema parsing outside Ring 0; trusted kernel only validates E1 transport and live cap authority.
- **IDL-D03:** Method names, ServiceID, publisher and version fields do not mint kernel capabilities or authenticate arbitrary endpoints.
- **IDL-D04:** Actual cap type/rights/scope/lineage/sponsor and sender/recipient identity derive from kernel, never serialized user integers.
- **IDL-D05:** Early SABI cap operations are MOVE-only; COPY requires explicit separately authorized DERIVE then MOVE, preserving issuer lineage.
- **IDL-D06:** Adopt no Fuchsia ChannelWrite failure ownership automatically; UniOS NOT_COMMITTED preserves source grants by RFC-003.
- **IDL-D07:** Message bytes, transit rights, ReceiveClaim, cap-table install and service effect are separately accountable and observable phases.
- **IDL-D08:** Unknown resource fields/roles default reject+bounded close, or explicitly permitted typed proxy; not ignored as ordinary optional bytes.
- **IDL-D09:** Unknown required security fields/methods/status/result variants fail closed and cannot downgrade to ordinary SUCCESS.
- **IDL-D10:** Optional value fields may evolve only if their absence is proven semantically safe under old server security policy.
- **IDL-D11:** Never recycle stable method/field/enum/handle-role/status discriminants; maintain tombstone registry.
- **IDL-D12:** Protocol feature negotiation is scoped to issuer-authorized Endpoint and effective rights are intersection, not feature union.
- **IDL-D13:** Mandatory current security permission, consent, durability and other safety contracts remain in force even for legacy clients; refuse if not supportable.
- **IDL-D14:** Separate KABI, kernel transport, SABI, Native SDK, Linux guest ABI, signed Boot manifest and durable user data schema versions.
- **IDL-D15:** Explicit logical transport/admission/effect/durability/result-observed phases; no queue ACK called saved-to-disk.
- **IDL-D16:** Deadline expiration and local cancellation never assert remote side effect undone; use UNKNOWN/reconciliation where needed.
- **IDL-D17:** Idempotency keys alone do not guarantee exactly-once; durable authenticated backend transaction evidence is mandatory before claiming it.
- **IDL-D18:** Native SABI status values do not automatically map to Linux errno, Windows NTSTATUS or Darwin Mach error codes.
- **IDL-D19:** Signed BootPolicyManifest uses independently checked canonical/exact signed bytes, validated Root+security floor and bounded kernel descriptor, never lenient generic RPC defaults.
- **IDL-D20:** Successful wire-version negotiation does not imply A/B rescue data schema/encryption key compatibility or trusted floor eligibility.
- **IDL-D21:** Native C ABI layout/source and generated Rust/C++ wrappers have separate stability/language runtime contracts.
- **IDL-D22:** Keep M0 bootstrap/recovery functional with wired minimal parser/runtime, independent of ordinary full IDL service reflection/discovery.
- **IDL-D23:** Run bidirectional client/server schema corpus plus cap/error and crash-state model proof before declaring Stable ABI; performance comes after safety.
- **IDL-D24:** No wire format, public numeric KABI/IDL status, ABI v1 freeze, codegen, kernel/app implementation or compatibility certification permitted by this research draft.
- **IDL-D25 [new]:** Unknown resources after committed kernel MOVE are the recipient/escrow's settlement obligation; decode failure never rewinds sender handle ownership.
- **IDL-D26 [new]:** Protect services against delegated Endpoint confused-deputy: bearer-authorized access only where explicitly intended; identity-dependent methods require issuer/kernel-authoritative per-call provenance.
- **IDL-D27 [new]:** Bind a negotiated contract to concrete Endpoint issuance, ResourceRole schema, current service incarnation, active issuer policy epoch and mandatory feature floor; revalidate before effects.
- **IDL-D28 [new]:** Version negotiation or optional-field flexibility cannot override concurrent revocation B or updated server minimum authorization.
- **IDL-D29 [new]:** Unknown resource-bearing union and unknown value-only union have distinct strictness/close/proxy behavior; FIDL current rules are research precedent, not imported transport semantics.
- **IDL-D30 [new]:** A Protobuf binary-to-ProtoJSON/proxy step is not assumed to retain unknown critical fields; require end-to-end evidence or refuse that bridge.
- **IDL-D31 [new]:** Boot signing format must bind exact bytes, signer purpose/domain, parser-equivalent meaning and independent root/floor/critical extension; generic deterministic Protobuf/CBOR are not accepted by name.
- **IDL-D32 [new]:** Old SDK/guest compatibility must explicitly refuse an unrepresentable EFFECT_UNKNOWN rather than report false success or false rollback; no implementation/Stable ABI authorization.

## 15. Cross-RFC consistency register

| RFC | Contract owned there | RFC-018 must NOT contradict |
|---|---|---|
| RFC-001 | Small kernel boundary and asset-specific TCB | no generic IDL/JSON/CBOR/proto method parser in kernel |
| RFC-002 | Typed cap rights/scope/sponsor, Task-local handle ABA, B/R1/R2-F/R2-O | raw IDL resource label cannot mint right or imply physical revoke |
| RFC-003 | E1 bounded Channel, sender NOT_COMMITTED/COMMITTED/UNKNOWN, ReceiveClaim, ReplyToken | byte+cap publish stages and actual transit proof remain distinct |
| RFC-004 | Pager FaultWaiter, shared MemoryObject and immutable backing | zero-copy decoder must snapshot/validate mapped bytes and Pager budgets |
| RFC-005 | CPU budget, preemption, service resource sponsorship | malicious large message cannot starve M0 while decoding |
| RFC-006 | Wired M0, service restart and authenticated BootGood | full SABI registry cannot be wired recovery prerequisite |
| RFC-007 | DeviceGroup, IOMMU and R2-F | resource role DeviceCap cannot promise DMA quiescence merely by serialization |
| RFC-008 | BufferedVisible vs FileSync/DirectorySync/Flush, crash safety | method success/receipt must state durability scope and error lifetime |
| RFC-009 | KABI/SABI/SDK/guest boundary, protected GuestDomain, copyout UNKNOWN | no number/struct layout frozen by new wire proposal |
| RFC-010 | Linux ELF/FD/epoll/futex/signal/exec/errno | native version negotiation cannot substitute for Linux binary semantics |
| RFC-011 | native package/service signature and distribution | signed package name not issuer of native endpoint/caps |
| RFC-012 | Win32/NT personalty feasibility | no bitwise NTSTATUS or HANDLE reuse as native cap |
| RFC-013 | GUI/graphics buffer sharing | GPU buffer cap role requires actual memory/DMA scope |
| RFC-014 | Darwin/Mach-O/Swift feasibility | Mach port name/ObjC ABI distinct from UniOS SABI |
| RFC-015 | proof, fuzz, differential and fault injection standards | static schema compiler safety is not assumed |
| RFC-016 | user consent/permission grants | old-client defaults cannot bypass current portal consent |
| RFC-017 | signing root, floor, BootPolicyManifest and recovery | canonical signing format, key root and schema versions separately trusted |
| RFC-018 | schema evolution, unknown types, status and version relationship | must coordinate, not overwrite already Proposed commitments |

**Necessary joint P0 reviews:** (002/003/009/018) cap transit and ReceiveClaim; (003/008/009/018) effect/UNKNOWN/receipt; (001/006/017/018) signed boot canonical bytes and parser/Root; (008/010/018) guest file/FD/errno and persistent schema; (009/018) stable C ABI/IDL generated ownership. None is discharged merely because RFC-018 exists.

## 16. Future finite proof model and experiments (no execution)

### 16.1 Model states and counterexamples

Finite model: two clients (old/new), two server incarnations (old/new), one issued Endpoint plus forged discovery string, one Channel capacity 1, one transferable FileCap and one fake serialized handle, one DeviceCap with unresolved DMA R2-F, one optional new resource-bearing union, one mandatory security field introduced in new schema, one ReplyToken, one ReceiveClaim, one durable and one volatile idempotency ledger, one DataVolume V1/V2, BootManifest signer old/new, and a client whose deadline expires after file mutation.

Transitions: negotiate secure/insecure methods, issue/attenuate endpoint, E1 preflight/commit/UNKNOWN, receive partial byte copyout, install/missing/extra cap, unknown union closure, requeue/restart, request ID replay, VFS rename/commit/lose reply, FS flush fail, resume old client with current security policy, guest errno conversion and signed manifest unknown-critical rejection.

**Safety checks:**
- kernel cap transfer never derived from forged user bytes, issuer, version label or guest UID0;
- no source+receiver double ownership of one MOVE after COMMITTED, no source loss after genuine NOT_COMMITTED;
- unknown resource role cannot leak live native cap and no decode bypass of mandatory security policy;
- older client cannot gain broader rights, skip required consent or label BufferedVisible as DurableReceipt;
- stolen old ServiceID/ReplyToken cannot impersonate new authorized server;
- timeout and non-idempotent method retry cannot silently duplicate external side effect;
- Boot manifest unknown-critical/duplicate-root and changed signature encoding never authorize untrusted image;
- no parser overflow, unbounded nested decode or memory/IPC budget drain defeating critical M0;
- signed old rescue and new DataVolume cannot be called compatible solely from SABI version match.

**Conditional liveness:** if kernel IPC/scheduler/MemoryObject, authorized endpoints, healthy Pager, available server instance and actual storage work, bounded request may complete. No guarantee when hardware/FloorWriter/dead SABI server/unstoppable DMA fails. A fail-closed denial/recovery-only state is a correct safety outcome.

### 16.2 Future research/test matrix (not done)

1. Draw fully specified IDL-A vs adapted FIDL/Proto/Thrift/Cap'n Proto/CBOR resource/lifetime architecture with concrete parser TCB and license study.
2. Before stable wire choice, create executable property-based and fuzz corpus for lengths, duplicate/unknown fields, enum values, union, handle vector count and nested DoS.
3. Model check cap MOVE and ReceiveClaim (send failure, client death, cap install, partial user byte copyout, receive abort) against RFC-002/003/009.
4. Compile client/server schema combinations OldOld, OldNew, NewOld, NewNew with mandatory feature and resource-role change, plus partial failed rolling upgrade.
5. Prove service incarnation+ReplyToken/request ID/idempotency replay safety across restart and persisted/volatile ledgers.
6. Differentiate raw queue COMMITTED, backend external effect, file sync/directory sync and actual result receipt under every cut.
7. Compare C-compatible SDK ABI layouts/FFI ownership across selected compiler versions and Rust/C++ wrappers, with old/new binaries.
8. Verify Linux guest semantics under selected syscall/errno and FD close/epoll/exec after SABI change; no guessed blanket compatibility.
9. Audit boot manifest canonical signed representation, duplicate fields, critical extension, trust root and version floor under old/new readers and signer rotation.
10. Ensure M0 boot/recovery can run without SABI full codec, dynamic loader or pageable reflection runtime.
11. Only after separately approved code work, measure CPU, memory, IPC size, cap count, p50/p99 and decoder DoS profiles in QEMU and a named physical reference PC.
12. Record actual third-party library versions, transitive dependencies, licenses and update advisories, including generator reproducibility.

No experiment, test, compiler generation, formal verification, disk write, software or hardware implementation was conducted for this document.

## 17. R1 exit gates and research roadmap

**RFC-018 R1 design acceptance: OPEN / NOT PASSED.**

1. Choose the actual candidate IDL/codec based on equal capabilities/resource-role semantics, parser attack surface, memory/CPU and versioning proof, not a brand/tooling preference.
2. Define a complete logical but machine-independent SABI transfer schema with strict bounds, unknown-cap cleanup and relation to kernel bytes+typed cap vector. Then separately choose numeric wire after tests.
3. Prove RFC-003 E1 NOT_COMMITTED/COMMITTED/UNKNOWN and RCV-A ReceiveClaim against partial bytes/cap install/Task death/hostile escrow and resource budget.
4. Demonstrate that old/new client/server versions cannot weaken current authority, consent, durability or required security extensions through flexible fields or negotiable defaults.
5. Specify effect phase and receipt/idempotency scopes; no exactly-once or deadline rollback claim without durable transaction evidence.
6. Complete service incarnation and trusted Endpoint issuance/rebind rules with stale ReplyToken prevention.
7. Prove signed BootPolicyManifest exact canonical bytes/root/floor/unknown-critical validation separately from runtime SABI codec, including old rescue compatibility.
8. Publish stable Native SDK ABI, guest Linux/Windows/Darwin and durable DataVolume version matrices only with actual evidence.
9. Define and review source/binary/semantic/security/persistent format compatibility windows and security exception procedure.
10. Produce at least a bounded state model and cross-version malformed-input corpus after separate design acceptance/implementation authorization; no pretend current proofs.
11. Keep D1–D8 Accepted only as broad architecture directions and all IDL-D proposals Proposed.

**Next suggested design action:** independently attack-review RFC-018 v0.1 into v0.2, including unknown transferred handle and mixed-version mandatory security downgraded field. Then a cross-RFC 002/003/008/009/010/017/018 consistency pass before any fixed syscall/header formats.

## 18. Official external references and interpretation scope (2026-10-25)

- **[S1] Fuchsia FIDL language and protocol openness:** https://fuchsia.dev/fuchsia-src/reference/fidl/language/language
- **[S2] Fuchsia FIDL wire format and resource envelopes:** https://fuchsia.dev/fuchsia-src/reference/fidl/language/wire-format
- **[S3] Fuchsia FIDL API rubric and resource evolution:** https://fuchsia.dev/fuchsia-src/development/api/fidl
- **[S4] Fuchsia Zircon Channel write and handle transfer:** https://fuchsia.dev/reference/syscalls/channel_write
- **[S5] Protocol Buffers proto3 language/field evolution:** https://protobuf.dev/programming-guides/proto3/
- **[S6] Protocol Buffers serialization not canonical:** https://protobuf.dev/programming-guides/serialization-not-canonical/
- **[S7] Cap'n Proto schema evolution:** https://capnproto.org/language.html
- **[S8] Cap'n Proto canonical encoding and limitations:** https://capnproto.org/encoding.html
- **[S9] Apache Thrift IDL required/optional/default fields:** https://thrift.apache.org/docs/idl
- **[S10] gRPC status codes (DEADLINE_EXCEEDED after completed effect):** https://grpc.io/docs/guides/status-codes/
- **[S11] gRPC cancellation limits:** https://grpc.io/docs/guides/cancellation/
- **[S12] IETF RFC 8949 CBOR deterministic encoding/duplicates/security:** https://www.rfc-editor.org/rfc/rfc8949.html
- **[S13] Linux userspace ABI stability classes:** https://docs.kernel.org/admin-guide/abi.html
- **[S14] Fuchsia FIDL unknown interactions/strictness:** https://fuchsia.dev/fuchsia-src/reference/fidl/language/language
- **[S15] Protocol Buffers Editions compatibility guidance:** https://protobuf.dev/programming-guides/editions/
- **[S16] Protobuf proto3 unknown enum values and presence:** https://protobuf.dev/programming-guides/proto3/
- **[S17] Fuchsia FIDL current strict/flexible and value vs resource union definitions:** https://fuchsia.dev/fuchsia-src/reference/fidl/language/language
- **[S18] Fuchsia RFC-0113 efficient envelopes unknown handle storage/closure (wire evolution caveat):** https://fuchsia.dev/fuchsia-src/contribute/governance/rfcs/0113_efficient_envelopes
- **[S19] Protobuf proto3 unknown binary fields lost in JSON and field-copy transforms:** https://protobuf.dev/programming-guides/proto3/
- **[S20] Protobuf ProtoJSON weaker schema evolution support:** https://protobuf.dev/programming-guides/json/
- **[S21] gRPC deadlines propagation and cancellation without server handler guarantee:** https://grpc.io/docs/guides/deadlines/

All references describe the *other project's documented interface*, not a UniOS code import, implementation, transfer of formal proof or automatic legal permission. The cited codec formats have different failure/unknown-resource assumptions; a promising source does not prove selected UniOS fit.

---

**Document history:** 2026-10-25 v0.1 created and preserved in Git history. 2026-10-25 v0.2 independent adversarial review: committed cap MOVE vs decode rejection; delegable bearer Endpoint and trusted per-call identity/confused deputy; session/issuer policy epoch; unknown resource vs value; Protobuf JSON extension loss; exact signed bytes vs parser/domain equivalence; CBOR duplicate key handling; guest UNKNOWN status limits and hardware cap R2-F cleanup. 54 unproved invariants; 49 open design issues; 32 Proposed decisions. Documentation only, no compiler/prototype/kernel/codec/ABI implementation, test or formal proof.

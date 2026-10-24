# UniOS RFC-008 — Storage, VFS, Filesystems & Crash Durability

> **Status:** Proposed — Specification v0.1 (design research ONLY; no code, formal proof, hardware test or accepted filesystem choice)
> **Date:** 2026-10-24 NZDT
> **Baseline:** [Architecture v0.2](../architecture/unios-architecture-v0.2.md)
> **Prior proposed contracts:** [RFC-002](rfc-002-object-capability-lifetime-delegation-revocation.md), [RFC-003](rfc-003-ipc-wire-protocol-and-synchronization.md), [RFC-004](rfc-004-virtual-memory-pager-cache-and-oom.md), [RFC-005](rfc-005-scheduling-preemption-and-cpu-budget.md), [RFC-006](rfc-006-boot-trust-chain-service-startup-and-recovery.md), [RFC-007](rfc-007-user-space-drivers-dma-and-iommu.md).
> **No-go:** This text grants NO permission to format disks, alter GPT/UEFI variables, enable NVMe drivers, implement kernel/userspace/filesystem code, execute power-cut tests, or modify boot/update settings.

## 0. Executive recommendation — STORE-A, conditional and not selected

**STORE-A: an independently verified, immutable read-only BootBundle/SystemImage + a separately authorized and crash-consistent writable DataVolume + userspace VFS/FileSystem service backed by restricted BlockStorageHost.** The kernel only enforces memory/caps/IPC/IRQ/DMA boundaries and earliest wired bootstrap, not POSIX inode/path traversal, full filesystem parsers, huge journal replay or arbitrary disk/RAID management in Ring 0.

**Strong split:** no code name, image checksum, PageSupply reply, IPC COMMITTED state, NVMe completion, device Flush return, filesystem log commit, user file handle close, or GUI "Saved" notification is by itself interchangeable with another layer's success. Durable data means a specifically promised set of bytes+namespace edges can be recovered after specified failures, **conditional on a correctly functioning block device and independently verified flush/order semantics**. A disk that lies about flush or suffers physical loss is not rescued by logical file transactions. Crash consistency is not necessarily durability, authenticity, confidentiality, availability or backup.

**Why this form:** RFC-006 M0 cannot demand-page its own storage executable; RFC-004 M0/M1 read-only Pager should precede M3 writable file mappings; RFC-007 NVMe DMA fences must be valid before buffer reuse, yet successful DMA is not durable media storage. A separate DataVolume limits A/B system release rollback coupling, although shared boot metadata, encryption keys, package state and user-data migration still require cross-layer design.

**Other live candidates, not straw men:**
- **STORE-B** — mature existing filesystem format and trusted user-space implementation/adaptation, subject to license, completeness and hardware flush interface verification. Reusing a *format* is not the same as inheriting that filesystem kernel's crash-consistency implementation and testing.
- **STORE-C** — new native copy-on-write/checksummed filesystem with transactional roots/snapshots. Potential cohesive model but enormous correctness, garbage collection, metadata, ENOSPC and recovery risk before a desktop exists.
- **STORE-D** — conventional metadata-journaled filesystem (new or adapted) with explicit data/writeback+barriers. Often simpler than full CoW but journaling metadata alone does **not** promise the newest user file data survives a power cut.
- **STORE-E** — read-only initial profile only (no general DataVolume) if stable write ordering/power-loss behavior is not evidenced; valid prototype *research state*, not a finished general-purpose desktop.

**Initial staged research:** S0 fully resident signed BootBundle + bounded diagnostic or reserved boot-slot metadata only; S1 verified read-only SystemImage/Pager (no writable file mappings); S2 basic DataVolume with bounded file operations and recovery; S3 durable namespace/transactions and explicit file sync; S4 writable mmap/Pager writeback, larger file API, snapshots and update migrations. Stages are **design proof gates, not implementation milestones**. Do not authorize code merely because a gate is named.

**Main P0:** define exact end-to-end durability handshake from user FileSync or DurableCommit through VFS, filesystem intent/metadata, Pager dirty generations, BlockStorageHost, DMA completion, device Flush/FUA/volatile cache and crash recovery. A spoofed service completion or silently lost writeback error must not become the app-visible promise of safety.

## 1. Goals, failures and threat model

### 1.1 Desired independent OS user experience

- A file-like and directory namespace usable by native apps and, later, Linux POSIX guest personality without making guest UID0 a native global device/file administrator.
- A bootable signed SystemImage whose bytes are revalidated on use as needed and whose loader/root-hash identity derives from trusted boot state (RFC-006/017).
- A writable DataVolume providing clearly labeled buffered write, file sync, metadata sync, atomic namespace visibility and crash-recovery contracts.
- A storage stack whose failure can be diagnosed, quarantined/read-only or safely stopped, instead of silently reporting durable writes after a device crash.
- Error reports for delayed writeback failures to applications that previously received successful buffered-write returns.
- Cross-version A/B updates that cannot claim fallback when old code cannot read the newly migrated writable user-data format.
- Safe no-op/rejection on unsupported block-device flush capabilities; degraded read-only operation where actually safe, rather than imagined write durability.
- Well-defined app-level database interfaces needed by existing applications, but no automatic claim of SQLite/POSIX/NTFS/APFS compatibility.

### 1.2 Adversaries and failure sources

Malicious apps/symlinks/mount requests, path trickery, capability laundering, forged FileID/namespace handles, guest sandbox escaping, malicious or compromised VFS/Pager/BlockStorageHost, disk firmware that lies or corrupts pages, misconfigured volatile write cache, torn/partial sectors, out-of-order writes, power loss at every transition, controller reset during flush, orphan extents, inode reuse/ABA, namespace rollback, quota denial/OOM, stale page fault writes, lost error events, snapshots that fill disk, and software restart in the middle of a non-idempotent block operation.

**Physical trust limits:** motherboard, NVMe device/firmware, flash translation layer, storage write cache and power-loss protection are hardware assumptions. A checksum detects only selected corruptions; a Merkle tree is not an authenticity guarantee unless its root is authenticated, and a valid signature cannot repair unreadable storage.

### 1.3 Required different truth claims

1. **Visibility:** a client can observe a new file content/name now.
2. **Accepted/queued:** IPC request or block write was submitted.
3. **Executed/completed:** host/device acknowledged a command; cache may remain volatile.
4. **Crash-consistent:** after the defined crash class, file metadata and log can be recovered into a valid format without impossible allocation/free contradictions.
5. **Durable:** explicitly synced content and naming state survive a defined loss-of-power class assuming truthful hardware flush/cache.
6. **Authentic:** bytes tie to signed/trusted content/root and do not accept adversarial substitution under a named threat model.
7. **Confidential:** encryption and key policy prevent unauthorized disclosure; does not follow from journaling/verity.
8. **Available/recoverable:** device/media/keys/backup/recovery services are operational; doesn't follow from durability success once the only disk physically fails.

No status implies another unless a separate proven ordering/recovery contract covers it.

## 2. Official primary-source comparison and provenance

| Source | Official external fact | UniOS consequence |
|---|---|---|
| Linux block volatile writeback cache [S1] | write completion may precede nonvolatile media write; preflush and FUA have distinct semantics | flush/order honored by entire device stack before a strong durable receipt |
| Linux fsync(2) [S2] | syncing file contents/inode does not necessarily sync directory entry | directory sync/namespace receipt required for create/rename/unlink persistence |
| Linux ext4 jbd2 journal [S3] | default ext4 journaling protects metadata, not all new file data; journal commit/replay has ordering | no "journal = every user write is durable" claim |
| SQLite atomic commit [S4] | correctness assumes certain filesystem/sector write and powersafe overwrite properties | expose storage assumptions; torn writes and failure windows demand tested transaction model |
| SQLite WAL / checkpoints [S5] | WAL commit and checkpoint are separate; readers can hold checkpoints; sync policy changes durability | write-ahead log and checkpoint need independent growth/recovery/flush rules |
| SQLite synchronous pragma [S6] | WAL NORMAL can preserve consistency yet lose recent committed transactions under power failure; FULL sync adds durability | a database/app may need a stronger receipt than ordinary write completion |
| Linux iomap writeback [S7] | dirty pages, writeback and later error sampling have independent lifecycle; writeback error can surface at fsync | track dirty generation and delayed writeback error, not just last syscall success |
| Linux fs-verity [S8] | immutable file reads checked against Merkle tree; root authentication is separate | read-only verified image as option, not writable data consistency or boot auth by itself |
| Linux dm-verity [S9] | read-only block integrity may fail I/O on mismatch; root hash needs independently trusted provenance | read-only system verified backing; do not add all hash-parser code to kernel automatically |
| Android Virtual A/B [S10] | snapshots and interrupted merge can resume; userspace daemon startup itself is a boot dependency | independent update crash state and wired initial snapshot reader; don't import Android topology |
| PostgreSQL WAL settings [S11] | multiple WAL sync strategies exist and fsync hardware/OS assumptions govern crash safety | journal commit must be tied to explicit durable backing, not a "COMMIT" string |
| NVMe official NVM command set [S12] | command-set standards and device feature negotiation define host persistent I/O semantics | require hardware-specific Flush/FUA/volatile-cache feature evidence, no universal controller behavior |
| Linux errseq [S13] | writeback errors are sampled by independent observers and reported later | close/reopen/restart cannot silently erase prior failed write generations |
| Linux ext4 orphan file [S14] | open-unlinked/truncate orphan cleanup is a crash-recovery concern | no naive immediate free on unlink or uncommitted truncate |
| Linux DMA and group reset [S15] | device DMA completion, quiesce and pin reuse differ from storage durable flush | storage failure must observe RFC-007 R2-F separately |
| PostgreSQL WAL introduction [S16] | write-ahead logging separates record durability and deferred data-file write | bounded replay and precisely defined commit point |

**Source caveat:** Linux POSIX semantics are an important compatibility target, **not automatically UniOS native KABI**. ext4/SQLite/PostgreSQL are distinct implementations with different failure assumptions. An article about fsync or WAL does not itself prove a particular SSD, network filesystem or UniOS service behaves correctly.

## 3. Layers, roles and asset-specific TCB

~~~text
Native App / Linux Persona / Future Windows-Darwin Compat
     |
     | File/Directory Capability, namespace-scoped open, FileSync/Transaction
     v
VFS / NamespaceBroker (userspace; scoped policy and paths)
     |               \
     |                -> FS Service / Metadata Journal / Transaction Manager
     |                   /                  \
     +-> file backing Pager (RFC-004)       -> verified read-only SystemImage
                 |                           /
                 -> BlockDeviceIO Broker (userspace)
                        |
                  Wired BlockStorageHost (NVMe/VirtIO, RFC-007)
                        |
              Kernel: typed IPC/caps, MMU/pin/IOMMU/IRQ/security fences
                        |
       Actual controller volatile cache / nonvolatile media / power
~~~

**Kernel owns:** physical MemoryObject/mapping isolation, FaultWaiter, Cap B/R1/R2-F/R2-O, IPC queue integrity and scoped hardware DMA/IRQ; *not* inode B-tree layout, general VFS path parsing, arbitrary fsck code, native POSIX fd table or journal parser by default. Any tiny kernel-owned boot metadata path would require explicit TCB comparison and must not silently grow into a complete fs.

**User space owns:** VFS path/directory policy, FileID/generation/namespace and mounted filesystem access, page contents and dirty tracking policy with kernel-assisted write faults, journal/CoW protocol, writeback selection/error propagation, disk operation ordering and recovery replay under typed device grants. These services are *asset-specific TCB-S* for user file integrity, despite living in Ring 3. Separate process isolation between Pager and VFS reduces fault domains only when dependency/authority/cost graph supports it; colocation is a deliberate TCB expansion.

**Trusted boot authority owns:** signed SystemImage identity, boot/update slot and rollback floor, restricted boot metadata writer, selected FileSystemVersion compatibility. Normal ServiceSupervisor must not grant itself access to foreign user volumes or slot version-floor writer just by changing a name.

### 3.1 No whole-disk ambient root

Physical LBA integer, inode number, UID/GID in guest ABI, FileID, mount path, disk UUID and snapshot label are identifiers, **not capabilities**. BlockStorageHost receives narrow device/partition grants; VFS receives named NamespaceRoot and FileSystem object rights; app receives attenuated File/Directory handles. Whoever can write arbitrary raw blocks can generally bypass that filesystem's access controls and remains within an asset TCB. No direct raw NVMe/IOMMU cap for guest root.

### 3.2 Media identity and stale mounts

Track physical DeviceGroup / BlockDevice generation, Namespace volume UUID/provenance, filesystem superblock incarnation/format version, FileObject+inode generation, PagerBinding and backing version. A newly attached device at the same PCI BDF or same synthetic mount path must not be authorized by an old numeric handle. A service restart does not erase previous dirty-page debts, pending writeback errors or hardware R2-F quarantine.

## 4. Candidate objects and scoped authority (NOT ABI)

| Candidate | Responsibility | Example scope/rights |
|---|---|---|
| BlockDevice / Partition | stable device/partition identity and logical blocks; hardware capabilities | QUERY, READ_BLOCKS, scoped WRITE_BLOCKS, FLUSH; physical volume boundary |
| BlockStorageSession | one client admission on a device, request generations and budgets | in-flight I/O count, error reporting and alignment |
| Volume / FileSystemMount | verified FS instance, format version, mount state and recovery status | MOUNT_READ, MOUNT_WRITE, CHECK, FREEZE |
| NamespaceRoot / DirectoryCap | directory traversal/create/delete/rename within authority envelope | LOOKUP, ENUMERATE, CREATE, LINK, UNLINK, RENAME scoped |
| FileCap / FileObject | stable file instance separate from pathname; content/metadata access | READ, WRITE, SYNC_DATA, SYNC_METADATA, MAP_READ |
| BackingObject / PagerBinding | mapping of file/snapshot identity to MemoryObject/page supply | SUPPLY restricted range; not foreign Task resume |
| FileDirtyGeneration | modified byte/block range and visible generation | tracks dirty vs being-written vs writeback-error |
| WritebackTicket / BarrierReceipt | reporting asynchronous pages, block requests and durability epoch | completion not equivalent to DurableReceipt |
| DurableReceipt | authenticated to owner volume/epoch, range/namespace dependencies and storage hardware profile | issuer only if full conditional sync pipeline passed |
| FsTransaction / JournalRecord | candidate atomic metadata/data update, intent and commit cut | authority bound to volume; no global cross-volume assumption |
| Snapshot / RootCommit | named coherent root/snapshot transaction and refs | data version and GC charge; not automatically external backup |
| MountFreeze / RecoveryReadOnly | stop new writes and resolve dirty dependency before unmount or fatal | cannot guarantee flushing on dead controller |
| StorageErrorEpoch | preserve asynchronous write/flush error across clients/snapshots/service generations | per-object/volume error reporting cursor |
| BootSlotMetadata / UpdateTrialState | A/B trial, Good/Failed, floor and rescue status (RFC-006/017) | separate trust writer; no normal app authority |

A file cap holder can close its handle without forcing already shared buffer/writeback IO to persist. Namespace records and link references outlive one handle. A sponsor must remain accountable for dirty RAM, journal buffers, metadata, unflushed writes, open handles and snapshots across process death; refund only after real dependency settlement.

## 5. Storage completion and durability protocol (P0)

### 5.1 Conceptual five-stage path

~~~text
App Write -> VFS_ACCEPTED -> Pager/FS DIRTY_VISIBLE
 -> FS_WRITEBACK_ISSUED -> BlockIO DEVICE_COMPLETED
 -> ORDERED_FLUSH/FUA + data/metadata dependencies satisfied
 -> FS_DURABLE (scope, epoch, hardware profile, error-free)
 -> App observes DurableReceipt or explicit UNKNOWN/FAILED
~~~

**VFS_ACCEPTED** means a buffered write is visible in a cache or file object, not that bytes are on media.
**DEVICE_COMPLETED** means the device/controller accepted/completed operation under its command contract; volatile write cache might still hold data.
**ORDERED_FLUSH/FUA** proves only what *that validated device and host stack* promise; failing power-loss protection or lying firmware may break the claim.
**FS_DURABLE** requires file data, allocation metadata, inode, journal commit/root, namespace links and storage-device ordering covered by the exact requested operation, not necessarily the entire filesystem.
**App-reported SUCCESS** must identify whether result is buffered-visible, hardware-completed, crash-consistent-commit or conditionally durable. A single ambiguous "Saved" result is dangerous.

### 5.2 Scope of durability promises

- A byte range update in an existing file can require data blocks plus allocation/size extents needed to reread those bytes.
- A file create or atomic replace requires new file bytes/inode **and** directory entry/link metadata; syncing file alone may leave the name missing [S2].
- Rename atomicity is *runtime namespace visibility* (for supported same filesystem operation), **not automatically power-loss durability**. Need explicit directory metadata persistence policy, possibly both directories for cross-directory rename; do not promise cross-filesystem atomic rename.
- A durable transaction over N files/names requires coherent atomic commit protocol or explicit partial-success status. Independent file fsync calls do not make multi-file operations one atomic transaction.
- File close / unmount is not guaranteed to sync successfully on storage failure; error propagation and a bounded stop outcome required.
- Recovery after an acknowledged DurableReceipt must either reveal the data and namespace in the promised form or report hardware/profile failure, filesystem corruption or explicit violation. Never silently replay uncommitted blocks as known good.

### 5.3 Block flush/FUA/volatile cache model

Linux's block cache interface distinguishes a preflush (complete earlier writes to nonvolatile medium before next operation) and FUA (completion only after this write reaches nonvolatile storage) [S1]. The Linux block layer may emulate FUA using an additional Flush if hardware supports the required semantics. **UniOS must not imply every NVMe controller supports identical flags, protects its cache on power loss, or serializes unrelated queues automatically**. Verify actual NVMe NVM command set support, VWC and flush behavior for selected hardware [S12]. If a driver reports unsupported/not-trustworthy flush, deny or downgrade a requested strong DurableReceipt rather than fabricate success.

Multi-queue devices and caches introduce write ordering: dependent data and journal commit records must be ordered across relevant queues/transport layers and flushed correctly. A mere completion of one command or a CPU memory barrier is not a persistent-media barrier; CPU DMA fence and IOMMU R2-F are separate. Discard/TRIM, write zeroes, zoned append and write-protect operations may not share standard write-ordering semantics; initial scope excludes their strong guarantees.

### 5.4 Crash and timeout classification

- NOT_SUBMITTED — request rejected before any external IO; app may retry with same authority.
- QUEUED_UNCERTAIN — IPC/message/driver request may or may not have reached device; retry can have side effects.
- DEVICE_COMPLETED_VOLATILE — device command completed, stable media not guaranteed.
- FLUSH_FAILED or WRITEBACK_FAILED — do not issue success DurableReceipt; surface delayed error to all relevant watchers.
- DURABLE_VERIFIED_CONDITIONAL — all programmed barriers and filesystem dependencies accepted by compliant device, with exact volume/epoch.
- UNKNOWN_REMOTE_EFFECT — process died, timeout or lost reply; on-disk replay can reveal completed/partial transactions. Never blind-retry non-idempotent operations.
- MEDIA_UNAVAILABLE, CORRUPT, READ_ONLY_RESCUE, NO_SPACE, QUOTA_DENIED, UNSUPPORTED_SYNC and FATAL are distinct from slow I/O and from one another.

These are **research labels, not frozen status constants**. Device flush success cannot certify that the drive will survive physical destruction or malicious firmware.

## 6. Transaction candidates, journal vs CoW vs existing format

### 6.1 Metadata-only journal (STORE-D)

A metadata journal can produce a coherent inode/directory allocation state after crash, but user-file data written earlier might not have persisted. ext4 default data=ordered journals metadata while writing associated data ahead of commit; even that is not equivalent to an app's newest data being sync-durable without an explicit required sync path [S3]. JBD2 uses commit records and can replay completed log transactions; torn or corrupt transactions need integrity checks and fail behavior.

**No journal magic:** a metadata-safe mount does not mean data bytes are current, unreadable blocks were repaired, names survived arbitrary unsynced operations, or journal itself is authentic against malicious rewriting.

### 6.2 Full journal/WAL

Write-ahead logging can make a logical transaction durable after verified append+commit marker+flush; home blocks/checkpoint can be written later without losing already committed facts **only when log preservation, replay ordering, barriers and device assumptions actually hold**. Correctness requires finite record format, sequence numbers/generations, checksums/integrity, idempotent replay, bounded recovery scan, checkpoint and storage-space reservation under ENOSPC.

WAL is not intrinsically durable at every COMMIT call. SQLite WAL's synchronous=NORMAL can lose recent acknowledged transactions in power failure while staying consistent; synchronous=FULL adds sync for every transaction commit under its documented VFS/hardware assumptions [S6]. A UniOS FS journal must not hide its chosen durability policy inside generic names.

### 6.3 Copy-on-write transactional roots (STORE-C)

CoW writes new blocks, computes metadata/root and publishes an ordered root commit after dependency data is stable. Avoids many in-place overwrite hazards and supports snapshots/rollback, but write amplification, GC/refcount and ENOSPC while trying to checkpoint/reclaim can deadlock. An atomic root pointer still needs real media atomicity/order; older roots must not be overwritten/recycled before new root proven durable. Checksum trees authenticate content *only if root itself trusted*; random bit corruption vs adversarial rollback require different guards.

**CoW snapshot != backup.** A snapshot on the same failed SSD is unavailable; rollback of on-disk user schema after update may be inconsistent with external application effects. A/B system snapshot and arbitrary DataVolume snapshot have different owners.

### 6.4 Existing mature format (STORE-B)

Consider using known format and independently audited user-space reader/writer after legal/license and implementation correctness research, rather than rushing a new experimental FS. It must provide a compatible writeback transaction/durability implementation, not merely a parser for the on-disk data structure. An untrusted third-party filesystem parser is still asset-specific TCB-S when allowed to edit user data. No third-party code/source adoption is approved by this RFC.

**Proposed conditional pick:** defer native writable filesystem algorithm. For S1 read-only boot research, prefer an independently verified immutable byte/extent image; for S2–S3 data choose between STORE-B/D/C via failure model and legal feasibility, not preference for a novel FS name.

## 7. File and namespace semantics

### 7.1 File identity vs path

A FileCap points to FileObject+mount/volume generation and rights. A pathname resolves relative to DirectoryCap with traversal controls, not global arbitrary path access. Linux symlinks, mountpoints, hardlinks, case behavior, normalization, Windows reparse semantics and Darwin extended attributes are guests' user-space responsibilities unless a native filesystem explicitly supports them.

- Path traversal must enforce directory authority at each component and resist TOCTOU, renamed-parent and untrusted link races.
- A DirectoryCap may allow children within a sandbox; mounting a new file system under it needs extra explicit policy, not an arbitrary string path.
- A file renamed/unlinked while open can remain readable by lawful held FileCap; its lifetime refs and orphan cleanup persist until safe deletion. Crash recovery must reclaim orphan extents idempotently [S14].
- Opaque FileID alone cannot be a security credential. A stale handle pointing to reused inode number must not open a newer secret file after reformat/mount/restart.
- Namespace changes/rename atomic *visibility* require locking/transaction design; partial multi-file rename during process death must not look like atomic commit.

### 7.2 Directory durability

Native API should explicitly distinguish FileSyncData/FileSyncMetadata/DirectorySync (or transaction commit encapsulating them) without guaranteeing the exact POSIX ABI in kernel. For an app save-via-temp-rename workflow:
~~~text
create temp file in destination directory
 -> write full replacement data
 -> sync temp file data and required allocation/metadata
 -> atomically replace namespace reference, under same filesystem
 -> sync destination directory and any separately changed parent directories
 -> report durable named replacement (conditional on FS and disk support)
~~~
This is a **logical dependency checklist, not code**; exact old/new parent and link durability under crash need filesystem-specific proof and model. Merely calling rename() successfully guarantees neither post-crash new name nor persistence of replacement bytes [S2].

### 7.3 Cross-volume/external effects

No atomic rename/transaction spanning independent VolumeIDs by default. Copy-then-remove can leave duplicates or partial copies. A remote filesystem, cloud sync or database over another device has independent durability/availability; local FileSync cannot assert remote commit. An application needing exactly-once payment, email send, write to POS or remote network service needs application-level idempotency/reconciliation (RFC-003), not local journal semantics.

## 8. Pager-backed files, dirty generations, mmap and error lifetime

### 8.1 M0/M1/M2/M3 coordination

- **M0:** boot/recovery code, memory, minimal IRQ/IPC/storage control are wired. No pageable journal replay, normal FS service or network dependency.
- **M1:** immutable read-only verified SystemImage; PageSupply is content supply to MemoryObject, not durable write. Failed backing triggers bounded fault errors, not silent unverified bytes.
- **M2:** anonymous/COW/clean file eviction, depending RFC-004; dirty user-file writeback still unavailable.
- **M3:** writable FileMapping and asynchronous dirty/page writeback; only after race, truncation, mmap writer, error reporting and sync dependency proof.

### 8.2 Dirty/writeback state

~~~text
Clean(version v) -> Dirty(v+1, sponsor, write fault scoped)
 -> WritebackSnapshot(v+1, stable page content)
 -> InFlight(v+1) -> DEVICE_COMPLETED(v+1) -> FLUSHED(v+1)
 -> CleanDurable(v+1) only when FS metadata+write ordering confirmed
 OR WritebackFailed(v+1, err epoch) -> DirtyRetryable / ReadOnly / Lost
Concurrent writer -> Dirty(v+2) independent from v+1 in-flight
~~~

A page mapped into multiple Tasks or a shared MemoryObject may be modified while older contents are in flight. If new dirty generation v+2 is blindly marked clean by v+1 completion, subsequent fsync can lie and data is lost. A truncation, hole punch, COW or snapshot while writeback in flight needs explicit generation/offset version recheck before releasing references, reusing physical blocks or completing FileSync. A returning PagerSupply and file backing Snapshot identity cannot change another Task's mapping without kernel validation (RFC-004).

### 8.3 Error epoch survives service restart

Buffered write() returning length N does not establish a durable result. Asynchronous writeback may later fail; Linux iomap documents recording errors to report via later fsync [S7] and Linux errseq documents error cursors for multiple observers [S13]. UniOS needs an error ledger keyed to file/mapping/volume generation and observer sync epoch, **not simply a volatile flag owned only by one dying Pager Task**.

If the storage controller reset loses dirty buffers with no durable log, a fresh Pager service may be running but user data unrecoverable. Report LOST/UNKNOWN and prevent recycling the old FileDirtyGeneration as clean. An old FileSync result from a dead service must not be accepted as receipt of the new service incarnation's durable write.

### 8.4 Dirty RAM quotas and backpressure

Charge dirty frames, metadata log records, pending flushes, pinned DMA, device queue slots and snapshot references to appropriate ResourceBudget sponsors. Dirty memory can't be evicted as clean under OOM; an already damaged device may keep RAM effectively pinned or force safe abort/read-only. Reserve bounded kernel/M0 error reporting (RFC-004/005/006), don't let one malicious writer produce unbounded system-critical dirty journal storage. Denying new writes with NO_SPACE/QUOTA is preferable to reporting an impossible persisted save.

## 9. Physical block device and hardware truth

### 9.1 Device contracts required before strong durability

- logical/physical sector size and write granularity, alignment and atomic write-unit claims;
- volatile write cache feature and actual enablement (including whether power-loss protection is independently trusted);
- flush/FUA support and ordering across commands, queues, namespaces and bridges;
- discard/TRIM/Write Zeroes behavior and deallocation reads;
- normal DMA completion vs flash translation programming and confirmed persistent state;
- controller reset, fatal error, thermal throttle, hot removal, namespace format change and shared reset collateral (RFC-007);
- M0 read-only essential storage bootstrap independence;
- device checksum/ECC error reporting, media health and physical failures (not confidentiality/security by itself).

Do not infer "NVMe" means PLP or atomic 4KiB sector writes in every failure. The official NVMe spec [S12] covers command set; selected devices/firmware must be examined separately. Lost/garbled command completion means result UNKNOWN, not automatically failed/no-write.

### 9.2 Driver reset and DMA fencing

An IO driver may die after host submission but before returning completion; device may have finished writing blocks and DMA to host memory. RFC-007 physical R2-F quarantines unsafe DMA frames and forbids unsafe reset/reuse, but **stopping physical writes is not a durable filesystem transaction rollback**. The journal may already contain partial or valid commit records. After reset, replay must use on-disk authenticated/checksummed generation markers and correct barrier assumptions, not old IPC ReplyToken state.

### 9.3 Power-loss profiles (unproven)

| Device profile | Conditional durability claim | Failure policy |
|---|---|---|
| Media-X simulated lossless+ordered ideal block device | model logical FS atomicity only | not physical evidence |
| Media-Y real drive, flush/FUA contract validated on reference hardware | possible scoped strong FileSync under assumptions | error on unsupported barrier |
| Media-Z commodity drive, volatile cache behavior not validated or suspected lies | only weak visibility/accepted completion; no production-level durable receipt | disable strong durable profile or read-only |
| Media-P verified power-loss-protected enterprise device | greater hardware assurance if model/firmware+test support it | still not equivalent to independent backup |
| Media-F device I/O errors/reset/IOMMU DMA quiescence failed | no new strong durability | read-only, isolate/quarantine or fail-stop |
| Remote/network backing | independent server+network+ack contract; not local crash proof | separate protocol/RFC before guarantee |

Tests on QEMU can check logical write reordering and injected failure states but cannot establish flash controller PLP, true cache flush or electrical power cut behavior on a physical device.

## 10. Mount lifecycle, volumes, corruption and recovery

### 10.1 Mount state machine

~~~text
Absent -> DeviceAdmitted -> ScanReadOnly -> FormatRecognized
 -> RecoveryNeeded -> ReplayOnReservedResources -> CheckedReadOnly
 -> WritableAdmitted -> Active
 -> FreezeNewWrites -> DrainDirtyAndSync -> CleanUnmounted
Failures: Corrupt / UntrustedImage / UnsupportedVersion / NO_SPACE /
  DeviceUnavailable / ReadOnlyDegraded / Quarantined / Fatal
~~~

No arbitrary writable mount before volume identity, format version, checksum/journal and trust/ACL rules are validated. An untrusted/removable user volume may contain adversarial filesystem structures; parsing them in privileged kernel expands TCB-K, so prefer separately scoped parser and bounded limits. A mount error cannot be "fixed" by trusting attacker-supplied metadata claiming an older volume or different root capability.

### 10.2 Replay policy

- Journal or snapshot replay is **bounded for declared profile** or returns an explicit failure/recovery mode; "loop until healthy" can hang boot and spend all M0 resources.
- Replay must be idempotent across repeated power cuts: the same partially persisted commit cannot allocate/free blocks twice or resurrect old FileID owners.
- A torn log header/CRC/sequence may be detected and last valid prefix selected only under specified atomic-write and checksum collision assumptions. Ordinary checksums don't authenticate against malicious storage rewriting or defend against rollback of the entire valid disk image.
- Unrecoverable data errors should not silently rewrite an unknown volume until valid state appears. Offer read-only inspection or fail-stop as security/asset impact warrants; never promise repair of physically destroyed bytes without redundancy.
- Orphans from open-unlinked files, abandoned temp replacements, pending truncate and snapshot refcounts must be accounted or reclaimed under a well-defined replay state [S14].
- If encryption keys unavailable, valid disk sectors may remain unreadable; successful journal replay cannot replace key recovery policy (RFC-017 or a future encryption RFC).

### 10.3 Clean shutdown is conditional

Freeze namespace/new writes; drain bounded in-flight work or mark uncertain; coordinate Pager dirty generations; issue required filesystem data/metadata/log sync and drive barriers while driver still runs; report errors; only mark clean unmount or BootGood/slot state with independent correct writer after all conditions. An interrupted unmount or dying controller cannot be upgraded to success because the UI or Task exited normally.

## 11. A/B slot, secure floor and writable data compatibility

A signed good OS image and a crash-consistent DataVolume are **different objects**. RFC-006 BOOT_GOOD and trusted secure floor advancement must not consume an ordinary writable FS path just because the normal supervisor reports READY. Boot attempt counters/security floor may need independently trusted crash-safe storage/firmware/TPM semantics not available under this RFC.

### 11.1 Update state with storage cuts

~~~text
System A Good, DataVolume schema V1
 -> stage B's signed immutable image and authenticated manifest without destroying A
 -> flush/verify new image and trial-slot metadata
 -> boot B TRIAL; data remains separately versioned
 -> B may write data schema V2 (migration MUST be rollback-aware)
 -> independently verified health + durable slot Good receipt
 -> confirm bootable floor-eligible rescue and user-data access
 -> only then consider irreversible security-floor promotion
 -> crash/reboot at ANY step must choose supported recovery state
~~~

- A signed B blob written to a volatile drive cache is not yet bootable after power loss.
- Image verification before writing isn't enough: validate stored bytes and immutable read paths, detect unauthorized replacement at execution.
- If B migrates data to V2 before being marked good, A might still boot but not understand V2. Rollback eligibility must include old app/system-data schema compatibility, reversibility, or separately retained V1 snapshot with power-cut-safe root.
- Copy-on-write update snapshot/merge is a possible design with long-term space/GC/replay costs. Android Virtual A/B documents resumable snapshot merge and a userspace daemon bootstrap deadlock concern [S10]; UniOS may borrow only the **conceptual constraints**, not Android's kernel device-mapper design.
- Security-floor advancement is not atomic with ordinary DataVolume write transactions and may ban A even if filesystem snapshot still exists.
- Actual rescue needs readable authenticated code and compatible storage/key drivers on the selected hardware, not a manifest entry alone.

### 11.2 Slot transaction is not user data transaction

Slot attempt count/Good/Failed+rollback floor is a boot policy ledger. User app files and WAL are general user-data ledgers. These should not share unlimited writer rights or trust user-space VFS to change the trusted boot key/floor. A corrupt user volume should not silently make the boot authority accept an insecure older OS.


## 12. Crash, service restart, pending IO and reconciliation

### 12.1 IPC and disk effects are not exactly once

RFC-003's asynchronous Channel COMMITTED status only means that a message/handles were successfully enqueued. StorageHost may already have submitted DMA or durable log writes by the time the client learns nothing and times out. FileSystemService or StorageHost restart cannot distinguish "write never issued" from "write reached media" merely by an old ReplyToken. Recover by durable transaction IDs and on-disk replay, not blind repetition.

- File write to a fixed offset with identical data may *look* idempotent yet not be safe if concurrent writes, truncate, encrypt-nonce, checksums, snapshot refcounts or external side effects differ.
- Metadata create/link/unlink/rename and allocate/free transactions are not generally replayable twice without coherent transaction generation/intent logs.
- A successful fsync call followed by a lost reply may be durable but UNKNOWN to the caller. A durable transaction ID/recovery query can establish state only if its recovery ledger itself survived and is trusted for that volume.
- Flush completion messages from a replaced StorageHost must not be accepted for a later incarnation. Verified device/volume epoch and mapped writeback generation are prerequisites.

### 12.2 Crash classes and expected state

| Failure | What may remain | What must not be falsely asserted |
|---|---|---|
| App Task crash, OS/FS healthy | previously buffered pages and queued writebacks may survive; handle refs remain | every prior buffered write was durable |
| VFS namespace service crash | kernel Task/Cap objects live; committed log may be on disk; volatile namespace cache lost | new VFS name/endpoint inherits old requests and is equivalent |
| File Pager crash | existing resident dirty pages may remain depending RFC-004; backing faults may fail | Pager detach = writeback completed |
| StorageHost crash | I/O may be in flight/committed, DMA still active; controller cache dirty | old host stopped so old DMA gone |
| Device reset with supported R2-F | hardware no longer writes old frames after physical fence, under assumptions | on-disk transaction rolled back / fsync honored automatically |
| Controller volatile-cache loss | unflushed acknowledged writes may vanish | DEVICE_COMPLETED = durable |
| Sudden power loss | on-disk durable prefix and arbitrary unsynced writes according to model | all writes are last-in-first-out or sector atomic by default |
| Hostile/tampered storage | signed read-only image may detect mismatch | checksum detects purposeful rollback of entire valid image |
| ENOSPC during log/checkpoint | older committed state, uncommitted temporary extents, perhaps unrecoverable low free space | log always has space to finish commit/recover |
| Corrupt journal/superblock | read-only inspection, selected valid root if authenticated and safe | arbitrary automatic repair retains all file bytes |
| A/B B boot fails after schema V2 migration | verified A code but unreadable V2 data | A fallback restores pre-upgrade user state |

### 12.3 Recovery receipt semantics

A **DurableReceipt** is a *conditional statement* tied to (volume identity/version, FileID or namespace transaction, exact content/metadata commit generation, defined dependency closure, storage device identity and verified flush profile, and the completed on-disk evidence point). It must NOT be a bearer token that grants new file rights. Signing a receipt may help auditing but cryptography cannot make a dishonest device truthful. A persistent transaction ledger can support application idempotency across service restart only if it includes enough on-disk data to reject duplicate requests and itself respects durable ordering.

**Error surfaces:** if a sync call returns an error, some or all bytes may nonetheless have persisted. Returning an error cannot assert rollback; caller may need reread/reconcile. A timeout may be even more ambiguous. An error class "NOT_SUBMITTED" requires actual proof no external effect occurred.

## 13. Integrity, verification, encryption and permission boundary

### 13.1 Verified read-only system vs writable files

RFC-006 verifies the boot chain and signed manifests, but the loader's verification of one image does not transitively authenticate every page later faulted from disk. A read-only content-addressed SystemImage or per-file/block Merkle tree may check bytes when loaded, using authenticated roots from trusted boot state [S8,S9]. **A hash tree alone is integrity verification, not authorization:** root hash trust comes from RFC-017 signer/floor and authenticated file identity policy.

If a malformed/failed read returns a zero page or an attacker-substituted block without reporting verification failure, app isolation can fail even though BootBundle was signed. A verified-image Pager that can supply arbitrary raw bytes without an independent check is a potential trusted asset TCB-S. Reverify on use or hold cryptographically authenticated immutable backing under the declared policy.

### 13.2 DataVolume checksums vs malicious rollback

Journal CRC/checksum can detect accidental torn records and corruption under assumptions, not resist an attacker who substitutes a consistent older set of data+metadata/checksum. A mutable DataVolume needs anti-rollback *if* the product claims it, which requires a separately trusted monotonic epoch/root or an authenticated external log and failure policy. Most general consumer filesystems do not promise malicious rollback protection; don't silently add it.

Encryption is separate. A successful filesystem mount does not grant access to an unwrapped user key; key lifecycle and encrypted volume format/access control need independent security research (RFC-016/017 or new RFC). Device encryption without integrity may permit malicious bitflips; integrity without encryption may reveal private data.

### 13.3 Namespace and compatibility boundary

A native FileCap cannot be generated from an untrusted POSIX path/UID integer. Linux guest root is a property of a guest personality, not a capability to mount host EFI partition or overwrite BootSlotMetadata. A Linux process can request fsync() and expect documented guest semantics; it must receive failure/not-supported when native backing cannot guarantee the requested sync contract. Windows FlushFileBuffers, macOS full-fsync and future APFS/NTFS compatibility deserve independent documented mapping; do not promise parity with POSIX fsync based on a single FileSync primitive.

## 14. Resource budgets, concurrent writers and sustained ENOSPC

### 14.1 Distinct budgets

Disk byte space/extent allocation, in-memory dirty page count, writeback queue/range count, journal commit-space/headroom, filesystem metadata transaction slots, snapshot refcounts/GC work, mounted-volume capability objects, outstanding device DMA pins, CPU/IRQ time and protected M0 reserve are separate resources. Every Task and Service sponsor remains charged until references/effects are safely retired; deleting an on-disk file that remains open or snapshotted may reclaim zero disk blocks.

**Dangerous success under overcommit:** virtual allocation or an accepted buffered write can succeed while physical disk reservation to persist it fails later. The API should distinguish reserved-space write modes vs overcommit mode and surface final FileSync NO_SPACE/IO_ERROR. Never represent accepted dirty RAM as guaranteed durable free disk capacity. Journal itself must reserve enough emergency metadata/log space to perform cleanup/replay or enter read-only.

### 14.2 Space reclamation vs new writes

If low disk space prevents log checkpoint or CoW root completion, aggressive background GC can be blocked by the same shortage and may require even more scratch space to move live extents. Analyze bounded emergency free-space reserve and reserve-only transaction behavior; if unrecoverable, fail cleanly without modifying previously committed state in ways that prevent mount.

Snapshots share extents and may prevent freeing data that a user just deleted. A malicious client cannot create unbounded snapshots, mount replicas or orphan files on a global system budget; quota hierarchy and ownership remains separate from rights to enumerate or overwrite other files.

### 14.3 Concurrent write ordering

- Two writers to same file range need actual serialization/last-write policy, dirty generation and conflict semantics; pagecache lock alone does not imply multi-file transaction.
- mmap changes and buffered write must coordinate with truncation, extent replacement and snapshot root publication; a prior fsync must not accidentally claim bytes written concurrently *after* its sync range snapshot.
- A successful delete/rename while a buffer is pinned for storage DMA must not reuse that extent for another file before IO/ref and dependency settlements.
- FileSync needs a **snapshot/barrier of the set of changes covered** and a completion point; if writers continue indefinitely, sync must either report exactly which prefix was durable or use a defined bounded serialization method.

## 15. Cross-layer orderings, linearization and explicit state proof

### 15.1 Transaction cut points

1. **WRITE_VISIBLE:** bytes become readable through VFS/FilePager cache under a precise file generation; crash may lose them.
2. **IO_ACCEPTED:** block request is admitted to driver queue and sponsor charged.
3. **DEVICE_ACK:** host receives completion. Flush/FUA and cache semantics still determine persistence.
4. **PERSIST_BARRIER:** relevant data+metadata writes ordered to stable storage under hardware assumptions.
5. **FS_COMMIT:** commit record/root point has become durable under verified dependency graph.
6. **RECEIPT_OBSERVED:** client learns sync success or receives an unknown/failure status.
7. **REPLAY_RECOVERED:** after reboot, valid on-disk state is rederived without relying on volatile client/driver state.

FS_COMMIT can precede a successful reply that gets lost; thus receipt-observed is *not* a physical commit marker. A crash at any boundary requires a defined surviving state and cannot result in contradictory allocated/free metadata or accepting invalid authentication.

### 15.2 Formal ordered dependency DAG candidate

~~~text
Write data/extent new version
  -> Sync data blocks needed for next root
  -> Sync allocator/inode/extent/necessary namespace metadata
  -> Journal/root commit with suitable barrier and version
  -> DurableReceipt for scope if no recorded error
  -> Later checkpoint/home-block update / old-root GC
~~~ 

The actual DAG depends on journal vs CoW choice: a full WAL may persist log records and commit before home data blocks; a metadata journal/ordered data needs different ordering. **Do not freeze the schematic above as one universal write algorithm.** It is a proof checklist for dependency edges and the point after which the result becomes observable as durable.

### 15.3 Queue and shutdown race

An IRQ for write completion can race storage DriverHost death, device reset, timeout, Pager rebind, FileSync cancellation, user Task termination or revoke B/R1/R2-F. Every event must test volume/device/driver generations. An old completion cannot pay down a new generation's durability barrier. Device DMA pin releases follow RFC-007 physical fences independent of the filesystem's commit status.

### 15.4 Time and recovery deadlines

Storage might not respond within any bounded time after power failure or controller hang. RFC-005 M0 minimum CPU entitlement may allow an error/fail-stop response while a critical store waits, but does not guarantee disk contents, flush completion or whole-volume repair. A timeout should terminate retry attempts; it is not evidence that hardware has stopped writes or a transaction has not persisted.

## 16. Failure/attack matrix

| Scenario | Forbidden shortcut | Required proposed response |
|---|---|---|
| App BufferedWrite successful, sudden power loss | "Saved" implies bytes survive | visible/dirty only unless separate durable sync |
| NVMe write completion with volatile cache | device completion = persistent | explicit flush/FUA and device trust conditions |
| Device has no known flush support | silently return fsync success | UNSUPPORTED_SYNC/degraded or read-only |
| Two NVMe queues reorder log and data | journal commit first -> dangling pointers | enforce cross-queue ordering/barriers |
| Controller firmware lies about Flush | fsync guaranteed universally | label hardware trust assumption, deny stronger profile |
| Rename replacement then power loss | rename atomicity = on-disk name survives | file+affected directory dependency sync |
| Temp file data fsync, directory not fsynced | new filename promised durable | explicit DirectorySync/namespace durability |
| File unlink while open | immediately reclaim same blocks | orphan references + crash cleanup |
| fdatasync after allocate extent | contents persisted but metadata omitted | include metadata needed to read bytes |
| User modifies mmap page during writeback | old completion marks new dirty gen clean | per-generation snapshot/recheck and sync barrier |
| Pager dies with dirty pages | detach guarantees writeback | report pending/lost, preserve if possible |
| StorageHost restarts during Flush | old reply satisfies new sync | epochs, hardware evidence and reconciliation |
| IPC COMMITTED after client timeout | durable transaction succeeded | result UNKNOWN; query persistent journal |
| Transaction written but receipt lost | blind resubmit commit | idempotency key + recovery lookup |
| Metadata journal valid, latest file bytes lost | full user data durable claimed | clearly separate metadata consistency vs data |
| Log header torn write | replay random bytes as valid | checksum/version and safe valid-prefix/abort |
| Malicious disk returns old valid checksum root | checksum = malicious rollback prevention | trusted epoch/authenticated root or no claim |
| Disk flips bit inside system executable | signed loader enough | read-time verity check; failure safe |
| Snapshot retention fills volume | delete appears to free bytes | account snapshot refs; bounded emergency GC |
| ENOSPC while committing CoW root | overwrite last known good root | reserve commit space, atomic root policy |
| Read-only rescue mounting corrupt user disk | replay with full write privilege automatically | explicit separate auth + safe RO inspection |
| A/B B migrates V1→V2 before Good | A rollback guarantees data readable | schema-compatible fallback/snapshot or deny |
| Floor raised while rescue not bootable | old slot still guaranteed usable | gate floor on actual rescue+media and data format |
| Unexpected reset takes down storage+Pager | CPU recovery equals storage recovery | safe error/fail-stop per reset-coupling |
| DMA quiesce fails | pinned page can be cleaned/reused | RFC-007 quarantine and no forged persisted result |
| Data under encryption key revoked | valid disk means readable | key availability independent |
| Linux guest root passes raw LBA | native filesystem access granted | typed device/volume caps only |
| Cross-filesystem rename attempted | one atomic native filesystem operation | no cross-volume atomic guarantee |
| Recovered FS has orphan extents | leak forever or double free | bounded replay/refcount correction |
| App tries path traversal through symlinks | DirectoryCap scope bypassed | component-by-component namespace cap authority |
| QEMU power-cut simulation success | production SSD PLP confirmed | explicit simulated evidence label |
| Device removed mid-fsync | success with fabricated durable receipt | lost/error/unknown, hardware trust failure |
| Malicious rollback of valid FS root | checksum detects old-but-valid data | define anti-rollback separately if required |
| Persistent boot retry counter stored in normal VFS | dying VFS controls secure floor | separate trusted boot metadata writer |
| Recovery daemon itself demand-paged by broken drive | deadlock during repair | M0 wired critical path or fail-stop |

## 17. Proposed correctness/security invariants FS-01…FS-42 — NOT verified

| ID | Proposed invariant — no validation claim |
|---|---|
| FS-01 | IPC COMMITTED, buffered-write visible, storage IO completed, FS committed and durable receipt are distinct predicates. |
| FS-02 | FileSync success covers a declared file byte/metadata dependency set and compliant hardware persistence profile. |
| FS-03 | File contents synced does not imply directory entry synced; naming durability needs explicit namespace commit. |
| FS-04 | Atomic runtime rename visibility is not equated with persistence or a cross-volume transaction. |
| FS-05 | Flush/FUA ordering is enforced across relevant queues and layer boundaries before any strong durable receipt. |
| FS-06 | Unsupported/failed device flush cannot silently produce a strong durable receipt. |
| FS-07 | Controller volatility/PLP/sector atomicity are explicit hardware assumptions, not inferred from an NVMe label. |
| FS-08 | Torn log writes and checksum failures cannot be replayed as known-valid committed metadata. |
| FS-09 | Crash recovery under declared block behavior yields a valid prior committed state or explicit corrupt/unavailable status. |
| FS-10 | Journal or snapshot metadata consistency never automatically claims survival of every unsynced user byte. |
| FS-11 | On-disk transaction replay is idempotent across multiple repeated crashes/power cuts. |
| FS-12 | Journal root/commit sequence guards ABA and rejects stale generations under stated checksum assumptions. |
| FS-13 | DurableReceipt does not grant FileCaps or new I/O authority and is bound to file/volume/epoch/scope. |
| FS-14 | Old StorageHost/FS service generation completions cannot mark new work as flushed or durable. |
| FS-15 | Buffered writeback errors persist as observable error epochs until legitimately acknowledged by every relevant client. |
| FS-16 | File Pager detach, Task exit or object close does not silently discard dirty data while reporting durable success. |
| FS-17 | Concurrent new dirty writes survive earlier writeback completion without incorrect CleanDurable transition. |
| FS-18 | mmap faults and PagerSupply cannot authorize a foreign MemoryObject or suppress file backing generation checks. |
| FS-19 | Outstanding DmaPin/IRQ/DeviceGroup refs and safety fences remain independent of storage command completion. |
| FS-20 | Dirty RAM, unflushed journal and snapshots have bounded charged budgets, distinct from protected M0 reserves. |
| FS-21 | ENOSPC/QUOTA_DENIED does not destroy a provably committed prior FS state through unchecked allocation/reclaim. |
| FS-22 | Multi-volume operations cannot claim one atomic commit without a separate proven coordination protocol. |
| FS-23 | Path traversal and mount operations respect DirectoryCap/NamespaceRoot and do not follow hostile links outside scope. |
| FS-24 | FileID/inode reuse/renamed mount does not resurrect stale capabilities of another security owner. |
| FS-25 | Unlink/truncate with live references preserves needed extents or reconciles orphan refs after crash. |
| FS-26 | Data authenticity relies on authenticated root/manifest; a checksum alone cannot resist adversarial rollback/substitution. |
| FS-27 | Read-only signed image pages are authenticated on load/use under approved trust policy, not by name-only lookup. |
| FS-28 | Encryption/key availability and volume integrity/durability/availability have separate predicates. |
| FS-29 | A signed A/B slot does not imply old software can read user data migrated by new B. |
| FS-30 | Secure floor/slot writer authority is not granted to normal VFS/DataVolume writer by registry identity. |
| FS-31 | Power cut after an observed strong durable receipt retains promised file/namespace state under stated device assumptions. |
| FS-32 | Failed/timed-out operation does not assert absence of side effects or safe blind retry. |
| FS-33 | Non-idempotent replay requires durable transaction identity and deduplication or exposes UNKNOWN_REMOTE_EFFECT. |
| FS-34 | FileSync/DirectorySync/Transaction scope is explicitly named, not inferred from generic close() or GUI "Saved". |
| FS-35 | Corrupt volume may enter read-only rescue/failed state without running a writable parser in Ring 0. |
| FS-36 | Critical recovery filesystem/StorageHost path has wired executable, IRQ, queue, DMA and CPU resources or fails boundedly. |
| FS-37 | A file system replay/repair deadline is conditional on functioning hardware; timer expiration does not invent success. |
| FS-38 | Checkpoint/CoW GC does not silently discard still-live snapshot/older valid root until proven safe. |
| FS-39 | Disk block address and untrusted POSIX UID/path never authorize access to host boot metadata or foreign volumes. |
| FS-40 | Storage reset/DMA fencing cannot retroactively undo already persisted writes or restore corrupted data. |
| FS-41 | Physical disk loss and dishonest firmware are stated exclusions from software-only durable recovery proof. |
| FS-42 | Emulator power-cut result is distinguished from physical NVMe cache/PLP and reboot/rescue reliability evidence. |

## 18. Comparative feasibility/TCB decision matrix

| Candidate | Advantage | P0 risk | Possible refusal reason |
|---|---|---|---|
| STORE-A immutable system + separate data service | trust/update separation, simple early verified boot, scoped writable risks | real block flush/rollback/data schema; userspace FS crash TCB | fails if no independent wired boot storage reader |
| STORE-B adapted existing format | mature on-disk ecosystem, data tooling potential | licensing/reader-writer correctness, parser and journaling parity, compatibility | only read parser exists, unsupported license or no crash-consistent writer |
| STORE-C new CoW/checksum FS | snapshots, on-disk root model, checksum opportunity | ENOSPC, GC/refcount, space amplification, root durability, many unsolved races | proof surface dominates rest of OS before basic desktop |
| STORE-D metadata journal with explicit data ordering | known recovery ideas and bounded normal paths | data vs metadata exposure, journal resource and flush ordering | cannot define persistent semantics on selected hardware |
| STORE-E read-only early-only | smallest write integrity TCB, signed image possible | not full general-purpose user desktop | insufficient end-state features |

**Comparative gates:** no algorithm selection without (1) signed/verified read-only image provenance, (2) crash-state model, (3) selected device flush/atomic write assumptions, (4) file/namespace sync API, (5) allocation/ENOSPC proof, (6) isolated user-space VFS attack surface, (7) license and third-party artifact provenance, (8) physical PC plan. Native storage format and POSIX facade can evolve independently only if precise semantic mapping is documented.

## 19. Formal state model and future fault injection plan

### 19.1 Finite state space (research only)

Model 2 files+2 directories, 1 parent FileCap, 2 client writers, 1 shared mapped page with dirty gen v/v+1, 2 device queues, 1 volatile write cache, 2 journal/CoW roots, 2 service incarnations, 1 DMA pin, a small ENOSPC space budget, 1 snapshot, A/B slots+DataVolume schema v1/v2, 1 hardware Flush that can fail/lie and 1 persisted writeback error epoch.

Transitions: create, buffered write, rename, unlink, truncate, mmap write, fault, FileSyncData, DirectorySync, transaction begin/commit, IO enqueue/completion, cache flush/FUA, queue reorder, partial sector write, process crash, controller reset, power cut, replay valid-prefix selection, failed journal checkpoint, snapshot GC, rollback from B to A, secure floor promotion.

### 19.2 Safety counterexamples to enumerate

- FileSync acknowledges but after power cut written bytes unreachable because directory entry not persisted.
- Rename new path visible and previously closed old file lost after power cut, despite no namespace sync.
- Journal commit record persisted before referenced data/extent due to queue reorder.
- New dirty page modification v+2 falsely marked clean by older v+1 completion.
- FileSystemService restarts while old device writes in flight and reuses old buffer without physical fence.
- fsync delayed write error overwritten/forgotten by new handle or service generation.
- Power failure at 2-phase CoW root/boot slot update picks an unauthenticated/half-written root.
- Snapshot reference leaks block after unlink or double-free on repeated replay.
- Power failure during checkpoint causes old WAL/log to be garbage-collected before durable home blocks.
- DataVolume schema V2 blocks older A after B fails, although A's boot binary verified.
- Encrypted user volume key unavailable: mount appears "successful" without readable user data.
- One logical transaction commits in RAM but its receipt lost; client retries and performs external effect twice.
- Malicious app tries to open raw host boot partition through forged VolumeID/path/guest UID0.
- Corrupt mount metadata triggers unbounded recursive parser and consumes M0 recovery reserve.

### 19.3 Future experiments after separate Go only

1. Deterministic simulated block backend with random command reordering, volatile cache, drop/reorder flush, partial/torn sectors and "device lies" mode to characterize claims. Model failure only—not actual user data operations.
2. Every instruction/log stage power-cut campaigns against a *sacrificial test image*; recover independent checker and replay multiple consecutive crash cycles.
3. Reference PC dedicated removable/sacrificial NVMe/SSD: verify controller VWC/FUA/Flush features and power-loss behavior against known good data, with user storage never in scope.
4. Disk full while preserving journal reserve, snapshot retention and CoW checkpoint; verify old committed root never incorrectly discarded.
5. Storage driver restart + DMA R2-F fail/quarantine, dirty Pager, disconnected FS service, IRQ storms and post-UEFI timer assumptions.
6. File rename/create/unlink over two directories, fsync file and directory combinations; compare observed exact persistence envelope.
7. Virtualized guest requests fsync and errors through POSIX interface: no false success on unsupported hardware, no guest root/native volume escalation.
8. A/B trial during user schema migration, floor advancement and interrupted snapshot merge; confirm physical rescue readability if targeted.
9. Future throughput/latency p50/p95/p99 write, FileSyncData/DirectorySync, metadata journal/WAL/CoW root, rebuild time and write amplification on specified hardware.
10. Security and memory-safety reviews of chosen existing filesystem parser/library, format fuzz and full license/provenance audit prior to code import.

**None have been executed for UniOS**. Formal model language TLA+/Alloy or equivalent is a future decision; proposing a model is not a completed proof.

## 20. Open design blockers O8-01…O8-36

| ID | Priority | Unresolved research question | Co-review |
|---|---|---|---|
| O8-01 | P0 | STORE-A separate immutable SystemImage/DataVolume and whether writable FS STORE-B/C/D? | 004/006/008 |
| O8-02 | P0 | End-to-end bounded DurableReceipt state and exact trusted issuer/scope? | 003/004/007/008 |
| O8-03 | P0 | Selected NVMe/other drive VWC/Flush/FUA and error/partial write ordering? | 007/008 |
| O8-04 | P0 | Write/cache/metadata/journal ordering across queues/namespaces with verified barrier? | 007/008 |
| O8-05 | P0 | FileSyncData/metadata vs DirectorySync/rename/create post-crash contract? | 008/009/010 |
| O8-06 | P0 | Torn sector/atomic write size/partial overwrite and recovery log root consistency? | 008/015 |
| O8-07 | P0 | Dirty-generation snapshot vs mmap/truncate/COW concurrency, Pager rebind? | 004/008 |
| O8-08 | P0 | Delayed writeback error delivery across multiple handles/client epochs/service crashes? | 003/004/008 |
| O8-09 | P0 | StorageHost reset and R2-F DMA memory quarantine vs disk commit uncertainty? | 006/007/008 |
| O8-10 | P0 | Bounded journal/CoW replay with ENOSPC, corrupted log and repeated power cut? | 006/008 |
| O8-11 | P0 | Wired storage driver/Pager bootstrap, program bytes and protected buffer/CPU/IRQ cut? | 004/005/006/007/008 |
| O8-12 | P0 | A/B floor vs rescue/image physical durability and user-data schema migration? | 006/008/017 |
| O8-13 | P0 | Boot/slot metadata durable writer independent of ordinary VFS and UEFI NVRAM claims? | 006/008/017 |
| O8-14 | P0 | Power loss "sync success" under dishonest device/FTL/controller firmware? | 007/008/015 |
| O8-15 | P0 | File namespace cap/path traversal/symlink TOCTOU, mount authority and guest root scope? | 002/008/009/010 |
| O8-16 | P0 | Full crash-consistency contract of existing format/third-party filesystem implementation and license? | 008/015/018 |
| O8-17 | P0 | Transaction replay/dedup query following ambiguous IPC/device completion? | 003/008 |
| O8-18 | P0 | Trusted immutable SystemImage per-page verification root/key authority and failed page behavior? | 004/006/008/017 |
| O8-19 | P1 | File/DirectoryCap lifetime, inode reuse and orphan blocks after unlink/restart? | 002/008 |
| O8-20 | P1 | Concurrency of FileSync snapshot vs later app writes and background writeback? | 004/008 |
| O8-21 | P1 | Async journal commit batching/performance vs actual durability semantics? | 005/008 |
| O8-22 | P1 | GC/CoW snapshot quota, emergency free space and root rollback? | 004/008 |
| O8-23 | P1 | System partition vs user DataVolume mounts and mutable package/app state? | 006/008/011 |
| O8-24 | P1 | Full KABI/vfs user API errors on partial/unknown sync, fdatasync and msync? | 008/009/010 |
| O8-25 | P1 | Block LBA/partition/volume grant narrowing and compromised FilesystemService? | 002/007/008 |
| O8-26 | P1 | Data integrity checksums vs malicious rollback protection and encryption keys? | 008/016/017 |
| O8-27 | P1 | Multi-volume snapshot and transaction expectations for compatibility guests? | 008/010 |
| O8-28 | P1 | Physically honest hardware fault reporting, ECC, bad sectors, namespace format? | 007/008 |
| O8-29 | P1 | Suspend/hibernate/writeback and power-loss caches/clock domain semantics? | 005/006/008 |
| O8-30 | P1 | Safe read-only degraded mode, replay on corrupted volume and user consent to repair? | 006/008/016 |
| O8-31 | P1 | File and directory metadata normalization/case/symlink native vs guest semantics? | 008/009/010 |
| O8-32 | P1 | Power-loss testing harness evidence and comparator oracle/false passes? | 008/015 |
| O8-33 | P1 | Durable user-data migration and rollback compatibility after B Trial and user writes? | 006/008/017 |
| O8-34 | P2 | TRIM/discard, thin provisioning, deallocated reads and advanced NVMe namespaces? | 007/008 |
| O8-35 | P2 | Remote filesystem/net storage and independent server/fsync guarantees? | 008/010 |
| O8-36 | P2 | Whole-volume backup, incremental snapshot replication, ransomware rollback, recovery key UX? | 008/016/017 |

## 21. Proposed decision register FS-D01…FS-D20 — NOT Accepted

- **FS-D01:** STORE-A immutable verified SystemImage and separately scoped writable DataVolume with userspace FS/VFS is primary research hypothesis, not final filesystem selection.
- **FS-D02:** STORE-B mature format, STORE-C native CoW, STORE-D metadata journal and STORE-E read-only remain evidence-driven alternatives.
- **FS-D03:** Kernel enforces caps/memory/IPC/DMA/IRQ, not general inode/path/journal parsers by default; FS is asset-specific user-space TCB-S.
- **FS-D04:** Distinguish accepted buffered write, storage device ACK, filesystem commit, hardware flush/FUA and scoped strong DurableReceipt.
- **FS-D05:** Strong sync return requires declared file bytes/inode/directory namespace dependency plus compliant hardware barrier; unsupported flush denies strong durability.
- **FS-D06:** Namespace rename visibility and crash persistence differ; FileSyncData and DirectorySync/Transaction must be explicit.
- **FS-D07:** Default metadata journaling alone cannot advertise full user-data power-loss durability; chosen algorithm requires precise guarantee.
- **FS-D08:** Writes and mmap dirty/writeback carry generation and sponsor; old completion does not clean newer dirty contents.
- **FS-D09:** Async writeback errors must be observable after successful buffered writes and survive relevant client/server changes until accounted.
- **FS-D10:** StorageHost/FilesystemService death does not undo committed IO, drain DMA or make old pending FS requests atomic rollback.
- **FS-D11:** R0/B/R1/R2-F/R2-O and DeviceGroup isolation remain separate from persistent media order or filesystem commit.
- **FS-D12:** Journal/WAL/CoW replay must be idempotent and bounded or return corruption/read-only/fail-stop rather than fabricate repaired user data.
- **FS-D13:** User path resolution/mount/namespace access derives from File/DirectoryCaps; guest UID0 doesn't grant whole disk/root native rights.
- **FS-D14:** Verified SystemImage needs authenticated content hash roots and on-read checks under RFC-017 policy; integrity not equivalent to confidentiality.
- **FS-D15:** A/B slot-good and floor writer stay outside ordinary DataVolume; data schema versions and rollback eligibility jointly considered.
- **FS-D16:** M0 must not depend on a demand-paged storage/VFS/Pager path to load/recover itself; fatal media failure can stop safely.
- **FS-D17:** Snapshot/GC and ENOSPC are budgeted; space reclamation cannot break committed prior root/refcounts.
- **FS-D18:** No stable POSIX/Windows/Darwin mapping, on-disk FS format, atomic block size or numeric timings adopted before RFC-009/010/018 and real device evidence.
- **FS-D19:** Product claims name the hardware/power fault model; QEMU results and physical NVMe PLP/Flush reliability have distinct evidence levels.
- **FS-D20:** All FS choices and R1 exit gates remain Proposed; no implementation, boot metadata writes, real power-cut test or third-party code imports authorized.

## 22. Cross-RFC integration and conflict register

| RFC | Interface/proof needed |
|---|---|
| RFC-001 | kernel vs userspace FS parser/TCB, ring-0 privileged IO only through typed authority |
| RFC-002 | Directory/File/Volume capabilities, sponsor and actual shared block/pin/file lifetime |
| RFC-003 | queued IPC COMMITTED/UNKNOWN vs durable transaction; rebind/idempotency after service death |
| RFC-004 | PagerBind/FaultWaiter generations, M0–M3 staging, writable mmap dirty/writeback, OOM dirty frame safety |
| RFC-005 | bounded dirty/replay/flush worker CPU/IRQ and recovery reservation, no implicit client→server CPU donation |
| RFC-006 | boot trust/immutable image, M0 read-only bootstrap, A/B trial/Good/floor/eligible real rescue |
| RFC-007 | NVMe I/O/DMA/IRQ, device cache vs physical R2-F, reset impact, volume IOVA/dirty pin sponsor |
| RFC-009 | native FileCap/FileSync/DirectorySync/DurableReceipt semantics and ABI versioning |
| RFC-010 | Linux POSIX fsync/fdatasync/msync/rename, fd tables and errno without global guest hardware rights |
| RFC-011 | packaging/update and app install on verified vs mutable volumes, permission and version migrations |
| RFC-015 | malicious FS image parser fuzz, power-cut failure state model, delayed writeback error and data-corruption detection |
| RFC-016 | user portal/file permission and recovery encryption/user-data consent |
| RFC-017 | system image root authenticity, boot floor/slot counters, authenticated rescue, keys and data rollback |
| RFC-018 | persistent format version, FileID/reply/gen, logical errors, transaction records and native vs guest statuses |

**Joint P0 groups:** (003+004+007+008) writeback→DMA→storage completion→FileSync result; (006+008+017) slot/floor/crash metadata, signed image/rescue and data schema; (002+008+009+010) namespace capability vs guest VFS API; (004+005+006+008) M0 boot storage/Pager deadlock. No single service declaring "Ready" proves these integration boundaries.

## 23. Design research exit gates and next review

**RFC-008 v0.1 R1 storage-design gate: OPEN/NOT PASSED.** Before selecting writable FS/claims:
1. Draw typed File/Directory/Volume/BlockDevice authority and sponsor graph including raw block escalation and hostile guest path.
2. Build complete durability partial-order graph for write, file create, rename, multi-file transaction, journal/CoW root, Flush/FUA and directory sync; identify exact reply commit point.
3. Model honest volatile cache, failed Flush, reordered queues and torn/partial sectors; enumerate every power-cut point, recovery outcome and ambiguous reply.
4. Formally separate metadata crash consistency, latest data survival, malicious rollback/authenticated image, confidentiality and independent backups.
5. Verify dirty page generations, concurrent mmap writers/truncate and delayed writeback errors after Pager/StorageHost restart, ENOSPC and forced RO.
6. Cost staged M0 immutable boot and reserved early storage stack without file self-Pager dependency.
7. Compare STORE-B/C/D with equivalent failure assumptions, user-space filesystem asset TCB, legal/third-party code licensing and real hardware usability.
8. Propose exact Reference PC/drive VWC, Flush/FUA, PLP, atomic unit, controller reset and DMA conditions; emulator is not physical assurance.
9. Model A/B trial good/secure floor vs user schema migration, snapshot space, physically accessible rescue and independent trusted slot writer.
10. No approval until explicit Go/No-Go and comprehensive adversarial v0.2 review; do not use real user data disks in any future tests.

**Next research:** Adversarial second review RFC-008 v0.1→v0.2 (as done for RFC-001…007), then prioritize RFC-017 Boot Trust/Update and RFC-009 Native ABI/File API interface alignment. The overall eight D1–D8 accepted directions remain unchanged. This is a proposal, not a statement that tests or products exist.

## 24. Official primary references and interpretation notes (researched 2026-10-24)

- **[S1] Linux block volatile write cache / REQ_PREFLUSH and REQ_FUA:** https://docs.kernel.org/next/block/writeback_cache_control.html
- **[S2] Linux fsync(2), fdatasync and directory entry durability:** https://man7.org/linux/man-pages/man2/fsync.2.html
- **[S3] Linux ext4 JBD2 journaling and metadata/data distinction:** https://docs.kernel.org/filesystems/ext4/journal.html
- **[S4] SQLite atomic commit and powersafe overwrite assumptions:** https://www.sqlite.org/atomiccommit.html
- **[S5] SQLite WAL checkpoint and crash lifecycle:** https://www.sqlite.org/wal.html
- **[S6] SQLite PRAGMA synchronous durability policy:** https://www.sqlite.org/pragma.html
- **[S7] Linux iomap file writeback and error accounting:** https://docs.kernel.org/filesystems/iomap/operations.html
- **[S8] Linux fs-verity read-only Merkle content authentication:** https://docs.kernel.org/filesystems/fsverity.html
- **[S9] Linux dm-verity read-only block verification:** https://docs.kernel.org/admin-guide/device-mapper/verity.html
- **[S10] Android current Virtual A/B snapshots/merge and boot dependency:** https://source.android.com/docs/core/ota/virtual_ab
- **[S11] PostgreSQL WAL writer and fsync methods:** https://www.postgresql.org/docs/current/runtime-config-wal.html
- **[S12] NVM Express official NVM Command Set (2026 Revision 1.3):** https://nvmexpress.org/specification/nvm-command-set-specification/
- **[S13] Linux errseq_t and multiple observer writeback error sampling:** https://docs.kernel.org/5.16/core-api/errseq.html
- **[S14] Linux ext4 orphan file cleanup after crash:** https://docs.kernel.org/filesystems/ext4/orphan.html
- **[S15] Linux DMA API mapping vs hardware operation lifecycle:** https://docs.kernel.org/core-api/dma-api-howto.html
- **[S16] PostgreSQL introduction to WAL and crash recovery:** https://www.postgresql.org/docs/current/wal-intro.html

**Interpretation:** Official external documents describe existing systems and standards. They do not prove UniOS safety or authorize copying third-party filesystems. License/provenance review is separate from selecting a design idea. Actual device behavior and correctness require verified platform-specific evidence.

---

**History:** 2026-10-24 — RFC-008 v0.1 Proposed Specification drafted for architecture research; no code, filesystem implementation, driver, boot, disk formatting, hardware experiment, power interruption test, model checking or validated performance result.

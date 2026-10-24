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


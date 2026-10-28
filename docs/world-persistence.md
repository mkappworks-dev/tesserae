# World persistence contract

Tickets 22–23 share PR 4. The current implementation is a **storage
foundation**, not an application save feature. Neither ticket is complete. The
application still opens an ephemeral World and does not call this storage API.

## Authoritative records

`core::save::Commit` carries format/generator versions, a random UUID World
identity independent of seed, a monotonic commit sequence, seed, Voxel size,
named generation parameters, exact required Game/Mod builds, content names,
current Player Eye, authoritative Chunks and simulation checkpoints. Global
preferences and creative shortcuts are separate data. No inventory, movement,
character or camera implementation is introduced.

Changed Chunks are complete run-length encoded level-0 records, including Air.
Traversal is x/y/z, z fastest. Material and Shape references are names,
including packed lake sources, Overlay fluid, pending flow and fire residue.
Runtime ids are resolved against a supplied mapping; a missing name fails. Fluid
records preserve source identity and each Void's surface, reach and descent.
Burning records preserve fuel. Generation records resolve named ground and lake
content.

Simulation checkpoints contain the logical step, count bounds, flow deadlines
and desired changes, running and frozen fire clocks, ignition/burn-out changes,
Agenda wakes, current and next rescan passes, exact cursors, parked spans and
fairness turns. Deadlines are remaining logical steps, never wall time. Agenda
checkpointing refuses an outstanding step. Timer checkpointing requires the
Server first to gather timers owned by workers; calling it on only the idle
Server timer collection would lose running work.

Only the Player Eye exists on the Server today. Sequence numbers and pose refill
budgets are session state. A closed extension map reserves independently
versioned future Player data; nonempty unknown extensions refuse loading. The
Client's current look direction is not Server-owned and is not persisted here.
Future movement/view work must explicitly define and version its durable fields.

Light, meshes, connections, coarse Chunks, Jobs and Lifetimes are derived and
must be rebuilt. A saved revision is data identity, not a reusable Lifetime.

## File protocol and recovery

A World directory has immutable `00000000000000000001.commit` files and one
exclusive `writer.lock`. A commit contains an eight-byte `TESSAVE1` magic,
32-byte SHA-256 of the remaining bytes, and UTF-8 JSON. JSON records reject
unknown fields. Sequence in the filename must agree with the body. Format and
generator version 1 have no implicit migrations. The current prototype caps a
whole commit at 256 MiB; exceeding it reports an error, not successful saving.

Writing uses a unique create-new `.pending` file, writes header/checksum/body,
syncs the complete file, renames it to its final sequence, then syncs the parent
directory. Only success after directory sync may acknowledge a snapshot. A
failure after rename can leave a complete visible commit; the failed sequence is
reserved and a retry uses a newer sequence. Previous commits are retained. There
is no garbage collection yet; storage use grows with successful commits.

Recovery is read-only, newest first. Pending files are ignored. Incomplete,
checksum-invalid or semantically corrupt commits can fall back to an older
complete commit, with explicit skipped-file diagnostics. A complete incompatible
format, generator or required-content set refuses loading instead of rolling
back to an older compatible World. Required Mods include id, version, contract,
Game identity and exact build hash. Content migration is separate work.

A Writer validates content before acquiring a lock and revalidates while holding
it. An interrupted process leaves its lock: an application recovery flow must
verify no writer remains before explicit stale-lock removal. This API
deliberately does not guess that a lock is stale. Do not open an incompatible
World for writing, or interpret a failed load as a new World.

Tests cover interrupted/truncated files, checksum corruption, write/file-sync/
rename/directory-sync error injection, recovery, exclusive writers, reserved
sequences and stale completions. They verify the chosen filesystem protocol on
the test host. They do **not** prove survival of power loss, storage-controller
failure, arbitrary filesystem behavior or Windows directory syncing. No broad
crash-safety guarantee is made.

## Required Server integration, still outstanding

The Server must capture immutable Chunk handles and the complete simulation at a
step boundary, then serialize/write on a worker. Snapshot identity includes
World id, Chunk coordinate, Lifetime and revision. `DirtyState::complete` only
accepts an exact current identity; failed writes must never call it. A newer
edit or new Lifetime stays dirty after an old snapshot completes.

The runtime must request autosaves every 30 seconds and commit changed Chunks
before releasing them. Evicted simulation stores remaining logical delays and
resumes against the admission step; offline elapsed time is never added. Saved
changes must participate in coarse derivation at every affected level. An
uncommitted or failed snapshot cannot authorize eviction. The current storage
library alone does not implement any of these runtime guarantees.

New/open must cross WIT explicitly, carrying World identity and opening kind.
Declarations and registrations run for both; initial Game construction runs only
for new Worlds. An old contract unable to distinguish reopening must refuse
persistent reopening rather than replay construction. Mod process memory is not
a save, and this feature does not restore arbitrary guest state. The existing
WIT and demo have not yet been changed.

The smallest intended initial entry point is an explicit World directory: create
a new save there or reopen its existing complete commit. One application process
opens one World; switching Worlds requires restart until a later lifecycle
implementation can clear all old Jobs and session messages. The World
list/create/rename/seed UI belongs to PR 7. This entry point is **not yet
implemented**, and no environment variable or CLI flag currently enables it.

Save & Quit must send a Server intent, wait for a successful current snapshot,
then close. Failure leaves the World open with Retry and Return. Exiting dirty
without saving needs confirmation. Minimal feedback must distinguish saving,
saved, unsaved, recovery and error states. Window-close interception must use
this same lifecycle. Runtime integration, WIT/demo lifecycle, save feedback,
coarse reconstruction, native evidence and the full ticket-23 matrix remain
required before PR 4 can be called complete.

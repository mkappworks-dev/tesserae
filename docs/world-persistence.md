# World persistence contract

Tickets 22–23 share PR 4. A launch opens the World list, which manages the
folders under `worlds/` beside the user Mods directory.
`TESSERAE_WORLD_DIR=/absolute/path` bypasses the list and opens that directory
directly; an empty directory creates a distinct World, and an existing directory
reopens its latest recoverable commit. Each process opens one World: the list
switches by re-executing the binary with `--world <folder>`, so no job or answer
from one World can land in another.

## World list and folders

Each World folder is named once from the World's name, sanitized for macOS,
Linux and Windows and suffixed on a case-insensitive collision; it is never
renamed. Identity stays the commits' UUID. A copied folder keeps that UUID,
which is harmless: nothing global is keyed by it, and the list is keyed by path.
A `world.json` sidecar holds display details only: name, Game, resolved seed,
Voxel size, created and last-played times, the UUID and a `pending` flag.
Commits stay authoritative. A missing or corrupt sidecar never hides a folder;
Rename rebuilds it from the newest commit. The single pre-list `worlds/default`
World is adopted as "Default World".

Create writes the folder and a `pending` sidecar, then restarts on it. A folder
without commits takes its UUID, seed and Voxel size from that sidecar and writes
its first commit at once; any other folder without a save is refused rather than
becoming a new World. Saved seed and Voxel size always win over the build
defaults and `TESSERAE_VOXEL_SIZE_M`. Rename and Delete try the writer lock and
refuse while any process holds the World open; Delete moves the folder to the OS
Trash. Create requires an installed Game: the Engine ships only Air, so a World
without one would be empty sky. A World saved without a Game, such as one
created before the World list, still opens and loads no Mods.

Seed text resolves in `tesserae_core::seed`: blank is random; unsigned decimal
within `u64`, or `-` and decimal within `i64` as its two's complement, is the
number; any other digits are rejected; anything else is the first 8 bytes of its
SHA-256, big-endian. The generator folds the `u64` into a `u32`, so distinct
seeds may share terrain.

`TESSERAE_EPHEMERAL=1` explicitly disables persistence. Profiling and screenshot
fixtures also default to ephemeral unless given an explicit World directory. Use
a fresh directory under `/tmp` for measurements, never a personal World.

The menu's Save & Quit control and the window close action request a Server
save. Normal close automatically finishes the final save and exits; it does not
ask whether to save or discard. While finishing, a compact, noninteractive
progress indicator replaces the previous quit-choice dialog. Repeated close
requests cannot dismiss an outstanding save-failure or discard-confirmation
prompt. Saving is automatic; ordinary play shows only a temporary saving
indicator, with no persistent saved/unsaved panel. A failed quit leaves the
World open, with Retry, Return to World, and a separately confirmed exit without
saving. Ordinary autosave failures leave play running and retain changed Chunks
that cannot safely be evicted.

## Authoritative records

`core::save::Commit` carries format/generator versions, a random UUID World
identity independent of seed, a monotonic commit sequence, seed, Voxel size,
named generation parameters, exact required Game/Mod builds, content names,
current Player Eye, authoritative Chunks and simulation checkpoints. Global
preferences and creative shortcuts are separate data. No inventory or character
implementation is introduced.

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

The Player stores its authoritative Eye and a `movement_v1` extension with
flight/physics mode, actual crouched stance and finite yaw/pitch. The optional
`crouching` field defaults to false for earlier `movement_v1` saves. Missing
extension explicitly restores legacy flight; velocity and held input are session
state. Unknown extensions refuse loading. The base format stays 1: older engines
already refuse nonempty extensions and cannot overwrite these saves. Save
dirtiness and completion include the extension, including mode/view changes at a
stationary Eye. Rebuilds of a required Game still change its exact build hash:
existing commits with a different required build refuse loading and remain
untouched. There is no automatic content migration. Movement policy is Game
declaration data.

Light, meshes, connections, coarse Chunks, Jobs and Lifetimes are derived and
must be rebuilt. A saved revision is data identity, not a reusable Lifetime.

## File protocol and recovery

A World directory has immutable `00000000000000000001.commit` files and one
OS-locked `writer.lock` file. A commit contains an eight-byte `TESSAVE1` magic,
32-byte SHA-256 of the remaining bytes, and UTF-8 JSON. JSON records reject
unknown fields. Sequence in the filename must agree with the body. Format and
generator version 1 have no implicit migrations. The current prototype caps a
whole commit at 256 MiB; exceeding it reports an error, not successful saving.

Writing uses a unique create-new `.pending` file, writes header/checksum/body,
syncs the complete file, renames it to its final sequence, then syncs the parent
directory on macOS/Linux. Windows uses a write-through `MoveFileExW` rename
instead of POSIX directory sync. Only completion of the platform's entire
protocol may acknowledge a snapshot. A failure after rename can leave a complete
visible commit; the failed sequence is reserved and a retry uses a newer
sequence. Previous commits are retained. There is no garbage collection yet;
storage use grows with successful commits.

Recovery is read-only, newest first. Pending files are ignored. Incomplete,
checksum-invalid or semantically corrupt commits can fall back to an older
complete commit, with explicit skipped-file diagnostics. A complete incompatible
format, generator or required-content set refuses loading instead of rolling
back to an older compatible World. Required Mods include id, version, contract,
Game identity and exact build hash. Content migration is separate work.

A Writer validates content before acquiring a lock and revalidates while holding
it. The lock is held by the OS and releases on process interruption; its file
remains to avoid unlink/recreate races. A second process cannot acquire a writer
while the first still holds it. Do not open an incompatible World for writing,
or interpret a failed load as a new World.

Tests cover interrupted/truncated files, checksum corruption, write/file-sync/
rename/directory-sync error injection, recovery, exclusive writers, reserved
sequences and stale completions. They verify the chosen filesystem protocol on
the test host. A subprocess test is forcibly killed after syncing a pending file
and before rename, then successfully recovers the previous commit and acquires
the released writer lock. These tests do **not** prove survival of power loss,
storage-controller failure or arbitrary filesystem behavior. Windows is not
runtime-tested on this Mac. Its write-through rename follows
[Microsoft's API contract](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw);
no POSIX directory-sync guarantee is claimed there. No broad crash-safety
guarantee is made.

## Server snapshots and streaming

Every authoritative Player/Mod/simulation change advances its Chunk revision. At
a completed logical-step boundary the Server gathers all worker results and
captures immutable Chunk handles, the bounded Agenda and timers. Chunk name
encoding, serialization, checksumming and filesystem writes run on the IO pool.
Paused Chunk checkpoints are immutable shared handles, and simulation work is
partitioned once for a batch of eviction candidates.

Completion is landed by the Server. World identity is fixed for the process;
Chunk coordinate, Lifetime and revision must still match before an eviction is
approved. Old snapshots can become recoverable commits but cannot mark later
edits clean. Save & Quit freezes new authoritative edits, Mod dispatch/drain and
new simulation steps, finishes the running step, and exits only after a commit
covers the current revision set, Player Eye and simulation revision. Return
resumes play; failed writes never authorize eviction or report success.

Autosave requests run every 30 seconds of real elapsed time, independent of
clamped virtual time and logical simulation progress. A changed or simulated
Chunk leaving the streaming radius first requests a checkpoint and remains held
until its current state commits. Its simulation pauses at the boundary while
that write runs. Failures retain it. Returning before eviction cancels the
request and resumes its remaining logical delays. Evicted and reopened Chunks
resume on admission after light is rebuilt; boundary readers are rescanned
through the existing bounded scheduler. No wall-clock catch-up is performed.
Paused deadlines stay relative to their own pause, even while other Chunks run.

A snapshot stores all changed full-detail Chunks, plus active and paused
simulation checkpoints. Dirty detection compares captured revisions and the
current Server Player, including simulation progress between visible changes.
The initial new-World commit may precede Game Commands that have not yet
applied; only changes actually landed by the Server are authoritative save
state. Pending Commands and guest process memory are not restored.

Saved changes remain authoritative while evicted. Level-0 admission restores
these before generation can replace them. Coarse Jobs recursively derive
branches containing saved changes at every affected level, taking unaffected
branches from their existing level-specific generator. Mesh/light data is
rebuilt; the established volume vote and natural thin-feature loss remain.

## Game and Player lifecycle

The amended unreleased 0.3 WIT `world-info` carries `world-id` and `reopened`.
Declarations and registrations run for both opening kinds. The demo starts its
construction/scenarios only for a new World; reopening starts in its completed
stage and cannot overwrite restored edits. Older contract lines still work for
new Worlds, but refuse persistent reopening because they cannot observe the
opening kind. Mods must reconstruct their transient state through WIT; arbitrary
WASM heap memory, queued Commands and PRNG state are not snapshots.

Required content hashes are taken from the exact trusted bytes compiled by the
host, not a later reread of the component path. Missing or changed content and
incompatible versions refuse loading before acquiring a writer or creating an
application World. Corrupt-newer-commit recovery is explicitly reported.

The Player's current Server-owned Eye is restored exactly for entry. The
original generated respawn point stays separate; the existing 18-metre spawn
altitude is not added to a restored Eye. The camera receives that entry point
through the published spawn contract, without World storage access. Existing
Client-only look orientation is outside current Server Player state. Future
movement/view fields require versioned extensions; this PR invents no modes.

## Limits and validation

The full commit currently has a 256 MiB encoded size limit. Previous complete
commits are retained without garbage collection. Restored changed Chunks remain
in memory as shared handles when unloaded. Large, long-lived Worlds therefore
need later compaction/indexing work; a limit or storage failure is a visible
error and retains unsaved state, never a successful save report. Checkpoints
bound simulation work by the existing count limits, not a universal time bound.

Library and headless Server regressions cover content remapping, independent
same-seed identities, exact timers/Agenda cursors, eviction/return/restart,
new/open demo WIT behavior, stale revision/Lifetime completions, missing
content, Save & Quit failure/retry/return/confirmation, corruption, interrupted
writes and coarse removals. Native review and save-related frame/tick evidence
are recorded in the save-specific implementation validation document under
`.scratch`. User visual review and Windows/Linux runtime validation remain
separate from macOS automated results. No broad crash-safety or performance
guarantee is made.

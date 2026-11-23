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
A `world.json` sidecar holds display details: name, Game, Mods (id and version),
resolved seed, Voxel size, created and last-played times, the UUID, a `pending`
flag, the World's default game mode and the Player's own as it last opened. It
also carries requests for the Server: Create World's Biome edits, applied to the
Biome table before it is snapshotted at the first open, and Edit World's mode
changes, applied at the next open and cleared by the first commit after it
([ADR-0024](adr/0024-game-mode-is-per-player-with-a-world-default.md)). Commits
stay authoritative, and their required content decides what loads: a missing or
reversioned Game or Mod refuses the open before any write. A missing or corrupt
sidecar never hides a folder; Rename rebuilds it from the newest commit. The
single pre-list `worlds/default` World is adopted as "Default World".

The Client writes a `thumbnail.png` beside the sidecar once, when it creates the World. Like the sidecar it is display data, not World state: nothing loads it, a missing or unreadable one shows the neutral picture, so the Server stays the sole authority ([ADR-0002](adr/0002-client-server-from-day-one.md)).

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
current Player Eye, authoritative Chunks and simulation checkpoints. Dropped
items are a separate `entities` record, below, and the World's default game mode
is an optional `game_mode`, absent meaning Creative. Global preferences are
separate data.

Changed Chunks are complete level-0 records, including Air. Traversal is x/y/z,
z fastest. Format 2 names each distinct cell of a Chunk once in a palette and
stores one palette index a Voxel, as a low-byte plane then a high-byte plane,
deflated and base64: a few kilobytes a Chunk where format 1's run-length
records, which spelled a cell's names in every run, took a few hundred. Format 3
stores each Chunk version once, as a binary record in a pack (below), with the
same palette and planes, the high plane only past 256 cells and neither for a
single cell, deflated at the default level: about half of format 2's fast
deflate on a flown World's Chunks, and 8% smaller than indices bit-packed at the
fewest bits. Format 1 and 2 commits still open. Material and Shape references
are names, including packed lake sources, Overlay fluid, pending flow and fire
residue. Runtime ids are resolved against a supplied mapping; a missing name
fails. Fluid records preserve source identity and each Void's surface, reach and
descent. Burning records preserve fuel. Generation records resolve named ground
and lake content.

A World also snapshots its own Climate, Biome table and ores in its generation
parameters, so a Game that gains a Biome later cannot reshape it. The three
fields are optional on read and written only for a non-empty table, so a World
without Biomes restores with none. Ores name their Material and Biomes, and a
Biome names its ground Materials, all through the Mapping like the World's own
ground. A Biome's optional sky and fog colours, cave scale and surface features
ride in its snapshot, absent when it has none, so a World saved before a Biome
gained features keeps its bare ground. A tree Plant's frozen form carries its
`vines` (the climber's Material and `plane` Shape by name, the Orientation
against each side, density and length), absent when it hangs none. A surface
feature that names a Plant freezes what generation reads of it, by Material and
Shape name: a crop's or spreader's Stages, a column's Material, height and
fluid, a tree's Materials, size and fruit, a vine crop's ripe Material, fruit
and reach, gap and cap (a `vine` form, absent in a World made before vine
crops); a feature that names a Material has none of it. A feature's `field`
(group number, region scale and strays) and a Biome's `crop_fields` (each crop's
Stage Materials, Shapes and weight, plot range, spacing, path and ditch
Materials by name, and distance from the lake) ride in the same snapshot, absent
when there are none, so a World made before fields generates none. The snapshot
is separate from `entities` and changes neither the format nor the generator
version.

Plants add nothing else to the save. A Plant Voxel is a Material and its Stage
is that Material, a tree is Voxels, and the Overlay, the Agenda and the
simulation checkpoints carry no per-plant record, so a grown crop and a grown
tree reopen as the Chunk saved them. Growth state beyond the Voxels is not
saved:

- A Sample draws from the World seed, the Chunk and the Logical step, so nothing
  is owed on reopen and no wall time is credited for a Chunk that was away.
- A change deferred past the per-step landing bound waits in memory and is not
  saved; its Voxel Samples again.
- A pending Felling queue is not saved. A World closed mid-fall leaves what was
  queued standing until a neighbouring change asks the Server again.
- A World saved with a Plant that its Game has since dropped still opens: its
  Voxels are Materials and stop growing. The snapshot of a Biome's Plant
  features pins generation only; growth reads the Game's Plant rows, which the
  World already freezes with its exact builds.

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
state. Unknown extensions refuse loading. Extensions alone never moved the base
format: older engines already refuse nonempty extensions and cannot overwrite
these saves. Save dirtiness and completion include the extension, including
mode/view changes at a stationary Eye. Rebuilds of a required Game still change
its exact build hash: existing commits with a different required build refuse
loading and remain untouched. There is no automatic content migration. Movement
policy is Game declaration data.

Dropped items are saved whole in every commit as `entities`: the next entity id
and, per drop, its id, Item by name, count, position and velocity in metres,
whether it is resting, and its remaining steps to despawn and to pickup. The
field is optional on read, so a commit written before dropped items opens with
none, and it changes neither the format nor the generator version. An unknown
Item name, a non-finite number, a duplicate id or a timer out of range makes the
commit corrupt. A drop in an unloaded Chunk is saved and still frozen on
reopening. The live set is capped at 2048, which bounds what each commit adds
and is why there is no per-Chunk entity storage yet. Save dirtiness includes a
revision that moves when a drop spawns, despawns, merges, is picked up or comes
to rest. Save & Quit stops spawning, stepping and Pickup, so the final commit is
what the Player last saw.

The rest of the Player is one extension per key, each registered once on the
Server as a key this build opens; an older engine refuses a save with a key it
does not know: `inventory_v1`, the Hotbar and backpack slots by Item name and
the selected slot; `game_mode_v1`, the Player's own mode; `vitals_v1`, health,
hunger and air in seconds; and `death_v1`, while the Player is dead, with the
cause, where the feet were, what was dropped and the ids of the Dropped items it
became, so a reopened World shows the death screen and its despawn countdown.
Every commit carries them with the Chunks, so the Inventory is never saved apart
from the Edits it paid for, and save dirtiness compares them. `carried_v1`, a
map from Item name to count written before the Inventory, is read once: its
totals pack into slots, any overflow drops at the Player, and it is never
written again
([ADR-0023](adr/0023-the-inventory-is-server-owned-and-pays-inside-the-undo-journal.md)).

A Player's Map is its own commit section, like Containers, not a Player
extension: `map`, a list of tiles, each a position and its samples as six
deflated byte planes (heights as the change from the sample before) in base64. A
Map revision that moves as ground is captured gates re-encoding. The section is
optional on read, so a save without it opens with an empty Map, and it changes
neither the format nor the generator version. The Player's Waypoints are the
`waypoints_v1` extension. Measured on 256 tiles, a Map is about 6 percent of its
raw size. A World whose Game forbids the Map keeps its saved Map
([ADR-0028](adr/0028-the-map-is-a-server-held-per-player-capture.md)).

Light, meshes, connections, coarse Chunks, Jobs and Lifetimes are derived and
must be rebuilt. A saved revision is data identity, not a reusable Lifetime.

## File protocol and recovery

A World directory has immutable `00000000000000000001.commit` files, immutable
`00000000000000000001.pack` files and one OS-locked `writer.lock` file. A commit
contains an eight-byte `TESSAVE1` magic, 32-byte SHA-256 of the remaining bytes,
and UTF-8 JSON. JSON records reject unknown fields. Sequence in the filename
must agree with the body. There are no implicit migrations. A commit's generator
version is 3 and any other is refused: a World keeps the version it was created
with, and new Worlds are stamped 3. A World stamped 1 or 2 is refused with a
message naming its version and saying it predates version 3; those versions were
deleted before the first release, with no migration (ADR-0033). The current
prototype caps a whole commit at 256 MiB; exceeding it reports an error, not
successful saving.

A format 3 commit is a manifest. It holds everything earlier formats did, except
that each Chunk is its coordinate, revision and a record reference, and each
paused simulation share is its home and a record reference. A reference is a
pack sequence, an offset, a length and the SHA-256 of the record's bytes, as a
JSON array. A pack is a `TESPACK1` magic and then records back to back; the
records a commit is the first to need go in the pack of its own sequence, and a
commit with none writes no pack. A record is a kind and a version, then binary
content whose integers are LEB128, signed ones zigzagged, and whose Material and
Shape names go once each through a record-local name table, so a record is
self-contained and compaction copies its bytes. A Chunk record is the length of
a deflated part holding the table, coordinate, revision and palette, that part,
then the deflated index planes as capture made them. A share record is one
deflated part: the table, then the share's timers, wakes, rescans and landing
cursors. A truncated or overlong record, or one of another kind or version, is
corrupt. A share's other fields are the active simulation's, which resuming
reads nothing of but the fairness turn, and only when the active one has none.
The manifest keeps the commit's shape so that an engine knowing only formats 1
and 2 refuses it as an unsupported format; an unreadable file would be skipped
as corrupt and an older commit opened and written over. Opening reads, verifies
and restores every record a manifest names; a missing pack, or a record that is
short or fails its hash, makes the commit corrupt.

Writing uses a unique create-new `.pending` file, writes header/checksum/body,
syncs the complete file, renames it to its final sequence, then syncs the parent
directory on macOS/Linux. A commit with new records first does the same for its
pack, and one directory sync covers both renames; a failure before the
manifest's rename removes the pack again. Windows uses a write-through
`MoveFileExW` rename instead of POSIX directory sync. Only completion of the
platform's entire protocol may acknowledge a snapshot. A failure after rename
can leave a complete visible commit; the failed sequence is reserved and a retry
uses a newer sequence.

A World keeps its three newest commits (`KEPT_COMMITS`). Once a writer has
completed three since it acquired the lock, each later completion deletes every
commit older than the third newest it completed; before that it deletes none, so
a reopened World keeps the commits it opened from until newer ones replace them.
A corrupt or reserved sequence sits below the new ones and ages out the same
way. Deletion follows the completed protocol, so the newest commit is durable
first, and its order does not matter: recovery falls back newest first. A failed
delete is a warning, never a save failure, and the next completion retries it.
Acquiring the lock removes leftover `.pending` files and reserves the sequence
of every pack, so an orphan pack's sequence is never reused.

A writer remembers the records the newest commit it opened or wrote names. A
Chunk whose coordinate and revision it already stores, and a paused share that
is the same shared handle, are named again without being captured, encoded or
validated; only new records are validated. The first write after opening a
format 1 or 2 commit therefore stores every Chunk, and the first after any open
stores every paused share again. After retention, if every commit on disk
parses, each pack none of them names is deleted; a commit that does not parse
may name any pack, so its presence skips the sweep. A missing file is success,
so a crash only leaves an orphan for the next sweep. When a commit would name
less than half of an older pack's record bytes, it copies those records byte for
byte into its own pack and names them there, so the old pack ages out with the
commits that name it and the disk holds about twice the live records at most.

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

Tests cover interrupted/truncated files, checksum corruption, damaged and
missing records, write/file-sync/rename/directory-sync error injection for the
pack and the manifest, recovery, exclusive writers, reserved sequences, garbage
collection, compaction and stale completions. They verify the chosen filesystem
protocol on the test host. A subprocess test is forcibly killed after renaming a
pack and before renaming its manifest, then recovers the previous commit,
acquires the released writer lock and sweeps the orphan pack. These tests do
**not** prove survival of power loss, storage-controller failure or arbitrary
filesystem behavior. Windows is not runtime-tested on this Mac. Its
write-through rename follows
[Microsoft's API contract](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw);
no POSIX directory-sync guarantee is claimed there. No broad crash-safety
guarantee is made.

`tesserae save dump <world-dir> [--sequence N]` prints the newest commit, or the
one numbered N, as JSON on stdout: its fields, each Chunk's palette by name and
indices as runs of `[count, index]`, and each paused share, each with the record
reference it came from in format 3. It reads any format, takes no lock and
changes no file.

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
until its current state commits. A Chunk never changed whose only simulation
work is a rescan is not held: it leaves as dry terrain does, its rescan dropped,
since admission queues the rescan again when it returns. Its simulation pauses
at the boundary while that write runs. Failures retain it. Returning before
eviction cancels the request and resumes its remaining logical delays. Evicted
and reopened Chunks resume on admission after light is rebuilt; boundary readers
are rescanned through the existing bounded scheduler. No wall-clock catch-up is
performed. Paused deadlines stay relative to their own pause, even while other
Chunks run.

A snapshot names all changed full-detail Chunks, plus active and paused
simulation checkpoints, and captures and encodes only the Chunk versions and
shares its writer does not already store. Dirty detection compares captured
revisions and the current Server Player, including simulation progress between
visible changes. The initial new-World commit may precede Game Commands that
have not yet applied; only changes actually landed by the Server are
authoritative save state. Pending Commands and guest process memory are not
restored.

Saved changes remain authoritative while evicted. Level-0 admission restores
these before generation can replace them. Coarse Jobs recursively derive
branches containing saved changes at every affected level, taking unaffected
branches from their existing level-specific generator. Mesh/light data is
rebuilt; the established volume vote and natural thin-feature loss remain.

## Game and Player lifecycle

The amended unreleased 0.3 WIT `world-info` carries `world-id` and `reopened`.
Declarations and registrations run for both opening kinds. Terra starts its
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

A manifest and a pack each have a 256 MiB encoded size limit, and a record 64
MiB. Three complete commits are kept, and the packs they name. A manifest still
names every changed Chunk, so its size grows with them, at tens of bytes a
Chunk. Restored changed Chunks remain in memory as shared handles when unloaded.
A limit or storage failure is a visible error and retains unsaved state, never a
successful save report. Checkpoints bound simulation work by the existing count
limits, not a universal time bound.

Library and headless Server regressions cover content remapping, independent
same-seed identities, exact timers/Agenda cursors, eviction/return/restart,
new/open terra WIT behavior, stale revision/Lifetime completions, missing
content, Save & Quit failure/retry/return/confirmation, corruption, interrupted
writes and coarse removals. Native review and save-related frame/tick evidence
are recorded in the save-specific implementation validation document under
`.scratch`. User visual review and Windows/Linux runtime validation remain
separate from macOS automated results. No broad crash-safety or performance
guarantee is made.

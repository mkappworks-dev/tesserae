# PR 4 implementation and validation

Scope: tickets 22–23 only. Base: b7997b025a4f31e959ed090e23469f7665820670.
Worktree: `.worktrees/feat-world-persistence`, branch `feat/world-persistence`.
No subagents, PR publication, push or merge. PR 2 source/workspace was not
used or changed. A requested fetch/rebase confirmed origin/main still at
b7997b0; the branch was already based on it. Local notes were preserved.

## Local batches

- b9d7f56: named World records, UUID identity, full Chunk/Overlay records,
  exact timers/Agenda checkpoints and recoverable filesystem commits.
- 610852c: Server integration, asynchronous snapshots, paused eviction,
  saved coarse sources at affected levels, explicit new/open WIT lifecycle,
  exact Player entry, save feedback and guarded Save & Quit.
- f8b96e5: real-time autosave independent of virtual clock clamping,
  explicit Retry after failed Save & Quit, readable save feedback and
  window-close regression.

The initial foundation-only state and its incomplete handoff are superseded
by these batches. Technical format, lifecycle, entry point and limits:
`docs/world-persistence.md`. Relevant ADRs have narrow persistence addenda.
No general execution-plan table or unrelated feature scope was rewritten.

## Implemented behavior

The application opens a durable default World or an explicit directory from
TESSERAE_WORLD_DIR. An empty directory creates a new UUID, independent of
seed. Reopen uses saved seed, Voxel size and named generation parameters.
TESSERAE_EPHEMERAL disables persistence; profiling/capture fixtures default
to ephemeral unless an explicit directory is supplied. World management UI
and seed entry remain PR 7 work.

The Server captures immutable Chunk handles and complete simulation state
at logical-step boundaries. Chunk encoding, serialization/checksumming and
filesystem writes run on the IO pool. Dirty revisions, coordinates and
Lifetimes are checked at completion; old snapshots never authorize eviction
of later edits or a replacement Chunk. Unchanged dry generated Chunks need
no save; changed or active simulation Chunks stay held until a matching
commit completes. Paused simulation preserves remaining logical delays and
resumes on admission, without wall-clock catch-up.

Full changed Chunks include Air/removals, packed sources, per-Void fluid and
burning fuel. Timers retain pending flow changes, ignition and burn-out,
running/frozen clocks, pending wakes and exact rescan cursors. Saved changes
feed coarse derivation at all affected levels; geometric thin-feature loss
is unchanged. Derived light, meshes and Lifetimes are rebuilt.

Required Game/Mods bind exact trusted bytes actually compiled by the host,
manifest version/contract and stable content names. Missing or incompatible
content/version refuses load before a writer is acquired. Corrupt newer
commits can recover an older complete commit with explicit diagnostics.
Checksummed immutable commits retain all prior complete files; the OS
writer lock releases on process interruption without deleting its marker.
macOS/Linux sync file and directory; Windows uses write-through rename.
No claim of universal power-loss durability or tested Windows support.

The unreleased 0.3 WIT world-info distinguishes new/open and carries World
identity. The real demo guest emits construction/scenario Commands for a
new World and none on reopen. Older contract lines refuse reopen rather
than guessing lifecycle. Arbitrary WASM heap, queued Commands and PRNG
state are not snapshots. Only changes landed by the Server are saved.

Player entry restores the exact Server Eye without applying the 18-metre
spawn offset twice. The generated respawn point remains separate. Existing
Client-only look orientation is not current Server Player state. No future
movement, character, view or inventory behavior was implemented.

Autosave uses 30 seconds of real elapsed time, scheduled on the Server,
independent of Bevy's clamped virtual time. Save & Quit and window close
freeze new authoritative changes, finish a running simulation step and exit
only after a current save completes. Failure keeps the World open with
Retry/Return. A failed quit requires an explicit Retry; automatic autosave
cannot close its error dialog later. Unsafe exit requires confirmation.

## Automated evidence

All Cargo calls used caffeinate, RUSTFLAGS="-D warnings" and shared target
`/Users/mk/.cache/cargo-target`. Existing Bevy variants were preserved.
All guests/demo were rebuilt after WIT changes.

- Full scripts/cargo-ci passed: formatting, all-feature Clippy, rustdoc,
  default-feature build, guests, workspace tests, benchmark compilation,
  architecture boundaries and dependency checks.
- Final workspace tests with dev and profiling: 935 passed, zero failures,
  five existing ignored measurements. Includes exact entry/respawn,
  window-close routing and the final real-time/failed-quit regressions.
- Final all-feature workspace Clippy passed on f8b96e5. Rustfmt and diff
  whitespace checks passed. Rustdoc/default checks ran after final native
  captures to avoid competing with measurements.
- Eight Server persistence regressions exercise actual save/recovery,
  eviction/return/restart, active fluid/fire, later-edit and Lifetime races,
  autosave, real demo WIT lifecycle, missing content, Player restore,
  failed Save & Quit, Retry, Return and confirmed unsafe exit.
- The core forced-interruption test kills a subprocess after pending-file
  sync and before rename, then recovers the previous commit and acquires
  the released writer lock. Separate injection covers body/file-sync/
  rename/directory-sync failures. These do not establish power-loss safety.
- One pre-final concurrent fixture failed without an error detail. Its
  assertion was strengthened; the isolated rerun, 20 concurrent regression
  rounds and the final full workspace run passed. The earlier log is
  retained; no failure was hidden or converted into an ignored test.

Other regressions cover same-seed independent directories/identities,
Material/Shape remapping, named generation parameters, missing required
content without mutation, version refusal, corruption/truncation, pending
files, malformed data and paused Agenda/timer deadlines. Coarse tests prove
archived removals affect levels 1 through 4, including negative coordinates.
Existing Player/Mod/simulation authority tests remain passing.

## Native evidence and interpretation

All artifacts are retained in `/tmp/tesserae-pr4-review-DENphQ`.
No PNG, executable, raw trace or measurement export is committed.

Six preliminary interleaved save/ephemeral captures used one dev/profiling
binary at 1920 x 1080 offscreen on Apple M5/Metal, current free-flight and
first-person view, normal Game textures/quality and 160/3000 m reach. They
observed snapshot capture below 0.13 ms and worker encoding/write about
24–29 ms. They also exposed autosave's incorrect use of clamped virtual
time: the first delayed autosave appeared at 34.6 seconds. That bug was
fixed and regression-tested. These captures and their summary are retained
as preliminary evidence, excluded from final cadence acceptance. They are
not evidence of an optimization or a dev/release speedup.

The first screenshot exposed a missing bullet glyph and ambiguous combined
saved/unsaved wording. The final text uses plain characters, clearly shows
new unsaved changes and identifies the last complete save. Native captures
of the corrected binary are retained separately under `final-native`.
Final provenance, timing/event summaries and reopen evidence follow below.

## Material limits and review

The current whole-commit size limit is 256 MiB. Prior commits have no garbage
collection, and restored changed Chunks remain in memory as shared handles.
Very large/long-lived Worlds need later indexing/compaction; storage/size
failures are explicit and preserve unsaved state. Existing simulation count
bounds remain intact; measured snapshot times are not universal stall bounds.

Native evidence is macOS/offscreen; Windows/Linux runtime and power-loss
behavior were not tested here. GPU/present elapsed times and physical input
support are not inferred from frame intervals. User visual review of the
save panel/dialogs is pending. Automated lifecycle tests do not substitute
for that review. Artifacts remain until the user completes visual review.


### Final native acceptance

The f8b96e5 dev/profiling binary SHA-256 is:
e7920051942aa99fb876fcbe56478be95744227bca44a63d6d40d81cfeb838ba

Three fresh persistent Worlds and a reopen capture completed successfully.
Each fresh World produced two complete commits, preserved six changed
Chunks and retained the same numeric seed/Player Eye with distinct UUIDs.
The real-time autosaves started at 30.786, 30.644 and 30.704 seconds from the
profiler origin (which includes pre-World host startup). Snapshot wall times
were 0.022834, 0.030334 and 0.049917 ms. Corresponding full Server ticks were
0.371541, 0.603041 and 0.347500 ms; those whole frame intervals were 7.958,
10.634 and 16.358 ms. Worker wall times, including Chunk encoding,
validation, checksumming and filesystem writes, were 28.628, 29.318 and
37.218 ms. Worker CPU was not separately measured.

Final frame rows: 4363, 2878 and 2700; the reopen capture retained 612 more.
The common 20–32 second windows had per-run frame p50s of 7.763/9.456/
11.277 ms, p95s of 10.885/25.906/22.995 ms and p99s of 14.416/41.143/
46.835 ms. Window maxima were 39.006/121.940/123.168 ms. Main CPU p99s were
0.735/0.810/0.811 ms. These wide wall-time tails and host drift are retained;
small snapshot spans do not imply that every frame is short or that all
stalls are explained. No causal overhead subtraction or optimization claim
is made. Shared-host activity outside known builds/benchmarks is not ruled
out. No competing Cargo/rustc/Tesserae job was observed immediately before
the final captures, and no build/test ran alongside them.

The reopening process logged the same durable World UUID, loaded its saved
commit without constructing a new World and completed its native capture.
Headless tests separately prove demo construction/scenario Commands do not
replay, exact Eye entry/respawn behavior and paused timers across reopen.
Reopen's short capture is a lifecycle/visual smoke check, not a matched
startup-performance or complete distant-LOD-fill measurement.

Artifacts: final-provenance.json, final-native-summary.json, the four
final-native subdirectories with native.log, frames.csv, render CSV,
metadata and world.png. The original six preliminary captures remain in
native and are excluded from final cadence acceptance. Snapshot/worker
fields were added to PR 1's raw frame profiler; frame timestamps remain
unclamped and simulation progress is retained. No GPU/present time is
inferred. The corrected save panel was visually inspected by the agent;
user review of the panel and failure/confirmation dialogs remains pending.

## Automatic-save UI follow-up

The user approved removing the persistent save HUD. Save & Quit now lives
in the catalogue/menu header. Normal play has no saved/unsaved panel or
manual-save button; a noninteractive indicator exists only while a write
is running. Save failures remain visible with Retry, and failed quit keeps
its Return/unsafe-exit confirmation controls. Autosave timing and Server
persistence behavior are unchanged.

Client regressions: 108 passed, including normal-play suppression,
transient saving, persistent errors, menu regressions and window-close
routing. All-target/all-feature workspace Clippy passed. Rustfmt,
Prettier on the changed technical document and diff whitespace checks passed.
Native offscreen play/menu captures at 1024 x 640 are retained in
`/tmp/tesserae-pr4-review-DENphQ/menu-ui`. These are visual checks, not timing
comparisons; user visual review remains pending.

The first follow-up captures used the shared target executable after a
parallel build replaced it. Those images are excluded under
`menu-ui/foreign-binary-excluded`. The corrected captures use a private
executable copied immediately after rebuilding this branch; its SHA-256 is
retained in `menu-ui/binary.sha256`. No other workspace or process was changed.

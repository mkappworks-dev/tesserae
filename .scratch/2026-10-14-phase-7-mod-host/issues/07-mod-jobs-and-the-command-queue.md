# Mod Jobs, the Command queue and its drain

Status: ready-for-agent
Blocked by: 05, 06

Spec decisions 7, 8 and 9 in the Server. The ticket where a Mod changes the
World.

## What to build

- `server/mods.rs`, a plugin: a `ModJobs` resource that owns each `ModInstance`
  while it is idle and the `Task` while it is running, in the pattern
  `server/jobs.rs` uses (`AsyncComputeTaskPool`, a map of running tasks, polled
  once per tick). One call per Mod at a time; the call is started in
  `FixedPostUpdate` and landed in the next tick that finds it finished.
- The Edit log: `edit::apply_pending` records each **player** Edit that applies
  as an `EditEvent` (position in metres, before and after) in a bounded buffer;
  a Mod's `TickContext` takes everything since its last call. Mod-origin and
  `dev`-builder Edits are not recorded. A full buffer drops the oldest and
  counts it.
- Landing: a finished task returns `Result<CommandBatch, ModFault>`. `Ok` pushes
  to `PendingCommands`; `Err` drops the instance, writes a `ModDisabled` message
  with the Mod's name and reason, and logs once.
- `PendingCommands` (`pub(in crate::server)`): per Mod, a FIFO of validated
  Commands with a fill cursor. `drain` in `FixedPreUpdate` right after
  `apply_pending`: up to `MOD_VOXELS_PER_TICK` Voxels, round-robin across Mods,
  each through `edit::set_voxel` exactly as `apply_pending` calls it. A Voxel
  whose Chunk is not held is skipped; one `set_voxel` rejects is skipped; both
  count into the Mod's next `BatchReport`.
- Backpressure: a Mod whose backlog is over `MAX_BACKLOG_VOXELS` is not called
  until it drains.
- Set `MOD_VOXELS_PER_TICK` by measurement: time a Voxel through `set_voxel`
  with a relight (the `dev` gallery's builders are the harness), choose the
  number that keeps the worst main-thread tick where phase 5 left it, record the
  table, and compare the worst tick with a filling Mod against none using the
  interleaved method of the earlier tickets, load average beside each figure.

## Tests

- A fixture Mod returns a `fill` into a held, lit Chunk: at quiescence the
  Voxels are there, the Chunk's mesh was redelivered, light is correct (equal to
  a fresh solve of the same Voxels), and the level-1 cover was invalidated.
- A fill of 32 768 Voxels takes `ceil(32768 / MOD_VOXELS_PER_TICK)` ticks and a
  second Mod's small `place` lands within a tick of its own call (round-robin).
- A `place` into an unloaded Chunk is counted rejected, the Mod's next
  `TickContext.last` says so, and nothing else is affected.
- A shaped `place` past `MAX_SHAPED_PER_CHUNK` is rejected by `set_voxel`'s own
  check, not a second one.
- A Mod that traps is disabled, its queued Commands still apply, a second Mod
  continues, and `ModDisabled` is written once.
- Player Edits reach the next `TickContext`; a Mod's own Commands do not.
- A Mod over the backlog cap is not called, and is again once drained.
- Privacy: no `pub` mutator outside `crate::server`, and no `client/` file
  mentions `PendingCommands`, `ModJobs` or `server::mods`.

## Acceptance criteria

- `./scripts/cargo-ci` passes.
- The `MOD_VOXELS_PER_TICK` table and the worst-tick comparison are in Comments.

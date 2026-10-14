# The tick, Command batches and the host-call bench

Status: ready-for-agent
Blocked by: 03, 04

Spec decisions 7 and 8, host side, and the Performance section.

## What to build

- `TickContext { tick, dt_s, edits: Vec<EditEvent>, last: BatchReport }` and
  `ModInstance::tick(ctx) -> Result<CommandBatch, ModFault>`.
- `CommandBatch`: the validated form of a guest's `list<command>`, plain owned
  data. Validation is a pure function `validate(batch, &Tables,
  &WorldSettings-like { voxel_size }) -> Result<CommandBatch, BatchError>` that
  checks, per spec decision 8: finite, ordered regions; `MAX_FILL_VOXELS`;
  `MAX_COMMANDS`; `MAX_VOXELS_PER_CALL`; material, shape and orientation known;
  no air Place. Counts are checked **while lowering the guest's list**, so an
  oversized list is rejected before it is copied whole.
- Region to Voxels: a function from `box-m` and `voxel_size` to an inclusive
  Voxel box using the floor of lower corners, and a lazy iterator over it for a
  `fill`, so the queue holds a region and a cursor and not Voxels. The same
  function is what the Server's drain uses; do not write it twice.
- The Voxel checks `edit::resolve` makes for a player's Place (unknown Material,
  Shape, bad Orientation, no air) move to `tesserae-mods` or `tesserae-core` so
  both call one function. If that means a small public function in core, keep it
  dependency-free and say so in Comments.
- `benches/host_calls.rs` per spec's Performance table, including the bench-only
  `naive_per_voxel` world in the bench's own WIT directory, which the shipped
  Linker never defines. Add `[[bench]] harness = false`, and the criterion
  dev-dependency as `tesserae-core` has it.

## Tests

- The same `fill` at `voxel_size` 1.0 and 0.25 covers the same physical region
  in metres, with 64 times the Voxels at 0.25: assert both the region and the
  count.
- A region on a Voxel boundary includes the Voxel whose lower corner is on `min`
  and excludes the one on `max`.
- Every rejection in the list above, with the batch counted and not applied.
- A fixture guest `builder` that returns N `place`s for N in 16, 256, 4096
  returns exactly those Commands after validation.
- Events in: a `TickContext` with 3 edits is seen by a fixture that echoes the
  count back as a `place` at `x = count`.

## Acceptance criteria

- `./scripts/cargo-ci` passes; benches compile.
- Run the benches with `uptime` before and after, record the load average next
  to every figure in Comments, and do not report a run at a load that distorts
  it. Report `tick_empty`, the per-command cost at each N, native `validate`,
  and `naive_per_voxel` against `tick_commands`.

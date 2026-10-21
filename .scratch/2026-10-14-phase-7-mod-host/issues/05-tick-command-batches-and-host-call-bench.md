# The tick, Command batches and the host-call bench

Status: done
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

## Comments

- Voxel validity lives in dependency-free `Tables::vet_placed` in core because
  both player Edits and Mod Commands need precisely the same check. - Region
  conversion and the lazy cursor live once in `tesserae-mods`; membership   uses
  lower corners in `[min,max)`, so indices are `ceil(min / size)..ceil(max   /
  size)`. Empty regions still validate all axes; sizes must be finite,
  positive and have a finite inverse. - Measured on the M5 (10 cores), wasmtime
  49.0.4, release, process CPU time, 60 interleaved   rounds with rotating
  order; `uptime` before and after was 9.01/10.88/7.69 and   8.55/10.69/7.68.
  The short-call calibration now times batches, avoiding zero   readings. The
  earlier calibration run was discarded.

| N | empty µs | batch µs | ns/place | validate µs | naive µs | load 1m |
| - | - | - | - | - | - | - |
| 16 | 34.48 | 34.24 | 2140.0 | 0.06 | 0.93 | 9.01→8.55 |
| 256 | 34.82 | 35.88 | 140.2 | 0.48 | 11.57 | 9.01→8.55 |
| 4096 | 35.10 | 85.16 | 20.8 | 7.27 | 178.99 | 9.01→8.55 |

- N means placements in one `place` Command, not N outer Commands (capped at
  1024). At load 9.01→8.55, batched tick plus native validation costs
  33.76/36.20/92.40 µs at N=16/256/4096. Native validation is about 1.8
  ns/placement at the larger sizes. - The naive harness refuels and validates
  each Voxel but has no deadline timer;   production tick spawns a deadline
  thread per call. Thus small-batch totals do   not establish a batching speedup
  or flat per-placement cost. At N=4096 naive   is 2.10 times tick alone, or
  1.94 times tick plus validation; the 34–35 µs   empty floor is about 0.2% of a
  15.6 ms tick (load 9.01→8.55). A shared timer   is a future optimization, not
  hidden by subtracting this floor. - The flooder measurement interleaves five
  full-lift and `WasmList` calls,   reporting minimum process CPU and peak live
  host allocation; `uptime`   before/after was 8.55/10.69/7.68. At 1024
  commands: 195 µs lifted, 9 µs   counted first, 165 ns post-lift check (load
  8.55). At 100000: 1.599 ms, 0.500   ms, 39 ns, 3 MiB lifted peak (load 8.55).
  At 1000000: 15.611 ms, 6.519 ms, 42   ns, 30 MiB lifted peak versus under 1
  MiB counted first (load 8.55). - Counting during lift is not worth
  hand-maintaining the typed WIT return path   here: it saves about 9.1 ms and
  30 MiB only for a malformed million-command   batch; guest memory is capped at
  64 MiB and that Mod is disabled. The   post-lift check is constant-time for an
  oversized outer list and runs   before the second conversion. Valid lists also
  count nested placements before copying.   This explicitly replaces the
  ticket's unattainable generated-bindings promise   to reject before the
  initial whole-list lift. - Updated wasmtime/WASI and Cranelift to patch
  49.0.4/0.136.4 because   cargo-deny found RUSTSEC-2026-0378 and
  RUSTSEC-2026-0379 in locked 49.0.3;   the manifest minimum is patched too, so
  a lock refresh cannot regress it.
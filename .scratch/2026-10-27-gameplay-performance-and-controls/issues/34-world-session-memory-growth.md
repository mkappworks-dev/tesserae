# 34: W1.1: RSS grows about 3 MB per World session

Status: done
Blocked by: 30

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: W1.1; keep this separate from other PR scopes.

## What to build

Find and bound the process memory that still rises each time a World is
opened and left in one process. Ticket 30's criterion was RSS within 5% of
cycle 2 after cycle 10 in a release build; four runs measured 7.0%, 5.1%,
8.5% and 5.4%, rising about 3 MB a cycle with no plateau.

## Notes

Session code is not the source: entities, resources, meshes and images are
identical at every return and the malloc allocation count is flat. Evidence
from ticket 30 (macOS, release, soak harness `TESSERAE_SOAK_CYCLES`, with
`TESSERAE_SOAK_HOLD=1` to attach `vmmap`, `heap` or `malloc_history` with
`MallocStackLogging=1`):

- GPU memory (IOAccelerator) 26.8 to 28.2 MB between cycles 1 and 8.
- Malloc live bytes +3.8 MB and fragmentation +7.5 MB, same period.
- `malloc_history` diff of cycle 1 against 7: growth in wgpu buffers and bind
  groups, Metal command-buffer pools and a bevy_ecs slab of wakers.
- `malloc_zone_pressure_relief` after teardown: no consistent gain.

Likely work: check whether Bevy's render world and wgpu release a session's
buffers, bind groups and pipelines when its assets drop; check whether
`ModHost` building a wasmtime `Engine` per load adds to it (not yet isolated);
repeat on Windows and Linux. Needs a longer soak (30+ cycles) to tell a
plateau from a leak before any fix.

## Comments

### Measured and triaged 2026-10-31

Numbers, method, scripts:
[issue-34/research.md](../issue-34/research.md).
Nothing was fixed; instrumentation is saved as a patch and reverted.

- Reproduced on `feat/world-session-memory`: RSS cycle 2 to 10 rose 2.7%,
  4.6%, 4.9% and 8.3% in four release runs, so the 5% line is noise at ten
  cycles. 100 cycles: 519 to 628 MB, still about 1 MB a cycle from 40 to 100,
  slowing but no plateau. Longer sessions (600 frames, not 120) raise it to
  about 7 MB a cycle: it follows work done in a session.
- Not session code: entities, resources, meshes and images flat; `leaks`
  finds 0 leaks; the Rust live heap rises 0.07 MB a cycle (7.4 MB in 100).
- About 85% of the malloc growth is memory malloc holds but nothing uses
  (+110 MB zone-allocated beyond live, 100 cycles); `malloc_zone_pressure_
  relief` frees 0. Live non-Rust malloc rises about 11 MB: Apple's FramePacing
  and IOGPU command-buffer pools. wgpu keeps creating about 60 buffers and 120
  bind groups a cycle that stay live (under 1 MB). IOAccelerator memory shows
  no trend.
- Mod memory is not the cause: a new wasmtime `Engine` per load (not cached;
  `server/mods.rs:334`) drops cleanly, live bytes flat, RSS +10 to 12 MB once
  and level by the tenth load.
- A stale `demo.wasm` makes every open fail at `declare` and return to the
  list; the soak still counts those cycles. Rebuild the demo before a soak.

Recommendation: accept as a documented cost and close. There is no bounded
fix in our code that moves the number: the growth is allocator retention
around Apple's Metal pools. Replace ticket 30's RSS line with one that tests
what we own, and record the cost in the validation note.

Acceptance criteria if closed this way (small docs and test change, no
teardown edit):

- [ ] Ticket 30's criterion reads: entities, resources, meshes, images and
      Rust live bytes within 1% of cycle 2 after cycle 10, from the soak
      harness; RSS recorded, not gated.
- [ ] Validation note states about 1 MB a cycle after cycle 40, no plateau
      by 100 cycles, and that it is allocator retention plus Apple pools.

Optional bounded PR, only if the user wants the 0.07 MB a cycle and the wgpu
object climb chased: find what holds the extra buffers and bind groups (Bevy
render world or wgpu-core) and whether they drop with their assets; accept if
the `create_buffer` and `create_bind_group` live counts are flat between
cycles 3 and 25 in `malloc_history`. Touch no job or Chunk teardown:
`tech/streaming-performance` edits `server/jobs.rs`, `streaming.rs` and
`client/chunk_view.rs`; re-baseline after it merges. Ticket 31 reworks only
the discovery layout around `load_with_opening`; no Engine cache is needed.

### Optional bounded PR taken 2026-10-31

You asked for it: find what holds the wgpu buffers and bind groups and
release them at teardown. Spec:
[world session GPU objects](../../2026-10-31-world-session-gpu-objects/spec.md).
Branch `fix/world-session-gpu-objects`.

- [x] Live wgpu buffers and bind groups equal at cycles 3 and 25.
- [x] Rust live bytes flat across cycles.
- [x] Ticket updated with the holder, the fix and the numbers.

### Holder and fix 2026-10-31

The holder is in `bevy_pbr` 0.20, `render/gpu_preprocess.rs`:
`prepare_preprocess_bind_groups` inserts into `BinUnpackingBindGroups` and
`UniformAllocationBindGroups`, both keyed by (phase, `RetainedViewEntity`), for
every live view each frame and never removes a key. Every camera and shadow
cascade a session spawned kept its bind groups, and through them the per-view
bin-unpacking buffers they bind, after the views were gone. Bevy's own
`SceneUnpackingBuffers` and `ViewBinnedRenderPhases` are pruned each frame; the
bind groups kept their buffers alive anyway.

Found from the stack-logged soak of the instrumented build, grouping the live
`create_buffer` and `create_bind_group` blocks by the Bevy system in their
stack (`issue-34/growth.py`, demangled with `rustfilt`). Cycles 3 to 10:
`write_binned_instance_buffers` for the `Shadow`, `Opaque3d` and `AlphaMask3d`
phases +570, +324 and +241 buffers; `prepare_preprocess_bind_groups` +588 bind
groups. Nothing in Bevy prunes the two maps, so this is reachable from our code
without a patch.

Fix: `render/view_caches.rs`, a render-world system after
`prepare_preprocess_bind_groups` that retains only the keys of views extracted
this frame. It also covers a camera removed inside a session.

Release `tesserae/dev` soaks, offscreen, `main`'s demo dist, wgpu counters
(`issue-34/instrumentation.patch`, reverted):

| Build      | Cycles | Buffers, c3 to last | Bind groups | Rust live MB      |
| ---------- | ------ | ------------------- | ----------- | ----------------- |
| without    | 25     | 346 to 1,226        | 237 to 589  | 97.5 to 99.0      |
| with       | 25     | 186, then 196 at 16 | 173, 185    | 97.3 to 98.3      |
| with, MSL  | 27     | 184 throughout      | 173         | 99.0, 99.6 at 10+ |
| with       | 40     | 186 throughout      | 173         | 97.3 to 97.9      |

- Without the fix: +40 buffers and +16 bind groups a session, linear, and Rust
  live bytes +0.065 MB a cycle.
- With it: counts are equal at cycles 3 and 25 in two of three runs; one run
  stepped once, by 10 buffers and 12 bind groups at cycle 16, and stayed there.
  Rust live bytes level off: +0.016 MB a cycle over 40 cycles, inside the
  counter's 0.1 MB noise after cycle 15.
- The rest of the cycle-3-to-25 live growth in the stack-logged run (19 MB) is
  wgpu-hal's Metal command-encoder storage (`begin_render_pass`,
  `begin_encoding`, `enter_blit`), Apple's pooled command buffers from finding
  4; it is flat after cycle 7.
- Entities 606, resources 400, meshes 5, images 25 at every return. RSS still
  rises (508 to 538 MB over 40 cycles): allocator retention, finding 2.


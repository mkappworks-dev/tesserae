# Ticket 34: where the per-session RSS growth comes from

Measured 2026-10-31 on `feat/world-session-memory` (main ef6835d), macOS 27.2,
Apple M5 / Metal, `--release --features tesserae/dev`, host load 2-4 for the
quiet runs. Measure and triage only: no fix is written. The instrumentation
that produced the allocator columns is saved as `instrumentation.patch` and
was reverted from the tree.

## Method

- `soak.sh <cycles> <binary> <demo-dist> [periodic]` makes a unique
  `/tmp/t34-run.XXXXXX` with its own `HOME`, a copy of the Game, a World
  created through `TESSERAE_LIST_SCRIPT=new,create`, then runs the existing
  `TESSERAE_SOAK_CYCLES` loop (open, 120 frames after first terrain, leave).
  `T34_WINDOWED=1` uses the real window; the default is the offscreen path.
- Temporary `[DEBUG-t34]` code in `client/soak.rs` adds, at each return to the
  list: a counting `#[global_allocator]` (Rust live bytes), `malloc_zone_
  statistics` (all-zone live and zone-allocated bytes), and `vmmap -summary`
  plus `footprint` dumps. `T34_HEAP=3,25` also runs `heap` and `leaks`; with
  `MallocStackLogging=1` it runs `malloc_history -allBySize` for the diff in
  `mhdiff.py`. `table.py` tabulates a run.
- A throwaway example loaded and dropped the Mod host outside Bevy, to isolate
  wasmtime.

## A trap in the first runs

The demo in the main checkout's `games/demo/dist` was older than the contract
in this tree: `declare` trapped, so every "open" in my first five runs was a
failed open that returned to the list. Those runs are not session evidence.
They are kept only as a control: Mod load, compile and trap with no first
terrain and no World render grew RSS 275 to 291 MB over 40 cycles and levelled
off after about 10. Everything below uses a demo rebuilt from this tree
(`/tmp/t34-dist`). Rebuild the demo before any soak.

## Findings

**1. The growth reproduces, but its size depends on the run.**

| Run (release, windowed unless noted)   | c2  | c10 | c20 | c40 | c100 | c2 to c10 |
| -------------------------------------- | --- | --- | --- | --- | ---- | --------- |
| 100 cycles                             | 519 | 533 | 551 | 565 | 628  | 2.7%      |
| 40 cycles                              | 547 | 572 | 585 | 619 |      | 4.6%      |
| 40 cycles, offscreen, relief each      | 531 | 575 | 593 | 616 |      | 8.3%      |
| 20 cycles, 120 frames in World         | 511 | 536 | 546 |     |      | 4.9%      |
| 20 cycles, 600 frames in World         | 626 | 702 | 749 |     |      | 12.1%     |

RSS in MB. Ticket 30's 5.1 to 8.5% sits inside this spread, so the 5% line
fails or passes on noise at cycle 10. The growth slows but has not stopped at
100 cycles: about 1.05 MB per cycle from cycle 40 to 100, against 3 MB or more
early on. Longer sessions raise it sharply (600 frames: about 6.8 MB per cycle
from cycle 2 to 20, against 1.9), so it follows how much a session does, not
only how often one opens.

**2. It is not a Rust leak.** Over 100 cycles (windowed):

| Quantity (MB)                              | c0    | c100  | change |
| ------------------------------------------ | ----- | ----- | ------ |
| Rust live bytes (counting allocator)       | 93.7  | 101.1 | +7.4   |
| All-zone live malloc bytes                 | 166.8 | 185.0 | +18.2  |
| Zone-allocated minus live (retained, free) | 176.6 | 286.4 | +109.8 |

- `leaks` reports 0 leaks for 0 bytes at cycles 3 and 25.
- Entities 606, resources 400, meshes 5 and images 24 at every return.
- Rust live bytes rise a steady 0.07 MB per cycle: small, real, linear.
- Non-Rust malloc (Metal, ObjC, drivers) rises about 10.8 MB.
- About 110 of the roughly 130 MB gained is memory malloc holds but the
  process no longer uses (fragmentation / high-water). `malloc_zone_pressure_
  relief(NULL, 0)` after every return freed 0 bytes in a 40-cycle run, as
  ticket 30 found. Only the default malloc zone grows; the other seven zones
  stay under 1 MB. In vmmap the growth is "Malloc Small (empty)" and "Malloc
  Small" regions.

**3. GPU memory has no trend.** IOAccelerator (graphics) dirty swings 163 to
240 MB between returns with no slope over 40 cycles; IOSurface is a constant
42.9 MB. Ticket 30's 26.8 to 28.2 MB was not a trend either way on this run.
`phys_footprint` is too noisy (compression) to use; `ps` RSS is steadier.

**4. The live non-Rust growth is Apple's Metal pools, plus a slow wgpu object
climb.** `malloc_history` live bytes, cycle 3 to 25 (+49 MB, run with stack
logging):

- `com.apple.FramePacing` `FPMTLMetricsGPUTimeTrackerCommandBuffer` vector:
  +18.7 MB (6 new blocks).
- IOGPU command-buffer storage (`IOGPUMetalCommandBufferStoragePool`,
  `MTLResourceListPoolCreateResourceList`, shmem, pooled resources): about
  +25 MB; resource lists 1012 to 1104.
- wgpu: `create_buffer` live blocks 526 to 2011 and `create_bind_group` 221 to
  1292 plus 217 to 1180 (about 60 buffers and 120 bind groups a cycle, under
  1 MB in bytes). This is the only count that climbs without bound in code we
  link; whether Bevy's render world or wgpu-core holds them is not isolated.

`heap` agrees: `AGXG17GFamilyBuffer` objects 454 to 1821 between cycles 3 and
25, `MTLResourceList` flat at 1158 x 48 KB (56.9 MB, constant).

**5. Mod instance memory is bounded and small.** `ModHost::new` builds a new
wasmtime `Engine` and Linker per load (`server/mods.rs:334`); nothing caches
it, and it drops with `LoadedMods`. Outside Bevy, 30 loops of the demo:

| Loop                      | RSS at loop 0 | RSS at loop 30 | live bytes |
| ------------------------- | ------------- | -------------- | ---------- |
| Engine and Linker only    | 3 MB          | 4 MB           | 0.0 MB     |
| plus compile              | 30 MB         | 40 MB          | 0.1 MB     |
| plus instantiate, declare | 32 MB         | 44 MB          | 0.2 MB     |

Live bytes are flat; RSS rises about 10 MB for the first ten loads and then
stops (allocator retention of compile scratch). The answer to ticket 30's
question is: the Engine cache is not process-lifetime, there is none, and it
is not the cause. It adds at most about 10 to 12 MB, once.

**6. Frames in a session drive the retained memory.** 600-frame sessions
retain 3 times as much zone memory per cycle as 120-frame ones (zone
allocated +156 MB against +48 MB over 20 cycles) while live bytes grow the
same. That is the signature of allocator high-water: a session's transient
peak (streaming jobs, meshes, Chunk copies) is freed, but the live Metal
pools and wgpu objects allocated in between keep the pages pinned.

## Verdict by category

| Suspect                            | Verdict                                             |
| ---------------------------------- | --------------------------------------------------- |
| Session code / ECS / assets        | Ruled out: counts flat, `leaks` 0                   |
| Rust heap                          | 0.07 MB per cycle live, linear, cause not found     |
| Allocator high-water, fragmentation | About 85% of the growth; not releasable by relief   |
| GPU memory (IOAccelerator)         | No trend                                            |
| Apple Metal / FramePacing pools    | Main live non-Rust growth; outside our code         |
| Bevy and wgpu object caches        | Slow count climb, under 1 MB; not isolated          |
| Mod (wasm) memory                  | Bounded, about 10 to 12 MB once, plateaus by 10     |

## Other worktrees

- `tech/streaming-performance` (1581432, 9 commits, 36 files) does not touch
  `session.rs`, `open.rs`, `client/soak.rs` or `tesserae-mods`. It changes
  `server/jobs.rs`, `streaming.rs`, `lod.rs`, `fluids.rs` and
  `client/chunk_view.rs`, and adds `client/holes.rs` and `profiling/gpu.rs`.
  Finding 6 means it will move the retained-memory numbers, so re-baseline
  after it merges. Any fix that edits job or Chunk teardown would conflict
  with it; none is proposed.
- `feat/multiple-installed-games` (ticket 31) has only ticket and spec edits.
  The single load site is `load_with_opening` -> `ModHost::new`. A per-process
  `Engine` cache belongs with 31, not here, and is not needed for memory.

## Not done

Windows and Linux; a plateau beyond 100 cycles; the source of the 0.07 MB per
cycle Rust growth and of the wgpu object climb; the same run under
`MALLOC_NANO` or `MallocSpaceEfficient` settings.

# Bulk relight: write a batch of Voxels, relight once

Status: ready-for-agent; decisions grilled and confirmed 2026-11-04. Branch `tech/bulk-relight`, stacked on
`feat/demo-games-and-shared-mods` (PR #52). Primary-source findings:
[research.md](research.md). Tickets: [issues/](issues/). Closes
[world composition ticket 07](../2026-11-03-world-composition/issues/07-faster-mod-builds.md).

## Problem

A Game's build lands one Voxel at a time. Every Mod Command goes through the
player's single-Voxel Edit path, `edit::apply_intent` → `set_voxel` →
`set_occupants`, and each Voxel re-floods light around itself before the next
is written. ADR-0012 measured that at about 40 us a Voxel under a relight and
capped the drain at `MOD_VOXELS_PER_TICK = 2`, so a 32³ fill takes 256 s.

The demo Games' builds on this branch:

| Game   | 1 m Voxels | 0.5 m Voxels | 0.5 m at 2 a tick |
| ------ | ---------- | ------------ | ----------------- |
| lumen  | 2,476      | 19,712       | 154 s             |
| meadow | 3,181      | 25,448       | 199 s             |
| ember  | 2,537      | 20,289       | 159 s             |

Target: lumen at 0.5 m builds in under 10 s, which needs at least 31 Voxels a
64 Hz tick, within a tick budget streaming and the simulation can live beside.

The same per-Voxel relight makes lava draining outdoors at 0.25 m the costliest
fluid step measured (2.75 ms a Voxel, phase 8 ticket 09); ADR-0014 names a bulk
relight as its follow-up.

## What the code shows

- **One relight per Voxel.** `set_occupants` (`edit.rs:1068`) writes the
  Voxel and overlay, then calls `tesserae_core::relight` if the Voxel's optics
  changed. `relight` darkens what the old light could have reached and
  brightens from the sources that uncovers (`light.rs:876`). Neighbouring
  writes in one build darken and refill overlapping regions over and over: a
  roof Voxel over open sky darkens its whole column, and the next one darkens
  the column beside it, which the first one's refill just lit.
- **The bulk solver cannot take an edit's place.** `light_chunk` converges
  upwards only (`light.rs:857-875`): two Chunks holding light at each other's
  level is a stable point with no source, so a removed light would stay. An
  edit must keep provenance tracking.
- **The write itself has other costs per Voxel.** Each `set_voxel` and
  `set_overlay` is a hash lookup and an `Arc::make_mut`, which copies the whole
  Chunk when a mesh or light Job holds a Snapshot of it. `vet_capacity` sums
  the Chunk's shaped units over the whole palette per shaped write.
- **No-op writes are paid in full.** Air over air still journals, wakes the
  fluid simulation around it, bumps the save revision and marks meshes stale.
- **Edit invariants** a batch must keep, with their lines, are in
  [research.md §0](research.md): canonical encoding and `vet_occupants` (I5),
  the shaped cap (I6), relight only when optics change (I7), the change journal
  (I8), the level-1 cover (I9), four kinds of stale mesh (I4), no write into an
  unlit Chunk (I1), and the incremental and bulk solvers never mixed (I2).

## What other engines do

From [research.md](research.md): the engines that store light per Voxel
(Minecraft with Starlight, and vanilla since 1.20; Luanti) batch with
**multi-source queues**. Every changed position goes into one decrease queue,
then one increase pass runs, so overlapping regions are paid once. For dense
regions both Luanti's `VoxelManip` and FAWE also have a **region relight**:
clear the interior, unlight outward at the rim, flood once. Engines that light
at mesh time (Veloren, Craft) or not at all (Godot Voxel) have nothing to say
about removal.

## Decisions

1. **`tesserae_core::relight_many`** (research option A). It takes every edited
   position with the light it held before the batch. It sets each one's own
   light, darkens from all of them in one queue, then brightens once from every
   regrow seed, draining brightest level first as the bulk `flood` already does.
   `relight` becomes its one-element case. It is the same provenance-tracking
   algorithm, so invariant I2 holds and the result is exactly what the edits
   applied one at a time give. That equality, and equality with a fresh solve,
   are the tests.
2. **A Server write batch** (option C). `edit::Batch` writes each Voxel through
   the same per-Voxel rules as `set_occupants` (I5-I9, I1), into storage at
   once, so a later write in the batch sees earlier ones and the shaped cap
   counts them. It defers only the relight: `finish` runs `relight_many` once
   and marks each stale Chunk once. `set_occupants` becomes a batch of one, so
   there is still one World change, not a second writer (ADR-0002, ADR-0014).
   Nothing reads light between a batch's writes: Edit validation reads
   Occupants, and neither flow nor heat reads light (ADR-0013).
3. **A write that changes nothing is skipped.** If the canonical Voxel and
   overlay equal what is stored, the batch writes, journals and marks nothing.
   It still counts as applied in the Mod's report: the Voxel holds what the Mod
   asked for.
4. **The Mod drain writes one batch a tick** and its budget is re-measured.
   `MOD_VOXELS_PER_TICK` counts Voxels that change; skipped no-ops count against
   a larger, separate bound of reads, so a build over air is not held to the
   write rate. Both are operation counts, which keeps ADR-0014's rule that
   bounds count operations, not time. The write budget is the largest whose
   worst drain tick stays at or under 2 ms at the dev profile, swept with
   ticket 01's lumen harness.
5. **The drain keeps queue order.** Draining nearest the Player first across a
   Mod's queue was weighed and rejected: reordering Commands breaks last-write-
   wins between overlapping ones (a Game that fills air then places a door
   would lose the door), and "nearest" needs a Player the Mod queue does not
   know of with more than one. The Games already send nearest-first, and at the
   new rate the order matters for seconds, not minutes.
6. **Fluid landing writes one batch a step.** The step's chosen changes land in
   one `Batch` and relight once (the ADR-0014 follow-up). `EMITTERS_PER_STEP`
   and `LANDED_PER_STEP` are rules of the simulation and stay unchanged here;
   raising them changes when a World's Voxels change, which is its own
   decision.
7. **Not built: region relight (option B) and a deferred or off-thread relight
   (option E).** Both cost a new light state or a second flood path. Build B
   only if a measured full 32³ fill through `relight_many` does not fit the
   tick; E only if neither does.
8. **Not built: writing the start build before the spawn is lit.** Minecraft's
   worldgen features and Luanti's mapgen write into unlit Chunks and light them
   once. Here it would help only a Game's first build, not later Commands, and
   would undo ADR-0012's "startup dispatch waits for the held, lit spawn
   neighborhood" with a second startup state.
9. **If lumen at 0.5 m misses 10 s within 2 ms**, the remedies go in this
   order: cut per-access cost (a Chunk cache in `LightLattice`, a running
   shaped-unit count per batch); then region relight for Chunks a batch nearly
   fills; raising the 2 ms ceiling trades frame time for build speed and is
   asked first, not done.
10. **Docs.** ADR-0012's "two Voxels per tick" consequence and ADR-0014's "bulk
    relight remains a follow-up" are amended in place with the new measured
    figures: the decision that Commands land through the one World change
    stands, and dropping the batch is cheap, so it is no new ADR. DESIGN.md's
    Mod host and fluid follow-ups follow. GLOSSARY.md is unchanged: neither a
    Mod author nor a Player sees the batch, and a "Write batch" term would blur
    with **Command batch**; the code calls it `edit::Batch`.

## Out of scope

- Player Layouts and the `dev` builders stay on single Edits at 16 a tick:
  each Layout Voxel is an undo entry and an Edit event, and 16 a tick is not the
  bottleneck anyone has reported.
- Undo: Mod Commands are not journalled for undo today, and are not here.
- The Mod contract: no new Command, no contract line. A Game gets faster builds
  without rebuilding.

## Measurement

Ticket 01 adds the harnesses before anything changes; every later ticket
reports against them. The host is often loaded, so every comparison is an
interleaved A/B of CPU time (`getrusage`/thread clocks), with `uptime` before
and after, not wall clock alone.

- **Core:** `cargo bench -p tesserae-core --bench relight`: Voxels a second
  for a plain fill (a 16³ stone box placed over open-sky air, and the same box
  removed), one `relight` per Voxel against `relight_many` per batch of 1, 64
  and 4096.
- **Server:** an ignored test that loads lumen at 0.5 m, streams and lights the
  World around its spawn, and drains its build through the real drain with
  streaming ticking beside it. It reports ticks to drain (and so seconds at 64
  Hz), drain CPU per tick mean and worst, and Voxels a second.

## Tickets

| #   | Ticket                                                   | Blocked by |
| --- | -------------------------------------------------------- | ---------- |
| 01  | [Measurement harnesses and baseline](issues/01-measurement-baseline.md) | none       |
| 02  | [`relight_many` in core](issues/02-relight-many.md)      | 01         |
| 03  | [The Server write batch](issues/03-write-batch.md)       | 02         |
| 04  | [The Mod drain on a batch](issues/04-mod-drain-batch.md) | 03         |
| 05  | [Fluid landing on a batch](issues/05-fluid-landing-batch.md) | 03     |
| 06  | [Docs, ADRs and closing](issues/06-docs-and-closing.md)  | 04, 05     |

## Outcome

- **01.** Core bench `relight` and the ignored drain tests
  (`measure_game_build_drain`, `measure_plain_fill_drain`). Baseline: lumen at
  0.5 m, 154 s at 2 a tick.
- **02.** `relight_many`, brightest level first. A 4096-Voxel batch relights
  3-12x faster than one at a time (release); a single relight is about 10%
  slower.
- **03.** `edit::Batch`. One existing fluid test changed its set-up, because a
  write of what is already there no longer delivers a mesh.
- **04.** Drain: 96 writes (emitter 6) and 4096 reads a tick. lumen 3.1 s,
  meadow 1.5 s, ember 1.9 s at 0.5 m; worst drain tick under 2 ms in the test
  profile. Against the old path at the same 96 a tick: 3x less CPU a Voxel, worst
  tick 4-8x shorter. The emitter weight and the running shaped count were not in
  the plan; profiling found both.
- **05.** Fluid steps land as one batch. The 0.25 m lava drain did not
  measurably speed up (inconclusive: noise matches the difference), so the
  emitter bound is unchanged.
- **Not verified:** the drain in a running client (no display here); release
  profile Server figures; the lava claim above; the full core bench A/B against
  the other benches (they call `light_chunk`, whose code is unchanged).
- **Region relight (option B) and a deferred relight (option E)** were not
  needed to reach the 10 s target and were not built.

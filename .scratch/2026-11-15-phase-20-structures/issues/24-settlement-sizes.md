# 24: Settlements from hamlets to towns

Status: done

Blocked by: 14, 15, 17, 20

Spec: [Phase 20](../spec.md).

## What to build

Every copy of a Settlement is about the same size today: a radius of at most
64 m and at most 24 Plots. Some Settlements should stay a handful of buildings,
as they are now, and others should be much larger.

- **A size range per copy.** A Settlement declares its Plots as a range
  (`plots-min`, `plots-max`) and its radius as a range (`radius-min-m`,
  `radius-max-m`). Each copy draws its size from the World seed and its lattice
  cell, with the radius following the Plots drawn, so one declaration gives
  copies from small to large.
  - The current single values stay valid and mean a fixed size, so existing
    Declarations load unchanged.
- **Larger caps.**
  - The radius goes up to 160 m, and the Plots to 96.
  - Roads branch from the centre and also fork part-way, so a large Settlement
    grows streets rather than long spokes. Branch roads follow the same grade
    and bend rules.
  - The Plot spread from ticket 20 (seeded gaps, setback, thinning towards the
    edge) still applies.
  - The spacing rule (at least twice radius plus margin) uses the declared
    maximum radius, so copies never overlap.
- **Games.** Each Game declares a range of Settlement kinds that fit it.
  - A hamlet: 3–6 Plots.
  - A village: 8–20 Plots.
  - Terra's town grows into a large town: 30–40 Plots, cobbled, with a second
    ring of streets.
  - Hamlets are common and towns rare, through each row's `chance` and
    spacing.
  - Tidewater's islands limit its size; keep its largest kind no wider than its
    land allows.
- **Cost.** A large Settlement is laid out once per lattice cell and cached
  (`worldgen/layouts.rs`). Bounds, claims and coarse levels widen with the
  declared maximum radius.

## Acceptance criteria

- [x] One Settlement declaration yields copies of different sizes across a
      World, from its minimum to its maximum Plots, tested through
      `Worldgen::chunk`.
- [x] A large Settlement has fork roads, and every Plot still faces a road with
      its door.
- [x] Copies never overlap, look the same in any Chunk order, and look the
      same at 1.0, 0.5 and 0.25 m.
- [x] Existing fixed-size Declarations load and generate exactly as before.
- [x] Each Game has at least a hamlet and a village, and Terra a large town. A
      scan at a fixed seed finds small and large copies.
- [x] The benchmark includes a Settlement at the new cap, and every variant
      stays within the 0.25 ms margin, or the margin is revisited with
      numbers.
- [x] WIT (both copies), TOML, the validator, the snapshot, the docs and
      ADR-0029 carry the ranges.

## Comments

Where it lives: `Settlement::radius_min_m` and `plots_min`, and the private
`size_at` (`tesserae-core/src/settlement.rs`); `Founding::radius_m` / `plots`,
`Reach` and `walk` for roads and forks (`worldgen/settlements.rs`); snapshot
`SavedSettlement::radius_min_m` / `plots_min`; WIT `settlement-def`
`radius-min-m` / `plots-min` (both copies); TOML `radius-min-m`, `plots-min`,
and `radius-max-m` / `plots-max` as aliases of `radius-m` / `plots`;
`demo_kit::structures::Village` takes `(min, max)` pairs and the macro writes
a hamlet and a village. Tests: `tests/settlement_worldgen.rs` (ranged copies,
voxel sizes, forks, pinned fixed-size digests), `tests/settlement_snapshot.rs`,
`tesserae-mods` `tests/settlements.rs`, `demo_games/structures.rs`.

Decisions:

- A copy's most Plots are drawn evenly over `plots-min..=plots` from its
  founding hash; its radius follows linearly from `radius-min-m` to
  `radius-m`. The draw is per lattice cell, so it is the same from every Chunk
  and Voxel size.
- The validator checks against the smallest copy: every Template no wider than
  `radius-min-m`, `minimum` and the rows' `min`s within `plots-min`. Spacing,
  the crowded-neighbour check, bounds, claims and `structure_band_m` take the
  largest radius (`reach_m`).
- Roads fork every 36 m out where at least 30 m of disc is left past the fork,
  so only a disc of 66 m or more forks, both ways at 3 in 4 chances, each fork
  0.35 to 0.55 of the ring between two spokes long, through the same `walk` as
  a spoke (grade, bends, water, the centre's keep-off). Spokes stop at 8 times
  their reach in steps so a road hemmed in by hills ends.
- A ranged copy puts a Template that fails its ground back at the end of the
  queue, so rough ground costs Plots last; a fixed-size copy keeps the old
  order. With no forks under 66 m, a fixed-size Settlement lays out exactly as
  before: `a_fixed_size_settlement_lays_out_exactly_as_before_ranges` pins
  digests computed on the pre-ticket code (a 24 m village on hills and a 64 m,
  24-Plot one). The snapshot omits both new fields for a fixed size.
- Games: every Game lists a village (8 to 20 Plots, 40 to 64 m) and a hamlet
  (3 to 6, 24 to 32 m), the village first so a hamlet gives way. Terra's town
  is 30 to 40 Plots, 96 to 150 m, cobbled, 384 m apart, with the village in
  its Biomes' second row. Tidewater's hamlet (22 to 26 m) and harbour (6 to 12
  Plots, 26 to 32 m) stay small for its islands; its test bar for "large" is 3
  buildings.
- After merging ticket 22 the larger houses moved Terra's layouts at the test
  seed: the town now stands at (456, -166), and the sizes scan is 768 m so it
  holds a town and villages of 8 or more in every Game (Terra finds 2 to 30).
  It takes about a minute in a debug build.

Benchmark (load about 3; ms a Chunk, warm / cold, bare in brackets), the new
"town at the cap (160 m, 96 Plots)" variant; the bench's centre scan now looks
384 m out, as such a town keeps over 224 m from spawn:

| Voxel  | warm / cold (bare) |
| ------ | ------------------ |
| 1.0 m  | 0.21 / 0.45 (0.19) |
| 0.5 m  | 0.13 / 0.37 (0.08) |
| 0.25 m | 0.12 / 0.36 (0.06) |

Warm stays within 0.25 ms; cold is 0.26 to 0.30 ms over bare, its one layout
(about 2.5 ms) charged to the 9 Chunks timed. The margin is revisited in
DESIGN.md: 0.35 ms cold for a Settlement wider than the old 64 m cap, 0.25 ms
otherwise. The other variants are unchanged.

Narrowed 2026-11-16 at the user's request: Terra's town is 30 to 40 Plots
(was 30 to 80), its per-Template maxima halved to match. Its disc stays 96
to 150 m: a 72 to 104 m disc rarely fit the 20-Plot minimum, so no town stood
within 1.5 km of spawn at the test seed. Copies laid about 30 buildings
before, so the cap mostly thins the largest.

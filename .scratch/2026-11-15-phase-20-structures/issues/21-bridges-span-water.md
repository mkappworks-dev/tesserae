# 21: Bridges span valleys and water

Status: done

Blocked by: 03, 04, 17, 20

Spec: [Phase 20](../spec.md).

## What to build

Terra's stone bridge, Meadow's footbridge and Tidewater's rope bridge stand on
level ground today, and nothing asks a bridge to cross anything. A Template can
now declare that it **spans**, and worldgen stands it only where it crosses a
gap: a valley, ravine or dip between two banks, dry or with a stream, a lake
neck or a channel in it. Flat dry ground never qualifies.

A spanning Template's long axis, after its turn, is its span.

- **Landings.** The first and last 2 m of the span must be dry ground, in a
  Biome that lists the Template, and within its level tolerance of each other.
  The Ground layer stands at the lower median of the landing samples, and the
  foundation fills only under the landings.
- **Middle.** Across at least the middle third of the span, the ground must lie
  a clearance below the landings' Ground layer, or water must cover it:
  - the clearance is 2 m, in metres;
  - where water rather than a drop makes the gap, it is at least half a metre
    deep somewhere, so every Voxel size up to 0.5 m shows it;
  - no sampled column between the landings rises into the deck's underside;
  - nothing under the span is a Cave.
- **Under the span nothing is carved or filled.** Between the landings a
  Template's Air and Ground cells keep whatever is there, terrain, water or
  air. Only its solid cells (deck, piers, arch, trestles, ropes) are placed,
  and they may replace ground or water where they meet it.
- **Finding a crossing.** A spanning row's lattice cell searches for one, since
  a random point rarely lands on a gap. It tries a coarse grid of positions
  inside the cell and all four turns, and keeps the deepest gap; ties go to the
  first tried, in an order from the seed and the cell, so the result is a pure
  function. The search keeps the footprint inside its cell with the usual
  inset, so neighbours never touch, and its result goes into the layout cache.
- **Declaration.** `spans` (bool, default false) on `template-def`, in the WIT
  (both copies synced), TOML, `DeclaredTemplates`, the snapshot's
  `SavedTemplate`, and contract-compatibility "Amended before release".
  - A spanning Template must be at least 6 m long with landings that fit.
  - The validator refuses one used in a Settlement.
- **Games.** Terra's stone bridge, Meadow's footbridge and Tidewater's rope
  bridge declare it. Their Biome rows get spacings that put a bridge within a
  few hundred metres of spawn where the terrain allows.

## Acceptance criteria

- [x] Through `Worldgen::chunk`, a spanning Structure:
  - crosses a dry valley, its landings dry and the ground under its middle
    third 2 m below its Ground layer;
  - crosses a valley with water in it and keeps every water cell under its
    span but where its own solid cells stand;
  - leaves the terrain under its span untouched but for its solid cells;
  - looks the same in any Chunk order and at 1.0, 0.5 and 0.25 m.
- [x] A spanning Template never stands on flat ground.
- [x] Terra and Tidewater each stand at least one bridge near spawn at a fixed
      seed, tested by presence only. ~~Meadow too~~: its ground has no gap, so
      its footbridge waits for step 22's brooks and valleys (decided
      2026-11-16).
- [x] A Template that does not span behaves exactly as before. Worlds without
      Structures stay byte-identical.
- [x] The benchmark still meets its 0.25 ms margin.
- [x] The authoring guide, data-mods, ADR-0029 and DESIGN.md describe the rule.

## Comments

Widened mid-ticket from "spans water" to "spans a gap": `spans-water` became
`spans`, and the middle rule became a drop or water.

Where it lives: `worldgen/crossings.rs` (`crossing`, the cell's search, and
`gap`, the rules, asked cheapest first); `Template::spanning`, `spans`,
`span_m`, `underside_m` and `LANDING_M`, `MIN_SPAN_M` in `template.rs`;
`stand_structure` keeps Air and Ground cells off the span and the foundation
off it via `on_landing`. Tests: `tests/bridge_worldgen.rs`, the loader's
`templates.rs` and `settlements.rs`, `structure_snapshot.rs`, and
`demo_games/structures.rs`.

Decisions:

- Clearance is a fixed 2 m (`GAP_M`), not one from the Template: the demo
  bridges' decks sit on the Ground layer, so a deck-derived clearance would be
  about 0 and bridge any dip.
- Dry 2 m gaps at a bridge's scale do not exist on the demo Games' rolling
  ground. The deepest dip across a span in 1 km² at seed 7 is 1.19 m (Terra,
  15 m), 1.72 m (Tidewater, 17 m) and 0.15 m (Meadow, 11 m). So water must
  count too, or no Game stands a bridge. Taken literally (water surface 2 m
  below the landings) Terra stood none either; so a column under water
  qualifies as a gap by itself.
- Water alone must be 0.5 m deep somewhere under the middle third
  (`DEPTH_M`). With no depth rule the found crossings were flat shores 0.2 m
  under an integer water level, which show no water Voxel at 1 m. A metre
  would show water at every size but leaves Terra none. At 1 m Voxels a 0.5 m
  crossing may still show ground at the waterline.
- "Rises into the deck" is measured from the higher landing, not the median:
  the landings already differ by the level tolerance, and from the median
  every bank beside the higher landing failed. The deck's underside is the
  lowest Voxel cell between the landings from the Ground layer up; piers
  below the Ground layer hang into the gap and do not count.
- Spanning rows skip ticket 20's Poisson neighbour check: the search keeps the
  whole footprint inside its own cell, inset 0.5 m, so it never touches a
  neighbour's. They still give way to earlier rows and Settlements, and to the
  48 m origin keep-out (`clear_of_origin`), checked last in `gap`.
- The search tries middles 3 m apart (`SEARCH_M`) and all four turns, after an
  exact prefilter (dry far ends, middle wet or low enough under both). At 2 m
  a cold cell cost +375 us a Chunk; at 3 m about +140 us. Ranking by depth
  needs every try, so there is no early exit.
- The mockup's `spans(tpl)` (bridge styles) feeds both its stream drawing and
  the exporter, so `spans` is single-sourced into `games/*/src/templates.rs`.
- Bridges get their own rows: Terra `structures::bridge` (96 m, chance 1),
  demo-kit `bridge` (128 m, chance 1) for Meadow and Tidewater.

Measured at seed 7, bridges standing in 1.05 km² round spawn, 1 m Voxels:

| Game      | Dry gaps | Wet gaps |
| --------- | -------- | -------- |
| Terra     | 0        | 1        |
| Tidewater | 0        | 3        |
| Meadow    | 0        | 0        |

Meadow has no water and no gap, so no rule short of a terrain change stands
its footbridge. With a pond level (0.3 to 0.5 of its relief) and no depth rule
a harness found 8.6 to 12.4 a km²; with the 0.5 m depth rule, none. Proposed,
not built:

- a terrain feature with bridge-scale relief (rivers or ravines carved into
  the height field), which would give every Game real valleys;
- or Meadow gains a pond level and the water depth rule drops to 0 for it,
  accepting crossings whose water shows only at finer Voxels;
- or Meadow's footbridge stops spanning and stands on level ground as before.

Decided 2026-11-16: Meadow keeps its footbridge spanning and stands none
until step 22 carves brooks and valleys into its ground.

Benchmark (`cargo bench -p tesserae-core --bench structures`), new variant
"bridge over a valley (96 m)" on its own hilly, wet bare World: warm +0, +1
and +6 us a Chunk at 1.0, 0.5 and 0.25 m; cold +143, +142 and +85 us. The
other variants are unchanged, all within the 0.25 ms margin.

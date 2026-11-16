# 21: Bridges span water

Status: ready-for-agent

Blocked by: 03, 04, 17, 20

Spec: [Phase 20](../spec.md).

## What to build

Terra's stone bridge, Meadow's footbridge and Tidewater's rope bridge stand on
dry ground today. The ground rules refuse any footprint over a lake, and nothing
asks a bridge to cross anything. A Template can now declare that it **spans
water**, and worldgen stands it only where it crosses water from one bank to the
other.

A spanning Template's long axis, after its turn, is its span.

- **Landings.** The Template's first and last landing columns along the span,
  default 2 m each end, must be dry ground above the water level and within its
  level tolerance of each other. The Ground layer stands at the lower median of
  the landing samples, and the foundation fills only under the landings.
- **Middle.** The columns between the landings must hold water at the World's
  water level:
  - at least the middle third of the span is wet;
  - no sampled column there rises above the deck's underside;
  - nothing under the span is a Cave.
- **The water stays.** In a spanning Structure, no cell at or below the water
  level carves or replaces water, Air and Ground cells included. Water still
  runs under the arch, and nothing is filled beneath the middle.
- **Finding a crossing.** A spanning row's lattice cell searches for one, since
  a random point rarely lands on a narrow neck of water. It tries a coarse grid
  of positions inside the cell and all four turns, first match wins. The search
  order comes from the seed and the cell, so the result is a pure function. The
  search keeps the footprint inside its cell with the usual inset, so neighbours
  never touch, and its result goes into the layout cache.
- **Declaration.** `spans-water` (bool, default false) on `template-def`, in
  the WIT (both copies synced), TOML, `DeclaredTemplates`, the snapshot's
  `SavedTemplate`, and contract-compatibility "Amended before release".
  - A spanning Template must be at least 6 m long with landings that fit.
  - The validator refuses one used in a Settlement.
- **Games.** Terra's stone bridge, Meadow's footbridge and Tidewater's rope
  bridge declare it. Their Biome rows get spacings that put a bridge within a
  few hundred metres of water near spawn where the terrain allows.

## Acceptance criteria

- [ ] Through `Worldgen::chunk`, a spanning Structure:
  - has water under its middle third and dry landings at both ends;
  - keeps every water cell under the deck;
  - looks the same in any Chunk order and at 1.0, 0.5 and 0.25 m.
- [ ] A spanning Template never stands on dry land or wholly in water.
- [ ] Each of Terra, Meadow and Tidewater stands at least one bridge over water
      in a scan near spawn at a fixed seed, tested by presence only.
- [ ] A Template that does not span water behaves exactly as before. Worlds
      without Structures stay byte-identical.
- [ ] The benchmark still meets its 0.25 ms margin.
- [ ] The authoring guide, data-mods, ADR-0029 and DESIGN.md describe the rule.

# 24: Settlements from hamlets to towns

Status: ready-for-agent

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
  - Terra's town grows into a large town: 30–80 Plots, cobbled, with a second
    ring of streets.
  - Hamlets are common and towns rare, through each row's `chance` and
    spacing.
  - Tidewater's islands limit its size; keep its largest kind no wider than its
    land allows.
- **Cost.** A large Settlement is laid out once per lattice cell and cached
  (`worldgen/layouts.rs`). Bounds, claims and coarse levels widen with the
  declared maximum radius.

## Acceptance criteria

- [ ] One Settlement declaration yields copies of different sizes across a
      World, from its minimum to its maximum Plots, tested through
      `Worldgen::chunk`.
- [ ] A large Settlement has fork roads, and every Plot still faces a road with
      its door.
- [ ] Copies never overlap, look the same in any Chunk order, and look the
      same at 1.0, 0.5 and 0.25 m.
- [ ] Existing fixed-size Declarations load and generate exactly as before.
- [ ] Each Game has at least a hamlet and a village, and Terra a large town. A
      scan at a fixed seed finds small and large copies.
- [ ] The benchmark includes a Settlement at the new cap, and every variant
      stays within the 0.25 ms margin, or the margin is revisited with
      numbers.
- [ ] WIT (both copies), TOML, the validator, the snapshot, the docs and
      ADR-0029 carry the ranges.

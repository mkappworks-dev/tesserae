# 15: Docs, ADRs and GLOSSARY

Status: done

Blocked by: 01, 02, 03, 04, 05, 06, 07, 08, 09, 10, 11, 12, 13, 14

Spec: [Phase 18](../spec.md).

## What to build

The docs describe what shipped. ADR-0027 records Plants as declared kinds, state
as Material, sampled growth and Felling; ADR-0006 is amended for the `cross` and
`plane` forms. GLOSSARY.md gains Plant, Stage, Growth, Sample, Seed and Felling.
The contract-compatibility doc lists `plants`, `texture` and the Shape `form`
under "Amended before release"; the authoring guide documents Plants, with the
example row for each kind. The Example Game declares one `crop`. The architecture
doc names where growth runs. DESIGN.md marks step 18 Done and records the bench
numbers.

## Acceptance criteria

- [x] Every new row, field and form is documented where Mod authors look.
- [x] The Example Game loads, and its starter test passes.
- [x] ADR-0027, the ADR-0006 amendment and GLOSSARY.md match the code.
- [x] DESIGN.md step 18 is Done; `npm run fmt:check` passes.

## Comments

Reviewed: boxes 1 to 3 hold (both starter loads pass; ADR-0027 now states the grounding rule and the known limits). Box 4 is the maintainer's call.

Left `ready-for-agent`: the last box is unmet on purpose. DESIGN.md has the step
18 notes paragraph, but the row stays Not started and the four constants read
"TBD by the benchmark pass" until the maintainer's final pass.

- ADR-0027 is new; ADR-0006 (flat forms) and ADR-0022 (a feature names a Plant)
  are amended; ADR-0009 and ADR-0022's cap amendment were checked against
  `MAX_MATERIALS` 248, `PALETTE_LEN` 256 and `MAX_SURFACE_FEATURES` 16 and stand.
- The Example Game is the data-only starter: a `crop` with a seed Item and a
  `cross` Shape, colour only so its install list is unchanged. The Rust starter
  declares none.
- Where the spec and the code differ, the docs follow the code: the Sample tick
  reads the live store inline, not a worker Snapshot; a column's fluid is
  `stands-in`, not `water`; needs are `[plants.needs]`; the refusal reads "needs
  soil"; a Plant names its Shape; a Player's Remove is journalled with its yield
  and only a crop's harvest and Felling are not; Use harvests nothing.

Closed by the maintainer on 2026-11-15: DESIGN.md now marks step 18 Done, and
`npm run fmt:check` passes.

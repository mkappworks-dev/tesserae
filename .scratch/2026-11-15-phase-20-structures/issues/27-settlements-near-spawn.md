# 27: Measure how far Settlements stand, and make them easier to find

Status: needs-info

Blocked by: 20, 24

Spec: [Phase 20](../spec.md).

## What to build

Play-testing found Settlements hard to find. Measure, for each demo Game at a
fixed seed:

- each Settlement row's spacing and `chance`;
- how many lattice cells round spawn draw a candidate, and how many of those
  the chance, the Poisson neighbour check, the origin keep-out, the Biome, the
  water and the ground rules turn away, and how many lay out;
- how far the nearest Settlement stands from spawn, and how far Settlements
  stand from each other.

Report the numbers to the user with a proposal to make Settlements easier to
find, such as denser lattices or a higher chance near spawn, and change the
Games only once the user agrees.

## Acceptance criteria

- [ ] The numbers are recorded in this ticket for every Game.
- [ ] The user decides what changes, if anything.

## Comments

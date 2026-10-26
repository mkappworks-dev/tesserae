# 13: Final verification

Status: ready-for-agent
Blocked by: 01, 02, 03, 04, 05, 06, 07, 08, 09, 10, 11, 12

Spec: [all decisions, and the Testing Decisions](../spec.md).

## What to build

Check the step as a whole before the PR.

- An interleaved thread-CPU A/B of the meshing, collide, LOD and fluids
  benches against main, as step 8.1's ticket 10 ran it. Every fixture id
  moves from 1 to 6 to 8 and up, and step 8.1 saw +7.5% on
  `greedy_mesh/terrain_water` from an id move alone, so a row that moves is
  investigated, not waved through.
- A `cargo dev` walk-through with captures in one `/tmp` folder: terrain, the
  gallery, the menu, the bar, the outline, a ghost and the corner model, and
  a World with no Game.
- Docs: DESIGN.md's step 8.2 entry is complete, and every doc the spec lists
  matches the code.
- An inline, copyable PR body following the template, concise bullets, no
  footer.

## Acceptance criteria

- [ ] The A/B shows no change beyond noise, or each change is explained in the
      Comments.
- [ ] The phase 7 drain cap and the Wasmtime, WASI and Cranelift pins are
      unchanged.
- [ ] The user has reviewed the captures, and the `/tmp` folder is deleted
      afterwards.
- [ ] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass on the
      final tree.

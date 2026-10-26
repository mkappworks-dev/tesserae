# 10: Final verification and the step 8.1 record

Status: ready-for-agent
Blocked by: 07, 09

Spec: [Testing Decisions](../spec.md#testing-decisions).

## What to build

Prove the phase end to end and record it. Run the demo by hand at 1.0, 0.5 and
0.25 m:
- place water, lava and mud from the menu;
- ignite `demo:wood` and put it out;
- build with `demo:brick` in a Mod Shape;
- watch the scripted scenarios.

Confirm that a World with no Game is dry. Add a step 8.1 entry to DESIGN.md's
build order. Remove "the player's fluid tool places water only" from its open
items. Record any limit found as an open item rather than fixing it in passing.

## Acceptance criteria

- [ ] Every ticket 01 to 09 is `Status: done` and its criteria are ticked.
- [ ] The manual pass above is done at all three Voxel sizes, and its
      screenshots are reviewed with the user and then deleted.
- [ ] Interleaved CPU-time A/B against main shows the meshing, LOD and fluids
      benches within noise.
      It also stands in for ticket 01's own A/B, whose interim run left
      `greedy_mesh/terrain_water` +7.5% unexplained: bench water at id 8 and
      at the built-in 7 in one binary to tell layout from cost.
- [ ] DESIGN.md has the step 8.1 entry and updated open items. Data-only Mods
      are listed as the agreed next session.
- [ ] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass on the
      final commit.
- [ ] No push, PR or merge without the user's go-ahead.

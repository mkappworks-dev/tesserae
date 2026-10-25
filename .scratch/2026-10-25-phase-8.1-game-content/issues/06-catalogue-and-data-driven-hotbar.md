# 06: The hotbar holds declared solids, fluids and fire

Status: ready-for-agent
Blocked by: 05

Spec: [decisions 11, 12, 13 and 16](../spec.md).

## What to build

The Server publishes a read-only content catalogue to the Client, beside the
existing palette message:
- each registered Material's full name and id, whether it is solid, whether it
  has a fluid row and whether it has a combustion row;
- each registered Shape's full name and id, and whether turning it changes
  anything.

The Client's hotbar becomes data: nine slots on keys 1 to 9 in every build,
each holding a solid, a fluid source, fire or nothing.
- **Defaults:** stone cube, slab, stair and post, lantern, glass and leaves.
  Then the Game's lake fluid, or else its first declared fluid. Then fire when
  anything burns.
- **Clicks follow the held slot:**
  - solid: left breaks, right places;
  - fluid: left removes a source, right places one;
  - fire: left puts out, right ignites;
  - empty: left breaks, right does nothing.
- **The Alt modifier goes.** The dev fence is no longer a fixed slot.
- **The HUD** names the held slot and what each button does with it.

The Client still never reads a Chunk.

## Acceptance criteria

- [ ] With the demo loaded, the default hotbar holds the seven built-in solids,
      `demo:water` and fire. Given a catalogue with no fluid and no fuel, it
      holds the seven solids and two empty slots. Until ticket 09, a World with
      no Game still has the built-in water.
- [ ] Holding a fluid sends place-source and remove-source intents for that
      fluid. With the demo, water, lava and mud each reach the World as sources
      when placed this way, which is verified at the Server seam.
- [ ] Holding fire sends ignite and put-out intents, which ticket 05's rules
      decide.
- [ ] Holding a solid sends the same place and break intents as today, with Q
      and E turning and flipping only Shapes the catalogue marks as turnable.
- [ ] The HUD text names the slot and its two actions for each slot kind.
- [ ] The catalogue is content only: it carries no Voxel, Chunk or Occupants.
- [ ] CONTRIBUTING's controls table matches the new bindings.
- [ ] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.

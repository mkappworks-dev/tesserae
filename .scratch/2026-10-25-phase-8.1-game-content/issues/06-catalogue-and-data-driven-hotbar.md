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
- **Scrolling** the wheel, or a two-finger swipe on a trackpad, moves the
  selection one slot at a time and wraps. A trackpad's pixel deltas accumulate
  to a notch, so one swipe does not skip several slots.
- **A click places and a right click removes** (spec decision 13, amended on
  2026-10-25):
  - solid: click places, right click breaks;
  - fluid: click places a source, right click removes one;
  - fire: click ignites, right click puts out;
  - empty: click does nothing, right click breaks.

  The right click is the secondary button macOS reports, a two-finger tap by
  default. Nothing waits to tell a double click from a single one.
- **The Alt modifier goes.** The dev fence is no longer a fixed slot.
- **The HUD** names the held slot and what each button does with it.

The Client still never reads a Chunk.

## Acceptance criteria

- [ ] With the demo loaded, the default hotbar holds the seven built-in solids,
      `demo:water` and fire. Given a catalogue with no fluid and no fuel, it
      holds the seven solids and two empty slots. Until ticket 09, a World with
      no Game still has the built-in water.
- [ ] A click sends the held slot's place intent and a right click its remove
      intent, each on the press, for every slot kind.
- [ ] Scrolling one notch either way selects the next or previous slot,
      wrapping at both ends.
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

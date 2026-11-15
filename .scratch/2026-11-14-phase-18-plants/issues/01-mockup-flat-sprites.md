# 01: Mockup: flat Plant sprites

Status: done

Blocked by: None (can start immediately)

Spec: [Phase 18](../spec.md).

## What to build

The Client's look for Plants lands in [`mockup/`](../../../mockup/README.md)
first, and the maintainer approves it before ticket 13 starts. Add a step 18
section showing a Seed, a sapling and a produce Item as flat sprites in the
hotbar, the Inventory grid and item details, the Crafting icons and the hand,
beside the cube of a solid Material for contrast; a Dropped item as an upright
turning sprite; the placement ghost of a `cross` Plant and of a Layout of Seeds;
and the Use prompt reading Harvest on a ripe crop (reversed: see Comments). Show Stage progression of one
crop in the World as crossed quads.

## Acceptance criteria

- [x] The mockup shows every surface in the spec's Client section, in the
      existing visual language.
- [x] `python3 mockup/server.py` serves the new section.
- [x] The maintainer has approved it; the approval is noted under Comments.
- [x] `npm run fmt:check` passes.

## Comments

Open the section with `python3 mockup/server.py`, then
<http://127.0.0.1:8133/plants> (or the "Plants (step 18)" entry in the
mockup's screen picker). Awaiting the maintainer's approval.

Maintainer approved the mockup on 2026-11-14 after hanging apples were changed
to 3D voxel apples (a fruit Voxel replaces a canopy cell). Ticket 13 is
unblocked.

- Reversed by the maintainer: pressing G to harvest was confusing, so the `G Harvest`
  scenes are gone and a `Breaking fruit` panel shows harvest by breaking.

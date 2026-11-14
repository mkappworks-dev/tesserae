# 01: Mockup: flat Plant sprites

Status: ready-for-agent

Blocked by: None (can start immediately)

Spec: [Phase 18](../spec.md).

## What to build

The Client's look for Plants lands in [`mockup/`](../../../mockup/README.md)
first, and the maintainer approves it before ticket 13 starts. Add a step 18
section showing a Seed, a sapling and a produce Item as flat sprites in the
hotbar, the Inventory grid and item details, the Crafting icons and the hand,
beside the cube of a solid Material for contrast; a Dropped item as an upright
turning sprite; the placement ghost of a `cross` Plant and of a Layout of Seeds;
and the Use prompt reading Harvest on a ripe crop. Show Stage progression of one
crop in the World as crossed quads.

## Acceptance criteria

- [ ] The mockup shows every surface in the spec's Client section, in the
      existing visual language.
- [ ] `python3 mockup/server.py` serves the new section.
- [ ] The maintainer has approved it; the approval is noted under Comments.
- [ ] `npm run fmt:check` passes.

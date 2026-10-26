# 05: The hotbar, catalogue and gallery read the Game

Status: done
Blocked by: 02, 03

Spec: [decisions 10, 11 and 12](../spec.md).

## What to build

The Client's defaults and the `dev` gallery stop naming built-in solids.

- The content catalogue carries the ground's rock id beside the lake fluid.
- The hotbar's defaults are the ground rock as a cube, slab, stair and post,
  then the other declared solids as cubes in registration order until seven
  slots are used, then the lake fluid or else the first fluid, then fire when
  anything burns. With no ground the solids start at the first declared one.
- The gallery picks by property: the ground rock, soil and surface for the
  plinths, Shape rows, basin rim, trunk and slab floor; the first registered
  emitting, Blend and Cutout solids for the lamp, glass house and canopy; the
  first registered connected Shape for the fence line. A part with nothing to
  draw it is left out, and with no ground there is no gallery.
- The `dev` Server no longer registers a fence Shape of its own. Its resource
  and the `dev:fence` catalogue name go. The catalogue's turnable check needs
  no Material.

## Acceptance criteria

- [ ] With the demo, the default bar is the demo's stone in four Shapes, then
      its other solids, then its lake fluid, then fire.
- [ ] With a catalogue of no solids the solid slots are empty.
- [ ] Gallery plan tests cover each part chosen by property and each part left
      out when nothing fits, and no gallery without ground.
- [ ] Every Voxel in the gallery is still one the edit path accepts, and it
      stays under the shaped cap in every Chunk.
- [ ] No production code outside the Material table names lantern, glass or
      leaves.
- [ ] A `cargo dev` capture shows the gallery as on main.
- [ ] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.

## Comments

### 2026-10-26: implementation

The catalogue carries the ground rock. Hotbar defaults read declared solids
in registration order after the rock's four Shapes, then lake/first fluid
and fire. The gallery selects ground layers, the first emitting, Blend and
Cutout solids, and the first connected Shape by property. Missing parts are
omitted and no ground produces no gallery. ShapeTable exposes a Shape-only
connection query; the catalogue needs no dummy Material. Production dev
fence registration and DevFence are removed; test tables retain the example.

Added empty-default and sparse-gallery coverage. Formatting, review, full CI
and visual captures remain deferred to the final verification pass.

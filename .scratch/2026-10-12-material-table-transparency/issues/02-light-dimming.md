# Light dimming in both solvers

Status: done
Blocked by: 01

Spec decisions 2 and 5.

## What to build

- `Chunk::palette_any` (or equivalent) and a `translucent` gate for a Chunk:
  a non-air palette entry whose Material is not Opaque.
- `light_chunk`: a `dim` array filled only for a translucent Chunk;
  `flood::<SHAPED, CHANNEL, DIMMED>` with `DIMMED = false` the loop that ships
  today; `seed_from_border` subtracts the entered cell's dimming. Voxels of a
  Material with dimming under 15 contribute a zero `blocked` mask whatever
  their Shape.
- The uniform shortcuts ask the table: a uniform Chunk of dimming 15 and
  no emission is dark; a uniform translucent Chunk is not.
- `relight`: `brighten` subtracts the entered Voxel's dimming; `darken` compares
  against the dimmed delivery, and never clears a Voxel that emits at least what
  it holds (the latent partial-emitter bug in the spec's loose ends).
- A translucent Voxel stores the light that reaches it.
- Benches for translucency: `light_chunk/straddling_water`,
  `light_chunk/leaves_canopy`.

## Acceptance criteria

- Sky 15 enters glass at 15 and water at 13 going down, 14 and 12 going
  sideways; a lantern behind a pane of leaves is one level dimmer than behind
  air.
- Glass does not cast a Shape's shadow; stone of the same Shape does.
- The two solvers agree on scenes with each translucent built-in, inside a
  Chunk and on a seam, extending the existing agreement tests.
- Placing and removing a glass, leaves and water Voxel restores every Voxel to
  where it was (the existing round-trip test, extended).
- A level-8 emitter beside a removed 15 survives `relight`.
- `light_chunk/*` and `flooded_air` unchanged against `before` for terrain, with
  numbers in the PR; `./scripts/cargo-ci` passes.

## Comments

**What differs from the ticket as written.**

- The gate is `Chunk::kinds`, the palette as a slice, not a `palette_any`.
- `Tables::light_params(voxel) -> (mask, dimming)` is the one rule: an opaque
  Material (dimming 15) is blocked by its geometry and subtracts nothing, so a
  stone stair keeps the light of its notch; anything clearer has no mask and
  subtracts its dimming. The spec said "an opaque cube needs no special case in
  the arithmetic", which is true for a cube and wrong for a stair, and is
  corrected.
- `flood` has a third const parameter and `light_chunk` a `sweep::<TRANSLUCENT>`;
  with both false they are the code that shipped before the table.
- `darken` compares against the dimmed delivery and never clears an emitter below
  its emission. The new test fails if that guard is removed (checked).

**Measured** (retired instructions per call, three runs, harness against
`f232c4e`): `light_chunk` straddling -3.5%, settled -2.7%. New, no baseline:
terrain under 3 Voxels of water 4.72 M, terrain with a leaf canopy 5.69 M,
against 3.22 M for the same terrain with neither.


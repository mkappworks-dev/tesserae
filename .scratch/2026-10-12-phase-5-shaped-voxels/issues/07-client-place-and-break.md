# The Client places and breaks

Status: done
Blocked by: 06

The least that makes the edit path usable, and the only thing in the step a
person can try with their hands. Spec decision 8. The Client has no access to
World state and none is added.

## What to build

`client/edit.rs`:

- With the cursor grabbed: left click breaks, right click places, from the
  camera's centre ray (`origin` and `direction` in metres). The click that grabs
  the cursor is not also an Edit.
- A hotbar on keys 1 to 5 (stone cube, stone slab, stone stair, stone post,
  lantern), `Q` stepping `turn`, `E` stepping `up`. No clash with WASD, Space,
  Ctrl, Shift, R, Esc.
- A crosshair, and a line of text for the held Voxel and the last `EditRejected`
  reason, which fades.
- Documented in CONTRIBUTING's controls.

The hotbar and the cycling are pure functions of the input and are unit-tested;
no Client file imports `ChunkStore`.

## Tests

- Selecting, turning and flipping produce the Voxel the HUD says they do, and a
  full `up`/`turn` cycle visits all 24 valid Orientations and no invalid one.
- A click that grabs the cursor sends nothing; the next click does.
- Breaking sends `Break` with the camera's ray; placing sends `Place` with the
  held Voxel.
- A received `EditRejected` is shown and expires.

## Comments

The grabbing click is not an Edit, and that is checked by remembering the
previous frame's grab rather than by ordering against `camera::grab_cursor`, so
it holds whichever system runs first. Removing the memory fails the test.

`Orientation::turned` and `flipped` live in core, with a test that alternating
them reaches all 24 and each comes home after 4 and 6.

Not exercised by hand: no tool here can click in the Bevy window. The Server end
of it is covered by tests that send the same message the Client does; the
Client's systems by tests that drive a headless App with synthetic input.

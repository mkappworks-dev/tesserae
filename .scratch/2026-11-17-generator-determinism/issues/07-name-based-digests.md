# 07: Golden digests hash names, not ids

Status: done

Spec: [Generator determinism](../spec.md).

Blocked by: 05.

## What to build

The registered tables carry no names, so the shared test fixtures gain a name
for each Material and Shape they register. The version 3 golden digests hash
each Voxel as its Material name, Shape name and Orientation, in one canonical
encoding written once in the test, and are re-recorded on macOS. No generation
code changes, so only the encoding moves the recorded values.

## Acceptance criteria

- [x] A copy of the fixtures registered in another order, with its own generator
      parameters, gives every digest unchanged (a test shows it).
- [x] Every existing version 3 digest case is kept and re-recorded by name.
- [x] No generation code is touched, and fixture ids that other tests assert do
      not move.

## Comments

- The names live on `Fixtures::mapping()`, a save `Mapping` built from the
  struct's own id fields: `core:air`, the `t:` names the snapshot tests already
  use for the fixture Materials, and the four built-in Shapes by their
  `core:` names. A fixture test asserts it validates and names every registered
  Material and Shape once, so a registration ticket 08 appends without a name
  fails there. No registration was added, moved or edited.
- The encoding, in `tests/generator_version.rs`: every Voxel in box order as its
  Material name and Shape name, each as a little-endian `u32` length then UTF-8
  bytes, then its Orientation byte. The palette and the uniform-Chunk shortcut
  are no longer hashed, as packing is not content.
- The reordered copy registers an unused Material first, then water, leaves,
  glass, lantern and ground, with rock, soil and surface rotated among the
  ground's ids, and builds its own `WorldgenParams` from those ids. It gives all
  twelve digests unchanged; under the old id encoding it gave other values.
- Re-recorded on this Mac (macOS, arm64): all six origin-box and six high-ground
  cases. No generation code changed, so only the encoding moved them.
- Left on id encoding: `coarse_generation` and `surface_features` keep their
  recorded digests by id. They pin derived data against its source rather than
  the Generator across platforms, and ticket 08 appends content without moving
  any id they use.

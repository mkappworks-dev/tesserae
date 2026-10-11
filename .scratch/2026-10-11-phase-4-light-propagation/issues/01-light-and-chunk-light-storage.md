# Light and ChunkLight storage

Status: done

`tesserae-core/src/light.rs`, which `docs/architecture.md`'s layout block
already reserves for this step. No dependencies: core has none and keeps none.

## What to build

- `pub const MAX_LIGHT: u8 = 15;`
- `Light(u8)` packing `sky:4 | block:4`, with `new`, `sky`, `block`,
  `with_sky`, `with_block`, `Default` = dark. `Copy`, `PartialEq`, `Eq`,
  `Debug`.
- `ChunkLight` with `Uniform(Light)` promoting to a dense
  `Box<[Light; CHUNK_VOLUME]>` on the first differing write, mirroring
  `Chunk`'s storage so the two read the same way. `get`/`set`/`uniform`/
  `filled`/`dark`/`heap_bytes`, and `PartialEq` so the Server can tell whether
  a re-solve changed anything.
- Reuse `chunk.rs`'s index order (x contiguous). Lift the private `index`
  helper to `pub(crate)` rather than writing a second copy: the padded light
  shell in ticket 04 assumes the same order as the Voxel shell.

## Why it is a parallel array and not a palette entry

Spec decision 1. Light is a property of the position, not of the Voxel, and
folding it into the palette would make the Chunk that compresses best - uniform
stone - the one that stops compressing.

## Tests

- A `filled` `ChunkLight` stays uniform and allocates nothing.
- Setting the uniform value does not promote.
- One differing Voxel promotes, and the rest of the Chunk keeps its value.
- Every position round-trips, both channels independently.
- The two nibbles never bleed into each other: `with_sky` leaves `block`
  alone and vice versa, at every level 0 to 15.
- `size_of::<Light>() == 1`.

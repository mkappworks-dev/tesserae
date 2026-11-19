# A saved Chunk is stored whole, not as a delta from generation

Status: accepted.

A changed Chunk is saved as its complete level-0 Voxels and Overlay. Storing
only the Voxels that differ from what the World's pinned Generator version
makes, with a hash of the generated Chunk checked on load, was considered and
rejected for now.

**What deltas would save is small once Commits are incremental.** With each
Chunk version stored once and three Commits kept, a World's saved Chunks are its
live records: a few kilobytes a Chunk, about 1 to 3 MB for the thousand Chunks
of a 3-minute flight. Deltas would cut that by a few times, about a megabyte per
thousand edited Chunks. Many Chunks a version 3 World saves were changed by
water settling or plants growing, not by the Player, and differ from generation
in many Voxels anyway.

**They would turn drift into a World that does not open.** Unchanged Chunks are
already regenerated from the seed, so a generator that gives a different answer
on another platform or after a toolchain update shifts untouched terrain today.
With deltas every edited Chunk would also depend on it, and a mismatch would
refuse the whole World. That drift is a present risk, not a theoretical one:

- Generator versions 2 and 3's Settlements, crop fields and spawn search call
  `sin`, `cos` and `atan2` from `std`, and their Surface features and landforms
  `powi`, which Rust documents as varying by platform and by version. The basic
  operations and `sqrt` are exact.
- CI runs on Linux only; releases build for macOS, Linux and Windows.
- The golden digests hash runtime ids and cover version 2 only without Biomes,
  and version 2's generated Overlay changed on step 17's branch.

**Cost declined.** About 0.1 to 0.6 s at open per thousand edited Chunks to
regenerate and check them, regeneration on every save, and a rule that any fix
to a shipped Generator version can stop a World opening.

## Revisit when

Both hold: saved Chunks are still a measured storage problem in real play after
incremental Commits, and generation is shown deterministic — version 3, the only
one left ([ADR-0033](0033-generator-version-3-is-the-only-version.md)), avoids
`std`'s platform maths ([ADR-0032](0032-generation-uses-no-platform-maths.md))
and name-based golden digests pass on macOS, Linux and Windows CI. That work is
[generator determinism](../../.scratch/2026-11-17-generator-determinism/spec.md).

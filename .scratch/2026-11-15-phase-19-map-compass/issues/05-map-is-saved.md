# 05: The Map is saved with the World

Status: ready-for-agent

Blocked by: 03

Spec: [Phase 19](../spec.md).

## What to build

A Player's Map saves as its own section of the World's save, in the manner of
Containers: a compact, tile-granular encoding, a revision counter compared to
decide whether the section is dirty, and a load that starts with an empty Map
when the section is absent. It does not ride on the per-frame JSON capture that
small per-Player records use. No Generator version moves and no save format
version changes: a save without a Map opens with an empty one.

At 4 bytes a sample a 2 km square is about 4 MB raw, so the encoding must be
compact (run-length or per-tile) and only changed tiles may cost anything on a
save.

## Acceptance criteria

- [ ] Explore, save, reopen: the Map reads back identical (a round trip through the persistence rig).
- [ ] A save made before this section opens with an empty Map and loads unchanged otherwise.
- [ ] A save with an unchanged Map does not re-encode it; a test shows the revision counter gates the work.
- [ ] A long-journey Map's saved size is a small fraction of raw; the measured size is noted in the commit message.
- [ ] The section's validation refuses a corrupt Map with a clear error, as other sections do; fmt, clippy and tests pass.

# 06: The Map is saved with the World

Status: done

Blocked by: 03, 04

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

- [x] Explore, save, reopen: the Map reads back identical (a round trip through the persistence rig).
- [x] After reopening, the first Map the Client receives is the saved one, sent under the same per-step budget (ticket 04's full sync), and every tile arrives within a bounded number of steps.
- [x] A save made before this section opens with an empty Map and loads unchanged otherwise.
- [x] A save with an unchanged Map does not re-encode it; a test shows the revision counter gates the work.
- [x] A long-journey Map's saved size is a small fraction of raw; the measured size is noted in the commit message.
- [x] The section's validation refuses a corrupt Map with a clear error, as other sections do.
- [ ] fmt, clippy and tests pass. fmt and the new tests pass; clippy and the full suite are deferred to the end of the phase.

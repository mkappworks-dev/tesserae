# Edit path, streaming, the cap, and the dev gallery

Status: ready-for-agent
Blocked by: 05, 06

Spec decision 10. The claim to test is that nothing needs to change.

## What to build

- Server edit tests, using a table that registers a fence: placing a fence next
  to another across a Chunk boundary marks both Chunks dirty and the re-meshed
  neighbour grows its arm; breaking one removes the arm on the other; a fence
  placed beside a cube with `to_sealed` grows an arm. If any of these fails,
  the dirty rule is not enough and the fix is in this ticket.
- The edit path treats a connected Shape's orientation byte the way it treats a
  cube's (read `server/edit.rs` first and match it; do not invent a rule).
- **The cap measurement.** Add `terrain_fences_at_cap` (a fully armed fence on
  top of every column, `MAX_SHAPED_PER_CHUNK` of them) beside
  `terrain_stairs_at_cap`. If the Chunk meshes within +50% of
  `mesh_chunk/straddling`, leave the cap alone and record the number in the
  constant's doc comment. If not, apply the fallback in spec decision 10 and
  record both numbers.
- The `dev` build registers a demo fence at startup and the gallery shows a
  fence line, a corner, a fence meeting a stair and a wall of cubes; the hotbar
  gets it. It is registered through `register`, as a Mod will, and is not added
  to `ShapeTable::builtin()` (ids 0-3 are permanent, 4 onwards are the Mods').
- Take a `dev` capture with `TESSERAE_SCREENSHOT_OFFSCREEN=1` and show it.

## Done when

`./scripts/cargo-ci` passes; the streaming worst tick is re-measured against the
15.6 ms a tick has at 64 Hz and `StreamingBudget::default`'s comment updated if
it moves.

# Edit path, streaming, the cap, and the dev gallery

Status: done
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

## Comments

Done.

**Nothing needed to change in the edit path's dirty rule.** Placing a fence next
to another across a Chunk boundary marks the Chunk next door dirty and its mesh
gains the arm; breaking it takes the arm away again (tests count real quads from
`mesh_chunk`, not only the dirty flag). The edit path now drops a connected
Shape's orientation byte as it does a cube's.

**The cap failed and is now weighted** (spec decision 10, outcome added). Same
run, loaded machine, so read the ratios: terrain 0.33 ms, 1024 stairs 1.6 ms,
1024 fences 5.2 ms. A fence is charged `cap_units` = 7 stairs, 146 fit; at 146,
terrain with fences meshes in 1.5 ms and a flat fully armed row in 0.8 ms. Edit
checks `Chunk::shaped_units` against the cap, net of what it replaces.

**Dev gallery and hotbar.** The dev `Shapes` registers the reference fence as id
4 (`examples::FENCE_ID`), not the builtin table. The gallery has a fence line, a
corner, a stair at one end and a cube wall at the other; the hotbar gets key 6
under `dev`. Capture: `TESSERAE_SCREENSHOT_OFFSCREEN=1 TESSERAE_SCREENSHOT_FLY_S=0
cargo run -p tesserae --features dev`; the crop shows rails running into the
stair's back and the cube, and the corner. (The default capture flies past the
gallery.)

**Streaming worst tick.** `capture_cost worst_tick_ms=8.3` against the 15.6 ms
budget, on a machine at load average ~100, so an upper bound; this branch
changes nothing on the streaming path and terrain holds no shaped Voxel, so
`StreamingBudget::default`'s comment stands.

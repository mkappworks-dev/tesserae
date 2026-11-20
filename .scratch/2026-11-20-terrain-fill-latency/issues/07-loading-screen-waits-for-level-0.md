# 07: The loading screen waits for the ground around the Eye

Status: ready-for-agent

Blocked by: 01

Spec: [Terrain fill latency](../spec.md).

## What to build

The loading screen over a World opened from the World list hides once the Client holds a level-0 mesh for every Cell of the mesh ring around the Eye, from what it was sent, not once any terrain is drawn. The 15 s timeout stays, so a World saved without a Game still opens. The stage text names the wait as it names the others; the mockup shows it first and is approved before the Client implements it.

## Acceptance criteria

- [x] The mockup's stage text approved.
- [x] A Client test: the screen stays up while a level-0 Cell of the mesh ring has no mesh, hides when every one has, and hides at the timeout.
- [x] No Client system reads Server World state (ADR-0002).
- [ ] The time from opening a new Terra World to the screen hiding, and to level 0 settling, before and after, in Comments.

## Comments

- Mockup, for approval before any Client code: `/opening` shows the stage text under "Loading the World." as the Client does, and a fifth stage after "Building the first terrain": "Drawing the ground around you". First terrain drawn now ends the fourth stage instead of hiding the screen. Spans: the first terrain 0.75–0.8, the ground 0.8–1.0. In the ground stage the bar follows the larger of the usual sweep (which stops at 90% of the span) and the share of the ring's Cells holding a level-0 mesh, so waiting alone never finishes it and only the last mesh does, at which point the screen hides. "Example load" plays an illustrative load or freezes on any stage.

### Approved, 2026-11-20

The user approved "Wait for the ground": the stage "Drawing the ground around you", the bar's last fifth following the share of the ring drawn, and the screen hiding when the last mesh lands. Shown beside it for comparison and not chosen: "Fill in as it lands", where the screen lifts at the first terrain, as today, and the near ground appears patch by patch. The mockup keeps only the approved design; its review row is relabelled ("Preview stage", "Replay full load", "After it lifts"), and the stages before the ground no longer fade out, which `classList.toggle` given `undefined` had made them do.

### Built, 2026-11-20

The Client counts, of level 0's mesh ring around the Eye (`FlyCamera::eye`, the Eye the Server sends), the Cells it holds a level-0 mesh for, an empty one included, from what it was sent (`ChunkEntities::ground_held` over `delivered`); a coarser level drawing a Cell does not count. When every Cell is held it inserts `GroundDrawn`, which hides the screen; the 15 s timeout stays. `Stage::Ground`, "Drawing the ground around you", begins at the first terrain drawn (`TerrainArrived`), with the mockup's spans (first terrain 0.75–0.8, the ground 0.8–1.0); in it the bar shows the larger of the sweep and the share of the ring held, and never falls below the furthest it reached. The fault guard in `open.rs` now waits for `GroundDrawn`, since the World is still starting while the screen is up; the soak keeps `TerrainArrived`. Tests in `client::loading`: `the_screen_stays_up_while_a_level_zero_cell_of_the_ring_has_no_mesh`, `the_screen_hides_when_every_cell_of_the_ring_holds_a_level_zero_mesh`, `the_screen_hides_at_the_timeout_with_no_ground_drawn`, `the_bar_follows_the_ground_held_and_never_moves_back` and `the_bar_only_moves_forward_and_never_finishes_a_stage_by_waiting`.

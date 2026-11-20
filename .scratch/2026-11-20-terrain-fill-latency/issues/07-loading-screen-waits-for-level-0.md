# 07: The loading screen waits for the ground around the Eye

Status: ready-for-agent

Blocked by: 01

Spec: [Terrain fill latency](../spec.md).

## What to build

The loading screen over a World opened from the World list hides once the Client holds a level-0 mesh for every Cell of the mesh ring around the Eye, from what it was sent, not once any terrain is drawn. The 15 s timeout stays, so a World saved without a Game still opens. The stage text names the wait as it names the others; the mockup shows it first and is approved before the Client implements it.

## Acceptance criteria

- [x] The mockup's stage text approved.
- [ ] A Client test: the screen stays up while a level-0 Cell of the mesh ring has no mesh, hides when every one has, and hides at the timeout.
- [ ] No Client system reads Server World state (ADR-0002).
- [ ] The time from opening a new Terra World to the screen hiding, and to level 0 settling, before and after, in Comments.

## Comments

- Mockup, for approval before any Client code: `/opening` shows the stage text under "Loading the World." as the Client does, and a fifth stage after "Building the first terrain": "Drawing the ground around you". First terrain drawn now ends the fourth stage instead of hiding the screen. Spans: the first terrain 0.75–0.8, the ground 0.8–1.0. In the ground stage the bar follows the larger of the usual sweep (which stops at 90% of the span) and the share of the ring's Cells holding a level-0 mesh, so waiting alone never finishes it and only the last mesh does, at which point the screen hides. "Example load" plays an illustrative load or freezes on any stage.

### Approved, 2026-11-20

The user approved "Wait for the ground": the stage "Drawing the ground around you", the bar's last fifth following the share of the ring drawn, and the screen hiding when the last mesh lands. Shown beside it for comparison and not chosen: "Fill in as it lands", where the screen lifts at the first terrain, as today, and the near ground appears patch by patch. The mockup keeps only the approved design; its review row is relabelled ("Preview stage", "Replay full load", "After it lifts"), and the stages before the ground no longer fade out, which `classList.toggle` given `undefined` had made them do.

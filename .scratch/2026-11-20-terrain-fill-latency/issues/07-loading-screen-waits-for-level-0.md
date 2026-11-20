# 07: The loading screen waits for the ground around the Eye

Status: ready-for-agent

Blocked by: 01

Spec: [Terrain fill latency](../spec.md).

## What to build

The loading screen over a World opened from the World list hides once the Client holds a level-0 mesh for every Cell of the mesh ring around the Eye, from what it was sent, not once any terrain is drawn. The 15 s timeout stays, so a World saved without a Game still opens. The stage text names the wait as it names the others; the mockup shows it first and is approved before the Client implements it.

## Acceptance criteria

- [ ] The mockup's stage text approved.
- [ ] A Client test: the screen stays up while a level-0 Cell of the mesh ring has no mesh, hides when every one has, and hides at the timeout.
- [ ] No Client system reads Server World state (ADR-0002).
- [ ] The time from opening a new Terra World to the screen hiding, and to level 0 settling, before and after, in Comments.

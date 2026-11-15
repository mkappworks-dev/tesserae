# 01: Mockup: the place Tool

Status: ready-for-agent

Blocked by: None

Spec: [Phase 20](../spec.md).

## What to build

The browser mock in `mockup/` shows the `place` Tool: the Template list a Use
opens, the ghost of a Structure with each cell tinted by what placing it would
do (free, replaces ground, replaces water, blocked), the quarter turn, the HUD
hint, and the refusals (Survival, unloaded Chunks, out of reach). Nothing in the
Client is built until this is approved.

## Acceptance criteria

- [ ] The mock runs with `python3 mockup/server.py` and shows the list, the
      ghost in each tint, the turn and each refusal.
- [ ] It follows the Layout overlay's look and the mockup README's Engine states
      group.
- [ ] The user has approved it, recorded under Comments.

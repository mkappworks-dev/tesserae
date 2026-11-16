# 01: Mockup: the place Tool

Status: done

Blocked by: None

Spec: [Phase 20](../spec.md).

## What to build

The browser mock in `mockup/` shows the `place` Tool: the Template list a Use
opens, the ghost of a Structure with each cell tinted by what placing it would
do (free, replaces ground, replaces water, blocked), the quarter turn, the HUD
hint, and the refusals (Survival, unloaded Chunks, out of reach). Nothing in the
Client is built until this is approved.

## Acceptance criteria

- [x] The mock runs with `python3 mockup/server.py` and shows the list, the
      ghost in each tint, the turn and each refusal.
- [x] It follows the Layout overlay's look and the mockup README's Engine states
      group.
- [x] The user has approved it, recorded under Comments.

## Comments

- Approved by the user on 2026-11-15 ("lgtm"), at `/structures` and the Place
  Tool HUD on `/play`.
- The mockup grew during review: fifteen Templates per Game (ten Settlement
  buildings, five wilderness), a distinct shape per building kind, painted
  stand-in textures for about sixteen new Materials per Game, and three bridge
  builds over a stream. Those decisions feed tickets 16 and 17.
- The Outside and Inside views, the Example Game switch and the Example aim
  controls are review aids, not game UI. The game shows only what ticket 11
  builds: the Template list, the quarter turn, the Placing ghost and the HUD.

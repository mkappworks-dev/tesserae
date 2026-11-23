# 18: Sheet panels as dark as the mockup's

Status: ready-for-agent

Blocked by: None (can start immediately)

Spec: [Menus polish](../spec.md).

## What to build

On a real screen the Settings sheet is clearly lighter than the mockup's: the
scenery shows through it (sampled about (69,70,71) against the mockup's
(19,24,25) in the rows, (34,35,36) against (16,18,19) in the rail). Ticket 09
fixed the dim around overlays but not the sheet panels themselves. Every framed
sheet (Settings, Create World, Worlds, Edit World, Mods library, Trust) and its
rail reaches the mockup's darkness on screen, using `theme::css_dim` for any
alpha copied from the mockup's CSS.

## Acceptance criteria

- [ ] On-screen captures of Settings and Create World match the mockup's panel
      and rail darkness within a few levels
- [ ] Panels stay readable over bright and dark scenery
- [ ] A theme test pins the panel and rail colours to the mockup's CSS values

## Comments

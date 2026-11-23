# 11: Layout-ready card

Status: done

Blocked by: [02: Approve the mockup](./02-mockup-approval.md)

Spec: [Menus polish](../spec.md).

## What to build

The Layout-ready HUD card matches the approved mockup: its icon, the layout, its size, and L Edit, ⌫ Exit and RMB Place as key caps, with nothing clickable. The Quick layout screen also stops the Click to look around hint overlapping the prompt bar.

## Acceptance criteria

- [x] The card shows the three key caps and no sentence of keys
- [x] The look-around hint does not overlap the prompt bar

## Comments

The card keeps its live line under the size (Hold a solid to lay it out, Aim at a surface, the will-be-placed tally, the A/B/C step) though the mockup has none: it is status, not keys. The per-step key hints of point to point (hold and drag to B, step back) are gone with the sentence. The caps are the first binding of Layout, Clear tool and Place, so a rebind shows; Backspace is drawn as ⌫. The look-around hint hides while Quick layout is open rather than moving.

Superseded by [27: One panel per tool](./27-one-panel-per-tool.md): the Layout-ready card is gone, and Quick layout's panel shrinks in place to a compact one with L Edit, Esc Exit and RMB Place.

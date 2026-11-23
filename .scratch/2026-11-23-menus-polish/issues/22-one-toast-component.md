# 22: One toast component

Status: ready-for-agent

Blocked by: [02: Approve the mockup](./02-mockup-approval.md)

Spec: [Menus polish](../spec.md).

## What to build

`menu_ui::notify` and `notify_for` (about 30 call sites) draw the approved toast
from the mockup's `/toasts` study instead of today's top-right card sliding in
from the right: the position chosen there for menus and for play, and the menu
sheet's look (its dark fill, 1px border and square corners) with no coloured
edge, so tone shows only in the icon's colour (neutral ⓘ, green ✓, amber ⚠).
Cards in a stack share one width, the widest card's and at least 220 px, with
the text left-aligned after the icon. Text is 14 px in play and 13 px in menus.
A toast fades in with a short rise (fade only when reduced motion is on), at
most three stack, and a repeated message restarts its timer with a ×2 count
instead of adding a card. Errors stay longer, as save failures do today. The
pickup feed and the hotbar's Inventory-full line stay as they are.

## Acceptance criteria

- [ ] Toasts appear where the approved study puts them, in menus and in play,
      covering no HUD element
- [ ] At most three show; the same message again shows ×2 and restarts its
      timer
- [ ] Tone sets only the icon colour; errors stay longer
- [ ] Cards in a stack share one width (the widest, at least 220 px)
- [ ] Every existing `notify` call site still shows its message

## Comments

The user chose the positions from the `/toasts` study on 2026-11-23: bottom-centre above the prompt bar in menus, top-centre under the compass in play.

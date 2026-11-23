# 22: One toast component

Status: ready-for-agent

Blocked by: [02: Approve the mockup](./02-mockup-approval.md), including the
toast study at `/toasts`

Spec: [Menus polish](../spec.md).

## What to build

`menu_ui::notify` and `notify_for` (about 30 call sites) draw the approved toast
from the mockup's `/toasts` study instead of today's top-right card sliding in
from the right: the position chosen there for menus and for play, the panel
look with an icon and left edge by tone, a fade with a short rise (fade only
when reduced motion is on), at most three stacked, and a repeated message
restarting its timer with a ×2 count instead of adding a card. Errors stay
longer, as save failures do today. The pickup feed and the hotbar's
Inventory-full line stay as they are.

## Acceptance criteria

- [ ] Toasts appear where the approved study puts them, in menus and in play,
      covering no HUD element
- [ ] At most three show; the same message again shows ×2 and restarts its
      timer
- [ ] Tone sets the icon and edge colour; errors stay longer
- [ ] Every existing `notify` call site still shows its message

## Comments

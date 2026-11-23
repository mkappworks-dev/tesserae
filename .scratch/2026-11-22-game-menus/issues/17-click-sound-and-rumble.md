# 17: Click sound and controller rumble

Status: done

Blocked by: [02: Menu frame and prompt bar, on the title
screen](./02-menu-frame-and-prompt-bar.md)

Spec: [Menus that feel like a game](../spec.md).

## What to build

Split from 02 to keep that ticket to one context. Every menu press that 02's
press feedback covers also plays a single click sound, and a press made on a
controller rumbles it briefly. The sound and rumble hang off the same press
event as the shrink-and-darken, so no screen wires them by hand.

## Acceptance criteria

- [x] A click sound plays on every press, by key, click or controller
- [x] A controller press also rumbles the controller briefly; a key or mouse
      press does not
- [x] Neither delays the action

## Comments

- The click is synthesised in `client/click.rs` (a 40 ms decaying 1.8 kHz
  tone), so there is no audio file to ship or licence. It plays at the Effects
  volume, ignoring the in-game menu duck that silences World sounds.
- Rumble is 60 ms on the weak motor of `PadInput.active`; it still rumbles when
  the volume is muted.
- Only unit-tested at the `frame::Press` seam; not heard or felt on hardware.

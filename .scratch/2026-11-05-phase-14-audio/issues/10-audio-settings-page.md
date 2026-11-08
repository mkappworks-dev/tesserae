# 10: Audio settings page

Status: done

Blocked by: 04

## What to build

Settings gains the Audio category the menus and Inventory work (#57) left
hidden until this step. It holds four sliders, Master, Effects, Ambient and
Music, from 0 to 100, bound to the volumes from ticket 04. It sits where the
prototype puts it: Behavior, Key bindings, Controller, Display, Audio,
Accessibility.

The mockup is that work's prototype Audio page (Master, Music, Effects) with an
Ambient row added. It is agreed with the user before any implementation.

## Acceptance criteria

- [x] The mockup with the Ambient row is agreed before implementation.
- [x] The sliders go through the same draft, Apply, Discard and Reset as every
      other category; after Apply, playing sounds change at once.
- [x] The Settings sidebar lists Audio between Display and Accessibility, with
      the prototype's speaker icon, and LB/RB step through it in that order.
- [x] F10 can open Settings at Audio as the category used last.
- [x] Pad, keyboard and touch reach every slider, as on the other categories.
- [x] Tests at the Settings drafts seam.

## Comments

- Mockup agreed by the user: the menus prototype's Audio page, changed only as
  follows. The sidebar lists Audio between Display and Accessibility with the
  prototype's `audio` icon, and LB/RB cycle all six categories. Under a "Sound
  mix" label, four sliders from 0 to 100 with these hints: Master volume
  "Everything the World plays."; Effects "Voxels breaking and placed,
  footsteps, splashes, pickups, eating and harm."; Ambient "Loops around you:
  open world, underground, underwater and each Biome."; Music "Tracks now and
  then, with silence between them." Below them a notice: "New volumes reach
  sounds already playing when you apply them." No "Proposed category" pill.
  Restore defaults gives 100, 100, 70 and 50; volumes apply on Apply changes,
  not while dragging.

- Audio uses the shared Scalar controls and draft preferences. Regression coverage checks Apply, Discard, reset defaults, remembered Audio category, six-category order and preserving audio when resetting another page.

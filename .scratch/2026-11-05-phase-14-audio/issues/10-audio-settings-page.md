# 10: Audio settings page

Status: ready-for-agent

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

- [ ] The mockup with the Ambient row is agreed before implementation.
- [ ] The sliders go through the same draft, Apply, Discard and Reset as every
      other category; after Apply, playing sounds change at once.
- [ ] F10 can open Settings at Audio as the category used last.
- [ ] Pad, keyboard and touch reach every slider, as on the other categories.
- [ ] Tests at the Settings drafts seam.

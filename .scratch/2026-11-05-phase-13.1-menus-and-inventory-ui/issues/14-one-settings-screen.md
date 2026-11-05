# 14: One Settings screen

Status: done

Blocked by: 13

Spec: [Phase 13.1](../spec.md).

## What to build

Settings becomes one screen reached from the title screen and from the Menu, with Behavior, Key bindings, Controller (including touch), Display and Accessibility. Today's Movement, Building and Interface settings move into these. Drafts apply on Apply, with Discard and Reset, as today. Accessibility offers text from 100% to 200%, reduced motion (which also stills HUD pulses and slides) and a solid menu background. F10 opens Settings at the category used last. Audio stays hidden until step 14.

## Acceptance criteria

- [x] Every existing setting is reachable in its new category and keeps its saved value.
- [x] Apply, Discard and Reset behave as today across category changes.
- [x] Text scale, reduced motion and solid background take effect in menus and the HUD.
- [x] F10 reopens the last category.

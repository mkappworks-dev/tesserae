# 06: Game mode: Creative and Survival

Status: done

Blocked by: 05

Spec: [Phase 15](../spec.md).

## What to build

Each World has a default game mode, saved as an optional World field (absent means Creative) and mirrored in the sidecar; each Player has their own mode, saved as a Player extension, starting from the default. New Worlds default to Creative; a temporary mode choice is added to today's Create screen (ticket 17 replaces it). In Survival: placing spends one Item from the selected slot; an empty slot or an Item that places nothing is refused; a Layout is refused before any Voxel changes if the Inventory cannot pay for every Voxel; Catalogue takes are refused and C shows a Creative-only note; flight is refused. In Creative placing is free and removing a Voxel drops nothing.

## Acceptance criteria

- [x] Survival placing spends exactly one Item per Voxel; Creative spends none.
- [x] An unaffordable Layout is refused with no Voxel changed.
- [x] Catalogue takes and the switch to flight are refused in Survival.
- [x] A Creative removal spawns no Dropped item; a Survival removal drops as before.
- [x] Both modes survive save and reopen; a save without the field opens as Creative.

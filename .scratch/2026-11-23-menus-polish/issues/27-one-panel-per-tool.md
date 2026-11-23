# 27: One panel per tool

Status: done

Blocked by: None (can start immediately); ticket 26 edits the Biomes step only

Spec: [Menus polish](../spec.md).

## What to build

Quick layout and the place Tool each keep one panel docked on the right, as in
the approved mockup (`/layout`, `/place`, commit 4ec062ea): full size while
choosing, collapsed to a compact summary in the same place while aiming. The
separate Layout-ready HUD card (ticket 11) and the place Tool's HUD card go.

- Keys are the same in both states: L (Quick layout) or K (place Tool) opens
  the panel and toggles full and compact; Enter or Aim collapses it and returns
  the crosshair; Esc leaves the tool from either state, except that with a
  structure preview pinned the first Esc only cancels the pin; RMB places.
  Backspace stays as an alternative to Esc.
- The compact panel shows the tool name, what is selected (layout shape, size
  and count, or blueprint, facing and Voxel count), the placement status and
  the key caps (L or K Edit, Esc Exit, RMB Place), read from the bindings.
- After a blueprint is built whole, it is unselected and the place Tool panel
  opens full size with "Pick the next Template"; Enter and K do nothing until
  one is picked. Quick layout keeps its layout after building.
- Quick layout and the place Tool exclude each other: L does nothing with the
  place Tool in hand, K does nothing with Quick layout on. The minimap hides
  while either panel shows.

## Acceptance criteria

- [x] Each tool shows one panel, full while choosing and compact while aiming,
      in the same place
- [x] L/K toggle, Enter aims, Esc leaves from either state (a pinned preview
      first), RMB places
- [x] Building a blueprint unselects it and reopens the full panel
- [x] The two tools exclude each other

## Comments

The Server tells the placing Player when a Structure lands whole (`StructureBuilt`, beside the `EditRejected` refusal); only then does the Client unselect the Template and reopen the full panel, so a refused build keeps the Template chosen for another try. The mockup's Outside / Inside / Placing view switch is left out: in the game the ghost stands in the World. The tint legend and the Authored / This World totals of the old list window go, as the mockup dropped them. The hotbar still hides behind the full panel, as Quick layout's did, because the prompt bar takes the bottom edge where the mockup lifts the hotbar above it. Pinned, the place Tool's caps are Use Build, Esc Cancel and K Edit; G (Use) builds, as before.

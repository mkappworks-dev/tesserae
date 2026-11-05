# 13: In-game Menu

Status: ready-for-agent

Blocked by: 03

Spec: [Phase 13.1](../spec.md).

## What to build

Tab or Escape opens the in-game Menu over the World: Resume, Inventory, Settings, World details (Game, Voxel size, seed, Biomes, both game modes) and Return to Worlds, which uses the existing save-and-quit. It replaces the menu that combined the Catalogue and settings. Escape closes an open overlay (Inventory, Layout menu, dialog) before it opens the Menu, and closes the Menu to resume. Inside the Menu, Tab moves focus. The look follows the prototype's Menu.

## Acceptance criteria

- [ ] Tab and Escape open the Menu from play; Escape from an overlay closes only the overlay.
- [ ] Each Menu entry reaches its destination; Return to Worlds saves first.
- [ ] World details shows the World's facts as the Server reports them.

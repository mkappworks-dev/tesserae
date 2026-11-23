# 05: World step and Edit World match the mockup

Status: done

Blocked by: None (can start immediately)

Spec: [Menus polish](../spec.md).

## What to build

Game mode becomes one Creative | Survival segmented control with one line describing the chosen mode, Voxel size one selector, and the name hint, per-mode descriptions and seed echo go, so the World step fits without scrolling. Edit World uses the same Game mode control.

## Acceptance criteria

- [x] The World step shows every field including Voxel size and Climate scale without scrolling at 1280×720
- [x] Left and Right change Game mode and Voxel size
- [x] Edit World's Game mode matches the World step's

## Comments

Climate scale sits under the terrain preview, not in the fields column as in the mockup: there it is below the fold, and the criterion is no scrolling at 1280×720. The Game mode line now matches the mockup: Creative placement spends Items, as the Server does (ee8786c8).

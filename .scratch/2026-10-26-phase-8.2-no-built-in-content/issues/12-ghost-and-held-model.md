# 12: A ghost where the held item will land, and the held item in a corner

Status: done
Blocked by: 08, 11

Spec: [decision 16](../spec.md).

## What to build

Shape geometry reaches the Client as read-only content in the catalogue,
beside the Shape names. The Client meshes the single held Voxel with the core
mesher and draws it twice: see-through, in its placeholder colour, at the aim
target's ghost cell; and small, in a corner of the view. A connected Shape is
shown unconnected, a fluid as a cube of its colour, and fire as the flame
colour with no ghost. Both are hidden while the menu is open and update when
the held item, its Shape or its orientation change.

## Acceptance criteria

- [ ] The ghost appears at the ghost cell in the held Shape and orientation,
      and not when the target has no ghost cell.
- [ ] Turning, flipping or cycling the Shape updates both the ghost and the
      corner model.
- [ ] Both are hidden while the menu is open.
- [ ] The catalogue's Shape geometry is content only: no Voxel, Chunk or
      Occupants.
- [ ] Captures of a stair ghost in two orientations and of the corner model
      are in the `/tmp` review folder.
- [ ] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.

## Comments

### 2026-10-26: implementation

Catalogue Shapes now carry read-only quads for each orientation, with
connected Shapes represented without arms. The core mesher builds an
isolated unit Shape from those quads. The Client reuses its mesh for a
see-through ghost at the Server's accepted cell and a small corner model on
an isolated camera layer. Fluids and fire use cube geometry and their
placeholder colours; fire never shows a ghost. Models follow held-item,
Shape and orientation changes and hide in the menu. Old asset handles are
released when replacing a model.

Added headless model/position/orientation/menu visibility coverage. Final
formatting, review, CI, captures and benchmarks are ticket 13.

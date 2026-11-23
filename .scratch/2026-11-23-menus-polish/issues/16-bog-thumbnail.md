# 16: Bog thumbnail with textured mud and tar

Status: done

Blocked by: None (can start immediately)

Spec: [Menus polish](../spec.md).

## What to build

`scripts/generate-thumbnails` draws Bog's mud and tar blocks with a generated texture (noise in the Material's own colours), so they match the textured blocks in the other thumbnails instead of being flat colour.

## Acceptance criteria

- [x] Bog's thumbnail shows textured mud and tar
- [x] The thumbnail still validates (512×288 PNG)

## Comments

The noise is a new `("noise", RGBA, seed)` item kind in `MODS`; Waters keeps its plain water Voxel, as this ticket names Bog only.

# 15: Distinct material textures for content previews

Status: ready-for-agent
Blocked by: 14

User follow-up on 2026-10-26 for PR #21.

## What to build

Replace flat material previews with distinct voxel-style textures for all
demo Materials: lamp, lava, wood, ash, mud, brick, water, stone, dirt, grass,
glass and leaves. Fire uses an engine flame preview because it is an engine
kind, not a named Material. Textures appear in tiles, drag feedback, the
held model and the placement ghost.

Texture definitions stay Game content. Amend the unreleased 0.3 appearance
record with optional bounded RGBA preview data, supplied by the demo.
Older contract lines provide no preview texture and retain their colour
fallback. The engine draws generic texture data without naming Materials.
This adds preview appearance only; placed terrain still uses its existing
palette and the Look-pack work stays separate.

## Acceptance criteria

- [ ] Every demo Material has a recognisable textured preview.
- [ ] Grass top/sides and wood end grain/bark differ by face.
- [ ] Fire reads as flame, and fluid previews retain their source-cube form.
- [ ] Invalid preview payloads fail load before reaching GPU allocation.
- [ ] The Client reads content only; the engine declares no named Material.
- [ ] Guests/demo rebuild, relevant tests, formatting and full CI pass.
- [ ] Final captures are reviewed and removed afterwards.

## Comments

The user explicitly requested richer previews for lava, water, leaves,
grass, wood, lamp, fire, mud and the other Materials on 2026-10-26. This
supersedes flat placeholder appearance for previews while retaining Game
ownership of Material appearance.

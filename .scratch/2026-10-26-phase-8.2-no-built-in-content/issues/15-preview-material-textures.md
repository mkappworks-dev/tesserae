# 15: Distinct material textures for content previews

Status: ready-for-human
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

- [x] Every demo Material has a recognisable textured preview.
- [x] Grass top/sides and wood end grain/bark differ by face.
- [x] Fire reads as flame, and fluid previews retain their source-cube form.
- [x] Invalid preview payloads fail load before reaching GPU allocation.
- [x] The Client reads content only; the engine declares no named Material.
- [x] Guests/demo rebuild, relevant tests, formatting and full CI pass.
- [ ] Final captures are reviewed and removed afterwards.

## Comments

The user explicitly requested richer previews for lava, water, leaves,
grass, wood, lamp, fire, mud and the other Materials on 2026-10-26. This
supersedes flat placeholder appearance for previews while retaining Game
ownership of Material appearance.

### 2026-10-26: implementation and review

Every demo Material supplies deterministic 16-pixel artwork across top,
side and bottom faces. Grass has soil sides and a green cap; wood has bark
and end grain; water, lava and mud have distinct surfaces. Stone, dirt,
brick, ash, lamp, glass and leaves have their own patterns. Engine Fire
uses crossed planes with a masked flame image. Nearest sampling preserves
pixel edges, and face UVs stay inside their tile. One shared material path
serves the model atlas, drag follower, held model and translucent ghost.

The host validates all preview lengths and the 64-row appearance bound
before passing ownership to the Server; it no longer clones appearance
before validation. The Server publishes pixels under resolved Material
ids. The Client's texture cache releases replaced assets; missing pixels
retain the existing colour fallback. Frozen 0.1 and 0.2 contracts remain
unchanged, and the adapter supplies no texture. ADR-0016 and the spec
record the preview-only amendment to unreleased 0.3; there is no 0.4.

Relevant tests cover invalid/absent/valid texture payloads, face UVs, flame
geometry/alpha, texture asset replacement, colour fallback, ghost opacity,
and all twelve demo textures crossing the Game/Server/Client boundary with
their registered ids. Full CI passes: 885 tests (391 app, 327 core,
69 meshing integration, 98 Mods), zero failures and five existing manual
measurements ignored. Guests and demo rebuilt. Formatting, Clippy,
rustdoc, default build, bench compilation, boundaries and cargo-deny pass.
Review also fixed the atlas root's missing visibility state and disabled
ineffective dithering on its untonemapped camera.

Five PNGs are retained in /tmp/tesserae-p82-previews-review-wbb76ry6:
all Materials, water dragged over slot 5, water replacing that slot's lamp,
a held grass stair with its ghost, and the refined Material menu. The
user accepted the other textures
and drag/drop feedback, then requested smoother water, clearer glass and
flowing lava. Those three are revised; their review is pending. The folder
must not be deleted until accepted. The capture driver triggers actual UI
observers and does not automate physical OS pointer input. Existing
headless tests exercise movement, UI scaling, slot swaps and cancellation.

Exceptions: the five ignored tests are manual measurement harnesses. Core
algorithms did not change in these follow-ups, so ticket 13's interleaved
benchmark evidence remains applicable. Placed terrain uses its palette;
these textures are preview appearance, as specified above. Bevy/winit
logs a skipped Destroyed event during screenshot-app shutdown after the
PNG saves; the atlas visibility and dithering warnings are resolved.

### 2026-10-26: water, glass and lava refinement

The user accepted the other previews and drag/drop captures, requesting
smoother water, clearer glass and flowing lava. Water now has soft, sparse
ripples; lava has curved hot ribbons along its flow; glass has translucent
faces, thin edges and a small reflection. Preview materials derive opaque,
masked or blended rendering from the supplied alpha bytes, keeping this
choice generic and Game-owned. Cache tests cover all three alpha cases.

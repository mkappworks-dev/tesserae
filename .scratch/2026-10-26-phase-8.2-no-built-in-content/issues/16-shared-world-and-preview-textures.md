# 16: One Game texture for previews and placed Voxels

Status: done
Blocked by: 14

User follow-up on 2026-10-26 for PR #21. Ticket 15's implementation is
complete; its retained captures are superseded by this combined review.

## What to build

Use each Material's Game-supplied texture in the World as well as in
previews. Replace the preview-only appearance field with one optional
texture on the unreleased 0.3 contract. Keep colour as the fallback for
Materials with no texture, including older contract lines. No second Mod,
second authored texture or named engine Material is introduced.

The Client shares one texture atlas across terrain, menu, hotbar, drags,
held models and ghosts. World shading retains lighting, fog, declared
render classes, fluid transparency and LOD clipping. Greedy faces repeat
per Voxel; Shape faces use the same top/side/bottom mapping as previews.
Depth and shadow alpha tests match visible cutout textures. Fire keeps
engine-owned drawing and shares its flame artwork with the World.

## Acceptance criteria

- [x] The demo authors one texture per Material; previews and World share it.
- [x] Placed solids, fluids, glass and leaves use their declared textures.
- [x] Merged faces repeat; grass caps and wood end grain stay face-specific.
- [x] Cutout depth and shadow passes match visible texture alpha.
- [x] Missing textures retain the palette; 0.1 and 0.2 stay frozen.
- [x] Rendering changes neither World state nor core meshing/bench inputs.
- [x] Guests rebuild; relevant tests, formatting and full CI pass.
- [x] Native captures are retained until user review.

## Comments

The user noticed that placing a textured preview still produced a flat
Voxel and requested matching World textures. They also asked whether the
appearance belonged to the engine or Mod and whether separate definitions
were required. This explicitly extends ticket 15's preview-only scope:
the Game owns one appearance texture; the engine owns generic rendering.
The future Look-pack API stays separate.

### 2026-10-27: visual refinements

The user accepted water's pattern, asked for more transparent baby-blue
glass and a more consistent lava surface, and requested a textured
placement ghost with reduced opacity. Glass keeps faint clear faces and
soft blue edges; lava uses an even orange palette with gentle flow bands.
The ghost uses the shared texture at 40 percent opacity with back-face
culling so rear faces do not wash out its visible artwork.

The transparent icon check exposed double application of alpha when a
render-target image was shown as an ordinary UI image. A cached UI material
now uses premultiplied blending; tile and drag previews keep their intended
opacity without dark edges. The existing atlas lifecycle test also checks
that replaced UI materials are released.

### 2026-10-27: implementation and validation

The unreleased 0.3 appearance field is now one optional `texture` rather
than `preview`. The Game supplies its RGBA artwork once. The Client uploads
one shared 48 by 1040 atlas for 64 Material slots and the engine flame;
World surfaces and all item models reference the same Image handle. The
palette remains the fallback. Old contract lines supply no texture and
are unchanged. Texture byte/row bounds remain validated before GPU use.

The World shader selects top, side or bottom artwork by face, repeats it
per Voxel across merged faces, and retains PBR lighting, fog, transparency
and LOD clipping. Forward, depth and shadow passes share the vertex data
and texture sampling; texture alpha replaces the fallback cutout pattern
when artwork exists. Mesh vertex storage and every core algorithm remain
unchanged. Fire uses the same engine flame artwork in both contexts.

A headless integration test proves all three terrain materials and item
models share one Image, including the last legal Material id and removal
of texture rows. Existing tests cover fallback, texture bounds, atlas
replacement, Shape orientation, asset cleanup, drag movement and slot
updates. The ghost explicitly shares the texture at 40 percent opacity.
Native Metal captures compile and exercise the World, shadow and UI
pipelines. The UI atlas uses matching premultiplied blending so clear
pixels are not darkened twice. No shader or atlas warnings remain.

cargo fmt --all, the standalone demo formatter, npm run fmt and full
./scripts/cargo-ci pass: 886 tests (392 app, 327 core, 69 meshing
integration, 98 Mods), zero failures and five existing ignored manual
measurements. Clippy, rustdoc, default-feature build, guest/demo rebuild,
benchmark compilation, crate boundaries and cargo-deny pass. The first
full run caught the old glass fallback colour in the demo contract test;
its expectation now matches the requested baby blue. No failures remain.

Current review captures remain in
/tmp/tesserae-p82-previews-review-wbb76ry6: shared-materials-refined.png,
shared-glass-world.png, shared-lava-world.png and
shared-brick-placement-ghost.png. Additional captures check the gallery,
fire/wood and a transparent glass drag. They supersede the preview-only
review; final user review is pending and the folder is retained.

Exceptions: the existing five ignored tests are measurement harnesses.
Captures use native Metal, not every GPU backend. Core and benchmark
inputs did not change, so ticket 13's interleaved CPU results stand; no
new GPU performance claim is made. Bevy/winit logs its existing skipped
Destroyed event after the capture saves and exits. Capture drivers inject
UI observers; physical OS pointer input is not automated.

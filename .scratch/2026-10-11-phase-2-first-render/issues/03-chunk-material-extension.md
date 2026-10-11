# A `MaterialExtension` that colours a Voxel by its Material

Status: done
Blocked by: 02

## Task

A `MaterialExtension` over `StandardMaterial` that selects a colour from the
per-vertex Material index. A tiny palette of flat colours is enough —
`testgen` uses STONE=1, DIRT=2, GRASS=3. Texturing is step 9.

Extending `StandardMaterial` rather than writing a bare `Material` is the
decision that buys shadows, clustered lighting, fog and tonemapping for free;
see the "Bevy depth" row in [`DESIGN.md`](../../../DESIGN.md).

## Done when

Nine Chunks render with Stone, Dirt and Grass visibly distinct, lit and
shadowed by `bevy_pbr`.

## Notes

`bevy_pbr`'s `VertexOutput` is a fixed struct, so an extra per-vertex
attribute cannot be forwarded through pbr's own vertex entry point: the
extension needs a **vertex** shader as well as a fragment one. Both are WESL
(`.wesl`), where `@if(COND)` replaces `#ifdef` and a bind group is written
`@group(constants::MATERIAL_BIND_GROUP)`. The `constants` package is
synthesised from the pipeline's `ShaderDefVal`s, so the palette length can be
injected the same way rather than duplicated between Rust and WESL.

`Material::specialize` must rebuild `descriptor.vertex.buffers` to include the
custom attribute's shader location.

## Comments

Colours are picked in sRGB and converted: `base_color` is linear in the shader,
so an sRGB triple written straight into it renders several stops too bright.

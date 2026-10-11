# The vertex attribute and the shader

Status: done
Blocked by: 04

What the Client does with the baked light, and the end of the flat ambient
fill. Spec decision 5.

## What to build

`client/chunk_mesh.rs`:

- `ATTRIBUTE_LIGHT`, `VertexFormat::Uint32`, a fresh high attribute id beside
  `ATTRIBUTE_MATERIAL`'s. One `u32` per vertex carrying `Light`'s packed byte.
  Packing it into `ATTRIBUTE_MATERIAL`'s spare bits would save four bytes a
  vertex, but those bits are reserved for Shape in step 5 and four bytes is
  noise against the 32 in position, normal and UV.
- Insert it in `chunk_mesh_to_mesh`. Unscaled: a light level is dimensionless
  and `voxel_size` must not touch it.

`render/chunk_material.rs`: add it to `specialize`'s spelled-out vertex layout
at shader location 11. The base material's layout stops at `bevy_pbr`'s
attributes, so an attribute missing from that list is read as garbage.

`render/chunk_material.wesl`: carry it through `ChunkVertex` and
`ChunkVertexOutput` at location 11, `@interpolate(flat)` like the Material
index, then in the fragment stage:

| Channel     | `PbrInput` field                                    |
| ----------- | --------------------------------------------------- |
| Sky light   | `diffuse_occlusion` and `directional_shadow_factor` |
| Block light | `material.emissive`                                 |

Read from `PbrInput` rather than multiplied into `base_color` the way Minecraft
does it, so albedo stays albedo and the BRDF keeps working. `diffuse_occlusion`
multiplies the ambient dome in `bevy_pbr`'s `pbr_ambient.wesl`;
`directional_shadow_factor`'s own doc in `pbr_types.wesl` says "use this for
custom shadow maps", which is what a sky-light flood-fill is. Both, not one:
the shadow map alone stops working past the last cascade, and the ambient dome
alone leaves the sun leaking into a roofed space.

Block light goes to `emissive` tinted by the Voxel's palette colour, so a face
lit by a lantern still shows its Material. `lightmap_light` is the closer
semantic fit but needs the `LIGHTMAP` shader def and a second UV set.

The 0-15 to radiance curve is a WESL const, not a uniform. Step 9's look-pack
API is where a pack gets to choose it.

`client/chunk_view.rs`: `spawn_light`'s `ambient.brightness = 900.0` stops
being a flat fill. Keep a `GlobalAmbientLight` as the sky dome's radiance and
let the sky channel gate it per face; rewrite the comment, which currently
promises this step will delete it.

## Tests

- `every_attribute_the_chunk_material_reads_is_present` covers the new one.
- Light survives as `Uint32`, not as a float - the same trap
  `material_indices_survive_as_uint32` guards.
- `voxel_size` does not scale it.

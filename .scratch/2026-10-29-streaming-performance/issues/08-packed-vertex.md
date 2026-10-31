# 08: Pack the Chunk vertex

Status: ready-for-agent
Blocked by: 01

Spec: [streaming performance](../spec.md).
PR boundary: own branch `tech/streaming-packed-vertex`.

## What to build

Today a vertex is 40 B: position and normal `f32x3`, UV `f32x2`, material and
light `u32`.

- Pack it to 16 B, decoded in `chunk_vertex.wesl` and the prepass. Normals are
  an index into a table built from the `ShapeTable`'s face normals.
- Positions must represent every Shape vertex and fluid level exactly. If 16 B
  cannot, use 20 B; do not trade exactness. Leave room for positions local to
  a region of Chunks, which 09 needs.
- Each mesh carries an explicit `Aabb`, since Bevy cannot compute one from a
  packed position.
- Not per-quad packing (Voxy's 8 B a quad, pulled by the vertex shader): it
  leaves Bevy's standard mesh path for the prepass, shadows and batching. It
  becomes a follow-up ticket only if, after this one, upload or vertex cost
  still dominates in 01's attribution.

## Acceptance criteria

- [ ] A test decodes every Shape and fluid level to today's positions, normals
      and UVs exactly.
- [ ] Main-thread mesh upload time or GPU vertex time passes the spec's effect
      gate.
- [ ] Frustum culling still culls: 01's drawn-Chunk counts match the baseline.

## Validation and authorization

frame-profiling.md. Local work only; no push or PR without approval.

## Comments

### 2026-10-31: implemented; gate pending the final review series

Layout, 16 B, exact: `Sint16x4` position in sixteenths of a Voxel (the
`SUBDIV` grid every Shape, fluid surface and flame lies on) with the face index
in `w`; `Uint16x2` UV in sixteenths; `Uint32` Material in the low 24 bits (it
needs 17, for `FLAME`) and light in the high 8. Every Shape face is one of the
six axis faces, so the "normal table" is the face index and the shader rebuilds
the normal arithmetically (`normal[face / 2] = ±1`), as the mesher does. The
mesh is in Voxels; the entity's `Transform` scales it by the level's Voxel size,
which leaves ±2047 Voxels of range for 09's region-local positions. Each mesh
sets `Mesh::final_aabb`, which Bevy's `calculate_bounds` reads. Camera
collision (`client/view.rs`) decodes the packed mesh; the item preview builds
its own float mesh.

Tests: every Shape in every orientation, and a Chunk with fluid at all 16
surface levels standing and in every Shape's voids, flames outside both far
corners and a floor whose UVs span the Chunk, decode to the mesher's own
positions, normals, UVs, Material and light exactly; the `Aabb` is exactly the
decoded bounds.

Review captures (`/tmp/claude-streaming-impl/review08`, stationary and lifted
90 m at -5 deg) against `bin-int2`: 4 and 8 pixels differ by more than 24/255,
rule-hole frames 0, drawn Chunks identical (`visible_meshes_camera0` median 287
stationary, 274 lifted), so frustum culling still culls.

Frame A/B, `ab08`: `bin-int2` against `bin-08-7cd09f8`, all 8 fixtures, 3
interleaved rounds. Frame p50, ms (median of run medians, range):

| fixture       | int2              | packed            | change |
| ------------- | ----------------- | ----------------- | ------ |
| stationary    | 5.21 [5.19, 5.32] | 5.02 [5.01, 5.12] | -3.6%  |
| sprint-flight | 5.34 [5.32, 5.41] | 5.21 [5.13, 5.25] | -2.4%  |
| teleport      | 5.28 [5.27, 5.28] | 5.11 [5.10, 5.12] | -3.2%  |
| active        | 5.38 [5.37, 5.39] | 5.24 [5.24, 5.37] | -2.6%  |
| dive          | 3.92 [3.91, 4.07] | 3.80 [3.80, 3.96] | -3.1%  |
| flight        | 5.70 [5.63, 5.71] | 5.43 [5.42, 5.46] | -4.7%  |
| sprint-walk   | 5.18 [5.14, 5.21] | 5.00 [5.00, 5.01] | -3.5%  |
| transparency  | 5.88 [5.84, 5.88] | 5.65 [5.63, 5.69] | -3.9%  |

p99 equal or better everywhere; rule-hole and fill frames 0 for both; teleport
settle unchanged (0.99-1.12 s against 1.03-1.11 s). Main-thread upload is
unchanged: `upload_wall_ms` per upload 10-18 us for both, since packing three
attributes costs what cloning five did.

GPU series, `ab08-gpu` (`TESSERAE_PROFILE_GPU=1`, same binaries, 8 fixtures, 3
rounds): GPU frame (`frame_end`) median -1.0% to -3.5% per fixture; camera 0's
`main_start` to `opaque_end` 0.13-0.19 ms down to 0.03 ms on every fixture, its
whole view -4 to -5%; camera 1's spans within noise. Markers sit at encoder
boundaries, so this attributes direction, not size.

Every fixture improves, none by the 5% the effect gate asks. The gate is
decided by the combined interleaved review series that runs once every ticket
is implemented; status stays `ready-for-agent` until then.

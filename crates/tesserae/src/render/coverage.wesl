// The coverage rule: whether a finer level draws where a coarser face lies, so
// the coarser face must not. Shared by the colour pass and the camera's prepass,
// which must cut the same fragments or the depth they leave disagrees.

// Mirrors client::drawn_mask, which documents the layout.
@group(constants::MATERIAL_BIND_GROUP) @binding(102)
var<storage, read> drawn: array<u32>;

// How far behind a face its own Voxel is sampled, as a fraction of the finer
// Cell: a thirty-second of one of its Voxels, far enough to clear the float
// error in a world-space coordinate and far short of leaving the Voxel.
const BEHIND_A_FACE: f32 = 1.0 / 1024.0;

// `drawn_mask::SOME` and `drawn_mask::ALL`, which a test holds equal.
const SOME: u32 = 1u;
const ALL: u32 = 2u;

// The cover bits of the level-`level` Chunk holding the face at `point` facing
// `normal`: `SOME`, its own level or a finer one draws part of it; `ALL`, all
// of it.
// The Chunk asked about is the one behind the face, because a face lies on the
// boundary of the Voxel it is the surface of: water levelled onto a Chunk
// boundary by `level_the_water` would otherwise ask the Chunk above it, which
// holds no water at all.
fn cover_at(level: u32, point: vec3<f32>, normal: vec3<f32>) -> u32 {
    let slot = 1u + 8u * level;
    let size = vec3<u32>(drawn[slot + 4u], drawn[slot + 5u], drawn[slot + 6u]);
    if min(size.x, min(size.y, size.z)) == 0u {
        return 0u;
    }
    let origin = vec3<i32>(
        bitcast<i32>(drawn[slot + 1u]),
        bitcast<i32>(drawn[slot + 2u]),
        bitcast<i32>(drawn[slot + 3u]),
    );
    let cell_m = bitcast<f32>(drawn[slot + 7u]);
    let behind = point - normalize(normal) * cell_m * BEHIND_A_FACE;
    let local = vec3<i32>(floor(behind / cell_m)) - origin;
    if any(local < vec3<i32>(0)) || any(vec3<u32>(local) >= size) {
        return 0u;
    }
    let l = vec3<u32>(local);
    let i = (l.z * size.y + l.y) * size.x + l.x;
    return (drawn[drawn[slot] + i / 16u] >> ((i % 16u) * 2u)) & (SOME | ALL);
}

// Whether a level finer than `level` draws the Chunk at `point`, which then
// draws it instead. Usually one lookup: only a partly covered Cell sends the
// walk a level further down.
fn finer_drawn(level: u32, point: vec3<f32>, normal: vec3<f32>) -> bool {
    var finer = min(level, drawn[0]);
    while finer > 0u {
        finer -= 1u;
        let cover = cover_at(finer, point, normal);
        if cover == 0u {
            return false;
        }
        if (cover & ALL) != 0u {
            return true;
        }
    }
    return false;
}

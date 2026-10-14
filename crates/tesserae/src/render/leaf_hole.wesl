// Holes in a cutout face, four to a Voxel's edge, about a third of them open.
// On the face's UV, which counts the Voxels of its own level, so a coarse leaf
// has coarse holes and the pattern does not swim. Step 9's textures replace it.

// PCG's output permutation: neighbouring inputs land far apart, so the holes do
// not line up in rows the way a sum of multiplied coordinates does.
fn pcg(v: u32) -> u32 {
    let state = v * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn leaf_hole(uv: vec2<f32>) -> bool {
    let cell = vec2<u32>(vec2<i32>(floor(uv * 4.0)) + vec2<i32>(1 << 20));
    return pcg(cell.x ^ pcg(cell.y)) % 1024u < 360u;
}

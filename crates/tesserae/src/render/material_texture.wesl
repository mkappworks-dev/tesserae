// One layer per distinct Material face, each with its own mips.
@group(constants::MATERIAL_BIND_GROUP) @binding(103)
var artwork: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(104)
var artwork_sampler: sampler;
// Per slot, its top, side and bottom layers, as texture_atlas::face_word packs them.
@group(constants::MATERIAL_BIND_GROUP) @binding(105)
var<uniform> face_words: array<vec4<u32>, (constants::CHUNK_PALETTE_SLOTS + 3u) / 4u>;

const TEXTURED: u32 = 1u << 31u;

// A face UV's screen-space derivatives. Taken at the top of a fragment stage,
// before any discard or per-slot branch: derivatives need the whole 2x2 quad.
struct UvGradient {
    ddx: vec2<f32>,
    ddy: vec2<f32>,
}

fn uv_gradient(uv: vec2<f32>) -> UvGradient {
    return UvGradient(dpdx(uv), dpdy(uv));
}

fn material_slot(material: u32) -> u32 {
    return select(
        min(material, constants::CHUNK_PALETTE_LEN - 1u),
        constants::CHUNK_PALETTE_LEN,
        material == constants::CHUNK_FLAME,
    );
}

fn face_word(slot: u32) -> u32 {
    return face_words[slot / 4u][slot % 4u];
}

fn has_texture(slot: u32) -> bool {
    return (face_word(slot) & TEXTURED) != 0u;
}

fn sample_texture(slot: u32, uv: vec2<f32>, gradient: UvGradient, normal: vec3<f32>) -> vec4<f32> {
    var face = 1u;
    // Turns the face's UV upright on its texture: (u, -v).
    var axes = mat2x2<f32>(1.0, 0.0, 0.0, -1.0);
    if abs(normal.y) > 0.5 {
        face = select(2u, 0u, normal.y > 0.0);
        axes = mat2x2<f32>(0.0, 1.0, 1.0, 0.0);
    } else if abs(normal.x) > 0.5 {
        axes = mat2x2<f32>(0.0, -1.0, 1.0, 0.0);
    }
    if slot == constants::CHUNK_PALETTE_LEN {
        face = 1u;
    }
    // Unwrapped: the repeat sampler tiles a merged quad once per Voxel, and the
    // derivatives stay continuous across its Voxel edges, where fract's jump
    // would pick the smallest mip in a seam.
    let coord = axes * uv;
    let ddx = axes * gradient.ddx;
    let ddy = axes * gradient.ddy;

    // Magnified, linear filtering would blur the pixel art. Per texture axis,
    // where a pixel covers less than a texel, hold the coordinate at the texel's
    // centre, which a linear sample returns unblended, and cross each texel
    // edge within one pixel. Minified axes pass through to the mips and
    // anisotropy untouched.
    let edge = f32(constants::CHUNK_TEXTURE_EDGE);
    let texel = coord * edge;
    let per_pixel = min((abs(ddx) + abs(ddy)) * edge, vec2<f32>(1.0));
    let seam = floor(texel + 0.5);
    let crisp = seam + clamp((texel - seam) / max(per_pixel, vec2<f32>(1e-6)), vec2<f32>(-0.5), vec2<f32>(0.5));

    let bits: u32 = constants::CHUNK_FACE_LAYER_BITS;
    let layer = (face_word(slot) >> (face * bits)) & ((1u << bits) - 1u);
    return textureSampleGrad(artwork, artwork_sampler, crisp / edge, layer, ddx, ddy);
}

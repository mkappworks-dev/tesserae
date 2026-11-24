// One layer per distinct Material face, each with its own mips.
@group(constants::MATERIAL_BIND_GROUP) @binding(103)
var artwork: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(104)
var artwork_sampler: sampler;
// Per slot, its top, side and bottom layers, as texture_atlas::face_word packs them.
@group(constants::MATERIAL_BIND_GROUP) @binding(105)
var<uniform> face_words: array<vec4<u32>, (constants::CHUNK_PALETTE_SLOTS + 3u) / 4u>;

// A face word's top two bits, as texture_atlas::face_word sets them: zero is
// untextured, and bit 30 turns the tile per Voxel.
const LOOK_SHIFT: u32 = 30u;
const TURNED: u32 = 1u << 30u;
const TURNED_AND_PATCHED: u32 = 1u;
const LAYER_MASK: u32 = (1u << constants::CHUNK_FACE_LAYER_BITS) - 1u;

// About one tree crown: the lattice of the patches' value noise, in metres.
const PATCH_M: f32 = 5.0;
const PATCH_VALUE: f32 = 0.08;
const PATCH_HUE_RADIANS: f32 = 0.07;

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
    return (face_word(slot) >> LOOK_SHIFT) != 0u;
}

// 0 top, 1 side, 2 bottom; the flame's quads always draw its side.
fn face_of(slot: u32, normal: vec3<f32>) -> u32 {
    if slot == constants::CHUNK_PALETTE_LEN || abs(normal.y) <= 0.5 {
        return 1u;
    }
    return select(2u, 0u, normal.y > 0.0);
}

fn face_layer(word: u32, face: u32) -> u32 {
    return (word >> (face * constants::CHUNK_FACE_LAYER_BITS)) & LAYER_MASK;
}

// A face drawing a tile of its own, as grass's flank does, has an up; one
// drawing the top's tile has none.
fn draws_the_top(word: u32, face: u32) -> bool {
    return face_layer(word, face) == face_layer(word, 0u);
}

fn hash(p: vec3<i32>) -> u32 {
    let k = bitcast<vec3<u32>>(p) * vec3<u32>(0x8da6b343u, 0xd8163841u, 0xcb1ab31fu);
    var h = k.x ^ k.y ^ k.z;
    h ^= h >> 16u;
    h *= 0x7feb352du;
    h ^= h >> 15u;
    h *= 0x846ca68bu;
    h ^= h >> 16u;
    return h;
}

// One of the square's eight symmetries, as a swap of the axes then a sign per
// axis, so it applies to the UV and its derivatives alike. A face with an up
// only mirrors across it.
struct Turn {
    swap: bool,
    sign: vec2<f32>,
}

fn turn_for(cell: vec3<f32>, normal: vec3<f32>, free: bool) -> Turn {
    // The Voxel behind the face, whatever the quad it was merged into.
    let bits = hash(vec3<i32>(floor(cell - normal * 0.5)));
    let flip = vec2<bool>((bits & 1u) != 0u, (bits & 2u) != 0u && free);
    return Turn((bits & 4u) != 0u && free, select(vec2<f32>(1.0), vec2<f32>(-1.0), flip));
}

fn turned(turn: Turn, v: vec2<f32>) -> vec2<f32> {
    return select(v, v.yx, turn.swap) * turn.sign;
}

// Whether `patch_colour` shades this face: foliage, and grass's top but not its flank.
fn patched(slot: u32, normal: vec3<f32>) -> bool {
    let word = face_word(slot);
    return (word >> LOOK_SHIFT) == TURNED_AND_PATCHED && draws_the_top(word, face_of(slot, normal));
}

// Value noise on a lattice in metres, two channels from one hash, in -1..1.
fn patch_noise(world: vec3<f32>) -> vec2<f32> {
    let at = world / PATCH_M;
    let base = vec3<i32>(floor(at));
    let f = fract(at);
    let w = f * f * (3.0 - 2.0 * f);
    var sum = vec2<f32>(0.0);
    for (var corner = 0u; corner < 8u; corner++) {
        let o = vec3<u32>(corner & 1u, (corner >> 1u) & 1u, corner >> 2u);
        let h = hash(base + vec3<i32>(o));
        let value = vec2<f32>(f32(h & 0xffffu), f32(h >> 16u)) / 65535.0;
        let weight = select(1.0 - w, w, vec3<bool>(o));
        sum += value * weight.x * weight.y * weight.z;
    }
    return sum * 2.0 - 1.0;
}

// Crown-sized shifts of value and hue, the same at any Voxel size. Hue turns
// about the grey axis, so a grey stays grey.
fn patch_colour(rgb: vec3<f32>, world: vec3<f32>) -> vec3<f32> {
    let n = patch_noise(world);
    let angle = n.y * PATCH_HUE_RADIANS;
    let k = vec3<f32>(0.57735027);
    let c = cos(angle);
    let hued = rgb * c + cross(k, rgb) * sin(angle) + k * dot(k, rgb) * (1.0 - c);
    return max(hued, vec3<f32>(0.0)) * (1.0 + n.x * PATCH_VALUE);
}

// `cell` is the fragment's world position in its level's cells.
fn sample_texture(slot: u32, uv: vec2<f32>, gradient: UvGradient, normal: vec3<f32>, cell: vec3<f32>) -> vec4<f32> {
    let face = face_of(slot, normal);
    // Turns the face's UV upright on its texture: (u, -v).
    var axes = mat2x2<f32>(1.0, 0.0, 0.0, -1.0);
    if abs(normal.y) > 0.5 {
        axes = mat2x2<f32>(0.0, 1.0, 1.0, 0.0);
    } else if abs(normal.x) > 0.5 {
        axes = mat2x2<f32>(0.0, -1.0, 1.0, 0.0);
    }
    // Unwrapped: the repeat sampler tiles a merged quad once per Voxel, and the
    // derivatives stay continuous across its Voxel edges, where fract's jump
    // would pick the smallest mip in a seam.
    var coord = axes * uv;
    var ddx = axes * gradient.ddx;
    var ddy = axes * gradient.ddy;

    let word = face_word(slot);
    // The derivatives turn with the UV, or a turned Voxel samples the wrong
    // mip and anisotropy axis.
    if (word & TURNED) != 0u {
        let turn = turn_for(cell, normal, draws_the_top(word, face));
        coord = turned(turn, coord);
        ddx = turned(turn, ddx);
        ddy = turned(turn, ddy);
    }

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

    return textureSampleGrad(artwork, artwork_sampler, crisp / edge, face_layer(word, face), ddx, ddy);
}

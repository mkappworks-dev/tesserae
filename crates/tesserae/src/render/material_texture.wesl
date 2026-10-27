// Shared with item rendering: one row per Material, top / side / bottom across.
@group(constants::MATERIAL_BIND_GROUP) @binding(103)
var artwork: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(104)
var artwork_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(105)
var<uniform> textured: vec4<u32>;

fn material_slot(material: u32) -> u32 {
    return select(
        min(material, constants::CHUNK_PALETTE_LEN - 1u),
        constants::CHUNK_PALETTE_LEN,
        material == constants::CHUNK_FLAME,
    );
}

fn has_texture(slot: u32) -> bool {
    return (textured[slot / 32u] & (1u << (slot % 32u))) != 0u;
}

fn sample_texture(slot: u32, uv: vec2<f32>, normal: vec3<f32>) -> vec4<f32> {
    var face = 1u;
    var coord = vec2<f32>(uv.x, -uv.y);
    if abs(normal.y) > 0.5 {
        face = select(2u, 0u, normal.y > 0.0);
        coord = uv.yx;
    } else if abs(normal.x) > 0.5 {
        coord = vec2<f32>(uv.y, -uv.x);
    }
    if slot == constants::CHUNK_PALETTE_LEN {
        face = 1u;
    }
    // Merged faces repeat once per Voxel; an inset prevents atlas bleeding.
    let edge = f32(constants::CHUNK_TEXTURE_EDGE);
    let pixel = vec2<f32>(f32(face), f32(slot)) * edge + vec2<f32>(0.5)
        + fract(coord) * (edge - 1.0);
    let size = vec2<f32>(f32(constants::CHUNK_TEXTURE_WIDTH), f32(constants::CHUNK_TEXTURE_HEIGHT));
    return textureSampleLevel(artwork, artwork_sampler, pixel / size, 0.0);
}

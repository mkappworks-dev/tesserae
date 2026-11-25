//! Game artwork uploaded once for terrain, tiles and held models.
use super::sprite::Sprite;
use super::{
    edit::{Catalogue, Holding},
    menu::Swatches,
};
use crate::content::{ItemLooks, MaterialTextures};
use crate::render::{
    chunk_material::{ChunkMaterialExtension, FACE_WORD_VECS, FLAME_SLOT},
    texture_atlas::{EDGE, FACES, HEIGHT, ROWS, WIDTH, face_word},
    texture_mips::{LEVELS, mip_chain},
};
use crate::session::SessionApp;
use bevy::{
    asset::RenderAssetUsages,
    ecs::system::SystemParam,
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    render::{
        ExtractSchedule, MainWorld, Render, RenderApp, RenderSystems,
        render_asset::RenderAssets,
        render_resource::{
            Extent3d, Origin3d, TexelCopyBufferLayout, TexelCopyTextureInfo, TextureAspect,
            TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension,
        },
        renderer::RenderQueue,
        texture::GpuImage,
    },
};
use std::collections::HashMap;
use tesserae_mods::TileVariation;

const GHOST_OPACITY: f32 = 0.75;

#[derive(Resource)]
pub(super) struct Textures {
    /// Item models and tile icons, which sample one face up close.
    atlas: Option<Handle<Image>>,
    /// Terrain, which also sees faces far off and at grazing angles.
    faces: Option<Handle<Image>>,
    /// Each slot's layers in `faces`, `texture_atlas::face_word`.
    face_words: [u32; ROWS],
    present: [bool; ROWS],
    alpha: [AlphaMode; ROWS],
    /// Each Item's own sprite, by its place in the Server's item order.
    sprites: HashMap<u16, (Handle<Image>, AlphaMode)>,
}

impl Default for Textures {
    fn default() -> Self {
        Self {
            atlas: None,
            faces: None,
            face_words: [0; ROWS],
            present: [false; ROWS],
            alpha: [AlphaMode::Opaque; ROWS],
            sprites: HashMap::new(),
        }
    }
}

#[derive(SystemParam)]
pub(super) struct PreviewContent<'w> {
    pub(super) catalogue: Res<'w, Catalogue>,
    pub(super) swatches: Res<'w, Swatches>,
    pub(super) textures: Res<'w, Textures>,
}

impl PreviewContent<'_> {
    pub(super) fn changed(&self) -> bool {
        self.catalogue.is_changed() || self.swatches.is_changed() || self.textures.is_changed()
    }
}

impl Textures {
    fn slot(held: Holding) -> Option<usize> {
        match held {
            Holding::Solid { material, .. } | Holding::Fluid(material) => Some(material as usize),
            Holding::Sprite(Sprite::Material(material)) => Some(material as usize),
            Holding::Fire => Some(FLAME_SLOT),
            Holding::Sprite(Sprite::Item(_)) | Holding::Tool(_) | Holding::Empty => None,
        }
    }

    pub(super) fn texture(&self, held: Holding) -> Option<Handle<Image>> {
        if let Holding::Sprite(Sprite::Item(at)) = held {
            return self.sprites.get(&at).map(|(image, _)| image.clone());
        }
        Self::slot(held)
            .filter(|&slot| self.present.get(slot) == Some(&true))
            .and(self.atlas.clone())
    }

    #[cfg(test)]
    pub(super) fn faces(&self) -> Option<&Handle<Image>> {
        self.faces.as_ref()
    }

    pub(super) fn apply(&self, extension: &mut ChunkMaterialExtension) {
        extension.texture = self.faces.clone();
        extension.face_words = [UVec4::ZERO; FACE_WORD_VECS];
        for (slot, &word) in self.face_words.iter().enumerate() {
            extension.face_words[slot / 4][slot % 4] = word;
        }
    }

    pub(super) fn material(&self, held: Holding, rgba: Vec4, ghost: bool) -> StandardMaterial {
        let texture = self.texture(held);
        let alpha = match held {
            Holding::Sprite(Sprite::Item(at)) => self.sprites.get(&at).map(|(_, alpha)| *alpha),
            _ => Self::slot(held)
                .and_then(|slot| self.alpha.get(slot))
                .copied(),
        }
        .unwrap_or(AlphaMode::Opaque);
        let rgb = if texture.is_some() {
            Vec3::ONE
        } else {
            rgba.truncate()
        };
        StandardMaterial {
            base_color: Color::linear_rgba(
                rgb.x,
                rgb.y,
                rgb.z,
                if ghost { GHOST_OPACITY } else { 1.0 },
            ),
            base_color_texture: texture,
            alpha_mode: if ghost { AlphaMode::Blend } else { alpha },
            cull_mode: if held == Holding::Fire {
                None
            } else {
                Some(bevy::render::render_resource::Face::Back)
            },
            unlit: true,
            ..default()
        }
    }
}

/// The face array's texels, one buffer per mip level holding every layer, for
/// the render world to write once the texture exists. Bevy's own upload is one
/// write per layer per mip; after that many small writes Apple's GPU
/// performance controller held the clock low for the rest of the process once
/// the GPU got busy, which more than doubled the slowest `flight` frames.
#[derive(Resource, Default)]
pub(super) struct FaceUpload(Option<(AssetId<Image>, Vec<Vec<u8>>)>);

#[derive(Resource, Default)]
struct PendingFaceUpload(Option<(AssetId<Image>, Vec<Vec<u8>>)>);

fn take_face_upload(mut main: ResMut<MainWorld>, mut pending: ResMut<PendingFaceUpload>) {
    if let Some(mut upload) = main.get_resource_mut::<FaceUpload>()
        && upload.0.is_some()
    {
        pending.0 = upload.0.take();
    }
}

fn write_face_upload(
    mut pending: ResMut<PendingFaceUpload>,
    images: Res<RenderAssets<GpuImage>>,
    queue: Res<RenderQueue>,
) {
    let Some(gpu) = pending.0.as_ref().and_then(|(id, _)| images.get(*id)) else {
        return;
    };
    let Some((_, levels)) = pending.0.take() else {
        return;
    };
    for (level, bytes) in levels.iter().enumerate() {
        let edge = (EDGE >> level) as u32;
        queue.write_texture(
            TexelCopyTextureInfo {
                texture: &gpu.texture,
                mip_level: level as u32,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            bytes,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(edge * 4),
                rows_per_image: Some(edge),
            },
            Extent3d {
                width: edge,
                height: edge,
                depth_or_array_layers: gpu.texture.depth_or_array_layers(),
            },
        );
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<FaceUpload>();
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        render_app
            .init_resource::<PendingFaceUpload>()
            .add_systems(ExtractSchedule, take_face_upload)
            .add_systems(
                Render,
                write_face_upload.in_set(RenderSystems::PrepareResources),
            );
    }
    app.init_session_state::<Textures>()
        .add_session_message::<MaterialTextures>()
        .add_session_message::<ItemLooks>()
        .add_session_systems(
            Update,
            (receive.after(super::menu::receive_palette), receive_sprites),
        );
}

fn image(pixels: Vec<u8>) -> Image {
    let mut image = Image::new(
        Extent3d {
            width: WIDTH as u32,
            height: HEIGHT as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    image
}

pub(super) fn receive(
    mut messages: MessageReader<MaterialTextures>,
    mut textures: ResMut<Textures>,
    mut images: ResMut<Assets<Image>>,
    mut upload: ResMut<FaceUpload>,
) {
    if textures.atlas.is_some() && messages.is_empty() {
        return;
    }
    let mut pixels = vec![255; WIDTH * HEIGHT * 4];
    textures.present.fill(false);
    textures.alpha.fill(AlphaMode::Opaque);
    let row_bytes = WIDTH * EDGE * 4;
    pixels[FLAME_SLOT * row_bytes..(FLAME_SLOT + 1) * row_bytes].copy_from_slice(&flame());
    textures.present[FLAME_SLOT] = true;
    textures.alpha[FLAME_SLOT] = AlphaMode::Mask(0.5);
    let mut variation = [TileVariation::Fixed; ROWS];
    if let Some(message) = messages.read().last() {
        for texture in &message.0 {
            let slot = texture.material as usize;
            pixels[slot * row_bytes..(slot + 1) * row_bytes].copy_from_slice(&texture.pixels);
            textures.present[slot] = true;
            textures.alpha[slot] = alpha_of(&texture.pixels);
            variation[slot] = texture.variation;
        }
    }
    let (faces, words, levels) =
        face_array(&pixels, &textures.present, &textures.alpha, &variation);
    info!(
        "Material faces: {} layers",
        faces.texture_descriptor.size.depth_or_array_layers
    );
    textures.face_words = words;
    let faces = images.add(faces);
    upload.0 = Some((faces.id(), levels));
    if let Some(old) = textures.faces.replace(faces) {
        images.remove(old.id());
    }
    if let Some(old) = textures.atlas.replace(images.add(image(pixels))) {
        images.remove(old.id());
    }
}

/// One layer per distinct face, each with its own mip chain, so a face
/// repeats across a merged quad and filters down with no neighbour to bleed
/// into. Only textured slots get layers, and identical faces share one: the GPU
/// frame grows with layers times mips, which at one layer for every slot's
/// every face more than doubled the slowest `flight` frames. Anisotropy needs
/// every filter linear; the shader snaps magnified texels back to crisp ones.
/// The texels come back apart, by mip level, for [`FaceUpload`].
fn face_array(
    atlas: &[u8],
    present: &[bool; ROWS],
    alpha: &[AlphaMode; ROWS],
    variation: &[TileVariation; ROWS],
) -> (Image, [u32; ROWS], Vec<Vec<u8>>) {
    let mut levels = vec![Vec::new(); LEVELS as usize];
    let mut layers: HashMap<(Vec<u8>, Option<u32>), u32> = HashMap::new();
    let mut words = [0; ROWS];
    for slot in (0..ROWS).filter(|&slot| present[slot]) {
        let cutoff = match alpha[slot] {
            AlphaMode::Mask(cutoff) => Some(cutoff),
            _ => None,
        };
        let mut face_layers = [0; FACES];
        for (face, layer) in face_layers.iter_mut().enumerate() {
            let mut tile = Vec::with_capacity(EDGE * EDGE * 4);
            for y in 0..EDGE {
                let start = ((slot * EDGE + y) * WIDTH + face * EDGE) * 4;
                tile.extend_from_slice(&atlas[start..start + EDGE * 4]);
            }
            let next = layers.len() as u32;
            *layer = *layers
                .entry((tile, cutoff.map(f32::to_bits)))
                .or_insert_with_key(|(tile, _)| {
                    let chain = mip_chain(tile, cutoff);
                    let mut at = 0;
                    for (level, bytes) in levels.iter_mut().enumerate() {
                        let len = (EDGE >> level) * (EDGE >> level) * 4;
                        bytes.extend_from_slice(&chain[at..at + len]);
                        at += len;
                    }
                    next
                });
        }
        words[slot] = face_word(face_layers, variation[slot]);
    }
    // Up to every slot's every face, past WebGPU's default limit of 256 layers;
    // Bevy requests the adapter's own limits, and Metal allows 2048.
    let mut image = Image::new_uninit(
        Extent3d {
            width: EDGE as u32,
            height: EDGE as u32,
            depth_or_array_layers: layers.len() as u32,
        },
        TextureDimension::D2,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.mip_level_count = LEVELS;
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::D2Array),
        ..default()
    });
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        label: Some("material faces".into()),
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: 16,
        ..default()
    });
    (image, words, levels)
}

fn alpha_of(pixels: &[u8]) -> AlphaMode {
    let pixels = pixels.as_chunks::<4>().0;
    if pixels.iter().any(|p| p[3] > 0 && p[3] < 255) {
        AlphaMode::Blend
    } else if pixels.iter().any(|p| p[3] == 0) {
        AlphaMode::Mask(0.5)
    } else {
        AlphaMode::Opaque
    }
}

/// One image a sprite of an Item's `texture` samples whole.
pub(super) fn receive_sprites(
    mut messages: MessageReader<ItemLooks>,
    mut textures: ResMut<Textures>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(message) = messages.read().last() else {
        return;
    };
    for (_, (old, _)) in textures.sprites.drain() {
        images.remove(old.id());
    }
    for (at, look) in message.0.iter().enumerate() {
        let Some(pixels) = look.texture.as_deref() else {
            continue;
        };
        let mut image = Image::new(
            Extent3d {
                width: EDGE as u32,
                height: EDGE as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixels.to_vec(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        image.sampler = ImageSampler::nearest();
        let alpha = alpha_of(pixels);
        textures
            .sprites
            .insert(at as u16, (images.add(image), alpha));
    }
}

fn flame() -> Vec<u8> {
    let mut pixels = Vec::with_capacity(EDGE * EDGE * 3 * 4);
    for y in 0..16i32 {
        for _ in 0..3 {
            for x in 0..16i32 {
                let height = (15 - 2 * (x - 8).abs())
                    .max(11 - 3 * (x - 3).abs())
                    .max(12 - 3 * (x - 12).abs());
                let edge = (15 - y - height).abs();
                let lit = 15 - y <= height && !(edge == 0 && (x * 13 + y * 7) % 5 == 0);
                let rgb = if y > 8 && (x - 8).abs() < y / 4 {
                    [255, 252, 188]
                } else if y > 6 {
                    [255, 201, 50]
                } else if y > 2 {
                    [255, 123, 17]
                } else {
                    [235, 66, 9]
                };
                pixels.extend_from_slice(&[rgb[0], rgb[1], rgb[2], if lit { 255 } else { 0 }]);
            }
        }
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::MaterialTexture;
    use crate::render::texture_atlas::{
        FACE_LAYER_BITS, LOOK_FIXED, LOOK_SHIFT, LOOK_TURNED, LOOK_TURNED_AND_PATCHED,
    };
    use tesserae_core::SHAPE_FULL_CUBE;
    use tesserae_mods::MATERIAL_TEXTURE_BYTES;

    #[test]
    fn content_textures_replace_old_assets_and_materials_keep_the_colour_fallback() {
        let mut app = App::new();
        app.init_resource::<Assets<Image>>().add_plugins(plugin);
        let held = Holding::solid(23, SHAPE_FULL_CUBE);
        let rgba = Vec4::new(0.2, 0.3, 0.4, 1.0);
        let fallback = app
            .world()
            .resource::<Textures>()
            .material(held, rgba, false);
        assert_eq!(fallback.base_color, Color::linear_rgb(0.2, 0.3, 0.4));
        assert!(fallback.base_color_texture.is_none());
        app.world_mut()
            .write_message(MaterialTextures(vec![MaterialTexture::new(
                23,
                vec![255; MATERIAL_TEXTURE_BYTES],
                TileVariation::Fixed,
            )]));
        app.update();
        let first = app.world().resource::<Textures>().texture(held).unwrap();
        let textured = app
            .world()
            .resource::<Textures>()
            .material(held, rgba, false);
        assert_eq!(textured.base_color, Color::WHITE);
        assert_eq!(textured.base_color_texture, Some(first.clone()));
        assert_eq!(textured.alpha_mode, AlphaMode::Opaque);
        app.world_mut()
            .write_message(MaterialTextures(vec![MaterialTexture::new(
                23,
                vec![120; MATERIAL_TEXTURE_BYTES],
                TileVariation::Fixed,
            )]));
        app.update();
        assert!(
            app.world()
                .resource::<Assets<Image>>()
                .get(first.id())
                .is_none()
        );
        assert_eq!(
            app.world().resource::<Assets<Image>>().len(),
            2,
            "one atlas and one face array, each shared by Materials and the engine flame"
        );
        assert_ne!(
            app.world().resource::<Textures>().texture(held).unwrap(),
            first
        );
        let ghost = app
            .world()
            .resource::<Textures>()
            .material(held, rgba, true);
        assert_eq!(ghost.alpha_mode, AlphaMode::Blend);
        assert_eq!(ghost.base_color.alpha(), GHOST_OPACITY);
        assert_eq!(
            ghost.base_color_texture,
            app.world().resource::<Textures>().texture(held)
        );
        assert_eq!(
            ghost.cull_mode,
            Some(bevy::render::render_resource::Face::Back)
        );
        assert_eq!(
            app.world()
                .resource::<Textures>()
                .material(held, rgba, false)
                .alpha_mode,
            AlphaMode::Blend
        );
        let mut cutout = vec![255; MATERIAL_TEXTURE_BYTES];
        cutout[3] = 0;
        app.world_mut()
            .write_message(MaterialTextures(vec![MaterialTexture::new(
                23,
                cutout,
                TileVariation::Fixed,
            )]));
        app.update();
        assert_eq!(
            app.world()
                .resource::<Textures>()
                .material(held, rgba, false)
                .alpha_mode,
            AlphaMode::Mask(0.5)
        );
    }

    #[test]
    fn terrain_faces_are_mipmapped_layers_shared_by_identical_faces_with_an_anisotropic_sampler() {
        let mut app = App::new();
        app.init_resource::<Assets<Image>>().add_plugins(plugin);
        let mut leaves = vec![255; MATERIAL_TEXTURE_BYTES];
        for (i, texel) in leaves.chunks_mut(4).enumerate() {
            texel[3] = if i.is_multiple_of(3) { 255 } else { 0 };
        }
        app.world_mut().write_message(MaterialTextures(vec![
            MaterialTexture::new(23, leaves, TileVariation::Fixed),
            MaterialTexture::new(40, vec![200; MATERIAL_TEXTURE_BYTES], TileVariation::Fixed),
            MaterialTexture::new(41, vec![200; MATERIAL_TEXTURE_BYTES], TileVariation::Fixed),
        ]));
        app.update();
        let textures = app.world().resource::<Textures>();
        let layer =
            |slot: usize, face: u32| (textures.face_words[slot] >> (face * FACE_LAYER_BITS)) & 1023;
        assert_eq!(textures.face_words[24], 0, "untextured");
        assert_eq!(textures.face_words[23] >> LOOK_SHIFT, LOOK_FIXED);
        assert!(
            (0..3).all(|face| layer(40, face) == layer(41, 0)),
            "one layer for six faces"
        );
        let distinct: std::collections::HashSet<u32> = [23, 40, FLAME_SLOT]
            .iter()
            .flat_map(|&slot| (0..3).map(move |face| layer(slot, face)))
            .collect();
        let image = app
            .world()
            .resource::<Assets<Image>>()
            .get(textures.faces().unwrap().id())
            .unwrap();
        let size = image.texture_descriptor.size;
        assert_eq!(
            [size.width, size.height, size.depth_or_array_layers],
            [EDGE as u32, EDGE as u32, distinct.len() as u32]
        );
        assert_eq!(image.texture_descriptor.mip_level_count, LEVELS);
        assert!(image.data.is_none(), "the render world writes it, by level");
        let upload = app.world().resource::<FaceUpload>().0.as_ref().unwrap();
        assert_eq!(upload.0, textures.faces().unwrap().id());
        let levels = &upload.1;
        assert_eq!(levels.len(), LEVELS as usize);
        for (level, bytes) in levels.iter().enumerate() {
            let edge = EDGE >> level;
            assert_eq!(bytes.len(), distinct.len() * edge * edge * 4);
        }
        assert_eq!(
            image.texture_view_descriptor.as_ref().unwrap().dimension,
            Some(TextureViewDimension::D2Array),
            "wgpu would infer one layer as D2"
        );
        let ImageSampler::Descriptor(sampler) = &image.sampler else {
            panic!("{:?}", image.sampler);
        };
        assert_eq!(sampler.anisotropy_clamp, 16);
        assert_eq!(sampler.address_mode_u, ImageAddressMode::Repeat);
        for filter in [
            sampler.mag_filter,
            sampler.min_filter,
            sampler.mipmap_filter,
        ] {
            assert_eq!(
                filter,
                ImageFilterMode::Linear,
                "wgpu refuses anisotropy otherwise"
            );
        }
        // The cutout's side face, at 2x2: plain averaging would leave no texel
        // over the cutoff.
        let two = layer(23, 1) as usize * 2 * 2 * 4;
        let side = &levels[3][two..two + 16];
        assert!(side.chunks(4).any(|t| t[3] > 127), "{side:?}");
    }

    #[test]
    fn each_tile_variation_keeps_the_layers_and_sets_only_its_look() {
        let mut app = App::new();
        app.init_resource::<Assets<Image>>().add_plugins(plugin);
        let tile = vec![200; MATERIAL_TEXTURE_BYTES];
        app.world_mut().write_message(MaterialTextures(vec![
            MaterialTexture::new(40, tile.clone(), TileVariation::Fixed),
            MaterialTexture::new(41, tile.clone(), TileVariation::Turned),
            MaterialTexture::new(42, tile, TileVariation::TurnedAndPatched),
        ]));
        app.update();
        let words = app.world().resource::<Textures>().face_words;
        let layers = words[40] & ((1 << LOOK_SHIFT) - 1);
        for (slot, look) in [
            (40, LOOK_FIXED),
            (41, LOOK_TURNED),
            (42, LOOK_TURNED_AND_PATCHED),
        ] {
            assert_eq!(words[slot], look << LOOK_SHIFT | layers, "slot {slot}");
        }
    }

    #[test]
    fn fire_has_a_masked_flame_and_no_named_material_texture() {
        let mut app = App::new();
        app.init_resource::<Assets<Image>>().add_plugins(plugin);
        app.update();
        let textures = app.world().resource::<Textures>();
        let fire = textures.material(Holding::Fire, Vec4::ONE, false);
        assert_eq!(fire.alpha_mode, AlphaMode::Mask(0.5));
        assert_eq!(fire.cull_mode, None);
        let image = app
            .world()
            .resource::<Assets<Image>>()
            .get(fire.base_color_texture.as_ref().unwrap().id())
            .unwrap();
        let pixels = image.data.as_ref().unwrap();
        assert_eq!(pixels.len(), WIDTH * HEIGHT * 4);
        let row_bytes = WIDTH * EDGE * 4;
        let pixels = &pixels[FLAME_SLOT * row_bytes..(FLAME_SLOT + 1) * row_bytes];
        assert!(pixels.as_chunks::<4>().0.iter().any(|p| p[3] == 0));
        assert!(pixels.as_chunks::<4>().0.iter().any(|p| p[3] == 255));
        assert!(textures.texture(Holding::solid(23, 0)).is_none());
    }
}

//! Game artwork uploaded once for terrain, tiles and held models.
use super::{
    edit::{Catalogue, Holding},
    menu::Swatches,
};
use crate::content::MaterialTextures;
use crate::render::{
    chunk_material::{ChunkMaterialExtension, FLAME_SLOT},
    texture_atlas::{EDGE, HEIGHT, ROWS, WIDTH},
};
use bevy::{
    asset::RenderAssetUsages,
    ecs::system::SystemParam,
    image::ImageSampler,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

const GHOST_OPACITY: f32 = 0.75;

#[derive(Resource)]
pub(super) struct Textures {
    atlas: Option<Handle<Image>>,
    present: [bool; ROWS],
    alpha: [AlphaMode; ROWS],
}

impl Default for Textures {
    fn default() -> Self {
        Self {
            atlas: None,
            present: [false; ROWS],
            alpha: [AlphaMode::Opaque; ROWS],
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
            Holding::Fire => Some(FLAME_SLOT),
            Holding::Tool(_) | Holding::Empty => None,
        }
    }

    pub(super) fn texture(&self, held: Holding) -> Option<Handle<Image>> {
        Self::slot(held)
            .filter(|&slot| self.present.get(slot) == Some(&true))
            .and(self.atlas.clone())
    }

    pub(super) fn apply(&self, extension: &mut ChunkMaterialExtension) {
        extension.texture = self.atlas.clone();
        extension.textured = UVec4::ZERO;
        for (slot, &present) in self.present.iter().enumerate() {
            if present {
                extension.textured[slot / 32] |= 1 << (slot % 32);
            }
        }
    }

    pub(super) fn material(&self, held: Holding, rgba: Vec4, ghost: bool) -> StandardMaterial {
        let texture = self.texture(held);
        let alpha = Self::slot(held)
            .and_then(|slot| self.alpha.get(slot))
            .copied()
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

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Textures>()
        .add_message::<MaterialTextures>()
        .add_systems(Update, receive.after(super::menu::receive_palette));
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
    if let Some(message) = messages.read().last() {
        for (material, row) in &message.0 {
            let slot = *material as usize;
            pixels[slot * row_bytes..(slot + 1) * row_bytes].copy_from_slice(row);
            textures.present[slot] = true;
            textures.alpha[slot] = if row
                .as_chunks::<4>()
                .0
                .iter()
                .any(|p| p[3] > 0 && p[3] < 255)
            {
                AlphaMode::Blend
            } else if row.as_chunks::<4>().0.iter().any(|p| p[3] == 0) {
                AlphaMode::Mask(0.5)
            } else {
                AlphaMode::Opaque
            };
        }
    }
    if let Some(old) = textures.atlas.replace(images.add(image(pixels))) {
        images.remove(old.id());
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
        app.world_mut().write_message(MaterialTextures(vec![(
            23,
            vec![255; MATERIAL_TEXTURE_BYTES],
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
        app.world_mut().write_message(MaterialTextures(vec![(
            23,
            vec![120; MATERIAL_TEXTURE_BYTES],
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
            1,
            "one atlas shared by Materials and the engine flame"
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
            .write_message(MaterialTextures(vec![(23, cutout)]));
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

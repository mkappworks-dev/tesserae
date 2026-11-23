//! The World step's terrain preview, drawn off the main thread whenever what it shows changes, so
//! typing a seed never waits on the Generator.

use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::tasks::{AsyncComputeTaskPool, Task, futures::check_ready};
use tesserae_core::terrain_preview::{Frame, Image as Picture, Source};

use super::*;

/// Everything the picture is a function of.
#[derive(Clone, Debug)]
struct Inputs {
    source: Arc<Source>,
    seed: u64,
    voxel_size_m: f32,
    edits: Option<BiomeEdits>,
}

impl PartialEq for Inputs {
    fn eq(&self, other: &Self) -> bool {
        // The same read of the Game: comparing its declarations field by field costs a frame.
        Arc::ptr_eq(&self.source, &other.source)
            && self.seed == other.seed
            && self.voxel_size_m == other.voxel_size_m
            && self.edits == other.edits
    }
}

#[derive(Resource, Default)]
pub(super) struct TerrainPreview {
    shown: Option<Inputs>,
    /// One at a time: a seed typed meanwhile is drawn when this one lands, never queued behind it.
    drawing: Option<(Inputs, Task<Result<Picture, String>>)>,
    pub(super) image: Option<Handle<Image>>,
}

impl TerrainPreview {
    #[cfg(test)]
    pub(super) fn drawing(&self) -> bool {
        self.drawing.is_some()
    }
}

/// Where the page shows the preview; it takes the picture once one is drawn.
#[derive(Component)]
pub(super) struct TerrainPicture;

/// What the World step's preview should show now, or `None` to leave it as it is: off the step,
/// before the Game is read, or while the seed or a Climate size does not parse.
fn wanted(l: &Launcher) -> Option<Inputs> {
    if l.screen != Screen::Create(Step::World) {
        return None;
    }
    let (ids, source) = l.terrain.as_ref()?;
    let climate = l.form.climate.as_ref().map(ClimateDraft::params);
    let climate = match climate {
        Some(None) => return None,
        Some(params) => params,
        None => None,
    };
    // A draft read for other Mods names Biomes this source may not declare.
    let biomes = l
        .form
        .biomes
        .as_ref()
        .filter(|d| &d.ids == ids)
        .map(BiomesDraft::changed)
        .unwrap_or_default();
    Some(Inputs {
        source: Arc::clone(source),
        seed: l.form.seed().ok()?,
        voxel_size_m: l.form.voxel_size_m,
        edits: (!biomes.is_empty() || climate.is_some()).then_some(BiomeEdits { biomes, climate }),
    })
}

pub(super) fn draw(
    launcher: Option<Res<Launcher>>,
    mut preview: ResMut<TerrainPreview>,
    images: Option<ResMut<Assets<Image>>>,
) {
    if let Some((_, task)) = preview.drawing.as_mut()
        && let Some(drawn) = check_ready(task)
    {
        let (inputs, _) = preview.drawing.take().expect("checked above");
        match drawn {
            Ok(picture) => {
                if let Some(mut images) = images {
                    let image = image(picture);
                    match preview.image.clone() {
                        Some(handle) => {
                            let _ = images.insert(handle.id(), image);
                        }
                        None => preview.image = Some(images.add(image)),
                    }
                }
            }
            Err(e) => warn!("No terrain preview: {e}"),
        }
        preview.shown = Some(inputs);
    }
    let Some(wanted) = launcher.as_deref().and_then(wanted) else {
        return;
    };
    if preview.drawing.is_some() || preview.shown.as_ref() == Some(&wanted) {
        return;
    }
    let inputs = wanted.clone();
    let task = AsyncComputeTaskPool::get_or_init(Default::default).spawn(async move {
        inputs.source.render(
            inputs.seed,
            inputs.voxel_size_m,
            inputs.edits.as_ref(),
            &Frame::CREATE,
        )
    });
    preview.drawing = Some((wanted, task));
}

fn image(picture: Picture) -> Image {
    Image::new(
        Extent3d {
            width: picture.width,
            height: picture.height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        picture.rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

/// Hands the drawn picture to the page's preview box, which a rebuild spawns empty.
pub(super) fn show(
    mut commands: Commands,
    preview: Res<TerrainPreview>,
    boxes: Query<Entity, (With<TerrainPicture>, Without<ImageNode>)>,
) {
    let Some(handle) = &preview.image else {
        return;
    };
    for entity in &boxes {
        commands
            .entity(entity)
            .insert(ImageNode::new(handle.clone()));
    }
}

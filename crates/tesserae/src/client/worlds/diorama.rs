//! Each Biome's diorama on the Biomes step, painted off the main thread whenever its settings
//! change, so dragging a slider never waits on a picture.

use bevy::tasks::{AsyncComputeTaskPool, Task, futures::check_ready};
use tesserae_core::diorama::{Scene, paint};

use super::*;

/// Twice the list's thumbnail, so it stays sharp on a Retina screen.
const SIZE: UVec2 = UVec2::new(192, 108);
/// Pictures kept, so undoing an edit is instant. A slider drag passes through many looks; the
/// oldest that no thumbnail shows goes first.
const KEPT: usize = 64;

#[derive(Resource, Default)]
pub(super) struct Dioramas {
    drawn: Vec<(Scene, Handle<Image>)>,
    drawing: Vec<(Scene, Task<Vec<u8>>)>,
}

impl Dioramas {
    #[cfg(test)]
    pub(super) fn drawing(&self) -> bool {
        !self.drawing.is_empty()
    }

    fn drawn(&self, scene: &Scene) -> Option<&Handle<Image>> {
        self.drawn.iter().find(|(s, _)| s == scene).map(|(_, h)| h)
    }
}

/// Lands finished pictures, starts one for any look a thumbnail wants that is neither drawn nor
/// being drawn, and shows each thumbnail its picture. A thumbnail keeps its old picture until the
/// new one lands.
pub(super) fn draw(
    mut commands: Commands,
    mut dioramas: ResMut<Dioramas>,
    images: Option<ResMut<Assets<Image>>>,
    thumbs: Query<(Entity, &pages::BiomeThumb, Option<&ImageNode>)>,
) {
    let Some(mut images) = images else {
        return;
    };
    let dioramas = &mut *dioramas;
    let mut at = 0;
    while at < dioramas.drawing.len() {
        let Some(rgba) = check_ready(&mut dioramas.drawing[at].1) else {
            at += 1;
            continue;
        };
        let (scene, _) = dioramas.drawing.swap_remove(at);
        let handle = images.add(rgba_image(SIZE.x, SIZE.y, rgba));
        if dioramas.drawn.len() >= KEPT
            && let Some(unused) = (dioramas.drawn.iter())
                .position(|(s, _)| !thumbs.iter().any(|(_, t, _)| t.look == *s))
        {
            let (_, old) = dioramas.drawn.remove(unused);
            images.remove(&old);
        }
        dioramas.drawn.push((scene, handle));
    }
    for (entity, thumb, shown) in &thumbs {
        if let Some(handle) = dioramas.drawn(&thumb.look) {
            if shown.is_none_or(|s| s.image != *handle) {
                commands
                    .entity(entity)
                    .insert(ImageNode::new(handle.clone()));
            }
        } else if !dioramas.drawing.iter().any(|(s, _)| *s == thumb.look) {
            let scene = thumb.look;
            let task = AsyncComputeTaskPool::get_or_init(Default::default)
                .spawn(async move { paint(&scene, SIZE.x, SIZE.y) });
            dioramas.drawing.push((scene, task));
        }
    }
}

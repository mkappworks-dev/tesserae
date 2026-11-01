//! Drops `bevy_pbr`'s per-view bind groups for views that no longer exist.
//!
//! `prepare_preprocess_bind_groups` inserts into `BinUnpackingBindGroups` and
//! `UniformAllocationBindGroups` for every live view each frame and never
//! removes a key (bevy_pbr 0.20 `render/gpu_preprocess.rs`), so each camera and
//! shadow cascade a World session spawns would keep its bind groups, and through
//! them the buffers they bind, until the process ends: about 40 buffers and 16
//! bind groups a session.

use bevy::pbr::{
    BinUnpackingBindGroups, UniformAllocationBindGroups, prepare_preprocess_bind_groups,
};
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use bevy::render::view::{ExtractedView, RetainedViewEntity};
use bevy::render::{Render, RenderApp, RenderSystems};

pub(super) fn plugin(app: &mut App) {
    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        return;
    };
    render_app.add_systems(
        Render,
        forget_dead_views
            .in_set(RenderSystems::PrepareBindGroups)
            .after(prepare_preprocess_bind_groups),
    );
}

fn forget_dead_views(
    views: Query<&ExtractedView>,
    bin_unpacking: Option<ResMut<BinUnpackingBindGroups>>,
    uniform_allocation: Option<ResMut<UniformAllocationBindGroups>>,
) {
    let live: HashSet<RetainedViewEntity> = views.iter().map(|v| v.retained_view_entity).collect();
    if let Some(mut groups) = bin_unpacking {
        groups.retain(|key, _| live.contains(&key.view));
    }
    if let Some(mut groups) = uniform_allocation {
        groups.retain(|key, _| live.contains(&key.view));
    }
}

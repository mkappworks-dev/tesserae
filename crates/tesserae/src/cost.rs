//! What streaming costs the main thread, measured where it is spent. `dev` only.
//!
//! Frame time under vsync is the display's refresh interval, which changes from
//! run to run, so it cannot say what a change to streaming did. These are the
//! things on the main thread that streaming still pays for: the Server's tick,
//! the Client turning a delivered mesh into a Bevy mesh, and the Client
//! re-sorting a blended mesh as the Eye moves.

use std::time::{Duration, Instant};

use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct MainThreadCost {
    started: Option<Instant>,
    tick_s: f32,
    upload_s: f32,
    uploads: u32,
    sort_s: f32,
    sorts: u32,
}

/// What one frame spent, taken and cleared.
#[derive(Default, Clone, Copy)]
pub struct FrameCost {
    pub tick_s: f32,
    pub upload_s: f32,
    pub uploads: u32,
    pub sort_s: f32,
    pub sorts: u32,
}

impl MainThreadCost {
    pub fn uploaded(&mut self, took: Duration, meshes: u32) {
        self.upload_s += took.as_secs_f32();
        self.uploads += meshes;
    }

    /// `meshes` is how many were re-indexed, not how many were looked at.
    pub fn sorted(&mut self, took: Duration, meshes: u32) {
        self.sort_s += took.as_secs_f32();
        self.sorts += meshes;
    }

    pub fn take_frame(&mut self) -> FrameCost {
        let frame = FrameCost {
            tick_s: self.tick_s,
            upload_s: self.upload_s,
            uploads: self.uploads,
            sort_s: self.sort_s,
            sorts: self.sorts,
        };
        *self = MainThreadCost {
            started: self.started,
            ..default()
        };
        frame
    }
}

pub fn tick_started(mut cost: ResMut<MainThreadCost>) {
    cost.started = Some(Instant::now());
}

pub fn tick_ended(mut cost: ResMut<MainThreadCost>) {
    if let Some(started) = cost.started.take() {
        cost.tick_s += started.elapsed().as_secs_f32();
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<MainThreadCost>();
}

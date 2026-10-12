//! What streaming costs the main thread, measured where it is spent. `dev` only.
//!
//! Frame time under vsync is the display's refresh interval, which changes from
//! run to run, so it cannot say what a change to streaming did. These are the
//! two things on the main thread that streaming still pays for: the Server's
//! tick, and the Client turning a delivered mesh into a Bevy mesh.

use std::time::{Duration, Instant};

use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct MainThreadCost {
    started: Option<Instant>,
    tick_s: f32,
    upload_s: f32,
    uploads: u32,
}

/// What one frame spent, taken and cleared.
#[derive(Default, Clone, Copy)]
pub struct FrameCost {
    pub tick_s: f32,
    pub upload_s: f32,
    pub uploads: u32,
}

impl MainThreadCost {
    pub fn uploaded(&mut self, took: Duration, meshes: u32) {
        self.upload_s += took.as_secs_f32();
        self.uploads += meshes;
    }

    pub fn take_frame(&mut self) -> FrameCost {
        let frame = FrameCost {
            tick_s: self.tick_s,
            upload_s: self.upload_s,
            uploads: self.uploads,
        };
        self.tick_s = 0.0;
        self.upload_s = 0.0;
        self.uploads = 0;
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

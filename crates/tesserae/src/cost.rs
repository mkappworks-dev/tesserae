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
    longest_tick_s: f32,
    upload_s: f32,
    uploads: u32,
    sort_s: f32,
    sorts: u32,
}

/// What one frame spent, taken and cleared.
#[derive(Default, Clone, Copy)]
pub struct FrameCost {
    pub tick_s: f32,
    pub longest_tick_s: f32,
    pub upload_s: f32,
    pub uploads: u32,
    pub sort_s: f32,
    pub sorts: u32,
}

impl MainThreadCost {
    fn ticked(&mut self, took: Duration) {
        let seconds = took.as_secs_f32();
        self.tick_s += seconds;
        self.longest_tick_s = self.longest_tick_s.max(seconds);
    }

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
            longest_tick_s: self.longest_tick_s,
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
        cost.ticked(started.elapsed());
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<MainThreadCost>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catch_up_ticks_keep_the_frame_total_and_the_longest_tick_separate() {
        let mut cost = MainThreadCost::default();
        cost.ticked(Duration::from_millis(2));
        cost.ticked(Duration::from_millis(3));
        let frame = cost.take_frame();
        assert!((frame.tick_s - 0.005).abs() < f32::EPSILON);
        assert_eq!(frame.longest_tick_s, 0.003);
        let next = cost.take_frame();
        assert_eq!(next.tick_s, 0.0);
        assert_eq!(next.longest_tick_s, 0.0);
    }
}

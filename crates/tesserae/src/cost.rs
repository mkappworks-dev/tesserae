//! What streaming costs the main thread, measured where it is spent.
//! Available to dev captures and opt-in profiling builds.
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
    enabled: bool,
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
    pub fn enabled(&self) -> bool {
        self.enabled
    }
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

    pub fn frame(&self) -> FrameCost {
        FrameCost {
            tick_s: self.tick_s,
            longest_tick_s: self.longest_tick_s,
            upload_s: self.upload_s,
            uploads: self.uploads,
            sort_s: self.sort_s,
            sorts: self.sorts,
        }
    }

    pub fn take_frame(&mut self) -> FrameCost {
        let frame = self.frame();
        *self = MainThreadCost {
            started: self.started,
            enabled: self.enabled,
            ..default()
        };
        frame
    }
}

pub fn tick_started(mut cost: ResMut<MainThreadCost>) {
    cost.started = cost.enabled.then(Instant::now);
}

pub fn tick_ended(mut cost: ResMut<MainThreadCost>) {
    if let Some(started) = cost.started.take() {
        cost.ticked(started.elapsed());
    }
}

pub fn collecting() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| {
        cfg!(any(test, feature = "dev"))
            || std::env::var_os("TESSERAE_PROFILE_CSV").is_some()
            || std::env::var_os("TESSERAE_SCREENSHOT").is_some()
    })
}

/// This thread's CPU seconds; excludes waits. `None` where unsupported.
#[cfg(all(unix, feature = "profiling"))]
pub fn thread_cpu() -> Option<f64> {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let result = unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut time) };
    (result == 0).then_some(time.tv_sec as f64 + time.tv_nsec as f64 * 1e-9)
}
#[cfg(not(all(unix, feature = "profiling")))]
pub fn thread_cpu() -> Option<f64> {
    None
}

pub fn plugin(app: &mut App) {
    app.insert_resource(MainThreadCost {
        enabled: collecting(),
        ..default()
    });
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

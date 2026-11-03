use std::time::Instant;

use bevy::prelude::*;
use bevy::render::{
    Render, RenderApp, RenderSystems,
    renderer::{RenderGraph, RenderGraphSystems, render_system},
    view::window::prepare_windows,
};

#[derive(Resource, Default)]
pub(super) struct Intervals {
    points: [Option<Instant>; 5],
}

impl Intervals {
    pub(super) fn reset(&mut self) {
        self.points = [None; 5];
    }

    pub(super) fn seconds(&self) -> [Option<f64>; 3] {
        [(0, 1), (2, 3), (3, 4)].map(|(a, b)| {
            self.points[a].zip(self.points[b]).and_then(|(a, b)| {
                b.checked_duration_since(a)
                    .map(|duration| duration.as_secs_f64())
            })
        })
    }
}

// Exclusive markers bound these intervals without racing parallel systems;
// that serialization makes this an attribution instrument, not a timing baseline.
fn mark<const INDEX: usize>(world: &mut World) {
    world.resource_mut::<Intervals>().points[INDEX] = Some(Instant::now());
}

pub(super) fn plugin(app: &mut App) {
    let Some(render) = app.get_sub_app_mut(RenderApp) else {
        return;
    };
    render
        .init_resource::<Intervals>()
        .add_systems(
            Render,
            (
                mark::<0>
                    .in_set(RenderSystems::PrepareViews)
                    .before(prepare_windows),
                mark::<1>
                    .in_set(RenderSystems::PrepareViews)
                    .after(prepare_windows),
                mark::<4>.in_set(RenderSystems::Render).after(render_system),
            ),
        )
        .add_systems(
            RenderGraph,
            (
                mark::<2>.before(RenderGraphSystems::Begin),
                mark::<3>.after(RenderGraphSystems::Finish),
            ),
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_reversed_and_previous_frame_boundaries_are_unavailable() {
        let now = Instant::now();
        let later = now + std::time::Duration::from_millis(8);
        let mut intervals = Intervals {
            points: [Some(now), Some(later), None, Some(later), Some(now)],
        };
        assert_eq!(intervals.seconds(), [Some(0.008), None, None]);
        intervals.reset();
        assert_eq!(intervals.seconds(), [None; 3]);
    }
}

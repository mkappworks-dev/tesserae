//! The Quality curve: the coarse rings a Player asks for, which the Server clamps to the Game's
//! own distances and streams from. See
//! `docs/adr/0036-the-quality-curve-is-a-player-preference-the-server-clamps.md`.

use bevy::prelude::*;

use super::streaming::StreamingSettings;
use crate::session::SessionApp;

/// The engine's ceiling for a Game that declares no view, as `ViewParams::MAX_LOD_M` allows.
pub const DEFAULT_CEILING_M: f32 = 8192.0;

/// The distances the Game declares.
#[derive(Resource, Clone, Copy, PartialEq, Debug)]
pub struct DeclaredStreaming {
    /// What every request is clamped to.
    pub ceiling: StreamingSettings,
    /// Low's reach: the Game's own rings, streamed until a Client asks for another curve.
    pub low_reach_m: f32,
}

impl Default for DeclaredStreaming {
    /// The engine's own rings are `StreamingSettings::default()`'s.
    fn default() -> Self {
        let own = StreamingSettings::default();
        DeclaredStreaming {
            ceiling: StreamingSettings {
                lod_distance_m: DEFAULT_CEILING_M,
                ..own
            },
            low_reach_m: own.lod_distance_m,
        }
    }
}

impl DeclaredStreaming {
    /// A Game whose Low reaches its ceiling.
    pub fn own(ceiling: StreamingSettings) -> Self {
        DeclaredStreaming {
            ceiling,
            low_reach_m: ceiling.lod_distance_m,
        }
    }

    /// What the Server streams before any Client asks.
    pub fn low(self) -> StreamingSettings {
        QualityCurve::low(self).applied_to(self.ceiling)
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_session_resource::<DeclaredStreaming>()
        .add_session_message::<QualityCurve>()
        // Every frame rather than in a fixed tick, which a frame can skip and drop the request.
        .add_session_systems(
            RunFixedMainLoop,
            apply.in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop),
        );
}

/// The last request wins; one that changes nothing leaves `StreamingSettings` unchanged.
pub(crate) fn apply(
    mut requests: MessageReader<QualityCurve>,
    declared: Res<DeclaredStreaming>,
    mut settings: ResMut<StreamingSettings>,
) {
    if let Some(request) = requests.read().last() {
        settings.set_if_neq(request.applied_to(declared.ceiling));
    }
}

/// A Client's request for the coarse rings: level k ends at `min(unit_m * 2^k, reach_m)`.
#[derive(Message, Clone, Copy, PartialEq, Debug)]
pub struct QualityCurve {
    pub unit_m: f32,
    pub reach_m: f32,
}

impl QualityCurve {
    /// The default: 6 Chunks to 6 km, as 8 filled Terra's field in 13.5 s against a 10 s budget
    /// and 6 in 9.5.
    pub const MEDIUM: Self = QualityCurve {
        unit_m: 192.0,
        reach_m: 6000.0,
    };
    pub const HIGH: Self = QualityCurve {
        unit_m: 384.0,
        reach_m: 8000.0,
    };

    /// The Game's own rings: the doubling from its view distance to its own reach, which may
    /// be nearer than its ceiling.
    pub fn low(game: DeclaredStreaming) -> Self {
        QualityCurve {
            unit_m: game.ceiling.view_distance_m,
            reach_m: game.low_reach_m,
        }
    }

    /// What the Server streams for this request in a Game that declares `game`. Level 0 is the
    /// Game's; the unit runs from its view distance to High's, the widest level 1 the placement
    /// generator's cache is sized for, and the reach from its view distance to its ceiling. A unit
    /// that is not finite takes the view distance, a reach the ceiling.
    pub fn applied_to(self, game: StreamingSettings) -> StreamingSettings {
        let view_m = game.view_distance_m;
        let within = |v: f32, fallback: f32, max: f32| {
            let v = if v.is_finite() { v } else { fallback };
            v.min(max).max(view_m)
        };
        StreamingSettings {
            lod_unit_m: within(self.unit_m, view_m, Self::HIGH.unit_m),
            lod_distance_m: within(self.reach_m, game.lod_distance_m, game.lod_distance_m),
            ..game
        }
    }
}

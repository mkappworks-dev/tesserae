//! The Quality curve: the coarse rings a Player asks for, which the Server clamps to the Game's
//! own distances and streams from. See
//! `docs/adr/0036-the-quality-curve-is-a-player-preference-the-server-clamps.md`.

use bevy::prelude::*;

use super::streaming::StreamingSettings;
use crate::session::SessionApp;

/// The distances the Game declares: the curve streamed until a Client asks for another, and
/// the ceiling every request is clamped to.
#[derive(Resource, Clone, Copy, Default, Debug)]
pub struct DeclaredStreaming(pub StreamingSettings);

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
pub(super) fn apply(
    mut requests: MessageReader<QualityCurve>,
    declared: Res<DeclaredStreaming>,
    mut settings: ResMut<StreamingSettings>,
) {
    if let Some(request) = requests.read().last() {
        settings.set_if_neq(request.applied_to(declared.0));
    }
}

/// A Client's request for the coarse rings: level k ends at `min(unit_m * 2^k, reach_m)`.
#[derive(Message, Clone, Copy, PartialEq, Debug)]
pub struct QualityCurve {
    pub unit_m: f32,
    pub reach_m: f32,
}

impl QualityCurve {
    pub const BALANCED_UNIT_M: f32 = 192.0;
    pub const DETAILED_UNIT_M: f32 = 384.0;

    pub fn balanced(reach_m: f32) -> Self {
        QualityCurve {
            unit_m: Self::BALANCED_UNIT_M,
            reach_m,
        }
    }

    pub fn detailed(reach_m: f32) -> Self {
        QualityCurve {
            unit_m: Self::DETAILED_UNIT_M,
            reach_m,
        }
    }

    /// What the Server streams for this request in a Game that declares `game`. Level 0 is the
    /// Game's, and a NaN takes the Game's value.
    pub fn applied_to(self, game: StreamingSettings) -> StreamingSettings {
        StreamingSettings {
            lod_unit_m: self.unit_m.max(game.view_distance_m),
            lod_distance_m: self.reach_m.min(game.lod_distance_m),
            ..game
        }
    }
}

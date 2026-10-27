//! A one-line readout of what the Server is streaming. `dev` only: it exists to
//! make streaming visible while working on it, and a shipped build should not
//! put engine internals on screen.
//!
//! Reads `StreamingStats`, which the Server publishes, and not the Chunk store.
//! See `docs/adr/0002-client-server-from-day-one.md`.

use bevy::prelude::*;

use crate::server::streaming::stats::StreamingStats;

#[derive(Component)]
struct StreamingReadout;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn_readout)
        .add_systems(Update, update_readout);
}

fn spawn_readout(mut commands: Commands) {
    commands.spawn((
        StreamingReadout,
        super::help::DebugPanel,
        Visibility::Hidden,
        // ASCII only: Bevy's embedded default font is a FiraMono subset and
        // renders anything outside it, a middle dot included, as tofu.
        Text::new("streaming"),
        TextFont {
            font_size: bevy::text::FontSize::Px(14.0),
            ..default()
        },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.7)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(16.0),
            top: Val::Px(16.0),
            ..default()
        },
    ));
}

fn update_readout(
    stats: Res<StreamingStats>,
    diagnostics: Res<bevy::diagnostic::DiagnosticsStore>,
    readout: Single<&mut Text, With<StreamingReadout>>,
    drawn: Query<(), With<Mesh3d>>,
) {
    let fps = diagnostics
        .get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    // Through change detection rather than unconditionally, so the UI is not
    // re-laid out on a frame where nothing moved.
    readout.into_inner().set_if_neq(Text::new(format!(
        "chunk {}  -  loaded {}  meshed {}  drawn {}  -  {fps:.0} fps",
        stats.anchor,
        stats.loaded,
        stats.meshed,
        drawn.iter().count()
    )));
}

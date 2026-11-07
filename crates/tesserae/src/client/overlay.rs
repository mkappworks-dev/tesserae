//! The F3 readout: a line of what the Server is streaming and the frame rate.
//! It ships in every build but stays hidden until F3 is pressed.
//!
//! Reads `StreamingReadout`, which the Server publishes, and not the Chunk store.
//! See `docs/adr/0002-client-server-from-day-one.md`.

use bevy::prelude::*;

use crate::session::SessionApp;

use crate::server::streaming::readout::StreamingReadout;

#[derive(Component)]
struct ReadoutText;

pub(super) fn plugin(app: &mut App) {
    app.add_session_systems(Startup, spawn_readout)
        .add_session_systems(Update, update_readout.run_if(diagnostics_shown));
}

fn spawn_readout(mut commands: Commands) {
    commands.spawn((
        ReadoutText,
        super::help::DebugPanel,
        Visibility::Hidden,
        Text::new("streaming"),
        super::theme::tabular(super::theme::font(super::theme::TEXT_SMALL)),
        TextColor(super::theme::READOUT),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(16.0),
            top: Val::Px(16.0),
            ..default()
        },
    ));
}

fn diagnostics_shown(panels: Res<super::help::Panels>) -> bool {
    panels.debug
}

fn update_readout(
    stats: Res<StreamingReadout>,
    diagnostics: Res<bevy::diagnostic::DiagnosticsStore>,
    readout: Single<&mut Text, With<ReadoutText>>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::help::Panels;

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<Panels>()
            .init_resource::<StreamingReadout>()
            .init_resource::<bevy::diagnostic::DiagnosticsStore>()
            .add_systems(Startup, spawn_readout)
            .add_systems(Update, update_readout.run_if(diagnostics_shown));
        app
    }

    fn text(app: &mut App) -> String {
        app.world_mut()
            .query_filtered::<&Text, With<ReadoutText>>()
            .single(app.world())
            .unwrap()
            .0
            .clone()
    }

    #[test]
    fn the_readout_is_not_written_until_f3_shows_it() {
        let mut app = app();
        app.world_mut().resource_mut::<StreamingReadout>().loaded = 7;
        app.update();
        assert_eq!(text(&mut app), "streaming");

        app.world_mut().resource_mut::<Panels>().debug = true;
        app.update();
        assert!(text(&mut app).contains("loaded 7"), "{}", text(&mut app));
    }
}

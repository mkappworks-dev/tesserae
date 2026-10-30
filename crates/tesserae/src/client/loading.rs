//! A loading screen over a World opened from the World list. Its bar follows
//! the loader's real stages and ends when the first terrain is drawn.

use bevy::prelude::*;

use super::menu_ui as ui;
use crate::open::{Load, OpenRequest, Stage};
use crate::session::AppState;

#[derive(Component)]
struct Loading {
    since_s: f32,
    stage: Stage,
    stage_since_s: f32,
}

#[derive(Component)]
struct StageText;

#[derive(Component)]
struct BarFill;

/// The screen is drawn before the World has a camera of its own.
#[derive(Component)]
struct LoadingCamera;

/// Longest the loading screen waits for terrain, e.g. for a World saved without a Game.
const LOADING_TIMEOUT_S: f32 = 15.0;

const BAR_WIDTH_PX: f32 = 280.0;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(AppState::Loading), show_loading)
        .add_systems(OnExit(AppState::Loading), drop_camera)
        .add_systems(
            Update,
            (progress, hide_loading)
                .chain()
                .run_if(any_with_component::<Loading>),
        );
}

fn show_loading(mut commands: Commands, request: Option<Res<OpenRequest>>, time: Res<Time<Real>>) {
    let Some(request) = request.filter(|r| r.from_list) else {
        return;
    };
    let name = request.name.clone().unwrap_or_default();
    commands.spawn((Camera2d, LoadingCamera));
    commands
        .spawn((
            Loading {
                since_s: time.elapsed_secs(),
                stage: Stage::ReadingSave,
                stage_since_s: time.elapsed_secs(),
            },
            GlobalZIndex(200),
            BackgroundColor(ui::BG),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            },
        ))
        .with_children(|card| {
            opening(card, &name);
            card.spawn((
                StageText,
                Text::new(Stage::ReadingSave.label()),
                TextFont {
                    font_size: bevy::text::FontSize::Px(12.0),
                    ..default()
                },
                TextColor(ui::MUTED),
            ));
            card.spawn((
                Node {
                    width: Val::Px(BAR_WIDTH_PX),
                    height: Val::Px(4.0),
                    margin: UiRect::top(Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(ui::EDGE),
            ))
            .with_children(|track| {
                track.spawn((
                    BarFill,
                    Node {
                        width: Val::Px(0.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(ui::LEAF),
                ));
            });
        });
}

fn drop_camera(mut commands: Commands, cameras: Query<Entity, With<LoadingCamera>>) {
    for camera in &cameras {
        commands.entity(camera).despawn();
    }
}

/// Also drawn by the World list just before it starts loading.
pub(super) fn opening(parent: &mut ChildSpawnerCommands, name: &str) {
    ui::weighted_text(parent, format!("Opening {name}…"), 19.0, 620, ui::INK);
    ui::paragraph(parent, "Loading the World.", 12.0, ui::MUTED);
}

/// Inside a stage the bar sweeps toward the next one without reaching it: the
/// loader reports which stage it is in, not how far through it is.
fn bar_fraction(stage: Stage, elapsed_s: f32) -> f32 {
    let (from, to) = stage.span();
    from + (to - from) * (1.0 - (-elapsed_s / 2.0).exp()) * 0.9
}

fn progress(
    load: Option<Res<Load>>,
    time: Res<Time<Real>>,
    mut screens: Query<&mut Loading>,
    mut text: Query<&mut Text, With<StageText>>,
    mut fill: Query<&mut Node, With<BarFill>>,
) {
    let stage = load.map_or(Stage::FirstTerrain, |l| l.stage());
    let now = time.elapsed_secs();
    for mut screen in &mut screens {
        if screen.stage != stage {
            screen.stage = stage;
            screen.stage_since_s = now;
        }
        let fraction = bar_fraction(stage, now - screen.stage_since_s);
        for mut text in &mut text {
            text.0 = stage.label().into();
        }
        for mut node in &mut fill {
            node.width = Val::Px(BAR_WIDTH_PX * fraction);
        }
    }
}

fn hide_loading(
    mut commands: Commands,
    time: Res<Time<Real>>,
    arrived: Option<Res<TerrainArrived>>,
    screens: Query<(Entity, &Loading)>,
) {
    for (screen, loading) in &screens {
        if arrived.is_some() || time.elapsed_secs() - loading.since_s >= LOADING_TIMEOUT_S {
            commands.entity(screen).despawn();
        }
    }
}

/// Inserted by the Client once any terrain is drawn.
#[derive(Resource)]
pub(crate) struct TerrainArrived;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bar_only_moves_forward_and_never_finishes_a_stage_by_waiting() {
        let stages = [
            Stage::ReadingSave,
            Stage::LoadingGame,
            Stage::PreparingWorld,
            Stage::FirstTerrain,
        ];
        let mut previous = 0.0;
        for stage in stages {
            let (from, to) = stage.span();
            assert!(from >= previous, "{stage:?} starts where the last ended");
            let waited = bar_fraction(stage, 1_000.0);
            assert!(waited < to, "{stage:?} stays short of the next stage");
            assert!(bar_fraction(stage, 0.0) >= from);
            previous = to;
        }
        assert_eq!(previous, 1.0);
    }
}

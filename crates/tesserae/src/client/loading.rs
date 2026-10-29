//! A loading screen over a World opened from the World list, until the first
//! terrain is drawn.

use bevy::prelude::*;

use super::menu_ui as ui;

#[derive(Component)]
struct Loading;

/// Longest the loading screen waits for terrain, e.g. for an Empty World.
const LOADING_TIMEOUT_S: f32 = 15.0;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, show_loading)
        .add_systems(Update, hide_loading.run_if(any_with_component::<Loading>));
}

fn show_loading(mut commands: Commands, launched: Option<Res<crate::launch::Launched>>) {
    let Some(name) = launched
        .filter(|l| l.from_list)
        .and_then(|l| l.world_name.clone())
    else {
        return;
    };
    commands
        .spawn((
            Loading,
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
        .with_children(|card| opening(card, &name));
}

/// Also drawn by the World list just before it restarts into the World.
pub(super) fn opening(parent: &mut ChildSpawnerCommands, name: &str) {
    ui::weighted_text(parent, format!("Opening {name}…"), 19.0, 620, ui::INK);
    ui::paragraph(parent, "Loading the World.", 12.0, ui::MUTED);
}

fn hide_loading(
    mut commands: Commands,
    time: Res<Time<Real>>,
    arrived: Option<Res<TerrainArrived>>,
    screens: Query<Entity, With<Loading>>,
) {
    if arrived.is_some() || time.elapsed_secs() >= LOADING_TIMEOUT_S {
        for screen in &screens {
            commands.entity(screen).despawn();
        }
    }
}

/// Inserted by the Client once any terrain is drawn.
#[derive(Resource)]
pub(super) struct TerrainArrived;

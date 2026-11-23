//! The Map's add-mode banner and its pointer.

use bevy::prelude::*;
use bevy::window::{CursorIcon, PrimaryWindow, SystemCursorIcon};

use super::map_window::{MapWindow, closed};
use super::menu_ui as ui;
use super::minimap::Spot;
use super::theme;
use super::waypoints::{Marks, pick};
use crate::session::SessionApp;

const HOVER_REACH_PX: f32 = 13.0;

#[derive(Component)]
pub(super) struct HintBox;

pub(super) fn plugin(app: &mut App) {
    app.add_session_systems(Update, (hint.run_if(not(closed)), cursor));
}

/// The banner at the top of the Map while add mode waits for a click.
pub(super) fn banner(view: &mut ChildSpawnerCommands) {
    view.spawn((
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(18.0),
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
    ))
    .with_children(|row| {
        row.spawn((
            HintBox,
            Pickable::IGNORE,
            Node {
                display: Display::None,
                padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                border: UiRect::all(Val::Px(theme::RULE)),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_PANEL)),
                ..default()
            },
            BackgroundColor(theme::panel()),
            BorderColor::all(theme::SAGE),
        ))
        .with_child((
            Text::new("Click explored ground to place a Waypoint"),
            ui::font(theme::TEXT_CAPTION),
            TextColor(theme::INK),
            Pickable::IGNORE,
        ));
    });
}

/// The banner shows in add mode until a click has put a popover on the Map.
fn hint(window: Res<MapWindow>, mut boxes: Query<&mut Node, With<HintBox>>) {
    let display = if window.adding && window.pop.is_none() {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in &mut boxes {
        if node.display != display {
            node.display = display;
        }
    }
}

/// A crosshair while add mode waits for a click, a pointer over a mark, else the grab hand.
fn cursor(
    mut commands: Commands,
    window: Res<MapWindow>,
    marks: Res<Marks>,
    spot: Res<Spot>,
    primary: Single<Entity, With<PrimaryWindow>>,
    mut last: Local<Option<SystemCursorIcon>>,
) {
    let icon = window.pointer.filter(|_| window.open).map(|pixel| {
        if window.adding {
            SystemCursorIcon::Crosshair
        } else if pick(
            &marks,
            pixel,
            window.centre_m(spot.0.map(|(eye, _)| eye)),
            window.metres_per_pixel(),
            window.view,
            HOVER_REACH_PX,
        )
        .is_some()
        {
            SystemCursorIcon::Pointer
        } else {
            SystemCursorIcon::Grab
        }
    });
    if *last == icon {
        return;
    }
    *last = icon;
    commands.entity(*primary).insert(CursorIcon::System(
        icon.unwrap_or(SystemCursorIcon::Default),
    ));
}

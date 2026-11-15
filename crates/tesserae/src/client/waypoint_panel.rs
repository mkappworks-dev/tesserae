//! The Map window's Waypoints: the list, the new-Waypoint form and clicks on the Map view. Every change is
//! an intent to the Server; the list on screen is whatever the Server last sent.

use bevy::asset::RenderAssetUsages;
use bevy::input_focus::{InputFocus, tab_navigation::TabIndex};
use bevy::picking::events::{PointerClick, PointerPress};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::text::{EditableText, TextEdit};
use bevy::ui_widgets::{Activate, TextInput};
use tesserae_core::marker::MAX_NAME_CHARS;

use super::actions::Actions;
use super::buttons::{self, ButtonStyle};
use super::map_window::{MapView, MapWindow, closed, toggle};
use super::menu_ui as ui;
use super::minimap::Spot;
use super::theme;
use super::waypoints::{Draft, Grew, Marks, from_player, icon_pixels, pick};
use crate::server::waypoint::WaypointIntent;
use crate::session::SessionApp;

const LIST_MAX_PX: f32 = 150.0;
const ROW_STEP_PX: f32 = 40.0;
const ROW_ICON_SCALE: u32 = 2;
/// How near a click must land to a mark to select it.
const PICK_REACH_PX: f32 = 12.0;

#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub(super) enum WpControl {
    Select(u32),
    Centre(u32),
    Remove(u32),
    Kind(usize),
    Here,
    Place,
}

#[derive(Component)]
pub(super) struct NameField;
#[derive(Component)]
struct ListBox;
#[derive(Component)]
struct KindRow;
#[derive(Component)]
struct CountText;
#[derive(Component)]
struct HintText;
#[derive(Component)]
struct Distance(u32);

pub(super) fn plugin(app: &mut App) {
    app.add_observer(activate)
        .add_observer(clicked)
        .add_observer(pressed)
        .add_session_systems(
            Update,
            (
                focus.before(toggle),
                (rebuild, distances, hint, clear_name, scroll_list).run_if(not(closed)),
            ),
        );
}

/// Above the Explored section.
pub(super) fn sections(panel: &mut ChildSpawnerCommands) {
    panel
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|section| {
            section
                .spawn(Node {
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                })
                .with_children(|head| {
                    ui::text(head, "Waypoints", theme::TEXT_HEADING, theme::INK);
                    head.spawn((
                        CountText,
                        Text::new(""),
                        ui::font(theme::TEXT_SMALL),
                        TextColor(theme::muted()),
                        Pickable::IGNORE,
                    ));
                });
            section.spawn((
                ListBox,
                ScrollPosition::default(),
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    max_height: Val::Px(LIST_MAX_PX),
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
            ));
        });
    panel
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|form| {
            ui::text(form, "New Waypoint", theme::TEXT_HEADING, theme::INK);
            form.spawn((
                BackgroundColor(theme::FIELD),
                BorderColor::all(theme::field_edge()),
                Node {
                    height: Val::Px(theme::CONTROL_HEIGHT),
                    border: UiRect::all(Val::Px(theme::RULE)),
                    border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
                    padding: UiRect::horizontal(Val::Px(12.0)),
                    align_items: AlignItems::Center,
                    ..default()
                },
            ))
            .with_children(|frame| {
                frame.spawn((
                    NameField,
                    TextInput,
                    EditableText {
                        max_characters: Some(MAX_NAME_CHARS),
                        ..EditableText::new("")
                    },
                    TabIndex(0),
                    ui::font(theme::TEXT_BODY),
                    TextColor(theme::INK),
                    TextLayout::linebreak(bevy::text::LineBreak::NoWrap),
                    Node {
                        flex_grow: 1.0,
                        min_width: Val::Px(0.0),
                        ..default()
                    },
                ));
            });
            form.spawn((
                KindRow,
                Node {
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(4.0),
                    row_gap: Val::Px(4.0),
                    ..default()
                },
            ));
            form.spawn(Node {
                column_gap: Val::Px(8.0),
                ..default()
            })
            .with_children(|row| {
                buttons::button(row, "Here", WpControl::Here, ButtonStyle::Plain);
                buttons::button(
                    row,
                    "Place on map\u{2026}",
                    WpControl::Place,
                    ButtonStyle::Primary,
                );
            });
            form.spawn((
                HintText,
                Text::new(""),
                ui::font(theme::TEXT_SMALL),
                TextColor(theme::muted()),
                Pickable::IGNORE,
            ));
        });
}

type Fields<'w, 's> = Query<'w, 's, &'static EditableText, With<NameField>>;

fn name_of(fields: &Fields) -> String {
    fields
        .single()
        .map(|f| f.value().to_string())
        .unwrap_or_default()
}

fn add_at(
    at: Vec2,
    name: String,
    marks: &Marks,
    draft: &Draft,
    intents: &mut MessageWriter<WaypointIntent>,
) {
    let kind = marks
        .kinds()
        .kinds
        .get(draft.kind)
        .map(|k| k.name.clone())
        .unwrap_or_default();
    intents.write(WaypointIntent::Add {
        x_m: at.x,
        z_m: at.y,
        name,
        kind,
    });
}

#[allow(clippy::too_many_arguments)]
fn activate(
    event: On<Activate>,
    controls: Query<&WpControl>,
    fields: Fields,
    spot: Res<Spot>,
    (mut marks, mut draft, mut window): (ResMut<Marks>, ResMut<Draft>, ResMut<MapWindow>),
    mut intents: MessageWriter<WaypointIntent>,
) {
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    match *control {
        WpControl::Select(id) => marks.select(Some(id)),
        WpControl::Centre(id) => {
            marks.select(Some(id));
            if let Some(w) = marks.get(id) {
                window.centre = Some(Vec2::new(w.x_m, w.z_m));
            }
        }
        WpControl::Remove(id) => {
            intents.write(WaypointIntent::Remove { id });
        }
        WpControl::Kind(index) => draft.kind = index,
        WpControl::Here => {
            if let Some((eye, _)) = spot.0 {
                add_at(
                    Vec2::new(eye.x, eye.z),
                    name_of(&fields),
                    &marks,
                    &draft,
                    &mut intents,
                );
            }
        }
        WpControl::Place => draft.placing = !draft.placing,
    }
}

fn pressed(
    event: On<PointerPress>,
    views: Query<(), With<MapView>>,
    mut window: ResMut<MapWindow>,
) {
    if views.contains(event.entity) {
        window.dragged = false;
    }
}

/// A click on the Map view places a Waypoint when armed, otherwise selects the one under it; a drag
/// that ends over the view is not a click.
fn clicked(
    event: On<PointerClick>,
    views: Query<(), With<MapView>>,
    fields: Fields,
    spot: Res<Spot>,
    (mut marks, draft, window): (ResMut<Marks>, Res<Draft>, Res<MapWindow>),
    mut intents: MessageWriter<WaypointIntent>,
) {
    if !views.contains(event.entity) || window.dragged {
        return;
    }
    let Some(pixel) = window.pointer else {
        return;
    };
    let centre = window.centre_m(spot.0.map(|(eye, _)| eye));
    if draft.placing {
        let at = window.world_at(centre, pixel);
        add_at(at, name_of(&fields), &marks, &draft, &mut intents);
        return;
    }
    let found = pick(
        &marks,
        pixel,
        centre,
        window.metres_per_pixel(),
        super::map_window::view_size(),
        PICK_REACH_PX,
    )
    .map(|w| w.id);
    marks.select(found);
}

/// Typing in the name must not move the Player or close the Map, and Escape leaves the field first.
fn focus(
    focus: Option<ResMut<InputFocus>>,
    fields: Query<(), With<NameField>>,
    ui: Option<ResMut<super::input::UiInput>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut actions: ResMut<Actions>,
    mut mine: Local<bool>,
) {
    let (Some(mut focus), Some(mut ui)) = (focus, ui) else {
        return;
    };
    let on_field = focus.get().is_some_and(|e| fields.contains(e));
    if on_field && keys.just_pressed(KeyCode::Escape) {
        actions.consume_cancel();
        focus.clear();
        ui.text_entry = true;
        *mine = true;
    } else if on_field {
        ui.text_entry = true;
        *mine = true;
    } else if *mine {
        ui.text_entry = false;
        *mine = false;
    }
}

fn clear_name(mut grew: ResMut<Grew>, mut fields: Query<&mut EditableText, With<NameField>>) {
    if !std::mem::take(&mut grew.0) {
        return;
    }
    for mut field in &mut fields {
        field.queue_edit(TextEdit::SelectAll);
        field.queue_edit(TextEdit::Insert("".into()));
    }
}

fn hint(draft: Res<Draft>, marks: Res<Marks>, mut hints: Query<&mut Text, With<HintText>>) {
    let text = if draft.placing {
        "Click explored ground on the Map."
    } else if marks.kinds().kinds.is_empty() {
        "This World declares no Marker kinds."
    } else {
        ""
    };
    for mut hint in &mut hints {
        if hint.0 != text {
            hint.0 = text.into();
        }
    }
}

fn describe(distance_m: f32, bearing: f32) -> String {
    format!(
        "{} m \u{b7} {}",
        distance_m.round() as i32,
        super::compass::cardinal(bearing)
    )
}

fn distances(spot: Res<Spot>, marks: Res<Marks>, mut texts: Query<(&Distance, &mut Text)>) {
    let Some((eye, _)) = spot.0 else {
        return;
    };
    for (Distance(id), mut text) in &mut texts {
        if let Some(w) = marks.get(*id) {
            let (metres, bearing) = from_player(w, Vec2::new(eye.x, eye.z));
            let now = describe(metres, bearing);
            if text.0 != now {
                text.0 = now;
            }
        }
    }
}

fn scroll_list(
    actions: Res<Actions>,
    window: Res<MapWindow>,
    mut lists: Query<&mut ScrollPosition, With<ListBox>>,
) {
    if window.pointer.is_some() {
        return;
    }
    let step = match (actions.wheel.up, actions.wheel.down) {
        (true, _) => -ROW_STEP_PX,
        (_, true) => ROW_STEP_PX,
        _ => return,
    };
    for mut scroll in &mut lists {
        scroll.0.y = (scroll.0.y + step).max(0.0);
    }
}

#[derive(PartialEq)]
struct Built {
    revision: u32,
    kind: usize,
}

#[allow(clippy::too_many_arguments)]
fn rebuild(
    mut commands: Commands,
    marks: Res<Marks>,
    draft: Res<Draft>,
    spot: Res<Spot>,
    mut images: ResMut<Assets<Image>>,
    lists: Query<Entity, With<ListBox>>,
    fresh: Query<(), Added<ListBox>>,
    kinds: Query<Entity, With<KindRow>>,
    mut counts: Query<&mut Text, With<CountText>>,
    mut built: Local<Option<Built>>,
) {
    let now = Built {
        revision: marks.revision(),
        kind: draft.kind,
    };
    if built.as_ref() == Some(&now) && fresh.is_empty() {
        return;
    }
    let (Ok(list), Ok(row)) = (lists.single(), kinds.single()) else {
        return;
    };
    let eye = spot.0.map(|(eye, _)| Vec2::new(eye.x, eye.z));
    for mut count in &mut counts {
        count.0 = marks.list().len().to_string();
    }
    commands
        .entity(list)
        .despawn_children()
        .with_children(|list| {
            if marks.list().is_empty() {
                ui::text(list, "None yet.", theme::TEXT_SMALL, theme::muted());
            }
            for w in marks.list() {
                let (icon, colour) = marks.look(&w.kind);
                let (side, pixels) = icon_pixels(icon, colour, ROW_ICON_SCALE);
                let image = images.add(Image::new(
                    Extent3d {
                        width: side,
                        height: side,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    pixels,
                    TextureFormat::Rgba8UnormSrgb,
                    RenderAssetUsages::default(),
                ));
                let distance = eye.map_or_else(String::new, |eye| {
                    let (metres, bearing) = from_player(w, eye);
                    describe(metres, bearing)
                });
                let selected = marks.selected() == Some(w.id);
                list.spawn((
                    Node {
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(6.0),
                        padding: UiRect::all(Val::Px(3.0)),
                        border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
                        ..default()
                    },
                    BackgroundColor(if selected {
                        theme::SAGE.with_alpha(0.18)
                    } else {
                        Color::NONE
                    }),
                ))
                .with_children(|item| {
                    item.spawn((
                        Pickable::IGNORE,
                        ImageNode::new(image),
                        Node {
                            width: Val::Px(side as f32),
                            height: Val::Px(side as f32),
                            flex_shrink: 0.0,
                            ..default()
                        },
                    ));
                    item.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        flex_grow: 1.0,
                        min_width: Val::Px(0.0),
                        ..default()
                    })
                    .with_children(|names| {
                        buttons::button(
                            names,
                            w.name.clone(),
                            WpControl::Select(w.id),
                            ButtonStyle::Quiet {
                                text_px: theme::TEXT_SMALL,
                            },
                        );
                        names.spawn((
                            Distance(w.id),
                            Text::new(distance),
                            ui::font(theme::TEXT_SMALL),
                            TextColor(theme::muted()),
                            Pickable::IGNORE,
                        ));
                    });
                    buttons::button(
                        item,
                        "Centre",
                        WpControl::Centre(w.id),
                        ButtonStyle::Quiet {
                            text_px: theme::TEXT_SMALL,
                        },
                    );
                    if !w.death {
                        buttons::button(
                            item,
                            "\u{d7}",
                            WpControl::Remove(w.id),
                            ButtonStyle::Quiet {
                                text_px: theme::TEXT_SMALL,
                            },
                        );
                    }
                });
            }
        });
    commands
        .entity(row)
        .despawn_children()
        .with_children(|row| {
            for (index, kind) in marks.choosable() {
                buttons::button(
                    row,
                    ui::declared_title(&kind.name),
                    WpControl::Kind(index),
                    ButtonStyle::Tab {
                        selected: draft.kind == index,
                    },
                );
            }
        });
    *built = Some(now);
}

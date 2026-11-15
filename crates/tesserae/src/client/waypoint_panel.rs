//! The Map window's Waypoints: the list, the new-Waypoint form and clicks on the Map view. Every change is
//! an intent to the Server; the list on screen is whatever the Server last sent.

use bevy::asset::RenderAssetUsages;
use bevy::input_focus::{InputFocus, tab_navigation::TabIndex};
use bevy::picking::events::{PointerClick, PointerPress};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::text::{EditableText, TextEdit, TextLayout};
use bevy::text::{LetterSpacing, LineHeight};
use bevy::ui_widgets::Button;
use bevy::ui_widgets::{Activate, TextInput};
use tesserae_core::marker::MAX_NAME_CHARS;

use super::actions::Actions;
use super::buttons::{self, ButtonStyle};
use super::cinematic::Kind;
use super::map_window::{MapView, MapWindow, SidePanel, closed, toggle};
use super::menu_ui as ui;
use super::minimap::Spot;
use super::theme;
use super::waypoints::{Draft, Grew, Marks, from_player, icon_pixels, pick};
use crate::server::waypoint::WaypointIntent;
use crate::session::SessionApp;

const ROW_STEP_PX: f32 = 40.0;
const ROW_ICON_SCALE: u32 = 2;
const ROW_ICON_PX: f32 = 10.0;
/// How near a click must land to a mark to select it.
const PICK_REACH_PX: f32 = 12.0;
const ICON_DISC: Color = Color::srgb_u8(0x0b, 0x0f, 0x0c);
const KIND_ON: Color = Color::srgb_u8(0x26, 0x33, 0x2a);
const HEADING_TRACKING_PX: f32 = 0.96;

#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub(super) enum WpControl {
    Select(u32),
    Centre(u32),
    Remove(u32),
    Kind(usize),
    AtMyPosition,
    CancelPlace,
}

#[derive(Component)]
pub(super) struct NameField;
#[derive(Component)]
struct NamePlaceholder;
#[derive(Component)]
pub(super) struct NameFrame;
#[derive(Component)]
struct ListBox;
#[derive(Component)]
struct SelectedBox;
#[derive(Component)]
struct KindRow;
#[derive(Component)]
struct CountText;
#[derive(Component)]
struct HintText;
#[derive(Component)]
struct BannerBox;
#[derive(Component)]
struct BannerText;

/// Which line of a Waypoint a text keeps current as the Player moves.
#[derive(Clone, Copy, PartialEq)]
enum Line {
    Row,
    Detail,
}

#[derive(Component)]
struct Distance(u32, Line);

pub(super) fn plugin(app: &mut App) {
    app.add_observer(activate)
        .add_observer(clicked)
        .add_observer(pressed)
        .add_session_systems(
            Update,
            (
                focus.before(toggle),
                (
                    rebuild,
                    distances,
                    hint,
                    clear_name,
                    arm,
                    placeholder,
                    name_presentation,
                    banner_state,
                    scroll_list,
                )
                    .run_if(not(closed)),
            ),
        );
}

/// A panel section; every one after the first is set off by a rule above it.
pub(super) fn section<'a>(panel: &'a mut ChildSpawnerCommands, first: bool) -> EntityCommands<'a> {
    panel.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            flex_shrink: 0.0,
            padding: if first {
                UiRect::ZERO
            } else {
                UiRect::top(Val::Px(10.0))
            },
            border: if first {
                UiRect::ZERO
            } else {
                UiRect::top(Val::Px(theme::RULE))
            },
            ..default()
        },
        BorderColor::all(theme::edge()),
    ))
}

/// A section's small spaced capitals, with an optional count at the far end.
pub(super) fn heading(parent: &mut ChildSpawnerCommands, title: &str, count: bool) {
    parent
        .spawn((
            Pickable::IGNORE,
            Node {
                justify_content: JustifyContent::SpaceBetween,
                margin: UiRect::bottom(Val::Px(8.0)),
                ..default()
            },
        ))
        .with_children(|head| {
            head.spawn((
                Text::new(title.to_uppercase()),
                theme::weighted(theme::TEXT_EYEBROW, theme::LABEL),
                LineHeight::Px(14.0),
                LetterSpacing::Px(HEADING_TRACKING_PX),
                TextColor(theme::muted()),
                Pickable::IGNORE,
            ));
            if count {
                head.spawn((
                    CountText,
                    Text::new(""),
                    theme::weighted(theme::TEXT_EYEBROW, theme::LABEL),
                    LineHeight::Px(14.0),
                    LetterSpacing::Px(HEADING_TRACKING_PX),
                    TextColor(theme::muted()),
                    Pickable::IGNORE,
                ));
            }
        });
}

fn label(parent: &mut ChildSpawnerCommands, value: &str) {
    parent.spawn((
        Text::new(value),
        theme::weighted(theme::TEXT_BODY, theme::LABEL),
        LineHeight::Px(19.0),
        TextColor(theme::INK),
        Pickable::IGNORE,
    ));
}

/// The sections above Explored.
pub(super) fn sections(panel: &mut ChildSpawnerCommands) {
    section(panel, true).with_children(|section| {
        heading(section, "Waypoints", true);
        section.spawn((
            ListBox,
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },
        ));
    });
    section(panel, false).with_children(|form| {
        heading(form, "New Waypoint", false);
        form.spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(10.0),
            margin: UiRect::top(Val::Px(10.0)),
            ..default()
        })
        .with_children(|form| {
            form.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                margin: UiRect::bottom(Val::Px(18.0)),
                ..default()
            })
            .with_children(|field| {
                label(field, "Name");
                name_field(field);
            });
            form.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                margin: UiRect::bottom(Val::Px(18.0)),
                ..default()
            })
            .with_children(|kinds| {
                label(kinds, "Marker kind");
                kinds.spawn((
                    KindRow,
                    Node {
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: Val::Px(6.0),
                        row_gap: Val::Px(6.0),
                        ..default()
                    },
                ));
            });
            form.spawn(Node {
                column_gap: Val::Px(10.0),
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|row| {
                buttons::button(
                    row,
                    "At my position",
                    WpControl::AtMyPosition,
                    ButtonStyle::Small {
                        kind: Kind::Primary,
                    },
                );
            });
            form.spawn((
                HintText,
                Node {
                    display: Display::None,
                    ..default()
                },
                Text::new(""),
                ui::font(theme::TEXT_SMALL),
                TextColor(theme::muted()),
                Pickable::IGNORE,
            ));
        });
    });
    section(panel, false).insert((
        SelectedBox,
        Node {
            display: Display::None,
            flex_direction: FlexDirection::Column,
            flex_shrink: 0.0,
            padding: UiRect::top(Val::Px(10.0)),
            border: UiRect::top(Val::Px(theme::RULE)),
            ..default()
        },
    ));
}

fn name_field(field: &mut ChildSpawnerCommands) {
    field
        .spawn((
            NameFrame,
            theme::focus_ring(),
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
                theme::weighted(theme::TEXT_BODY, theme::LABEL),
                TextColor(theme::INK),
                bevy::text::TextCursorStyle {
                    color: theme::SAGE,
                    selection_color: theme::SAGE.with_alpha(0.3),
                    ..default()
                },
                TextLayout::linebreak(bevy::text::LineBreak::NoWrap),
                Node {
                    flex_grow: 1.0,
                    min_width: Val::Px(0.0),
                    ..default()
                },
            ));
            frame.spawn((
                NamePlaceholder,
                Text::new(""),
                theme::weighted(theme::TEXT_BODY, theme::LABEL),
                TextColor(theme::placeholder()),
                Pickable::IGNORE,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(12.0),
                    ..default()
                },
            ));
        });
}

/// The banner over the Map view while a click is armed to place a Waypoint.
pub(super) fn banner(view: &mut ChildSpawnerCommands) {
    view.spawn((
        BannerBox,
        Pickable::IGNORE,
        Node {
            display: Display::None,
            position_type: PositionType::Absolute,
            top: Val::Px(74.0),
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
    ))
    .with_children(|wrap| {
        wrap.spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                border: UiRect::all(Val::Px(theme::RULE)),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_PANEL)),
                ..default()
            },
            BackgroundColor(theme::panel()),
            BorderColor::all(theme::SAGE),
        ))
        .with_children(|banner| {
            banner.spawn((
                BannerText,
                Text::new(""),
                ui::font(theme::TEXT_CAPTION),
                TextColor(theme::INK),
                Pickable::IGNORE,
            ));
            buttons::button(
                banner,
                "Cancel",
                WpControl::CancelPlace,
                ButtonStyle::Small { kind: Kind::Ghost },
            );
        });
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
        WpControl::AtMyPosition => {
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
        WpControl::CancelPlace => draft.placing = false,
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
        window.view,
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
    (window, mut draft): (Res<MapWindow>, ResMut<Draft>),
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
    } else if window.open && draft.placing && actions.cancel_requested() {
        actions.consume_cancel();
        draft.placing = false;
    } else if on_field {
        ui.text_entry = true;
        *mine = true;
    } else if *mine {
        ui.text_entry = false;
        *mine = false;
    }
}

/// Typing arms placement and clearing the text disarms it; Enter arms it even on a blank name. Only a
/// change in the text arms, so Cancel keeps the text without it re-arming at once.
fn arm(
    fields: Fields,
    focus: Option<Res<InputFocus>>,
    entries: Query<(), With<NameField>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut draft: ResMut<Draft>,
    mut seen: Local<String>,
) {
    let typed = name_of(&fields);
    if *seen != typed {
        draft.placing = !typed.trim().is_empty();
        seen.clone_from(&typed);
    }
    let on_field = focus
        .and_then(|f| f.get())
        .is_some_and(|e| entries.contains(e));
    if on_field && keys.just_pressed(KeyCode::Enter) {
        draft.placing = true;
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

fn hint(marks: Res<Marks>, mut hints: Query<(&mut Text, &mut Node), With<HintText>>) {
    let text = if marks.kinds().kinds.is_empty() {
        "This World declares no Marker kinds."
    } else {
        "Type a name, then click the map to place it."
    };
    for (mut hint, mut node) in &mut hints {
        if hint.0 != text {
            hint.0 = text.into();
        }
        node.display = if text.is_empty() {
            Display::None
        } else {
            Display::Flex
        };
    }
}

/// The name field shows the name a blank one would get until something is typed.
fn placeholder(
    marks: Res<Marks>,
    fields: Query<&EditableText, With<NameField>>,
    mut placeholders: Query<(&mut Text, &mut Visibility), With<NamePlaceholder>>,
) {
    let empty = fields
        .single()
        .is_ok_and(|f| f.value().to_string().is_empty());
    let name = marks.next_default_name();
    for (mut text, mut visibility) in &mut placeholders {
        if text.0 != name {
            text.0.clone_from(&name);
        }
        let shown = if empty {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        visibility.set_if_neq(shown);
    }
}

/// The frame takes the leaf edge and the focus ring while the field has the focus; the edge alone
/// is too near the resting one under stronger contrast.
fn name_presentation(
    focus: Option<Res<InputFocus>>,
    fields: Query<(), With<NameField>>,
    mut frames: Query<(&mut BorderColor, &mut Outline), With<NameFrame>>,
) {
    let on = focus
        .and_then(|f| f.get())
        .is_some_and(|e| fields.contains(e));
    for (mut border, mut outline) in &mut frames {
        border.set_if_neq(BorderColor::all(if on {
            theme::SAGE
        } else {
            theme::field_edge()
        }));
        let ring = if on { theme::FOCUS } else { Color::NONE };
        if outline.color != ring {
            outline.color = ring;
        }
    }
}

fn banner_state(
    draft: Res<Draft>,
    marks: Res<Marks>,
    fields: Fields,
    mut boxes: Query<&mut Node, With<BannerBox>>,
    mut texts: Query<&mut Text, With<BannerText>>,
) {
    let display = if draft.placing {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in &mut boxes {
        node.display = display;
    }
    let typed = name_of(&fields);
    let name = if typed.trim().is_empty() {
        marks.next_default_name()
    } else {
        typed
    };
    let now = format!("Click the map to place \u{201c}{name}\u{201d}.");
    for mut text in &mut texts {
        if text.0 != now {
            text.0.clone_from(&now);
        }
    }
}

fn distance_text(distance_m: f32) -> String {
    if distance_m >= 1000.0 {
        format!("{:.2} km", distance_m / 1000.0)
    } else {
        format!("{} m", distance_m.round() as i32)
    }
}

/// `Kind \u{b7} 403 m SW`, or the kind alone while the Player's place is unknown.
fn describe(kind: &str, from: Option<(f32, f32)>) -> String {
    match from {
        Some((metres, bearing)) => format!(
            "{} \u{b7} {} {}",
            ui::declared_title(kind),
            distance_text(metres),
            super::compass::cardinal(bearing)
        ),
        None => ui::declared_title(kind),
    }
}

fn signed(n: f32) -> String {
    let n = n.round() as i32;
    if n < 0 {
        format!("\u{2212}{}", n.unsigned_abs())
    } else {
        n.to_string()
    }
}

/// `X 72 \u{b7} Z 66 \u{b7} bearing 225\u{b0} SW` for the selected Waypoint.
fn detail(w: &tesserae_core::marker::Waypoint, from: Option<(f32, f32)>) -> String {
    let place = format!("X {} \u{b7} Z {}", signed(w.x_m), signed(w.z_m));
    match from {
        Some((_, bearing)) => format!(
            "{place} \u{b7} bearing {}\u{b0} {}",
            bearing.round() as i32 % 360,
            super::compass::cardinal(bearing)
        ),
        None => place,
    }
}

fn distances(spot: Res<Spot>, marks: Res<Marks>, mut texts: Query<(&Distance, &mut Text)>) {
    let Some((eye, _)) = spot.0 else {
        return;
    };
    for (Distance(id, line), mut text) in &mut texts {
        if let Some(w) = marks.get(*id) {
            let from = Some(from_player(w, Vec2::new(eye.x, eye.z)));
            let now = match line {
                Line::Row => describe(&w.kind, from),
                Line::Detail => detail(w, from),
            };
            if text.0 != now {
                text.0 = now;
            }
        }
    }
}

fn scroll_list(
    actions: Res<Actions>,
    window: Res<MapWindow>,
    mut panels: Query<&mut ScrollPosition, With<SidePanel>>,
) {
    if window.pointer.is_some() {
        return;
    }
    let step = match (actions.wheel.up, actions.wheel.down) {
        (true, _) => -ROW_STEP_PX,
        (_, true) => ROW_STEP_PX,
        _ => return,
    };
    for mut scroll in &mut panels {
        scroll.0.y = (scroll.0.y + step).max(0.0);
    }
}

#[derive(PartialEq)]
struct Built {
    revision: u32,
    kind: usize,
}

type Boxes<'w, 's> = (
    Query<'w, 's, Entity, With<ListBox>>,
    Query<'w, 's, Entity, With<KindRow>>,
    Query<'w, 's, Entity, With<SelectedBox>>,
);

/// A kind's icon as an image node `side` pixels square.
fn icon_node(
    row: &mut ChildSpawnerCommands,
    images: &mut Assets<Image>,
    marks: &Marks,
    kind: &str,
    scale: u32,
    side: f32,
) {
    let (icon, colour) = marks.look(kind);
    let (px, pixels) = icon_pixels(icon, colour, scale);
    let image = images.add(Image::new(
        Extent3d {
            width: px,
            height: px,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ));
    row.spawn((
        Pickable::IGNORE,
        ImageNode::new(image),
        Node {
            width: Val::Px(side),
            height: Val::Px(side),
            flex_shrink: 0.0,
            ..default()
        },
    ));
}

fn colour_of(marks: &Marks, kind: &str) -> Color {
    let [r, g, b, a] = marks.look(kind).1;
    Color::srgba_u8(r, g, b, a)
}

#[allow(clippy::too_many_arguments)]
fn rebuild(
    mut commands: Commands,
    marks: Res<Marks>,
    draft: Res<Draft>,
    spot: Res<Spot>,
    mut images: ResMut<Assets<Image>>,
    (lists, kinds, selected_box): Boxes,
    fresh: Query<(), Added<ListBox>>,
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
    let (Ok(list), Ok(row), Ok(detail_box)) =
        (lists.single(), kinds.single(), selected_box.single())
    else {
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
                let selected = marks.selected() == Some(w.id);
                list.spawn((
                    Node {
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(theme::RULE)),
                        border_radius: BorderRadius::all(Val::Px(theme::RADIUS_ROW)),
                        ..default()
                    },
                    BackgroundColor(if selected { theme::RAISED } else { Color::NONE }),
                    BorderColor::all(if selected { theme::SAGE } else { Color::NONE }),
                ))
                .with_children(|item| {
                    item.spawn((
                        Button,
                        TabIndex(0),
                        WpControl::Select(w.id),
                        Node {
                            flex_grow: 1.0,
                            min_width: Val::Px(0.0),
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(10.0),
                            padding: UiRect::axes(Val::Px(8.0), Val::Px(7.0)),
                            ..default()
                        },
                    ))
                    .with_children(|button| {
                        button
                            .spawn((
                                Pickable::IGNORE,
                                Node {
                                    width: Val::Px(26.0),
                                    height: Val::Px(26.0),
                                    flex_shrink: 0.0,
                                    border: UiRect::all(Val::Px(2.0)),
                                    border_radius: BorderRadius::MAX,
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(ICON_DISC),
                                BorderColor::all(colour_of(&marks, &w.kind)),
                            ))
                            .with_children(|ring| {
                                icon_node(
                                    ring,
                                    &mut images,
                                    &marks,
                                    &w.kind,
                                    ROW_ICON_SCALE,
                                    ROW_ICON_PX,
                                );
                            });
                        button
                            .spawn((
                                Pickable::IGNORE,
                                Node {
                                    flex_direction: FlexDirection::Column,
                                    min_width: Val::Px(0.0),
                                    overflow: Overflow::clip(),
                                    ..default()
                                },
                            ))
                            .with_children(|names| {
                                names.spawn((
                                    Text::new(w.name.clone()),
                                    theme::weighted(theme::TEXT_SMALL, theme::LABEL),
                                    LineHeight::Px(17.0),
                                    TextLayout::linebreak(bevy::text::LineBreak::NoWrap),
                                    TextColor(theme::INK),
                                    Pickable::IGNORE,
                                ));
                                names.spawn((
                                    Distance(w.id, Line::Row),
                                    Text::new(describe(
                                        &w.kind,
                                        eye.map(|eye| from_player(w, eye)),
                                    )),
                                    theme::weighted(theme::TEXT_TAG, theme::REGULAR),
                                    LineHeight::Px(17.6),
                                    TextLayout::linebreak(bevy::text::LineBreak::NoWrap),
                                    TextColor(theme::muted()),
                                    Pickable::IGNORE,
                                ));
                            });
                    });
                    if !w.death {
                        item.spawn((
                            Button,
                            TabIndex(0),
                            WpControl::Remove(w.id),
                            Node {
                                padding: UiRect::axes(Val::Px(10.0), Val::Px(8.0)),
                                ..default()
                            },
                        ))
                        .with_children(|remove| {
                            ui::icon(remove, ui::IconKind::Close, theme::muted(), 12.0);
                        });
                    }
                });
            }
        });
    commands
        .entity(row)
        .despawn_children()
        .with_children(|row| {
            for (index, kind) in marks.choosable() {
                let on = draft.kind == index;
                row.spawn((
                    Button,
                    TabIndex(0),
                    WpControl::Kind(index),
                    Node {
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(6.0),
                        padding: UiRect::axes(Val::Px(9.0), Val::Px(5.0)),
                        border: UiRect::all(Val::Px(theme::RULE)),
                        border_radius: BorderRadius::MAX,
                        ..default()
                    },
                    BackgroundColor(if on { KIND_ON } else { theme::RAISED }),
                    BorderColor::all(if on { theme::SAGE } else { theme::edge() }),
                ))
                .with_children(|pill| {
                    icon_node(pill, &mut images, &marks, &kind.name, 1, 7.0);
                    pill.spawn((
                        Text::new(ui::declared_title(&kind.name)),
                        theme::weighted(theme::TEXT_EYEBROW, theme::LABEL),
                        LineHeight::Px(15.0),
                        TextColor(theme::INK),
                        Pickable::IGNORE,
                    ));
                });
            }
        });
    let chosen = marks.selected().and_then(|id| marks.get(id));
    commands
        .entity(detail_box)
        .despawn_children()
        .insert(Node {
            display: if chosen.is_some() {
                Display::Flex
            } else {
                Display::None
            },
            flex_direction: FlexDirection::Column,
            flex_shrink: 0.0,
            padding: UiRect::top(Val::Px(10.0)),
            border: UiRect::top(Val::Px(theme::RULE)),
            ..default()
        })
        .with_children(|section| {
            let Some(w) = chosen else {
                return;
            };
            heading(section, &w.name, false);
            section.spawn((
                Distance(w.id, Line::Detail),
                Text::new(detail(w, eye.map(|eye| from_player(w, eye)))),
                theme::figures(theme::TEXT_EYEBROW, theme::REGULAR),
                LineHeight::RelativeToFont(theme::LINE_HEIGHT),
                TextColor(theme::muted()),
                Pickable::IGNORE,
                Node {
                    margin: UiRect::bottom(Val::Px(8.0)),
                    ..default()
                },
            ));
            let centre = buttons::button(
                section,
                "Centre",
                WpControl::Centre(w.id),
                ButtonStyle::Small { kind: Kind::Plain },
            );
            section.commands().entity(centre).insert(Node {
                align_self: AlignSelf::FlexStart,
                min_height: Val::Px(theme::CONTROL_HEIGHT),
                padding: UiRect::axes(Val::Px(theme::SMALL_PAD_X), Val::Px(theme::SMALL_PAD_Y)),
                border: UiRect::all(Val::Px(theme::RULE)),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_CONTROL)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_shrink: 0.0,
                ..default()
            });
        });
    *built = Some(now);
}

//! Creative's Time of day: a dialog over the Menu of four hours, each with a scene of its sky, and a
//! custom one. The Client only asks; the Server moves its clock and sends it back (ADR-0002).

use bevy::input_focus::tab_navigation::{TabGroup, TabIndex};
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::prelude::*;
use bevy::text::{EditableText, TextCursorStyle, TextEdit, TextLayout};
use bevy::ui_widgets::{Activate, Button, TextInput};
use tesserae_core::sky::MINUTES_PER_DAY;

use super::actions::Actions;
use super::buttons::{self, ButtonStyle};
use super::cinematic as cine;
use super::menu::{Choice, Menu, MenuRoot};
use super::menu_ui as ui;
use super::theme;
use super::time_of_day::{ClientTimeOfDay, clock_label};
use crate::server::time_of_day::SetTimeOfDay;
use crate::session::SessionApp;

const QUARTER_HOUR: u16 = 15;
const DIALOG_WIDTH_PX: f32 = 600.0;
const TILE_GAP_PX: f32 = 10.0;
const THUMB_HEIGHT_PX: f32 = 64.0;
const CONTROL_PX: f32 = 32.0;
const FIELD_WIDTH_PX: f32 = 72.0;
const LANDS_WIDTH_PX: f32 = 96.0;
const SELECTED_RING_PX: f32 = 2.0;
/// The prototype draws each scene on a 160 by 80 canvas, cropped to the tile's width as an SVG's
/// `slice` does.
const SCENE: Vec2 = Vec2::new(160.0, 80.0);
const SCENE_SCALE: f32 = THUMB_HEIGHT_PX / SCENE.y;
/// The prototype's stepped hills: each step's left and right x and its top, down to the bottom.
const HILL: [(f32, f32, f32); 9] = [
    (0.0, 16.0, 58.0),
    (16.0, 30.0, 52.0),
    (30.0, 48.0, 44.0),
    (48.0, 60.0, 50.0),
    (60.0, 80.0, 40.0),
    (80.0, 94.0, 48.0),
    (94.0, 112.0, 54.0),
    (112.0, 128.0, 50.0),
    (128.0, 160.0, 58.0),
];
const NEAR_HILL: [(f32, f32, f32); 6] = [
    (0.0, 22.0, 68.0),
    (22.0, 48.0, 64.0),
    (48.0, 78.0, 70.0),
    (78.0, 102.0, 64.0),
    (102.0, 122.0, 68.0),
    (122.0, 160.0, 62.0),
];
const STARS: [(f32, f32); 9] = [
    (14.0, 10.0),
    (30.0, 24.0),
    (52.0, 8.0),
    (70.0, 30.0),
    (88.0, 12.0),
    (140.0, 26.0),
    (150.0, 8.0),
    (44.0, 38.0),
    (128.0, 40.0),
];
const STAR_PX: f32 = 1.5;
const GLOW_SCALE: f32 = 2.2;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Pick {
    Sunrise,
    Noon,
    Sunset,
    Midnight,
    Custom,
}

/// The four tiles in order, with their minute of the day; each indexes [`theme::TIME_SCENES`].
const HOURS: [(Pick, &str, u16); 4] = [
    (Pick::Sunrise, "Sunrise", 6 * 60),
    (Pick::Noon, "Noon", 12 * 60),
    (Pick::Sunset, "Sunset", 18 * 60),
    (Pick::Midnight, "Midnight", 0),
];

const INVALID: &str = "Type a time from 00:00 to 23:59.";
const RUNNING_NOTE: &str =
    "The clock moves on to the hour you pick, and the day count carries on from there.";
const HELD_NOTE: &str = "The Daylight cycle is off, so the sky holds at the hour you pick.";

/// What the open dialog has picked; reset each time it opens.
#[derive(Resource, Clone, Copy, Debug)]
pub(super) struct TimeDialog {
    pub(super) pick: Pick,
    /// The Custom row's minute of the day.
    pub(super) custom: u16,
}

impl Default for TimeDialog {
    fn default() -> Self {
        Self {
            pick: Pick::Noon,
            custom: 0,
        }
    }
}

impl TimeDialog {
    fn minute(&self, pick: Pick) -> u16 {
        HOURS
            .iter()
            .find(|(p, ..)| *p == pick)
            .map_or(self.custom, |(_, _, minute)| *minute)
    }
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum TimeControl {
    Hour(Pick),
    Earlier,
    Later,
    Cancel,
    Set,
}

#[derive(Component)]
struct TimeRoot;

#[derive(Component)]
pub(super) struct CustomField;

#[derive(Component)]
struct CustomRow;

/// Where a pick lands, "Today, 18:00" or "Day 4, 06:00", kept in step with the clock.
#[derive(Component)]
struct Lands(Pick);

#[derive(Component)]
struct Note;

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<TimeDialog>()
        .add_session_message::<SetTimeOfDay>()
        .add_observer(open)
        .add_observer(activate)
        .add_session_systems(
            Update,
            (
                (text_entry, escape)
                    .chain()
                    .before(super::menu::toggle)
                    .run_if(crate::server::persistence::playing),
                (render, commit_field, refresh).chain(),
            ),
        );
}

fn hhmm(minute: u16) -> String {
    format!("{:02}:{:02}", minute / 60, minute % 60)
}

/// `HH:MM`, `HHMM` or a bare hour, as the prototype's field takes them.
fn parse_hhmm(text: &str) -> Option<u16> {
    let text = text.trim();
    // ASCII from here on, so splitting at a byte never lands inside a character.
    if !text.bytes().all(|b| b.is_ascii_digit() || b == b':') {
        return None;
    }
    let (hour, minute) = match text.split_once(':') {
        Some((hour, minute)) if minute.len() == 2 => (hour, minute),
        Some(_) => return None,
        None if text.len() <= 2 => (text, "00"),
        None if text.len() <= 4 => text.split_at(text.len() - 2),
        None => return None,
    };
    let digits = |s: &str| (1..=2).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit());
    if !digits(hour) || !digits(minute) {
        return None;
    }
    let (hour, minute): (u16, u16) = (hour.parse().ok()?, minute.parse().ok()?);
    (hour < 24 && minute < 60).then_some(hour * 60 + minute)
}

/// "Today, 18:00" while `minute` is still ahead of the clock today, else the day it lands on.
fn lands(clock: Option<&ClientTimeOfDay>, minute: u16) -> String {
    let at = clock
        .and_then(|c| Some((c.time(), c.sky()?)))
        .map(|(now, sky)| (now.day(sky), now.next_at(minute, sky).day(sky)));
    match at {
        Some((today, day)) if day != today => format!("Day {}, {}", day + 1, hhmm(minute)),
        _ => format!("Today, {}", hhmm(minute)),
    }
}

fn now_minute(clock: Option<&ClientTimeOfDay>) -> u16 {
    clock.map_or(0, |c| (c.hour() * 60.0).floor() as u16 % MINUTES_PER_DAY)
}

fn open(
    event: On<Activate>,
    choices: Query<&Choice>,
    clock: Option<Res<ClientTimeOfDay>>,
    mut menu: ResMut<Menu>,
    mut dialog: ResMut<TimeDialog>,
) {
    if choices.get(event.entity) != Ok(&Choice::TimeOfDay) {
        return;
    }
    menu.time = true;
    let now = now_minute(clock.as_deref());
    *dialog = TimeDialog {
        pick: Pick::Noon,
        custom: now.div_ceil(QUARTER_HOUR) * QUARTER_HOUR % MINUTES_PER_DAY,
    };
}

type Fields<'w, 's> = Query<'w, 's, (Entity, &'static mut EditableText), With<CustomField>>;

#[allow(clippy::too_many_arguments)]
fn activate(
    event: On<Activate>,
    controls: Query<&TimeControl>,
    mut commands: Commands,
    clock: Option<Res<ClientTimeOfDay>>,
    (mut menu, mut dialog): (ResMut<Menu>, ResMut<TimeDialog>),
    mut fields: Fields,
    focus: Res<InputFocus>,
    mut requests: MessageWriter<SetTimeOfDay>,
) {
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    match *control {
        TimeControl::Hour(pick) => dialog.pick = pick,
        TimeControl::Earlier | TimeControl::Later => {
            let by = if *control == TimeControl::Later {
                QUARTER_HOUR
            } else {
                MINUTES_PER_DAY - QUARTER_HOUR
            };
            dialog.custom = (dialog.custom + by) % MINUTES_PER_DAY;
            dialog.pick = Pick::Custom;
            if let Ok((_, mut field)) = fields.single_mut() {
                replace(&mut field, &hhmm(dialog.custom));
            }
        }
        TimeControl::Cancel => menu.time = false,
        TimeControl::Set => {
            // Leaving the field took what it held; while it still has focus, Set takes it now.
            if let Ok((entity, mut field)) = fields.single_mut()
                && focus.get() == Some(entity)
            {
                commit(&mut commands, &mut dialog, &mut field);
            }
            let minute = dialog.minute(dialog.pick);
            requests.write(SetTimeOfDay { minute });
            menu.time = false;
            let name = HOURS
                .iter()
                .find(|(p, ..)| *p == dialog.pick)
                .map_or_else(|| hhmm(minute), |(_, label, _)| label.to_lowercase());
            let landing = clock
                .as_deref()
                .and_then(|c| Some((c.time(), c.sky()?)))
                .map(|(now, sky)| {
                    let at = now.next_at(minute, sky);
                    clock_label(at.day(sky), at.hour(sky))
                });
            let said = match landing {
                Some(landing) => format!("Time set to {name}, {landing}."),
                None => format!("Time set to {name}."),
            };
            ui::notify(&mut commands, &said, ui::Tone::Change);
        }
    }
}

fn replace(field: &mut EditableText, value: &str) {
    field.queue_edit(TextEdit::SelectAll);
    field.queue_edit(TextEdit::Insert(value.into()));
}

/// Takes what was typed in the Custom field, or puts its time back if it is not one.
fn commit(commands: &mut Commands, dialog: &mut TimeDialog, field: &mut EditableText) {
    let typed = field.value().to_string();
    if typed == hhmm(dialog.custom) {
        return;
    }
    match parse_hhmm(&typed) {
        Some(minute) => {
            dialog.custom = minute;
            dialog.pick = Pick::Custom;
            replace(field, &hhmm(minute));
        }
        None => {
            replace(field, &hhmm(dialog.custom));
            ui::notify(commands, INVALID, ui::Tone::Alert);
        }
    }
}

/// Typing in the field must not move the Player or close the Menu.
fn text_entry(
    focus: Option<Res<InputFocus>>,
    fields: Query<(), With<CustomField>>,
    ui: Option<ResMut<super::input::UiInput>>,
    mut mine: Local<bool>,
) {
    let (Some(focus), Some(mut ui)) = (focus, ui) else {
        return;
    };
    if focus.get().is_some_and(|e| fields.contains(e)) {
        ui.text_entry = true;
        *mine = true;
    } else if *mine {
        ui.text_entry = false;
        *mine = false;
    }
}

/// Escape closes the dialog, from the field too, and leaves the Menu open.
fn escape(keys: Res<ButtonInput<KeyCode>>, mut actions: ResMut<Actions>, mut menu: ResMut<Menu>) {
    if menu.open && menu.time && (actions.cancel_requested() || keys.just_pressed(KeyCode::Escape))
    {
        actions.consume_cancel();
        menu.time = false;
    }
}

/// Built once as it opens; [`refresh`] keeps it in step after. The Menu under it gives up its
/// modal Tab group while it shows, and takes focus back on its item as it closes.
fn render(
    mut commands: Commands,
    menu: Res<Menu>,
    settings: Option<Res<super::settings::Settings>>,
    (dialog, clock): (Res<TimeDialog>, Option<Res<ClientTimeOfDay>>),
    roots: Query<Entity, With<TimeRoot>>,
    mut groups: Query<&mut TabGroup, With<MenuRoot>>,
    (choices, mut focus): (Query<(Entity, &Choice)>, ResMut<InputFocus>),
) {
    let wanted = menu.open && menu.time && !settings.is_some_and(|s| s.open);
    let shown = !roots.is_empty();
    for mut group in &mut groups {
        let modal = !wanted && !menu.details;
        if group.modal != modal {
            group.modal = modal;
        }
    }
    if wanted == shown {
        return;
    }
    if !wanted {
        for root in &roots {
            commands.entity(root).despawn();
        }
        if let Some((item, _)) = choices.iter().find(|(_, c)| **c == Choice::TimeOfDay)
            && menu.open
        {
            focus.set(item, FocusCause::Navigated);
        }
        return;
    }
    spawn(&mut commands, &dialog, clock.as_deref());
}

fn spawn(commands: &mut Commands, dialog: &TimeDialog, clock: Option<&ClientTimeOfDay>) {
    commands
        .spawn((
            TimeRoot,
            super::frame::Frame,
            GlobalZIndex(9),
            BackgroundColor(theme::details_overlay()),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_children(|center| {
            center
                .spawn((
                    TabGroup::modal(),
                    super::frame::left_to_right(theme::details_shade(), theme::details_shade_far()),
                    Node {
                        width: Val::Px(DIALOG_WIDTH_PX),
                        max_width: Val::Percent(92.0),
                        padding: UiRect::axes(Val::Px(28.0), Val::Px(24.0)),
                        flex_direction: FlexDirection::Column,
                        border_radius: BorderRadius::all(Val::Px(theme::RADIUS_PANEL)),
                        ..default()
                    },
                ))
                .with_children(|d| {
                    cine::dialog_title(d, "Time of day");
                    tiles(d, dialog, clock);
                    custom_row(d, dialog, clock);
                    d.spawn((
                        Note,
                        Text::new(note(clock)),
                        theme::font(theme::TEXT_EYEBROW),
                        bevy::text::LineHeight::RelativeToFont(1.8),
                        TextColor(theme::muted()),
                        Pickable::IGNORE,
                    ));
                    d.spawn(Node {
                        margin: UiRect::top(Val::Px(7.0)),
                        ..cine::dialog_actions()
                    })
                    .with_children(|row| {
                        buttons::button(row, "Cancel", TimeControl::Cancel, ButtonStyle::Plain);
                        buttons::button(row, "Set time", TimeControl::Set, ButtonStyle::Primary);
                    });
                });
        });
}

fn note(clock: Option<&ClientTimeOfDay>) -> &'static str {
    if clock.is_some_and(|c| c.runs()) {
        RUNNING_NOTE
    } else {
        HELD_NOTE
    }
}

fn tiles(d: &mut ChildSpawnerCommands, dialog: &TimeDialog, clock: Option<&ClientTimeOfDay>) {
    d.spawn(Node {
        display: Display::Grid,
        grid_template_columns: vec![RepeatedGridTrack::flex(4, 1.0)],
        column_gap: Val::Px(TILE_GAP_PX),
        margin: UiRect::new(Val::ZERO, Val::ZERO, Val::Px(16.0), Val::Px(14.0)),
        ..default()
    })
    .with_children(|grid| {
        for ((pick, label, minute), scene) in HOURS.iter().zip(&theme::TIME_SCENES) {
            let mut tile = grid.spawn((
                Button,
                TabIndex(0),
                TimeControl::Hour(*pick),
                BackgroundColor(theme::TIME_TILE),
                BorderColor::all(theme::TIME_TILE_EDGE),
                Outline::new(Val::Px(SELECTED_RING_PX), Val::Px(1.0), Color::NONE),
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexStart,
                    row_gap: Val::Px(4.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    border: UiRect::all(Val::Px(theme::RULE)),
                    border_radius: BorderRadius::all(Val::Px(theme::RADIUS_ROW)),
                    min_width: Val::Px(0.0),
                    ..default()
                },
            ));
            if *pick == dialog.pick {
                tile.insert(super::frame::Autofocus);
            }
            tile.with_children(|tile| {
                thumbnail(tile, scene);
                tile.spawn((
                    Text::new(*label),
                    theme::font(theme::TEXT_BODY),
                    TextColor(theme::INK),
                    Pickable::IGNORE,
                ));
                tile.spawn((
                    Lands(*pick),
                    Text::new(lands(clock, *minute)),
                    theme::tabular(theme::font(theme::TEXT_CAPTION)),
                    TextColor(theme::TIME_TILE_SOFT),
                    Pickable::IGNORE,
                ));
            });
        }
    });
}

/// The sky at that hour over two stepped hills, with the sun or moon where it stands.
fn thumbnail(tile: &mut ChildSpawnerCommands, scene: &theme::TimeScene) {
    let px = |v: f32| Val::Px(v * SCENE_SCALE);
    let rect = |x: f32, y: f32, w: f32, h: f32| Node {
        position_type: PositionType::Absolute,
        left: px(x),
        top: px(y),
        width: px(w),
        height: px(h),
        ..default()
    };
    tile.spawn((
        Pickable::IGNORE,
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(THUMB_HEIGHT_PX),
            margin: UiRect::bottom(Val::Px(6.0)),
            justify_content: JustifyContent::Center,
            overflow: Overflow::clip(),
            border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
            ..default()
        },
    ))
    .with_children(|frame| {
        frame
            .spawn((
                Pickable::IGNORE,
                BackgroundGradient::from(LinearGradient::to_bottom(vec![
                    ColorStop::new(scene.sky[0], Val::Percent(0.0)),
                    ColorStop::new(scene.sky[1], Val::Percent(65.0)),
                    ColorStop::new(scene.sky[2], Val::Percent(100.0)),
                ])),
                Node {
                    width: px(SCENE.x),
                    height: px(SCENE.y),
                    flex_shrink: 0.0,
                    ..default()
                },
            ))
            .with_children(|canvas| {
                if scene.stars {
                    for (x, y) in STARS {
                        canvas.spawn((
                            Pickable::IGNORE,
                            BackgroundColor(theme::TIME_STAR),
                            rect(x, y, STAR_PX, STAR_PX),
                        ));
                    }
                }
                let [x, y, r] = scene.body;
                let disc = |r: f32| Node {
                    border_radius: BorderRadius::MAX,
                    ..rect(x - r, y - r, 2.0 * r, 2.0 * r)
                };
                if scene.glow {
                    canvas.spawn((
                        Pickable::IGNORE,
                        BackgroundColor(scene.body_ink.with_alpha(theme::TIME_GLOW_ALPHA)),
                        disc(r * GLOW_SCALE),
                    ));
                }
                canvas.spawn((Pickable::IGNORE, BackgroundColor(scene.body_ink), disc(r)));
                for (steps, colour) in [(&HILL[..], scene.hill), (&NEAR_HILL[..], scene.near)] {
                    for &(left, right, top) in steps {
                        canvas.spawn((
                            Pickable::IGNORE,
                            BackgroundColor(colour),
                            rect(left, top, right - left, SCENE.y - top),
                        ));
                    }
                }
            });
    });
}

fn custom_row(d: &mut ChildSpawnerCommands, dialog: &TimeDialog, clock: Option<&ClientTimeOfDay>) {
    let control = |fill: Color| {
        (
            BackgroundColor(fill),
            BorderColor::all(theme::TIME_TILE_EDGE),
            theme::focus_ring(),
        )
    };
    let control_node = |width: f32| Node {
        width: Val::Px(width),
        height: Val::Px(CONTROL_PX),
        border: UiRect::all(Val::Px(theme::RULE)),
        border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    };
    let step = |row: &mut ChildSpawnerCommands, glyph: &str, act: TimeControl| {
        row.spawn((Button, TabIndex(0), act, control(theme::TIME_STEP)))
            .insert(control_node(CONTROL_PX))
            .with_children(|b| {
                b.spawn((
                    Text::new(glyph),
                    theme::font(theme::TEXT_CAPTION),
                    TextColor(theme::INK),
                    Pickable::IGNORE,
                ));
            });
    };
    d.spawn((
        CustomRow,
        BackgroundColor(theme::TIME_TILE),
        BorderColor::all(theme::TIME_TILE_EDGE),
        Outline::new(Val::Px(SELECTED_RING_PX), Val::Px(1.0), Color::NONE),
        Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            padding: UiRect::axes(Val::Px(10.0), Val::Px(8.0)),
            margin: UiRect::bottom(Val::Px(14.0)),
            border: UiRect::all(Val::Px(theme::RULE)),
            border_radius: BorderRadius::all(Val::Px(theme::RADIUS_ROW)),
            ..default()
        },
    ))
    .with_children(|row| {
        row.spawn((
            Text::new("Custom"),
            theme::font(theme::TEXT_BODY),
            TextColor(theme::INK),
            Pickable::IGNORE,
            Node {
                flex_grow: 1.0,
                ..default()
            },
        ));
        step(row, "\u{25c0}", TimeControl::Earlier);
        row.spawn((control(theme::TIME_FIELD), control_node(FIELD_WIDTH_PX)))
            .with_children(|frame| {
                frame.spawn((
                    CustomField,
                    TextInput,
                    EditableText {
                        max_characters: Some(5),
                        ..EditableText::new(hhmm(dialog.custom))
                    },
                    TabIndex(0),
                    theme::tabular(theme::font(theme::TEXT_BODY)),
                    TextColor(theme::INK),
                    TextCursorStyle {
                        color: theme::SAGE,
                        selection_color: theme::SAGE.with_alpha(0.3),
                        ..default()
                    },
                    TextLayout::new(Justify::Center, bevy::text::LineBreak::NoWrap),
                    Node {
                        width: Val::Percent(100.0),
                        ..default()
                    },
                ));
            });
        step(row, "\u{25b6}", TimeControl::Later);
        row.spawn((
            Lands(Pick::Custom),
            Text::new(lands(clock, dialog.custom)),
            theme::tabular(theme::font(theme::TEXT_CAPTION)),
            TextColor(theme::TIME_TILE_SOFT),
            TextLayout::justify(Justify::Right),
            Pickable::IGNORE,
            Node {
                min_width: Val::Px(LANDS_WIDTH_PX),
                ..default()
            },
        ));
    });
}

/// The field's text is taken as it loses focus or on Enter, as the prototype's `change` does.
fn commit_field(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    focus: Res<InputFocus>,
    mut fields: Fields,
    mut dialog: ResMut<TimeDialog>,
    mut was: Local<bool>,
) {
    let Ok((entity, mut field)) = fields.single_mut() else {
        *was = false;
        return;
    };
    let focused = focus.get() == Some(entity);
    if (*was && !focused) || (focused && keys.just_pressed(KeyCode::Enter)) {
        commit(&mut commands, &mut dialog, &mut field);
    }
    *was = focused;
}

type Tiles<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static TimeControl,
        &'static mut BorderColor,
        &'static mut Outline,
    ),
    Without<CustomRow>,
>;

type Texts<'w, 's> =
    Query<'w, 's, (&'static mut Text, Option<&'static Lands>), Or<(With<Lands>, With<Note>)>>;

/// The picked tile and the focused one are ringed; every landing follows the clock.
fn refresh(
    dialog: Res<TimeDialog>,
    clock: Option<Res<ClientTimeOfDay>>,
    (focus, visible): (Res<InputFocus>, Option<Res<InputFocusVisible>>),
    mut tiles: Tiles,
    mut rows: Query<(Entity, &mut BorderColor, &mut Outline), With<CustomRow>>,
    mut texts: Texts,
    parents: Query<&ChildOf>,
) {
    if rows.is_empty() {
        return;
    }
    let clock = clock.as_deref();
    let ringed = ui::visible_focus(Some(&focus), visible.as_deref());
    let lit = |on: bool| {
        if on {
            (theme::TIME_TILE_ON, theme::TIME_TILE_ON)
        } else {
            (theme::TIME_TILE_EDGE, Color::NONE)
        }
    };
    for (entity, control, mut border, mut outline) in &mut tiles {
        let (edge, ring) = match control {
            TimeControl::Hour(pick) => lit(*pick == dialog.pick || ringed == Some(entity)),
            TimeControl::Earlier | TimeControl::Later => (
                theme::TIME_TILE_EDGE,
                if ringed == Some(entity) {
                    theme::FOCUS
                } else {
                    Color::NONE
                },
            ),
            TimeControl::Cancel | TimeControl::Set => continue,
        };
        border.set_if_neq(BorderColor::all(edge));
        if outline.color != ring {
            outline.color = ring;
        }
    }
    for (row, mut border, mut outline) in &mut rows {
        let within = focus
            .get()
            .is_some_and(|f| parents.iter_ancestors(f).any(|a| a == row));
        let (edge, ring) = lit(dialog.pick == Pick::Custom || within);
        border.set_if_neq(BorderColor::all(edge));
        if outline.color != ring {
            outline.color = ring;
        }
    }
    for (mut text, landing) in &mut texts {
        let wanted = match landing {
            Some(Lands(pick)) => lands(clock, dialog.minute(*pick)),
            None => note(clock).to_owned(),
        };
        if text.0 != wanted {
            text.0 = wanted;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_field_takes_hh_mm_hhmm_and_a_bare_hour() {
        for (typed, minute) in [
            ("14:30", 14 * 60 + 30),
            ("1430", 14 * 60 + 30),
            ("14", 14 * 60),
            ("9", 9 * 60),
            ("9:05", 9 * 60 + 5),
            (" 07:45 ", 7 * 60 + 45),
            ("00:00", 0),
            ("23:59", 23 * 60 + 59),
        ] {
            assert_eq!(parse_hhmm(typed), Some(minute), "{typed}");
        }
        for bad in [
            "", "24:00", "12:60", "dusk", "1:2", "12345", ":30", "12:", "-1", "1:2:3", "\u{e9}1",
        ] {
            assert_eq!(parse_hhmm(bad), None, "{bad}");
        }
    }
}

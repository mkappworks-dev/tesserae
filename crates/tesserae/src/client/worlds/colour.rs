//! The Biomes step's Sky and Fog colour picker: a swatch that opens presets, a
//! saturation/brightness square, a hue slider and a hex field. Everything inside it edits the
//! Biome in place, so the diorama and the terrain preview redraw while the picker keeps focus.

use bevy::input_focus::InputFocusVisible;
use bevy::input_focus::tab_navigation::{NavAction, TabGroup, TabIndex, TabNavigation};
use bevy::picking::events::{PointerDrag, PointerPress};
use bevy::picking::pointer::PointerButton;
use bevy::text::TextEdit;
use bevy::ui::{ColorStop, ComputedNode, InterpolationColorSpace, UiGlobalTransform};

use super::super::frame;
use super::super::menu_ui::IconKind;
use super::super::pad;
use super::*;

pub(super) const PRESETS: [(&str, [u8; 3]); 8] = [
    ("Sky blue", [0x87, 0xce, 0xeb]),
    ("White", [0xff, 0xff, 0xff]),
    ("Dusk orange", [0xf0, 0x90, 0x5a]),
    ("Overcast grey", [0x9a, 0xa0, 0xa6]),
    ("Night blue", [0x1c, 0x2a, 0x4a]),
    ("Jungle haze", [0x9f, 0xc2, 0x9a]),
    ("Desert sand", [0xe3, 0xc8, 0x91]),
    ("Ember red", [0xb2, 0x3a, 0x2a]),
];

const WIDTH: f32 = 264.0;
const GAP: f32 = 6.0;
const SQUARE_HEIGHT: f32 = 132.0;
const HUE_HEIGHT: f32 = 16.0;
/// One arrow's move on the square, as a fraction; Shift moves five.
const STEP: f32 = 0.02;
const HUE_STEP: f32 = 5.0;
const LAND_AROUND: &str = "Same as the land around it";

/// Which of a Biome's two colours a control edits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::client) enum Air {
    Sky,
    Fog,
}

impl Air {
    fn of(self, sky_fog: SkyFog) -> [u8; 3] {
        match self {
            Air::Sky => sky_fog.sky,
            Air::Fog => sky_fog.fog,
        }
    }
}

/// The open picker. `hsv` is kept apart from the colour so the hue survives a trip through grey
/// or black, where the colour alone forgets it.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Picking {
    pub(super) air: Air,
    /// The Biome's own colours when the picker opened: Esc puts them back, `None` included.
    start: Option<SkyFog>,
    /// Hue in degrees, then saturation and brightness from 0 to 1.
    hsv: Vec3,
}

/// The text of a closed Sky or Fog control.
#[derive(Component)]
pub(super) struct Shown(pub(super) Air);

#[derive(Component)]
pub(super) struct Swatch(Air);

/// The open picker's popup.
#[derive(Component)]
pub(super) struct Picker;

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Part {
    Preset(usize),
    Square,
    Hue,
}

#[derive(Component)]
pub(super) struct Cursor(Part);

#[derive(Component)]
pub(super) struct HexSwatch;

fn bytes(colour: Color) -> [u8; 3] {
    let c = colour.to_srgba();
    [c.red, c.green, c.blue].map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
}

fn rgb(hsv: Vec3) -> [u8; 3] {
    bytes(Color::hsv(hsv.x, hsv.y, hsv.z))
}

fn hsv(rgb: [u8; 3]) -> Vec3 {
    let c = Hsva::from(Color::srgb_u8(rgb[0], rgb[1], rgb[2]));
    Vec3::new(c.hue, c.saturation, c.value)
}

fn colour(rgb: [u8; 3]) -> Color {
    Color::srgb_u8(rgb[0], rgb[1], rgb[2])
}

pub(super) fn hex(rgb: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])
}

/// `#rgb` or `#rrggbb`, the `#` optional.
pub(super) fn parse(text: &str) -> Option<[u8; 3]> {
    let digits = text.trim().trim_start_matches('#');
    let digits = match digits.len() {
        3 => digits.chars().flat_map(|c| [c, c]).collect(),
        6 => digits.to_owned(),
        _ => return None,
    };
    let byte = |at: usize| u8::from_str_radix(digits.get(at..at + 2)?, 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
}

/// The colour Biome `at` shows for `air`, and whether it is only the land around it's.
fn current(draft: &BiomesDraft, at: usize, air: Air) -> ([u8; 3], bool) {
    let borrowed = draft.choices.get(at).is_some_and(|c| c.sky_fog.is_none())
        && draft.land_around(at).is_some();
    (air.of(draft.shown_sky_fog(at)), borrowed)
}

/// Sets one colour of the edited Biome; the other keeps what it showed.
fn write(draft: &mut BiomesDraft, air: Air, to: [u8; 3]) {
    let at = draft.selected;
    let mut pair = draft.shown_sky_fog(at);
    match air {
        Air::Sky => pair.sky = to,
        Air::Fog => pair.fog = to,
    }
    if let Some(choice) = draft.choices.get_mut(at) {
        choice.sky_fog = Some(pair);
    }
}

fn set_hsv(l: &mut Launcher, to: Vec3) {
    let (Some(picking), Some(draft)) = (l.biome_view.colour.as_mut(), l.form.biomes.as_mut())
    else {
        return;
    };
    picking.hsv = to;
    write(draft, picking.air, rgb(to));
}

pub(super) fn set_rgb(l: &mut Launcher, to: [u8; 3]) {
    let (Some(picking), Some(draft)) = (l.biome_view.colour.as_mut(), l.form.biomes.as_mut())
    else {
        return;
    };
    let (shown, _) = current(draft, draft.selected, picking.air);
    if shown == to {
        return;
    }
    picking.hsv = hsv(to);
    write(draft, picking.air, to);
}

/// The acts of the Sky and Fog controls and their picker.
pub(super) fn on_colour(l: &mut Launcher, act: Act) {
    match act {
        Act::ColourPop(air) if l.biome_view.colour.as_ref().is_some_and(|p| p.air == air) => {
            l.biome_view.colour = None;
            l.refocus = Some(act);
        }
        Act::ColourPop(air) => {
            let Some(draft) = l.form.biomes.as_ref() else {
                return;
            };
            let (shown, _) = current(draft, draft.selected, air);
            l.biome_view.colour = Some(Picking {
                air,
                start: draft.choices.get(draft.selected).and_then(|c| c.sky_fog),
                hsv: hsv(shown),
            });
            let preset = PRESETS.iter().position(|(_, p)| *p == shown);
            l.refocus = Some(Act::Preset(preset.unwrap_or(0)));
        }
        Act::Preset(at) => {
            set_rgb(l, PRESETS[at].1);
            close(l);
        }
        Act::ColourDone => close(l),
        _ => return,
    }
    l.version += 1;
}

/// Closes the picker on its control, keeping the colour.
fn close(l: &mut Launcher) {
    if let Some(picking) = l.biome_view.colour.take() {
        l.refocus = Some(Act::ColourPop(picking.air));
    }
}

/// Puts back the colours the open picker started from.
pub(super) fn revert(l: &mut Launcher) {
    let (Some(picking), Some(draft)) = (l.biome_view.colour.as_ref(), l.form.biomes.as_mut())
    else {
        return;
    };
    let at = draft.selected;
    if let Some(choice) = draft.choices.get_mut(at) {
        choice.sky_fog = picking.start;
    }
}

/// A Sky or Fog line's control: its swatch and hex, and the picker under it while open.
pub(super) fn field(parent: &mut ChildSpawnerCommands, l: &Launcher, air: Air) {
    let Some(draft) = l.form.biomes.as_ref() else {
        return;
    };
    let (shown, borrowed) = current(draft, draft.selected, air);
    let picking = l.biome_view.colour.as_ref().filter(|p| p.air == air);
    parent
        .spawn(Node {
            flex_grow: 1.0,
            min_width: Val::Px(0.0),
            ..default()
        })
        .with_children(|wrap| {
            wrap.spawn((
                bevy::ui_widgets::Button,
                TabIndex(0),
                Act::ColourPop(air),
                pages::Pick {
                    fill: theme::FIELD,
                    edge: theme::field_edge(),
                    hover: Some((theme::FIELD_HOVER, theme::field_edge())),
                },
                BackgroundColor(theme::FIELD),
                BorderColor::all(theme::field_edge()),
                Node {
                    flex_grow: 1.0,
                    min_width: Val::Px(0.0),
                    height: Val::Px(theme::CONTROL_HEIGHT),
                    padding: UiRect::axes(Val::Px(10.0), Val::Px(0.0)),
                    border: UiRect::all(Val::Px(theme::RULE)),
                    border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(8.0),
                    ..default()
                },
            ))
            .with_children(|select| {
                swatch(select, shown, 14.0, Swatch(air));
                select.spawn((
                    Shown(air),
                    Text::new(if borrowed {
                        LAND_AROUND.into()
                    } else {
                        hex(shown)
                    }),
                    theme::font(theme::TEXT_BODY),
                    TextColor(if borrowed {
                        theme::placeholder()
                    } else {
                        theme::INK
                    }),
                    TextLayout::linebreak(bevy::text::LineBreak::NoWrap),
                    Node {
                        flex_grow: 1.0,
                        min_width: Val::Px(0.0),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
                ui::icon(select, IconKind::Expanded, theme::muted(), 12.0);
            });
            if let Some(picking) = picking {
                picker(wrap, picking, shown);
            }
        });
}

fn swatch(parent: &mut ChildSpawnerCommands, rgb: [u8; 3], size: f32, marker: impl Bundle) {
    parent.spawn((
        marker,
        BackgroundColor(colour(rgb)),
        BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.22)),
        Node {
            width: Val::Px(size),
            height: Val::Px(size),
            flex_shrink: 0.0,
            border: UiRect::all(Val::Px(theme::RULE)),
            border_radius: BorderRadius::all(Val::Px(theme::RADIUS_TAG)),
            ..default()
        },
        Pickable::IGNORE,
    ));
}

/// The rim on the picker's parts, as on the mockup's.
fn rim() -> Color {
    Color::srgba(1.0, 1.0, 1.0, 0.22)
}

fn picker(wrap: &mut ChildSpawnerCommands, picking: &Picking, shown: [u8; 3]) {
    wrap.spawn((
        Picker,
        frame::Popup,
        TabGroup::modal(),
        // The panel's body scrolls, and must not cut the picker off at its edge.
        bevy::ui::OverrideClip,
        BackgroundColor(theme::RAISED),
        BorderColor::all(theme::edge()),
        GlobalZIndex(10),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(theme::CONTROL_HEIGHT + GAP),
            left: Val::Px(0.0),
            width: Val::Px(WIDTH),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(10.0)),
            row_gap: Val::Px(10.0),
            border: UiRect::all(Val::Px(theme::RULE)),
            border_radius: BorderRadius::all(Val::Px(theme::RADIUS_MENU)),
            ..default()
        },
    ))
    .with_children(|pop| {
        pop.spawn(Node {
            column_gap: Val::Px(GAP),
            ..default()
        })
        .with_children(|row| {
            let chosen = PRESETS.iter().position(|(_, p)| *p == shown);
            for (at, (_, preset)) in PRESETS.into_iter().enumerate() {
                let roving = at == chosen.unwrap_or(0);
                row.spawn((
                    bevy::ui_widgets::Button,
                    pad::Surface,
                    Part::Preset(at),
                    TabIndex(if roving { 0 } else { -1 }),
                    Act::Preset(at),
                    pages::Pick {
                        fill: colour(preset),
                        edge: rim(),
                        hover: Some((colour(preset), theme::INK)),
                    },
                    BackgroundColor(colour(preset)),
                    BorderColor::all(rim()),
                    Node {
                        flex_grow: 1.0,
                        aspect_ratio: Some(1.0),
                        border: UiRect::all(Val::Px(theme::RULE)),
                        border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
                        ..default()
                    },
                ));
            }
        });
        pop.spawn((
            Part::Square,
            pad::Surface,
            TabIndex(0),
            Act::ColourDone,
            BackgroundColor(Color::hsv(picking.hsv.x, 1.0, 1.0)),
            BackgroundGradient(vec![
                LinearGradient::to_right(vec![
                    ColorStop::auto(Color::WHITE),
                    ColorStop::auto(Color::WHITE.with_alpha(0.0)),
                ])
                .in_color_space(InterpolationColorSpace::Srgba)
                .into(),
                LinearGradient::to_bottom(vec![
                    ColorStop::auto(Color::BLACK.with_alpha(0.0)),
                    ColorStop::auto(Color::BLACK),
                ])
                .in_color_space(InterpolationColorSpace::Srgba)
                .into(),
            ]),
            BorderColor::all(rim()),
            theme::focus_ring(),
            Node {
                height: Val::Px(SQUARE_HEIGHT),
                border: UiRect::all(Val::Px(theme::RULE)),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
                ..default()
            },
        ))
        .with_children(|s| {
            cursor(s, Part::Square, Vec2::splat(14.0), rgb(picking.hsv));
        });
        let stops = [0.0, 60.0, 120.0, 180.0, 240.0, 300.0, 360.0]
            .map(|h| ColorStop::auto(Color::hsv(h, 1.0, 1.0)));
        pop.spawn((
            Part::Hue,
            pad::Surface,
            TabIndex(0),
            Act::ColourDone,
            BackgroundGradient::from(
                LinearGradient::to_right(stops.to_vec())
                    .in_color_space(InterpolationColorSpace::Srgba),
            ),
            BorderColor::all(rim()),
            theme::focus_ring(),
            Node {
                height: Val::Px(HUE_HEIGHT),
                border: UiRect::all(Val::Px(theme::RULE)),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
                ..default()
            },
        ))
        .with_children(|h| {
            cursor(
                h,
                Part::Hue,
                Vec2::new(8.0, 22.0),
                bytes(Color::hsv(picking.hsv.x, 1.0, 1.0)),
            );
        });
        pop.spawn(Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|row| {
            swatch(row, shown, 24.0, HexSwatch);
            ui::weighted_text(row, "Hex", theme::TEXT_SMALL, theme::LABEL, theme::INK);
            row.spawn(Node {
                flex_grow: 1.0,
                min_width: Val::Px(0.0),
                ..default()
            })
            .with_children(|f| {
                pages::text_field(f, Field::ColourHex, &hex(shown), "", "");
            });
        });
    });
}

fn cursor(parent: &mut ChildSpawnerCommands, part: Part, size: Vec2, fill: [u8; 3]) {
    let round = if part == Part::Square { size.x } else { 3.0 };
    parent.spawn((
        Cursor(part),
        BackgroundColor(colour(fill)),
        BorderColor::all(Color::WHITE),
        Outline::new(Val::Px(1.0), Val::ZERO, Color::srgba(0.0, 0.0, 0.0, 0.67)),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(size.x),
            height: Val::Px(size.y),
            margin: UiRect {
                left: Val::Px(-size.x / 2.0),
                top: Val::Px(-size.y / 2.0),
                ..default()
            },
            top: Val::Percent(50.0),
            border: UiRect::all(Val::Px(2.0)),
            border_radius: BorderRadius::all(Val::Px(round)),
            ..default()
        },
        Pickable::IGNORE,
    ));
}

type Painted<'w, 's> = Query<
    'w,
    's,
    (
        Option<&'static Shown>,
        Option<&'static Swatch>,
        Option<&'static mut Text>,
        Option<&'static mut TextColor>,
        &'static mut BackgroundColor,
    ),
    Or<(With<Shown>, With<Swatch>, With<HexSwatch>)>,
>;

/// Keeps the Sky and Fog controls on the edited Biome's colours, in place: both follow, so
/// picking one colour on a Biome that borrowed the land's shows the other at once.
pub(super) fn paint_controls(launcher: Option<Res<Launcher>>, mut painted: Painted) {
    let Some(l) = launcher else {
        return;
    };
    let Some(draft) = l.form.biomes.as_ref() else {
        return;
    };
    let open = l.biome_view.colour.as_ref().map(|p| p.air);
    for (shown, swatch, text, ink, mut fill) in &mut painted {
        let Some(air) = shown.map(|s| s.0).or(swatch.map(|s| s.0)).or(open) else {
            continue;
        };
        let (rgb, borrowed) = current(draft, draft.selected, air);
        fill.set_if_neq(BackgroundColor(colour(rgb)));
        if let (Some(mut text), Some(mut ink)) = (text, ink) {
            let (wanted, tint) = if borrowed {
                (LAND_AROUND.to_owned(), theme::placeholder())
            } else {
                (hex(rgb), theme::INK)
            };
            if text.0 != wanted {
                text.0 = wanted;
            }
            ink.set_if_neq(TextColor(tint));
        }
    }
}

type Parts<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Part,
        &'static mut TabIndex,
        Option<&'static mut pages::Pick>,
        &'static mut Outline,
    ),
>;

/// Keeps the open picker's cursors, square, presets and hex field on its colour.
pub(super) fn paint_picker(
    launcher: Option<Res<Launcher>>,
    mut cursors: Query<(&Cursor, &mut Node, &mut BackgroundColor)>,
    mut squares: Query<(&Part, &mut BackgroundColor), Without<Cursor>>,
    mut parts: Parts,
    mut hexes: Query<(Entity, &Field, &mut EditableText)>,
    (focus, visible): (Res<InputFocus>, Option<Res<InputFocusVisible>>),
) {
    let Some(l) = launcher else {
        return;
    };
    let (Some(draft), Some(picking)) = (l.form.biomes.as_ref(), l.biome_view.colour.as_ref())
    else {
        return;
    };
    let (shown, _) = current(draft, draft.selected, picking.air);
    let hsv = picking.hsv;
    let pure = Color::hsv(hsv.x, 1.0, 1.0);
    for (cursor, mut node, mut fill) in &mut cursors {
        let (x, y, tint) = match cursor.0 {
            Part::Square => (hsv.y, 1.0 - hsv.z, colour(shown)),
            _ => (hsv.x / 360.0, 0.5, pure),
        };
        let (left, top) = (Val::Percent(x * 100.0), Val::Percent(y * 100.0));
        if node.left != left || node.top != top {
            node.left = left;
            node.top = top;
        }
        fill.set_if_neq(BackgroundColor(tint));
    }
    for (part, mut fill) in &mut squares {
        if *part == Part::Square {
            fill.set_if_neq(BackgroundColor(pure));
        }
    }
    let chosen = PRESETS.iter().position(|(_, p)| *p == shown);
    let focused_preset = parts.iter().find_map(|(e, part, ..)| match part {
        Part::Preset(i) if focus.get() == Some(e) => Some(*i),
        _ => None,
    });
    let ring = ui::visible_focus(Some(&focus), visible.as_deref());
    for (entity, part, mut index, pick, mut outline) in &mut parts {
        if let (Part::Preset(i), Some(mut pick)) = (part, pick) {
            let roving = *i == focused_preset.or(chosen).unwrap_or(0);
            let wanted = TabIndex(if roving { 0 } else { -1 });
            if *index != wanted {
                *index = wanted;
            }
            let edge = if chosen == Some(*i) {
                theme::INK
            } else {
                rim()
            };
            if pick.edge != edge {
                pick.edge = edge;
            }
            continue;
        }
        let tint = if ring == Some(entity) {
            theme::FOCUS
        } else {
            Color::NONE
        };
        if outline.color != tint {
            outline.color = tint;
        }
    }
    for (entity, field, mut text) in &mut hexes {
        if *field != Field::ColourHex || focus.get() == Some(entity) {
            continue;
        }
        if text.pending_edits.is_empty() && parse(&text.value().to_string()) != Some(shown) {
            text.queue_edit(TextEdit::SelectAll);
            text.queue_edit(TextEdit::Insert(hex(shown).into()));
        }
    }
}

/// Arrows on a picker part: a preset moves to the next and picks it, the square and the hue
/// slider move their cursor.
pub(super) fn nudge(
    event: On<pad::Nudge>,
    parts: Query<(Entity, &Part)>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    launcher: Option<ResMut<Launcher>>,
    mut focus: ResMut<InputFocus>,
) {
    let (Ok((_, part)), Some(mut launcher)) = (parts.get(event.entity), launcher) else {
        return;
    };
    let launcher = launcher.bypass_change_detection();
    let Some(picking) = launcher.biome_view.colour.as_ref() else {
        return;
    };
    let big = keys.is_some_and(|k| k.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]));
    let by = event.by.as_vec2();
    let mut hsv = picking.hsv;
    match *part {
        Part::Preset(at) => {
            let forward = by.x > 0.0 || by.y < 0.0;
            let next = (at + if forward { 1 } else { PRESETS.len() - 1 }) % PRESETS.len();
            if let Some((entity, _)) = parts.iter().find(|(_, p)| **p == Part::Preset(next)) {
                focus.set(entity, FocusCause::Navigated);
            }
            set_rgb(launcher, PRESETS[next].1);
            return;
        }
        Part::Square => {
            let step = if big { STEP * 5.0 } else { STEP };
            hsv.y = (hsv.y + by.x * step).clamp(0.0, 1.0);
            hsv.z = (hsv.z + by.y * step).clamp(0.0, 1.0);
        }
        Part::Hue => {
            let step = if big { HUE_STEP * 6.0 } else { HUE_STEP };
            let along = if by.x != 0.0 { by.x } else { by.y };
            hsv.x = (hsv.x + along * step).clamp(0.0, 360.0);
        }
    }
    set_hsv(launcher, hsv);
}

/// Enter or Space on the square or the hue slider picks; LB and RB move between the parts as
/// Tab does.
pub(super) fn keys(
    keys: Res<ButtonInput<KeyCode>>,
    pad: Option<Res<pad::PadInput>>,
    launcher: Option<Res<Launcher>>,
    parts: Query<&Part>,
    nav: TabNavigation,
    (mut focus, mut visible): (ResMut<InputFocus>, Option<ResMut<InputFocusVisible>>),
    mut queue: ResMut<Queue>,
) {
    if !launcher.is_some_and(|l| l.biome_view.colour.is_some()) {
        return;
    }
    let on = focus.get().and_then(|e| parts.get(e).ok());
    if matches!(on, Some(Part::Square | Part::Hue))
        && keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter, KeyCode::Space])
    {
        queue.0.push(Act::ColourDone);
    }
    let Some(pad) = pad else {
        return;
    };
    let action = if pad.navigation.just_pressed(GamepadButton::RightTrigger) {
        NavAction::Next
    } else if pad.navigation.just_pressed(GamepadButton::LeftTrigger) {
        NavAction::Previous
    } else {
        return;
    };
    if let Ok(next) = nav.navigate(&focus, action) {
        focus.set(next, FocusCause::Navigated);
        if let Some(visible) = visible.as_mut() {
            visible.0 = true;
        }
    }
}

/// Pressing the square or the hue slider sets the colour under the pointer and takes the focus.
pub(super) fn press(
    event: On<PointerPress>,
    parts: Query<(&Part, &ComputedNode, &UiGlobalTransform)>,
    launcher: Option<ResMut<Launcher>>,
    mut focus: ResMut<InputFocus>,
) {
    if event.button != PointerButton::Primary {
        return;
    }
    if drag_to(event.entity, event.pointer.position, &parts, launcher) {
        focus.set(event.entity, FocusCause::Pressed);
    }
}

pub(super) fn drag(
    event: On<PointerDrag>,
    parts: Query<(&Part, &ComputedNode, &UiGlobalTransform)>,
    launcher: Option<ResMut<Launcher>>,
) {
    if event.button == PointerButton::Primary {
        drag_to(event.entity, event.pointer.position, &parts, launcher);
    }
}

fn drag_to(
    entity: Entity,
    pointer: Vec2,
    parts: &Query<(&Part, &ComputedNode, &UiGlobalTransform)>,
    launcher: Option<ResMut<Launcher>>,
) -> bool {
    let (Ok((part, node, transform)), Some(mut launcher)) = (parts.get(entity), launcher) else {
        return false;
    };
    if matches!(part, Part::Preset(_)) {
        return false;
    }
    // The pointer is in logical pixels; the node's size and transform are physical.
    let physical = pointer / node.inverse_scale_factor;
    let local = transform.inverse().transform_point2(physical) / node.size + 0.5;
    if !local.is_finite() {
        return false;
    }
    let local = local.clamp(Vec2::ZERO, Vec2::ONE);
    let launcher = launcher.bypass_change_detection();
    let Some(mut hsv) = launcher.biome_view.colour.as_ref().map(|p| p.hsv) else {
        return false;
    };
    if *part == Part::Square {
        hsv.y = local.x;
        hsv.z = 1.0 - local.y;
    } else {
        hsv.x = local.x * 360.0;
    }
    set_hsv(launcher, hsv);
    true
}

type Placed = (&'static ComputedNode, &'static UiGlobalTransform);

/// Keeps the open picker inside the window and its panel: it hangs above its control when there
/// is more room there, and moves left when it would run past the window's right edge. Layout has
/// placed it by then, so it moves a frame after it opens.
pub(super) fn hang(
    mut pickers: Query<(&mut Node, &ComputedNode, &UiGlobalTransform), With<Picker>>,
    roots: Query<&ComputedNode, (With<Root>, Without<Picker>)>,
    panels: Query<Placed, (With<pages::PanelScroll>, Without<Picker>)>,
) {
    let Some(window) = roots.iter().next().map(ComputedNode::size) else {
        return;
    };
    let floor = panels.iter().next().map_or(window.y, |(node, at)| {
        (at.translation.y + node.size().y / 2.0).min(window.y)
    });
    for (mut node, computed, transform) in &mut pickers {
        let size = computed.size();
        if size.y <= 0.0 {
            continue;
        }
        let scale = computed.inverse_scale_factor;
        let centre = transform.translation;
        let margin = 12.0 / scale;
        if node.bottom == Val::Auto {
            let top = centre.y - size.y / 2.0;
            let control = (theme::CONTROL_HEIGHT + GAP) / scale;
            let below = floor - top;
            let above = top - control;
            if below < size.y && above > below {
                node.top = Val::Auto;
                node.bottom = Val::Px(theme::CONTROL_HEIGHT + GAP);
            }
        }
        let past = centre.x + size.x / 2.0 - (window.x - margin);
        if past > 0.5
            && let Val::Px(left) = node.left
        {
            node.left = Val::Px(left - past * scale);
        }
    }
}

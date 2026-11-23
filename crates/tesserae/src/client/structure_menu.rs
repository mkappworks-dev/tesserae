//! The place Tool's panel, docked right while the Tool is in hand: full over
//! the live game to pick a Template, so the ghost stays drawn behind it, and
//! compact in the same place while aiming.
//!
//! The panel opens full on the Server's answer to Use, so Survival hears the
//! same refusal any Tool's Use gives. Its Templates are the World's own, from
//! the Catalogue; none is named here.
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::ui_widgets::{Activate, ScrollArea};
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use super::actions::{Action, Actions, Bindings, Labels};
use super::camera::FlyCamera;
use super::edit::{Catalogue, Hotbar};
use super::frame::{self, Cap, Prompt};
use super::inventory_window::look::{self, Flat, Flavour};
use super::menu_ui::grouped;
use super::pad::PadInput;
use super::structure::{Ghost, Outcome, PlaceTool, holding_place, turned_size_m};
use super::{theme, tool_panel};
use crate::content::ContentCatalogue;
use crate::server::tool::{ToolKind, UseIntent, UseOutcome, UseResult, UseWith};
use crate::session::SessionApp;
use tesserae_core::template::Turn;

const ROW_PX: f32 = 48.0;
const ROW_GAP_PX: f32 = 4.0;
const LIST_MAX_PX: f32 = 264.0;
const THUMB_PX: f32 = 40.0;
const COMPACT_THUMB_PX: f32 = 44.0;
const THUMB_BACKGROUND: Color = Color::srgb_u8(0x14, 0x14, 0x18);
const TURN_STEP_PX: f32 = 30.0;

#[derive(Resource, Default)]
pub(super) struct StructureList {
    pub(super) open: bool,
    dirty: bool,
    /// What is up now: the full panel, the compact one, or nothing.
    shown: Option<bool>,
    /// The Structures key asked to use the place Tool, so its answer opens the panel with the
    /// Tool in hand whatever was held.
    asked: bool,
    /// A key stepped the choice, so the list scrolls to show it.
    reveal: bool,
}

impl StructureList {
    /// Open, for a capture of the panel.
    #[cfg(any(feature = "dev", feature = "profiling"))]
    pub(super) fn show(&mut self) {
        self.open = true;
        self.dirty = true;
    }
}

/// The tool panels docked over play, which hold the cursor while open.
#[derive(SystemParam)]
pub(super) struct Docked<'w> {
    pub(super) layout: Option<Res<'w, super::layout_menu::LayoutMenu>>,
    pub(super) list: Option<Res<'w, StructureList>>,
    mode: Option<Res<'w, super::layout::LayoutMode>>,
    hotbar: Option<Res<'w, Hotbar>>,
    catalogue: Option<Res<'w, Catalogue>>,
}

impl Docked<'_> {
    pub(super) fn open(&self) -> bool {
        self.layout.as_ref().is_some_and(|menu| menu.open)
            || self.list.as_ref().is_some_and(|list| list.open)
    }

    /// Quick layout or the place Tool is in hand, so its panel shows, full or compact, and
    /// Escape leaves it rather than opening the Menu.
    pub(super) fn in_hand(&self) -> bool {
        self.open()
            || self.mode.as_ref().is_some_and(|mode| mode.on)
            || self
                .hotbar
                .as_ref()
                .zip(self.catalogue.as_ref())
                .is_some_and(|(hotbar, catalogue)| holding_place(hotbar, &catalogue.0))
    }
}

#[derive(Component)]
struct Panel;

/// The Template list, which scrolls inside the full panel.
#[derive(Component)]
struct Card;

#[derive(Component, Clone, Copy, PartialEq, Debug)]
#[require(frame::PointerOnly = frame::PointerOnly)]
enum Control {
    Pick(usize),
    /// A quarter turn: right for 1, left for -1.
    Turn(i32),
    /// Back to aiming with the chosen Template, as the Structures key does.
    Aim,
}

/// Each Template's thumbnail at each quarter turn, by its place in the Catalogue.
#[derive(Resource, Default)]
pub(super) struct Thumbs(Vec<[Handle<Image>; 4]>);

impl Thumbs {
    fn of(&self, at: usize, turn: Turn) -> Handle<Image> {
        self.0
            .get(at)
            .map_or_else(Handle::default, |t| t[usize::from(turn.quarters())].clone())
    }
}

/// The compact panel's thumbnail, of the chosen Template as it is turned.
#[derive(Component)]
struct Thumb;

/// The lines that follow the aim and the choices without a rebuild.
#[derive(Component, Clone, Copy, PartialEq)]
enum Info {
    Title,
    Facing,
    Size,
    Outcome,
    Extent,
    Voxels,
}

/// The outcome line's box, whose colours follow its tone.
#[derive(Component)]
struct OutcomeBox;

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<StructureList>()
        .init_session_state::<Thumbs>()
        .add_observer(activate)
        .add_session_systems(
            Update,
            (
                toggle
                    .after(super::menu::toggle)
                    .before(super::input::resolve)
                    .before(super::camera::grab_cursor),
                (make_thumbs, open_on_use, keys, render, update_info)
                    .chain()
                    .after(toggle)
                    .after(super::structure::track),
            ),
        );
}

type Blockers<'w> = (
    Res<'w, super::menu::Menu>,
    Option<Res<'w, super::settings::Settings>>,
    Option<Res<'w, super::layout_menu::LayoutMenu>>,
    Option<Res<'w, super::inventory_window::InventoryWindow>>,
    Option<Res<'w, super::map_window::MapWindow>>,
    Option<Res<'w, super::layout::LayoutMode>>,
);

fn blocked((menu, settings, layout, inventory, map, mode): &Blockers) -> bool {
    menu.open
        || settings.as_ref().is_some_and(|s| s.open)
        || layout.as_ref().is_some_and(|m| m.open)
        || inventory.as_ref().is_some_and(|w| w.open)
        || map.as_ref().is_some_and(|w| w.open)
        || mode.as_ref().is_some_and(|m| m.on)
}

/// The Structures key asks to use the place Tool, which opens the panel full with
/// it in hand, and with a Template chosen it shrinks the panel into play again.
/// Escape or the clear key cancels a pinned preview first, then puts the Tool
/// away from either. A build opens the panel full to pick the next Template.
#[allow(clippy::too_many_arguments)]
pub(super) fn toggle(
    mut actions: ResMut<Actions>,
    mut list: ResMut<StructureList>,
    mut tool: ResMut<PlaceTool>,
    (mut hotbar, catalogue): (ResMut<Hotbar>, Res<Catalogue>),
    blockers: Blockers,
    camera: Query<&GlobalTransform, With<FlyCamera>>,
    mut intents: MessageWriter<UseIntent>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let holding = holding_place(&hotbar, &catalogue.0);
    if !holding && tool.reopen {
        tool.reopen = false;
    }
    if list.open && !holding {
        close(&mut list, &mut cursor);
        return;
    }
    if !list.open && blocked(&blockers) {
        return;
    }
    if holding && (actions.cancel_requested() || actions.started(Action::ClearTool)) {
        actions.consume_cancel();
        actions.cancel(Action::ClearTool);
        if tool.pinned.is_some() {
            tool.pinned = None;
        } else {
            hotbar.outside = None;
            if list.open {
                close(&mut list, &mut cursor);
            }
        }
        return;
    }
    let key = actions.started(Action::Structures);
    if list.open {
        if key && tool.chosen(&catalogue.0).is_some() {
            close(&mut list, &mut cursor);
        }
        return;
    }
    if holding && tool.reopen {
        tool.reopen = false;
        open(&mut list, &mut cursor);
        return;
    }
    let place = catalogue.0.tools.iter().find(|t| t.kind == ToolKind::Place);
    if key && let (Some(place), Some(camera)) = (place, camera.iter().next()) {
        intents.write(UseIntent {
            with: UseWith::Tool(place.id),
            origin: camera.translation(),
            direction: *camera.forward(),
            expected: None,
        });
        list.asked = true;
    }
}

fn open(list: &mut StructureList, cursor: &mut CursorOptions) {
    list.open = true;
    list.dirty = true;
    list.reveal = true;
    cursor.grab_mode = CursorGrabMode::None;
    cursor.visible = true;
}

fn close(list: &mut StructureList, cursor: &mut CursorOptions) {
    list.open = false;
    cursor.grab_mode = CursorGrabMode::Locked;
    cursor.visible = false;
}

/// The Server allowed the place Tool's Use: the panel opens full, unless another
/// window opened while the answer was on its way. Asked by the Structures key,
/// it takes the place Tool into hand.
fn open_on_use(
    mut outcomes: MessageReader<UseOutcome>,
    mut list: ResMut<StructureList>,
    (mut hotbar, catalogue): (ResMut<Hotbar>, Res<Catalogue>),
    blockers: Blockers,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let mut answered = false;
    let mut allowed = false;
    for UseOutcome(outcome) in outcomes.read() {
        answered = true;
        allowed |= matches!(outcome, Ok(UseResult::TemplateList));
    }
    let by_key = list.asked;
    if answered {
        list.asked = false;
    }
    let place = catalogue.0.tools.iter().find(|t| t.kind == ToolKind::Place);
    if !allowed
        || list.open
        || !(by_key || holding_place(&hotbar, &catalogue.0))
        || blocked(&blockers)
    {
        return;
    }
    if by_key && let Some(place) = place {
        hotbar.outside = Some(super::edit::Holding::Tool(place.id));
    }
    open(&mut list, &mut cursor);
}

/// One change from a click, a prompt or a key alike.
fn apply(
    control: Control,
    list: &mut StructureList,
    tool: &mut PlaceTool,
    catalogue: &ContentCatalogue,
    cursor: &mut Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    match control {
        Control::Pick(at) => {
            if let Some(template) = catalogue.templates.get(at) {
                tool.choose(&template.name);
            }
        }
        Control::Turn(by) => {
            tool.turn = Turn::new(tool.turn.quarters() + if by > 0 { 1 } else { 3 });
        }
        Control::Aim => {
            if tool.chosen(catalogue).is_some() {
                for mut cursor in cursor {
                    close(list, &mut cursor);
                }
            }
        }
    }
    list.dirty = true;
}

fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    mut list: ResMut<StructureList>,
    mut tool: ResMut<PlaceTool>,
    catalogue: Res<Catalogue>,
    mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let Ok(&control) = controls.get(event.entity) else {
        return;
    };
    if list.open {
        apply(control, &mut list, &mut tool, &catalogue.0, &mut cursor);
    }
}

/// The keys and controller buttons the prompt bar names but cannot fire
/// itself: up and down through the Templates, Enter or A to aim.
fn keys(
    keys: Res<ButtonInput<KeyCode>>,
    pad: Option<Res<PadInput>>,
    mut list: ResMut<StructureList>,
    mut tool: ResMut<PlaceTool>,
    catalogue: Res<Catalogue>,
    mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if !list.open {
        return;
    }
    let pad = |button| {
        pad.as_ref()
            .is_some_and(|p| p.navigation.just_pressed(button))
    };
    let by = i32::from(keys.just_pressed(KeyCode::ArrowDown) || pad(GamepadButton::DPadDown))
        - i32::from(keys.just_pressed(KeyCode::ArrowUp) || pad(GamepadButton::DPadUp));
    if by != 0 {
        tool.step(&catalogue.0, by);
        list.reveal = true;
        list.dirty = true;
    } else if keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter])
        || pad(GamepadButton::South)
    {
        apply(
            Control::Aim,
            &mut list,
            &mut tool,
            &catalogue.0,
            &mut cursor,
        );
    }
}

fn size_text(size: [f32; 3], decimals: usize) -> String {
    let [x, y, z] = size.map(|n| format!("{n:.decimals$}"));
    format!("{x} × {y} × {z} m")
}

/// A thumbnail of a Template, as the list's rows and the compact panel show it.
fn thumb(image: Handle<Image>, px: f32) -> impl Bundle {
    (
        ImageNode::new(image),
        Node {
            width: Val::Px(px),
            height: Val::Px(px),
            flex_shrink: 0.0,
            border_radius: BorderRadius::all(Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(THUMB_BACKGROUND),
        Pickable::IGNORE,
    )
}

/// Images of the Catalogue's thumbnails, made again whenever the Catalogue changes.
fn make_thumbs(
    catalogue: Res<Catalogue>,
    mut images: ResMut<Assets<Image>>,
    mut thumbs: ResMut<Thumbs>,
) {
    if !catalogue.is_changed() {
        return;
    }
    let side = crate::content::THUMBNAIL_PX as u32;
    thumbs.0 = catalogue
        .0
        .templates
        .iter()
        .map(|t| {
            t.thumbnails.clone().map(|pixels| {
                if pixels.len() != (side * side * 4) as usize {
                    return Handle::default();
                }
                images.add(Image::new(
                    bevy::render::render_resource::Extent3d {
                        width: side,
                        height: side,
                        depth_or_array_layers: 1,
                    },
                    bevy::render::render_resource::TextureDimension::D2,
                    pixels.to_vec(),
                    bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                    bevy::asset::RenderAssetUsages::default(),
                ))
            })
        })
        .collect();
}

/// The World's Templates, one row each, in a list that scrolls on its own.
fn picks(
    body: &mut ChildSpawnerCommands,
    catalogue: &ContentCatalogue,
    chosen: Option<&str>,
    (thumbs, scroll): (&Thumbs, Vec2),
) {
    let game = catalogue
        .game
        .as_deref()
        .map(|game| format!(" · {}", super::menu_ui::declared_title(game)))
        .unwrap_or_default();
    body.spawn((
        Node {
            margin: UiRect::bottom(Val::Px(6.0)),
            flex_shrink: 0.0,
            ..default()
        },
        look::label(
            format!("Template{game}"),
            theme::TEXT_EYEBROW,
            theme::REGULAR,
            theme::muted(),
        ),
    ));
    body.spawn((
        Card,
        ScrollArea,
        ScrollPosition(scroll),
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(ROW_GAP_PX),
            max_height: Val::Px(LIST_MAX_PX),
            margin: UiRect::bottom(Val::Px(14.0)),
            overflow: Overflow::scroll_y(),
            flex_shrink: 0.0,
            ..default()
        },
    ))
    .with_children(|list| {
        if catalogue.templates.is_empty() {
            list.spawn(look::label(
                "This World declares no Templates.",
                theme::TEXT_EYEBROW,
                theme::REGULAR,
                theme::muted(),
            ));
        }
        for (at, template) in catalogue.templates.iter().enumerate() {
            look::flat(
                list,
                Control::Pick(at),
                Flat {
                    flavour: Flavour::Pick,
                    selected: chosen == Some(template.name.as_str()),
                },
                Node {
                    min_height: Val::Px(ROW_PX),
                    padding: UiRect {
                        left: Val::Px(4.0),
                        right: Val::Px(8.0),
                        top: Val::Px(4.0),
                        bottom: Val::Px(4.0),
                    },
                    border_radius: BorderRadius::all(Val::Px(6.0)),
                    ..default()
                },
                |pick, _| {
                    // `flat` centres its content whatever the node asks.
                    pick.spawn(Node {
                        width: Val::Percent(100.0),
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(10.0),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn(thumb(thumbs.of(at, Turn::NONE), THUMB_PX));
                        row.spawn(Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::FlexStart,
                            row_gap: Val::Px(2.0),
                            ..default()
                        })
                        .with_children(|lines| {
                            lines.spawn(look::label(
                                super::menu_ui::declared_title(&template.name),
                                theme::TEXT_SMALL,
                                theme::SEMIBOLD,
                                theme::INK,
                            ));
                            lines.spawn(look::label(
                                size_text(template.size_m, 0),
                                theme::TEXT_KEY,
                                theme::REGULAR,
                                theme::muted(),
                            ));
                        });
                    });
                },
            );
        }
    });
}

/// The facing, with a quarter turn either way.
fn turn_row(body: &mut ChildSpawnerCommands) {
    body.spawn((
        Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(6.0),
            padding: UiRect::axes(Val::Px(6.0), Val::Px(14.0)),
            border: UiRect::vertical(Val::Px(theme::RULE)),
            flex_shrink: 0.0,
            ..default()
        },
        BorderColor::all(theme::GAP_RULE),
    ))
    .with_children(|row| {
        row.spawn(Node {
            width: Val::Px(44.0),
            ..default()
        })
        .with_child(look::label(
            "Turn",
            theme::TEXT_CAPTION,
            theme::SEMIBOLD,
            theme::AXIS_TAG_INK,
        ));
        row.spawn((
            Node {
                flex_grow: 1.0,
                ..default()
            },
            Text::new("Facing "),
            theme::weighted(theme::TEXT_EYEBROW, theme::REGULAR),
            TextColor(theme::muted()),
            Pickable::IGNORE,
            children![(
                Info::Facing,
                TextSpan::new(""),
                theme::weighted(theme::TEXT_EYEBROW, theme::REGULAR),
                TextColor(theme::muted()),
            )],
        ));
        for (by, glyph) in [(-1, "↺"), (1, "↻")] {
            look::flat_text(
                row,
                glyph,
                Control::Turn(by),
                Flat {
                    flavour: Flavour::Preset,
                    selected: false,
                },
                Node {
                    width: Val::Px(TURN_STEP_PX),
                    height: Val::Px(TURN_STEP_PX),
                    border_radius: BorderRadius::all(Val::Px(theme::RADIUS_CONTROL)),
                    ..default()
                },
                (theme::TEXT_SMALL, theme::REGULAR),
            );
        }
    });
}

/// The size and the Voxel count, at the foot of the full panel.
fn total(body: &mut ChildSpawnerCommands, chosen: bool) {
    body.spawn(Node {
        justify_content: JustifyContent::SpaceBetween,
        align_items: AlignItems::Center,
        margin: UiRect::top(Val::Auto),
        padding: UiRect::top(Val::Px(14.0)),
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|row| {
        if !chosen {
            row.spawn(look::label(
                "Nothing selected",
                theme::TEXT_EYEBROW,
                theme::REGULAR,
                theme::TOTALS_TERM,
            ));
            return;
        }
        row.spawn((
            Info::Extent,
            look::label("", theme::TEXT_EYEBROW, theme::REGULAR, theme::TOTALS_TERM),
        ));
        row.spawn((
            Info::Voxels,
            Text::new(""),
            theme::figures(theme::TEXT_HEADING, theme::SEMIBOLD),
            TextColor(theme::INK),
            Pickable::IGNORE,
            children![(
                TextSpan::new(" Voxels"),
                theme::weighted(theme::TEXT_EYEBROW, theme::REGULAR),
                TextColor(theme::muted()),
            )],
        ));
    });
}

/// The keys the foot names, as caps from the bindings.
fn caps(
    (full, chosen, pinned): (bool, bool, bool),
    (bindings, labels): (&Bindings, &Labels),
) -> Vec<(String, &'static str)> {
    let cap = |action| tool_panel::cap(action, bindings, labels);
    let edit = if full { "Aim" } else { "Edit" };
    if pinned {
        vec![
            (cap(Action::Use), "Build"),
            (cap(Action::ReleaseCursor), "Cancel"),
            (cap(Action::Structures), edit),
        ]
    } else if !full {
        vec![
            (cap(Action::Structures), edit),
            (cap(Action::ReleaseCursor), "Exit"),
            (cap(Action::Place), "Place"),
        ]
    } else if chosen {
        vec![
            (cap(Action::Structures), edit),
            (cap(Action::ReleaseCursor), "Exit"),
        ]
    } else {
        vec![(cap(Action::ReleaseCursor), "Exit")]
    }
}

/// The full panel's keys; letters fire their prompt, the rest are read by [`keys`].
fn prompts(chosen: bool, pinned: bool) -> Vec<Prompt<Control>> {
    let mut prompts = vec![Prompt::new(Cap::Text("↑ ↓"), "Template")];
    if chosen {
        prompts.extend([
            Prompt::new(Cap::Letter('R'), "Turn")
                .act(Control::Turn(1))
                .pad(GamepadButton::West),
            Prompt::new(Cap::Enter, "Aim").act(Control::Aim),
        ]);
    }
    prompts.push(Prompt::new(
        Cap::Esc,
        if pinned { "Cancel" } else { "Exit" },
    ));
    prompts
}

/// Where the list must scroll to show row `at`, from `scroll`.
fn revealed(scroll: f32, at: usize) -> f32 {
    let top = at as f32 * (ROW_PX + ROW_GAP_PX);
    scroll.clamp((top + ROW_PX - LIST_MAX_PX).max(0.0), top)
}

#[allow(clippy::too_many_arguments)]
fn render(
    mut commands: Commands,
    mut list: ResMut<StructureList>,
    panels: Query<Entity, With<Panel>>,
    cards: Query<&ScrollPosition, With<Card>>,
    (tool, hotbar, catalogue): (Res<PlaceTool>, Res<Hotbar>, Res<Catalogue>),
    (bindings, labels, thumbs): (Res<Bindings>, Res<Labels>, Res<Thumbs>),
) {
    let want = if list.open {
        Some(true)
    } else if holding_place(&hotbar, &catalogue.0) {
        Some(false)
    } else {
        None
    };
    let rebuild = want != list.shown
        || (want.is_some()
            && (list.dirty
                || tool.is_changed()
                || catalogue.is_changed()
                || bindings.is_changed()
                || labels.is_changed()
                || thumbs.is_changed()));
    if !rebuild {
        return;
    }
    let chosen = tool.chosen(&catalogue.0);
    let at = catalogue.0.templates.iter().position(|t| Some(t) == chosen);
    // A rebuild must not throw the Player back to the top of a long list.
    let mut scroll = match (list.shown, want) {
        (Some(true), Some(true)) => cards.iter().next().map_or(Vec2::ZERO, |s| s.0),
        _ => Vec2::ZERO,
    };
    if std::mem::take(&mut list.reveal)
        && let Some(at) = at
    {
        scroll.y = revealed(scroll.y, at);
    }
    list.dirty = false;
    list.shown = want;
    for panel in &panels {
        commands.entity(panel).despawn();
    }
    let Some(full) = want else {
        return;
    };
    let pinned = tool.pinned.is_some();
    let caps = caps((full, chosen.is_some(), pinned), (&bindings, &labels));
    if !full {
        tool_panel::spawn(
            &mut commands,
            Panel,
            (),
            None::<Vec<Prompt<Control>>>,
            |body| {
                tool_panel::header(body, "Place Tool", false);
                tool_panel::selection(
                    body,
                    |frame| {
                        frame.spawn((Thumb, thumb(Handle::default(), COMPACT_THUMB_PX)));
                    },
                    (Info::Title, tool_panel::title("")),
                    (Info::Size, tool_panel::sub("")),
                );
                tool_panel::outcome(
                    body,
                    (OutcomeBox, tool_panel::QUIET_EDGE),
                    Info::Outcome,
                    0.0,
                );
                tool_panel::foot(body, None::<Option<Control>>, &caps);
            },
        );
        return;
    }
    tool_panel::spawn(
        &mut commands,
        Panel,
        (),
        Some(prompts(chosen.is_some(), pinned)),
        |body| {
            tool_panel::header(body, "Place Tool", true);
            picks(
                body,
                &catalogue.0,
                chosen.map(|t| t.name.as_str()),
                (&thumbs, scroll),
            );
            turn_row(body);
            tool_panel::outcome(
                body,
                (OutcomeBox, tool_panel::QUIET_EDGE),
                Info::Outcome,
                14.0,
            );
            total(body, chosen.is_some());
            tool_panel::foot(body, Some(chosen.map(|_| Control::Aim)), &caps);
        },
    );
    tool_panel::pointer_only::<Control>(&mut commands);
}

/// What each frame says about the choice and the aim, wherever it is shown.
#[allow(clippy::too_many_arguments)]
fn update_info(
    (tool, catalogue, ghost): (Res<PlaceTool>, Res<Catalogue>, Res<Ghost>),
    (bindings, labels): (Res<Bindings>, Res<Labels>),
    mut texts: Query<(&Info, &mut Text)>,
    mut spans: Query<(&Info, &mut TextSpan)>,
    mut boxes: Query<(&mut BackgroundColor, &mut BorderColor), With<OutcomeBox>>,
    (thumbs, mut shown): (Res<Thumbs>, Query<&mut ImageNode, With<Thumb>>),
) {
    let chosen = tool.chosen(&catalogue.0);
    let at = catalogue.0.templates.iter().position(|t| Some(t) == chosen);
    let image = at.map_or_else(Handle::default, |at| thumbs.of(at, tool.turn));
    for mut node in &mut shown {
        if node.image != image {
            node.image = image.clone();
        }
    }
    let cap = |action| tool_panel::cap(action, &bindings, &labels);
    let pinned = tool.pinned.is_some();
    let voxels: usize = ghost.tally.iter().sum();
    let line = |part: Info| -> String {
        let Some(chosen) = chosen else {
            return match part {
                Info::Title => "Nothing selected".to_owned(),
                Info::Facing => tool.facing().to_owned(),
                Info::Outcome => format!(
                    "Pick the next Template, or {} to put the Tool away.",
                    cap(Action::ReleaseCursor)
                ),
                Info::Size | Info::Extent | Info::Voxels => String::new(),
            };
        };
        let size = turned_size_m(chosen, tool.turn);
        match part {
            Info::Title => format!(
                "{} · facing {}",
                super::menu_ui::declared_title(&chosen.name),
                tool.facing()
            ),
            Info::Facing => tool.facing().to_owned(),
            Info::Size => match ghost.outcome {
                Some(Outcome::Whole(n)) => {
                    format!("{} · {} Voxels", size_text(size, 1), grouped(n as u64))
                }
                _ => size_text(size, 1),
            },
            Info::Extent => size_text(size, 0),
            Info::Voxels => grouped(voxels as u64),
            Info::Outcome if pinned => format!(
                "Preview pinned. Come close: {} builds it whole.",
                cap(Action::Use)
            ),
            Info::Outcome => ghost
                .outcome
                .map_or("Aim at a surface to place it.".to_owned(), Outcome::text),
        }
    };
    for (part, mut text) in &mut texts {
        let value = line(*part);
        if text.0 != value {
            text.0 = value;
        }
    }
    for (part, mut span) in &mut spans {
        let value = line(*part);
        if span.0 != value {
            span.0 = value;
        }
    }
    let (fill, edge) = match (chosen.is_some(), pinned, ghost.outcome) {
        (false, ..) | (_, false, None) => (Color::NONE, tool_panel::QUIET_EDGE),
        (_, true, _) | (_, _, Some(Outcome::Whole(_))) => (Color::NONE, tool_panel::WHOLE_EDGE),
        (_, _, Some(Outcome::Waiting)) => (tool_panel::WAIT_FILL, tool_panel::WAIT_EDGE),
        (_, _, Some(Outcome::Refused(_))) => (tool_panel::REFUSED_FILL, tool_panel::REFUSED_EDGE),
    };
    for (mut background, mut border) in &mut boxes {
        background.set_if_neq(BackgroundColor(fill));
        border.set_if_neq(BorderColor::all(edge));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{CatalogueTemplate, CatalogueTool};
    use crate::server::tool::{ToolId, ToolKind, UseRefusal};

    fn app(held: super::super::edit::Holding) -> App {
        let mut app = App::new();
        app.add_plugins(super::super::actions::test_plugin);
        app.add_plugins(MinimalPlugins)
            .init_resource::<StructureList>()
            .init_resource::<super::super::menu::Menu>()
            .init_resource::<PlaceTool>()
            .add_message::<UseOutcome>()
            .add_message::<UseIntent>()
            .add_observer(activate)
            .add_systems(
                Update,
                (super::super::menu::toggle, toggle, open_on_use, keys).chain(),
            );
        app.insert_resource(Catalogue(ContentCatalogue {
            tools: vec![CatalogueTool {
                id: ToolId(0),
                name: "terra:place".into(),
                kind: ToolKind::Place,
            }],
            templates: vec![
                CatalogueTemplate {
                    name: "terra:cottage".into(),
                    size_m: [7.0, 8.0, 5.0],
                    thumbnails: Default::default(),
                },
                CatalogueTemplate {
                    name: "terra:well".into(),
                    size_m: [3.0, 4.0, 3.0],
                    thumbnails: Default::default(),
                },
            ],
            ..default()
        }));
        let mut hotbar = Hotbar::default();
        hotbar.outside = Some(held);
        app.insert_resource(hotbar);
        app.world_mut().spawn((
            Window {
                focused: true,
                ..default()
            },
            PrimaryWindow,
            CursorOptions {
                grab_mode: CursorGrabMode::Locked,
                ..default()
            },
        ));
        app.world_mut().spawn((
            FlyCamera::default(),
            GlobalTransform::from(Transform::from_xyz(1.0, 2.0, 3.0)),
        ));
        app
    }

    /// One frame with `key` pressed; the Use intents it sent.
    fn press(app: &mut App, key: KeyCode) -> Vec<UseIntent> {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release_all();
        keys.clear();
        app.world_mut()
            .resource_mut::<Messages<UseIntent>>()
            .drain()
            .collect()
    }

    fn answered(app: &mut App) {
        app.world_mut()
            .write_message(UseOutcome(Ok(UseResult::TemplateList)));
        app.update();
    }

    fn held(app: &App) -> Option<super::super::edit::Holding> {
        app.world().resource::<Hotbar>().outside
    }

    #[test]
    fn k_from_play_asks_to_use_the_place_tool_and_the_answer_opens_the_list_with_it_in_hand() {
        let mut app = app(super::super::edit::Holding::Empty);
        let sent = press(&mut app, KeyCode::KeyK);
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].with, UseWith::Tool(ToolId(0)));
        answered(&mut app);
        assert!(app.world().resource::<StructureList>().open);
        assert_eq!(
            held(&app),
            Some(super::super::edit::Holding::Tool(ToolId(0)))
        );
        assert_eq!(cursor(&mut app), CursorGrabMode::None);
    }

    #[test]
    fn k_does_nothing_with_quick_layout_on() {
        let mut app = app(super::super::edit::Holding::Empty);
        let mut mode = super::super::layout::LayoutMode::default();
        mode.on = true;
        app.insert_resource(mode);
        assert!(press(&mut app, KeyCode::KeyK).is_empty());
        answered(&mut app);
        assert!(!app.world().resource::<StructureList>().open);
        assert_eq!(held(&app), Some(super::super::edit::Holding::Empty));
    }

    #[test]
    fn k_closes_the_list_into_play_and_backspace_puts_the_tool_away() {
        let mut app = app(super::super::edit::Holding::Tool(ToolId(0)));
        answered(&mut app);
        assert!(
            press(&mut app, KeyCode::KeyK).is_empty(),
            "closing asks nothing"
        );
        assert!(!app.world().resource::<StructureList>().open);
        assert_eq!(cursor(&mut app), CursorGrabMode::Locked);
        assert_eq!(
            held(&app),
            Some(super::super::edit::Holding::Tool(ToolId(0)))
        );

        answered(&mut app);
        press(&mut app, KeyCode::Backspace);
        assert!(!app.world().resource::<StructureList>().open);
        assert_eq!(cursor(&mut app), CursorGrabMode::Locked);
        assert_eq!(held(&app), None, "the Tool put away");
    }

    fn cursor(app: &mut App) -> CursorGrabMode {
        app.world_mut()
            .query::<&CursorOptions>()
            .single(app.world())
            .unwrap()
            .grab_mode
    }

    fn open(app: &App) -> bool {
        app.world().resource::<StructureList>().open
    }

    fn menu_open(app: &App) -> bool {
        app.world().resource::<super::super::menu::Menu>().open
    }

    fn chosen(app: &App) -> Option<String> {
        let catalogue = app.world().resource::<Catalogue>().0.clone();
        app.world()
            .resource::<PlaceTool>()
            .chosen(&catalogue)
            .map(|t| t.name.clone())
    }

    fn pin(app: &mut App) {
        app.world_mut().resource_mut::<PlaceTool>().pinned = Some(([1, 2, 3], Vec3::NEG_Z));
    }

    #[test]
    fn k_while_aiming_opens_the_panel_full_again_with_the_tool_still_in_hand() {
        let mut app = app(super::super::edit::Holding::Tool(ToolId(0)));
        assert_eq!(press(&mut app, KeyCode::KeyK).len(), 1, "asks, as Use does");
        answered(&mut app);
        assert!(open(&app));
        assert_eq!(
            held(&app),
            Some(super::super::edit::Holding::Tool(ToolId(0)))
        );
    }

    #[test]
    fn picking_a_template_keeps_the_panel_open_and_aim_returns_to_play() {
        let mut app = app(super::super::edit::Holding::Tool(ToolId(0)));
        answered(&mut app);
        let pick = app.world_mut().spawn(Control::Pick(1)).id();
        app.world_mut().trigger(Activate { entity: pick });
        app.update();
        assert!(open(&app));
        assert_eq!(chosen(&app).as_deref(), Some("terra:well"));
        let aim = app.world_mut().spawn(Control::Aim).id();
        app.world_mut().trigger(Activate { entity: aim });
        app.update();
        assert!(!open(&app));
        assert_eq!(cursor(&mut app), CursorGrabMode::Locked);
    }

    #[test]
    fn up_and_down_step_through_the_templates_and_enter_aims() {
        let mut app = app(super::super::edit::Holding::Tool(ToolId(0)));
        answered(&mut app);
        press(&mut app, KeyCode::ArrowDown);
        assert_eq!(chosen(&app).as_deref(), Some("terra:well"));
        press(&mut app, KeyCode::ArrowDown);
        assert_eq!(chosen(&app).as_deref(), Some("terra:cottage"), "round");
        press(&mut app, KeyCode::ArrowUp);
        assert_eq!(chosen(&app).as_deref(), Some("terra:well"));
        assert!(open(&app));
        press(&mut app, KeyCode::Enter);
        assert!(!open(&app));
        assert_eq!(cursor(&mut app), CursorGrabMode::Locked);
    }

    #[test]
    fn escape_puts_the_tool_away_from_either_panel_and_opens_no_menu() {
        for full in [true, false] {
            let mut app = app(super::super::edit::Holding::Tool(ToolId(0)));
            if full {
                answered(&mut app);
            }
            press(&mut app, KeyCode::Escape);
            assert_eq!(held(&app), None, "full: {full}");
            assert!(!open(&app) && !menu_open(&app), "full: {full}");
            assert_eq!(cursor(&mut app), CursorGrabMode::Locked);
            press(&mut app, KeyCode::Escape);
            assert!(menu_open(&app), "out of the tool, Escape opens the Menu");
        }
    }

    #[test]
    fn with_a_preview_pinned_the_first_escape_only_cancels_it() {
        for full in [true, false] {
            let mut app = app(super::super::edit::Holding::Tool(ToolId(0)));
            if full {
                answered(&mut app);
            }
            pin(&mut app);
            press(&mut app, KeyCode::Escape);
            assert!(app.world().resource::<PlaceTool>().pinned.is_none());
            assert_eq!(
                held(&app),
                Some(super::super::edit::Holding::Tool(ToolId(0))),
                "full: {full}"
            );
            assert_eq!(open(&app), full);
            assert!(!menu_open(&app));
            press(&mut app, KeyCode::Escape);
            assert_eq!(held(&app), None, "then it leaves");
        }
    }

    #[test]
    fn k_with_a_preview_pinned_opens_the_panel_and_keeps_the_pin() {
        let mut app = app(super::super::edit::Holding::Tool(ToolId(0)));
        pin(&mut app);
        assert_eq!(press(&mut app, KeyCode::KeyK).len(), 1);
        answered(&mut app);
        assert!(open(&app));
        assert!(app.world().resource::<PlaceTool>().pinned.is_some());
    }

    #[test]
    fn after_a_build_the_panel_opens_full_and_enter_and_k_wait_for_a_pick() {
        let mut app = app(super::super::edit::Holding::Tool(ToolId(0)));
        {
            let mut tool = app.world_mut().resource_mut::<PlaceTool>();
            tool.choose("terra:cottage");
            tool.spend();
        }
        app.update();
        assert!(open(&app), "opened to pick the next");
        assert_eq!(cursor(&mut app), CursorGrabMode::None);
        assert_eq!(chosen(&app), None);
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::KeyK);
        assert!(open(&app), "nothing to aim with");
        press(&mut app, KeyCode::ArrowDown);
        assert_eq!(chosen(&app).as_deref(), Some("terra:well"), "the next one");
        press(&mut app, KeyCode::KeyK);
        assert!(!open(&app));
    }

    #[test]
    fn the_servers_template_list_answer_opens_the_list_and_frees_the_cursor() {
        let mut app = app(super::super::edit::Holding::Tool(ToolId(0)));
        app.world_mut()
            .write_message(UseOutcome(Ok(UseResult::TemplateList)));
        app.update();
        assert!(app.world().resource::<StructureList>().open);
        assert_eq!(cursor(&mut app), CursorGrabMode::None);
    }

    #[test]
    fn a_refused_use_leaves_the_list_shut() {
        let mut app = app(super::super::edit::Holding::Tool(ToolId(0)));
        app.world_mut()
            .write_message(UseOutcome(Err(UseRefusal::CreativeOnly)));
        app.update();
        assert!(!app.world().resource::<StructureList>().open);
        assert_eq!(cursor(&mut app), CursorGrabMode::Locked);
    }

    #[test]
    fn the_answer_opens_nothing_once_the_tool_is_put_down() {
        let mut app = app(super::super::edit::Holding::Empty);
        app.world_mut()
            .write_message(UseOutcome(Ok(UseResult::TemplateList)));
        app.update();
        assert!(!app.world().resource::<StructureList>().open);
    }

    fn render_app(held: super::super::edit::Holding) -> App {
        let mut app = app(held);
        app.init_resource::<Labels>()
            .init_resource::<Thumbs>()
            .init_resource::<Ghost>()
            .init_resource::<bevy::input_focus::InputFocus>()
            .init_resource::<super::super::menu_ui::Searches>()
            .init_resource::<super::super::edit::Notice>()
            .init_resource::<super::super::input::UiInput>()
            .init_resource::<Assets<Image>>()
            .init_resource::<Assets<Font>>()
            .add_plugins(super::super::menu_ui::plugin);
        app
    }

    /// Draws the panel as it is now; its texts, and whether it holds an Aim to press.
    fn drawn(app: &mut App) -> (Vec<String>, bool) {
        use bevy::ecs::system::RunSystemOnce;
        app.world_mut().run_system_once(render).unwrap();
        app.world_mut().flush();
        app.world_mut().run_system_once(update_info).unwrap();
        let texts = app
            .world_mut()
            .query::<&Text>()
            .iter(app.world())
            .map(|text| text.0.clone())
            .collect();
        let aim = app
            .world_mut()
            .query_filtered::<&Control, With<bevy::ui_widgets::Button>>()
            .iter(app.world())
            .any(|control| *control == Control::Aim);
        (texts, aim)
    }

    fn has(texts: &[String], words: &[&str]) {
        for word in words {
            assert!(texts.iter().any(|t| t == word), "{word} in {texts:?}");
        }
    }

    #[test]
    fn aiming_the_panel_names_the_template_its_facing_and_the_keys() {
        let mut app = render_app(super::super::edit::Holding::Tool(ToolId(0)));
        let (texts, aim) = drawn(&mut app);
        has(
            &texts,
            &[
                "Place Tool",
                "Cottage · facing South",
                "K",
                "Edit",
                "Esc",
                "Exit",
                "RMB",
                "Place",
            ],
        );
        assert!(!aim, "nothing to press while aiming");
        pin(&mut app);
        let (texts, _) = drawn(&mut app);
        has(
            &texts,
            &[
                "G",
                "Build",
                "Cancel",
                "Edit",
                "Preview pinned. Come close: G builds it whole.",
            ],
        );
    }

    #[test]
    fn after_a_build_the_full_panel_asks_for_the_next_template_and_offers_no_aim() {
        let mut app = render_app(super::super::edit::Holding::Tool(ToolId(0)));
        app.world_mut().resource_mut::<StructureList>().open = true;
        let (texts, aim) = drawn(&mut app);
        has(
            &texts,
            &["Place Tool", "Template", "Cottage", "Well", "Aim", "Exit"],
        );
        assert!(aim);
        app.world_mut().resource_mut::<PlaceTool>().spend();
        let (texts, aim) = drawn(&mut app);
        has(
            &texts,
            &[
                "Pick the next Template, or Esc to put the Tool away.",
                "Nothing selected",
            ],
        );
        assert!(!aim, "Aim waits for a pick");
    }
}

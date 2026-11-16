//! The place Tool's Template list, docked over the live game so the ghost
//! stays drawn behind it, and its HUD card in play.
//!
//! The list opens on the Server's answer to Use, so Survival hears the same
//! refusal any Tool's Use gives. Its Templates are the World's own, from the
//! Catalogue; none is named here.
use bevy::ecs::system::SystemParam;
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::ui_widgets::{Activate, ScrollArea};
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use super::actions::{Action, Actions, Bindings, Labels};
use super::camera::FlyCamera;
use super::cinematic as cine;
use super::edit::{Catalogue, Hotbar};
use super::inventory_window::look::{self, Flat, Flavour};
use super::layout_menu::{eyebrow, term, total_row, totals, value_with};
use super::menu_ui::IconKind;
use super::structure::{Ghost, Outcome, PlaceTool, Tint, grouped, holding_place, turned_size_m};
use super::theme;
use crate::content::ContentCatalogue;
use crate::server::edit::StructureIntent;
use crate::server::tool::{ToolKind, UseIntent, UseOutcome, UseResult, UseWith};
use crate::session::SessionApp;
use crate::world::WorldSettings;
use tesserae_core::template::Turn;

const CARD_WIDTH: f32 = 360.0;
const PANEL_TOP_PX: f32 = 28.0;
const PANEL_BOTTOM_PX: f32 = 22.0;
const PAD_X: f32 = 20.0;
const HUD_WIDTH: f32 = 270.0;
const HUD_TOP_PX: f32 = 150.0;
const SWATCH_PX: f32 = 12.0;
const THUMB_PX: f32 = 40.0;
const THUMB_BACKGROUND: Color = Color::srgb_u8(0x14, 0x14, 0x18);
const ICON_PX: f32 = 18.0;
/// The prototype's outcome box: its edge and fill for each tone.
const QUIET_EDGE: Color = Color::srgb_u8(0x5c, 0x5c, 0x65);
const WHOLE_EDGE: Color = Color::srgb_u8(0x6c, 0x8f, 0x6c);
const WAIT_EDGE: Color = Color::srgb_u8(0xb0, 0x9a, 0x5a);
const WAIT_FILL: Color = Color::srgb_u8(0x28, 0x23, 0x0f);
const REFUSED_EDGE: Color = Color::srgb_u8(0xb0, 0x5a, 0x5a);
const REFUSED_FILL: Color = Color::srgb_u8(0x2a, 0x17, 0x17);
const OUTCOME_INK: Color = Color::srgb_u8(0xd9, 0xdd, 0xd2);

#[derive(Resource, Default)]
pub(super) struct StructureList {
    pub(super) open: bool,
    dirty: bool,
    /// What is up now: the list, the HUD card, or nothing.
    shown: Option<bool>,
    /// The Structures key asked to use the place Tool, so its answer opens the list with the
    /// Tool in hand whatever was held.
    asked: bool,
}

impl StructureList {
    /// Open, for a capture of the list.
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
}

impl Docked<'_> {
    pub(super) fn open(&self) -> bool {
        self.layout.as_ref().is_some_and(|menu| menu.open)
            || self.list.as_ref().is_some_and(|list| list.open)
    }
}

#[derive(Component)]
struct Panel;

#[derive(Component)]
struct Card;

#[derive(Component, Clone, Copy, PartialEq, Debug)]
enum Control {
    Pick(usize),
    Turn,
    Close,
    PlaceHere,
    PutAway,
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

/// The placement card's thumbnail, of the chosen Template as it is turned.
#[derive(Component)]
struct Thumb;

/// The lines that follow the aim and the choices without a rebuild.
#[derive(Component, Clone, Copy, PartialEq)]
enum Info {
    Title,
    Facing,
    Size,
    Count(usize),
    Outcome,
    Key(Action),
    PlaceKey,
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
                    .before(super::input::resolve),
                (make_thumbs, open_on_use, render, update_info)
                    .chain()
                    .after(toggle)
                    .after(super::structure::track),
            ),
        );
}

/// The Structures key asks to use the place Tool from play, held or not, as the
/// Layout key opens its menu; the Server's answer opens the list. With the list
/// open it closes it into play, the clear key closes it and puts the Tool away,
/// and Escape closes it with the cursor free, as it does the Layout menu.
/// Putting the Tool down closes it into play.
#[allow(clippy::too_many_arguments)]
pub(super) fn toggle(
    actions: Res<Actions>,
    mut list: ResMut<StructureList>,
    (mut hotbar, catalogue): (ResMut<Hotbar>, Res<Catalogue>),
    (menu, settings, layout, inventory, map): Blockers,
    camera: Query<&GlobalTransform, With<FlyCamera>>,
    mut intents: MessageWriter<UseIntent>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let key = actions.started(Action::Structures);
    if !list.open {
        let blocked = menu.open
            || settings.is_some_and(|s| s.open)
            || layout.is_some_and(|m| m.open)
            || inventory.is_some_and(|w| w.open)
            || map.is_some_and(|w| w.open);
        let place = catalogue.0.tools.iter().find(|t| t.kind == ToolKind::Place);
        if key
            && !blocked
            && let (Some(place), Some(camera)) = (place, camera.iter().next())
        {
            intents.write(UseIntent {
                with: UseWith::Tool(place.id),
                origin: camera.translation(),
                direction: *camera.forward(),
                expected: None,
            });
            list.asked = true;
        }
        return;
    }
    if !holding_place(&hotbar, &catalogue.0) || key {
        close(&mut list, &mut cursor, true);
    } else if actions.started(Action::ClearTool) {
        hotbar.outside = None;
        close(&mut list, &mut cursor, true);
    } else if actions.cancel_requested() {
        close(&mut list, &mut cursor, false);
    }
}

fn close(list: &mut StructureList, cursor: &mut CursorOptions, grab: bool) {
    list.open = false;
    cursor.grab_mode = if grab {
        CursorGrabMode::Locked
    } else {
        CursorGrabMode::None
    };
    cursor.visible = !grab;
}

type Blockers<'w> = (
    Res<'w, super::menu::Menu>,
    Option<Res<'w, super::settings::Settings>>,
    Option<Res<'w, super::layout_menu::LayoutMenu>>,
    Option<Res<'w, super::inventory_window::InventoryWindow>>,
    Option<Res<'w, super::map_window::MapWindow>>,
);

/// The Server allowed the place Tool's Use: the list opens, unless another
/// window opened while the answer was on its way. Asked by the Structures key,
/// it takes the place Tool into hand.
fn open_on_use(
    mut outcomes: MessageReader<UseOutcome>,
    mut list: ResMut<StructureList>,
    (mut hotbar, catalogue): (ResMut<Hotbar>, Res<Catalogue>),
    (menu, settings, layout, inventory, map): Blockers,
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
        || menu.open
        || settings.is_some_and(|s| s.open)
        || layout.is_some_and(|m| m.open)
        || inventory.is_some_and(|w| w.open)
        || map.is_some_and(|w| w.open)
    {
        return;
    }
    if by_key && let Some(place) = place {
        hotbar.outside = Some(super::edit::Holding::Tool(place.id));
    }
    list.open = true;
    list.dirty = true;
    cursor.grab_mode = CursorGrabMode::None;
    cursor.visible = true;
}

#[allow(clippy::too_many_arguments)]
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    mut list: ResMut<StructureList>,
    mut tool: ResMut<PlaceTool>,
    (catalogue, mut hotbar): (Res<Catalogue>, ResMut<Hotbar>),
    camera: Query<&GlobalTransform, With<FlyCamera>>,
    mut intents: MessageWriter<StructureIntent>,
    mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let Ok(&control) = controls.get(event.entity) else {
        return;
    };
    if !list.open {
        return;
    }
    let mut shut = || {
        for mut cursor in &mut cursor {
            close(&mut list, &mut cursor, true);
        }
    };
    match control {
        Control::Pick(at) => {
            if let Some(template) = catalogue.0.templates.get(at) {
                tool.choose(&template.name);
            }
        }
        Control::Turn => tool.turn = tool.turn.next(),
        Control::Close => shut(),
        Control::PlaceHere => {
            if let Some(intent) = camera
                .iter()
                .next()
                .and_then(|camera| tool.intent(&catalogue.0, camera))
            {
                intents.write(intent);
            }
            shut();
        }
        Control::PutAway => {
            hotbar.outside = None;
            shut();
        }
    }
    list.dirty = true;
}

/// The first binding only: a card has no room for every alternative.
fn key(bindings: &Bindings, labels: &Labels, action: Action) -> String {
    let all = bindings.label(action, labels);
    all.split(" / ").next().unwrap_or_default().to_owned()
}

fn small(value: impl Into<String>, color: Color) -> impl Bundle {
    look::paragraph(
        value,
        theme::TEXT_EYEBROW,
        theme::REGULAR,
        color,
        theme::LINE_HEIGHT,
    )
}

/// The band at the top: what the tool is, its name and the close cross.
fn header(card: &mut ChildSpawnerCommands) {
    card.spawn((
        Node {
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            column_gap: Val::Px(12.0),
            padding: UiRect::axes(Val::Px(PAD_X), Val::Px(18.0)),
            border: UiRect::bottom(Val::Px(theme::RULE)),
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundColor(theme::sheet_header()),
        BorderColor::all(theme::LAYOUT_RULE),
    ))
    .with_children(|header| {
        header
            .spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(5.0),
                ..default()
            })
            .with_children(|title| {
                title.spawn((
                    look::label(
                        "PLACE TOOL",
                        theme::TEXT_KEY,
                        theme::LABEL,
                        theme::item_eyebrow(),
                    ),
                    bevy::text::LetterSpacing::Px(theme::TOOL_TRACKING),
                ));
                title.spawn(look::label(
                    "Structures",
                    theme::ITEM_TEXT_TITLE,
                    theme::MEDIUM,
                    theme::INK,
                ));
            });
        look::close(header, Control::Close);
    });
}

fn size_text(size: [f32; 3]) -> String {
    format!("{:.1} × {:.1} × {:.1} m", size[0], size[1], size[2])
}

/// A thumbnail of a Template, as the list's rows and the placement card show it.
fn thumb(image: Handle<Image>) -> impl Bundle {
    (
        ImageNode::new(image),
        Node {
            width: Val::Px(THUMB_PX),
            height: Val::Px(THUMB_PX),
            flex_shrink: 0.0,
            border_radius: BorderRadius::all(Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(THUMB_BACKGROUND),
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

fn picks(
    body: &mut ChildSpawnerCommands,
    catalogue: &ContentCatalogue,
    chosen: Option<&str>,
    thumbs: &Thumbs,
) {
    body.spawn(Node {
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(7.0),
        margin: UiRect::bottom(Val::Px(20.0)),
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|list| {
        if catalogue.templates.is_empty() {
            list.spawn(small("This World declares no Templates.", theme::muted()));
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
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(10.0)),
                    border_radius: BorderRadius::all(Val::Px(5.0)),
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
                        row.spawn(thumb(thumbs.of(at, Turn::NONE)));
                        row.spawn(Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::FlexStart,
                            row_gap: Val::Px(3.0),
                            ..default()
                        })
                        .with_children(|lines| {
                            lines.spawn(look::label(
                                super::menu_ui::declared_title(&template.name),
                                theme::TEXT_SMALL,
                                theme::MEDIUM,
                                theme::INK,
                            ));
                            lines.spawn(look::label(
                                size_text(template.size_m),
                                theme::TEXT_EYEBROW,
                                theme::REGULAR,
                                theme::PRESET_INK,
                            ));
                            lines.spawn(look::label(
                                template.name.clone(),
                                theme::TEXT_EYEBROW,
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

fn turn_row(body: &mut ChildSpawnerCommands, turn_key: &str) {
    body.spawn(Node {
        justify_content: JustifyContent::SpaceBetween,
        align_items: AlignItems::Center,
        column_gap: Val::Px(12.0),
        margin: UiRect::bottom(Val::Px(20.0)),
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|row| {
        look::flat_text(
            row,
            format!("Turn · {turn_key}"),
            Control::Turn,
            Flat {
                flavour: Flavour::Pick,
                selected: false,
            },
            Node {
                min_height: Val::Px(36.0),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                border_radius: BorderRadius::all(Val::Px(5.0)),
                ..default()
            },
            (theme::TEXT_EYEBROW + 1.0, theme::REGULAR),
        );
        row.spawn((
            Text::new("Facing "),
            theme::weighted(theme::TEXT_EYEBROW + 1.0, theme::REGULAR),
            TextColor(theme::PRESET_INK),
            Pickable::IGNORE,
            children![(
                Info::Facing,
                TextSpan::new(""),
                theme::weighted(theme::TEXT_EYEBROW + 1.0, theme::STRONG),
                TextColor(theme::INK),
            )],
        ));
    });
}

/// One row per tint: its colour, what it means and how many Voxels.
fn legend(parent: &mut ChildSpawnerCommands, gap: f32, margin: f32) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(gap),
            margin: UiRect::bottom(Val::Px(margin)),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|keys| {
            for (slot, tint) in Tint::ALL.into_iter().enumerate() {
                keys.spawn(Node {
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        Node {
                            width: Val::Px(SWATCH_PX),
                            height: Val::Px(SWATCH_PX),
                            border: UiRect::all(Val::Px(1.0)),
                            border_radius: BorderRadius::all(Val::Px(2.0)),
                            margin: UiRect::right(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(tint.color().with_alpha(1.0)),
                        BorderColor::all(Color::srgba(0.0, 0.0, 0.0, 0.53)),
                    ));
                    row.spawn((
                        Node {
                            flex_grow: 1.0,
                            ..default()
                        },
                        look::label(
                            tint.label(),
                            theme::TEXT_EYEBROW,
                            theme::REGULAR,
                            theme::PRESET_INK,
                        ),
                    ));
                    row.spawn((
                        Info::Count(slot),
                        look::figures("0", theme::TEXT_EYEBROW, theme::MEDIUM, theme::INK),
                    ));
                });
            }
        });
}

fn outcome_box(parent: &mut ChildSpawnerCommands, margin: f32) {
    parent
        .spawn((
            OutcomeBox,
            Node {
                padding: UiRect::axes(Val::Px(11.0), Val::Px(9.0)),
                margin: UiRect::bottom(Val::Px(margin)),
                border: UiRect::all(Val::Px(theme::RULE)),
                border_radius: BorderRadius::all(Val::Px(5.0)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(Color::NONE),
            BorderColor::all(QUIET_EDGE),
        ))
        .with_child((
            Info::Outcome,
            look::paragraph(
                "",
                theme::TEXT_EYEBROW + 1.0,
                theme::REGULAR,
                OUTCOME_INK,
                theme::LINE_HEIGHT,
            ),
        ));
}

fn footer(card: &mut ChildSpawnerCommands, hint: String) {
    card.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            padding: UiRect::axes(Val::Px(PAD_X), Val::Px(14.0)),
            border: UiRect::top(Val::Px(theme::RULE)),
            flex_shrink: 0.0,
            ..default()
        },
        BorderColor::all(theme::LAYOUT_RULE),
    ))
    .with_children(|foot| {
        foot.spawn(look::paragraph(
            hint,
            theme::ITEM_TEXT_EYEBROW,
            theme::REGULAR,
            theme::muted(),
            theme::LINE_HEIGHT,
        ));
        look::button(
            foot,
            "Place here",
            Control::PlaceHere,
            cine::Kind::Primary,
            look::WIDE,
        );
        look::button(
            foot,
            "Put the Tool away",
            Control::PutAway,
            cine::Kind::Ghost,
            look::WIDE,
        );
    });
}

/// In play with the list shut: what is chosen, what the aim would do and the
/// keys.
fn hud(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Card,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(theme::SCREEN_PAD_X),
                top: Val::Px(HUD_TOP_PX),
                width: Val::Px(HUD_WIDTH),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(7.0),
                padding: UiRect::all(Val::Px(14.0)),
                border: UiRect::all(Val::Px(theme::FRAME)),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_MENU)),
                ..default()
            },
            BackgroundColor(theme::LAYOUT_SURFACE),
            BorderColor::all(theme::GROUT),
            theme::shadow(),
            Pickable::IGNORE,
        ))
        .with_children(|card| {
            card.spawn(theme::ring(theme::RADIUS_MENU));
            card.spawn(Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                ..default()
            })
            .with_children(|line| {
                super::menu_ui::icon(line, IconKind::Layout, theme::LAYOUT_STATUS_INK, ICON_PX);
                line.spawn(look::label(
                    "Place Tool",
                    theme::TEXT_EYEBROW,
                    theme::REGULAR,
                    theme::LAYOUT_STATUS_INK,
                ));
            });
            card.spawn(Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                ..default()
            })
            .with_children(|head| {
                head.spawn((Thumb, thumb(Handle::default())));
                head.spawn((
                    Info::Title,
                    look::label("", theme::TEXT_BODY, theme::MEDIUM, theme::INK),
                ));
            });
            card.spawn((Info::Size, small("", theme::muted())));
            legend(card, 3.0, 0.0);
            outcome_box(card, 0.0);
            card.spawn(Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.0),
                margin: UiRect::top(Val::Px(4.0)),
                ..default()
            })
            .with_children(|keys| {
                for (action, what) in [
                    (Action::Structures, "Templates"),
                    (Action::TurnStructure, "Turn"),
                ] {
                    look::key_cap(keys, "", Info::Key(action));
                    keys.spawn((
                        Node {
                            margin: UiRect::right(Val::Px(6.0)),
                            ..default()
                        },
                        small(what, theme::muted()),
                    ));
                }
            });
            card.spawn((Info::PlaceKey, small("", theme::muted())));
        });
}

#[allow(clippy::too_many_arguments)]
fn list_panel(
    commands: &mut Commands,
    (catalogue, thumbs): (&ContentCatalogue, &Thumbs),
    tool: &PlaceTool,
    voxel_size: f32,
    scroll: Vec2,
    (bindings, labels): (&Bindings, &Labels),
) {
    let game = catalogue
        .game
        .as_deref()
        .map(|game| format!(" · {}", super::menu_ui::declared_title(game).to_uppercase()))
        .unwrap_or_default();
    let count = catalogue.templates.len();
    let chosen = tool.chosen(catalogue).map(|t| t.name.as_str());
    let keys = |action| key(bindings, labels, action);
    let hint = format!(
        "{} opens and closes this list · {} puts the Tool away · {} turns · {} places the whole \
         Structure or nothing · {} removes it in one step",
        keys(Action::Structures),
        keys(Action::ClearTool),
        keys(Action::TurnStructure),
        keys(Action::Place),
        keys(Action::Undo),
    );
    commands
        .spawn((
            Panel,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(theme::SCREEN_PAD_X),
                top: Val::Px(PANEL_TOP_PX),
                bottom: Val::Px(PANEL_BOTTOM_PX),
                width: Val::Px(CARD_WIDTH),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(Val::Px(theme::FRAME)),
                border_radius: BorderRadius::all(Val::Px(theme::ITEM_RADIUS)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(theme::LAYOUT_SURFACE),
            BorderColor::all(theme::GROUT),
            theme::shadow(),
        ))
        .with_children(|panel| {
            panel.spawn(theme::ring(theme::ITEM_RADIUS));
            header(panel);
            panel
                .spawn((
                    Card,
                    ScrollArea,
                    ScrollPosition(scroll),
                    Node {
                        flex_grow: 1.0,
                        min_height: Val::Px(0.0),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::axes(Val::Px(PAD_X), Val::Px(17.0)),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                ))
                .with_children(|body| {
                    eyebrow(body, &format!("TEMPLATES{game} · {count}"));
                    picks(body, catalogue, chosen, thumbs);
                    eyebrow(body, "TURN");
                    turn_row(body, &keys(Action::TurnStructure));
                    eyebrow(body, "WHAT PLACING DOES");
                    legend(body, 6.0, 14.0);
                    totals(body, |rows| {
                        total_row(rows, term("Authored"), value_with("1.0 m", " grid"));
                        total_row(
                            rows,
                            term("This World"),
                            value_with(format!("{voxel_size} m"), " Voxels"),
                        );
                    });
                    body.spawn(Node {
                        height: Val::Px(18.0),
                        flex_shrink: 0.0,
                        ..default()
                    });
                    outcome_box(body, 18.0);
                });
            footer(panel, hint);
        });
}

#[allow(clippy::too_many_arguments)]
fn render(
    mut commands: Commands,
    mut list: ResMut<StructureList>,
    panels: Query<Entity, With<Panel>>,
    (controls, cards): (Query<&Control>, Query<&ScrollPosition, With<Card>>),
    (tool, hotbar, catalogue): (Res<PlaceTool>, Res<Hotbar>, Res<Catalogue>),
    (bindings, labels, world): (Res<Bindings>, Res<Labels>, Res<WorldSettings>),
    (focus, thumbs): (Res<InputFocus>, Res<Thumbs>),
) {
    let want = if list.open {
        Some(true)
    } else if holding_place(&hotbar, &catalogue.0) {
        Some(false)
    } else {
        None
    };
    let rebuild = want != list.shown
        || (want == Some(true)
            && (list.dirty
                || tool.is_changed()
                || catalogue.is_changed()
                || labels.is_changed()
                || thumbs.is_changed()));
    if !rebuild {
        return;
    }
    // A rebuild must not throw the Player back to the top of a long list.
    let scroll = match (list.shown, want) {
        (Some(true), Some(true)) => cards.iter().next().map_or(Vec2::ZERO, |s| s.0),
        _ => Vec2::ZERO,
    };
    list.dirty = false;
    list.shown = want;
    let focused = focus
        .get()
        .and_then(|entity| controls.get(entity).ok())
        .copied();
    for panel in &panels {
        commands.entity(panel).despawn();
    }
    match want {
        None => {}
        Some(false) => {
            commands
                .spawn((
                    Panel,
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(hud);
        }
        Some(true) => {
            list_panel(
                &mut commands,
                (&catalogue.0, &thumbs),
                &tool,
                world.voxel_size,
                scroll,
                (&bindings, &labels),
            );
            let chosen = catalogue
                .0
                .templates
                .iter()
                .position(|t| Some(t) == tool.chosen(&catalogue.0));
            let target = focused
                .or(chosen.map(Control::Pick))
                .unwrap_or(Control::Close);
            commands.queue(move |world: &mut World| {
                let mut query = world.query::<(Entity, &Control)>();
                let found = query
                    .iter(world)
                    .find_map(|(entity, control)| (*control == target).then_some(entity));
                if let Some(entity) = found {
                    world
                        .resource_mut::<InputFocus>()
                        .set(entity, FocusCause::Auto);
                }
            });
        }
    }
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
    let keys = |action| key(&bindings, &labels, action);
    let line = |part: Info| -> String {
        match part {
            Info::Title => chosen.map_or("No Templates".to_owned(), |t| {
                format!(
                    "{} · facing {}",
                    super::menu_ui::declared_title(&t.name),
                    tool.facing()
                )
            }),
            Info::Facing => tool.facing().to_owned(),
            Info::Size => {
                let size = chosen.map(|t| size_text(turned_size_m(t, tool.turn)));
                match (size, ghost.outcome) {
                    (Some(size), Some(Outcome::Whole(n))) => {
                        format!("{size} · {} Voxels", grouped(n))
                    }
                    (Some(size), _) => size,
                    (None, _) => String::new(),
                }
            }
            Info::Count(slot) => grouped(ghost.tally[slot]),
            Info::Outcome => ghost
                .outcome
                .map_or("Aim at a surface to place it.".to_owned(), Outcome::text),
            Info::Key(action) => keys(action),
            Info::PlaceKey => format!("{} places it whole", keys(Action::Place)),
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
    let (fill, edge) = match ghost.outcome {
        None => (Color::NONE, QUIET_EDGE),
        Some(Outcome::Whole(_)) => (Color::NONE, WHOLE_EDGE),
        Some(Outcome::Waiting) => (WAIT_FILL, WAIT_EDGE),
        Some(Outcome::Refused(_)) => (REFUSED_FILL, REFUSED_EDGE),
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
            .add_message::<UseOutcome>()
            .add_message::<UseIntent>()
            .add_systems(Update, (toggle, open_on_use).chain());
        app.insert_resource(Catalogue(ContentCatalogue {
            tools: vec![CatalogueTool {
                id: ToolId(0),
                name: "terra:place".into(),
                kind: ToolKind::Place,
            }],
            templates: vec![CatalogueTemplate {
                name: "terra:cottage".into(),
                size_m: [7.0, 8.0, 5.0],
                thumbnails: Default::default(),
            }],
            ..default()
        }));
        let mut hotbar = Hotbar::default();
        hotbar.outside = Some(held);
        app.insert_resource(hotbar);
        app.world_mut().spawn((
            Window::default(),
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
}

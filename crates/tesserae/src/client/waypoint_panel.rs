//! The Map window's side panel: the Waypoints and Explore tabs and the Waypoint list. Every change
//! is an intent to the Server; the list on screen is whatever the Server last sent.

use bevy::asset::RenderAssetUsages;
use bevy::input_focus::{FocusCause, InputFocus, tab_navigation::TabIndex};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::text::LineHeight;
use bevy::text::TextLayout;
use bevy::ui_widgets::Activate;
use bevy::ui_widgets::Button;

use super::actions::Actions;
use super::buttons::{self, ButtonStyle};
use super::frame::PointerOnly;
use super::map_pop;
use super::map_window::{MapWindow, SidePanel, closed};
use super::menu_ui as ui;
use super::minimap::Spot;
use super::theme;
use super::waypoints::{Draft, Marks, from_player, icon_pixels};
use crate::server::waypoint::WaypointIntent;
use crate::session::SessionApp;

const ROW_STEP_PX: f32 = 40.0;
const ROW_ICON_SCALE: u32 = 2;
const ROW_ICON_PX: f32 = 10.0;
const ICON_DISC: Color = Color::srgb_u8(0x0b, 0x0f, 0x0c);
pub(super) const KIND_ON: Color = Color::srgb_u8(0x26, 0x33, 0x2a);
const ROW_SELECTED: Color = Color::srgba(1.0, 1.0, 1.0, 0.08);

/// What the side panel shows; kept while the window is closed.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(super) enum PanelTab {
    #[default]
    Waypoints,
    Explore,
}

impl PanelTab {
    const ALL: [PanelTab; 2] = [PanelTab::Waypoints, PanelTab::Explore];

    fn label(self) -> &'static str {
        match self {
            PanelTab::Waypoints => "Waypoints",
            PanelTab::Explore => "Explored",
        }
    }

    /// The tab Q or E pages to; with two, either key turns to the other.
    pub(super) fn other(self) -> Self {
        match self {
            PanelTab::Waypoints => PanelTab::Explore,
            PanelTab::Explore => PanelTab::Waypoints,
        }
    }
}

#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub(super) enum WpControl {
    /// A list row: opens the Waypoint's popover.
    Select(u32),
    Remove(u32),
    Kind(usize),
    Tab(PanelTab),
}

#[derive(Component)]
struct ListBox;
#[derive(Component)]
pub(super) struct Pane(pub(super) PanelTab);

#[derive(Component)]
struct Distance(u32);

pub(super) fn plugin(app: &mut App) {
    app.add_observer(activate).add_session_systems(
        Update,
        (
            (rebuild, distances, scroll_list, show_tab, name_tabs).run_if(not(closed)),
            paint_chips,
        ),
    );
}

/// A panel section; every one after the first is set off by a rule above it.
pub(super) fn section<'a>(
    panel: &'a mut ChildSpawnerCommands,
    tab: PanelTab,
) -> EntityCommands<'a> {
    panel.spawn((
        Pane(tab),
        Node {
            flex_direction: FlexDirection::Column,
            flex_shrink: 0.0,
            ..default()
        },
    ))
}

/// A key cap beside the tabs, as Key bindings shows Q and E.
fn key_cap(tabs: &mut ChildSpawnerCommands, key: &str) {
    tabs.spawn((
        BorderColor::all(theme::key_edge()),
        Node {
            padding: UiRect::axes(Val::Px(5.0), Val::Px(2.0)),
            margin: UiRect::horizontal(Val::Px(10.0)),
            border: UiRect::all(Val::Px(theme::RULE)),
            border_radius: BorderRadius::all(Val::Px(theme::RADIUS_TAG)),
            ..default()
        },
        Pickable::IGNORE,
    ))
    .with_child((
        Text::new(key),
        theme::font(theme::TEXT_KEY),
        TextColor(theme::chrome_ink()),
        Pickable::IGNORE,
    ));
}

/// The tabs between their Q and E caps, and the Waypoints pane; the window builds the Explored one.
pub(super) fn sections(panel: &mut ChildSpawnerCommands) {
    panel
        .spawn(Node {
            flex_shrink: 0.0,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            margin: UiRect::new(Val::Px(0.0), Val::Px(0.0), Val::Px(4.0), Val::Px(10.0)),
            ..default()
        })
        .with_children(|tabs| {
            key_cap(tabs, "Q");
            for tab in PanelTab::ALL {
                let button = buttons::button(
                    tabs,
                    tab.label(),
                    WpControl::Tab(tab),
                    ButtonStyle::Tab {
                        selected: tab == PanelTab::default(),
                    },
                );
                // Q and E reach the tabs; focus stays in the list.
                tabs.commands().entity(button).insert(PointerOnly);
            }
            key_cap(tabs, "E");
        });
    section(panel, PanelTab::Waypoints).with_children(|section| {
        section.spawn((
            ListBox,
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                ..default()
            },
        ));
    });
}

fn activate(
    event: On<Activate>,
    controls: Query<&WpControl>,
    spot: Res<Spot>,
    (mut marks, mut draft, mut window): (ResMut<Marks>, ResMut<Draft>, ResMut<MapWindow>),
    mut intents: MessageWriter<WaypointIntent>,
) {
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    match *control {
        WpControl::Select(id) => {
            let eye = spot.0.map(|(eye, _)| eye);
            map_pop::open_edit(&mut window, &mut marks, &mut draft, id, eye);
        }
        WpControl::Remove(id) => {
            intents.write(WaypointIntent::Remove { id });
            if window.pop == Some(map_pop::Pop::Edit(id)) {
                window.pop = None;
            }
        }
        WpControl::Kind(index) => draft.kind = index,
        WpControl::Tab(tab) => window.tab = tab,
    }
}

fn show_tab(window: Res<MapWindow>, mut panes: Query<(&Pane, &mut Node)>) {
    for (Pane(tab), mut node) in &mut panes {
        let display = if *tab == window.tab {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
}

/// The selected tab is underlined, and the Waypoints tab counts the list.
fn name_tabs(
    window: Res<MapWindow>,
    marks: Res<Marks>,
    mut tabs: Query<(&WpControl, &mut ui::Styled, &Children)>,
    mut labels: Query<&mut Text>,
) {
    for (control, mut styled, children) in &mut tabs {
        let WpControl::Tab(tab) = *control else {
            continue;
        };
        let on = tab == window.tab;
        if styled.selected != on {
            styled.selected = on;
        }
        let label = match tab {
            PanelTab::Waypoints => format!("{} {}", tab.label(), marks.list().len()),
            PanelTab::Explore => tab.label().to_string(),
        };
        for child in children {
            if let Ok(mut text) = labels.get_mut(*child)
                && text.0 != label
            {
                text.0.clone_from(&label);
            }
        }
    }
}

/// The chips light the kind the popover has chosen.
fn paint_chips(
    draft: Res<Draft>,
    mut chips: Query<(&WpControl, &mut BackgroundColor, &mut BorderColor)>,
) {
    for (control, mut fill, mut edge) in &mut chips {
        let WpControl::Kind(index) = control else {
            continue;
        };
        let on = draft.kind == *index;
        fill.set_if_neq(BackgroundColor(if on { KIND_ON } else { theme::RAISED }));
        edge.set_if_neq(BorderColor::all(if on {
            theme::SAGE
        } else {
            theme::edge()
        }));
    }
}

/// A kind's icon as an image node `side` pixels square.
pub(super) fn icon_node(
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

/// A kind's round badge: its icon in a ring of its colour.
pub(super) fn badge(
    parent: &mut ChildSpawnerCommands,
    images: &mut Assets<Image>,
    marks: &Marks,
    kind: &str,
) {
    parent
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
            BorderColor::all(colour_of(marks, kind)),
        ))
        .with_children(|ring| {
            icon_node(ring, images, marks, kind, ROW_ICON_SCALE, ROW_ICON_PX);
        });
}

/// The kind chips of a popover.
pub(super) fn kind_chips(
    row: &mut ChildSpawnerCommands,
    images: &mut Assets<Image>,
    marks: &Marks,
) {
    for (index, kind) in marks.choosable() {
        row.spawn((
            Button,
            TabIndex(0),
            WpControl::Kind(index),
            theme::focus_ring(),
            Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.0),
                padding: UiRect::axes(Val::Px(9.0), Val::Px(5.0)),
                border: UiRect::all(Val::Px(theme::RULE)),
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BackgroundColor(theme::RAISED),
            BorderColor::all(theme::edge()),
        ))
        .with_children(|pill| {
            icon_node(pill, images, marks, &kind.name, 1, 7.0);
            pill.spawn((
                Text::new(ui::declared_title(&kind.name)),
                theme::weighted(theme::TEXT_EYEBROW, theme::LABEL),
                LineHeight::Px(15.0),
                TextColor(theme::INK),
                Pickable::IGNORE,
            ));
        });
    }
}

fn distance_text(distance_m: f32) -> String {
    if distance_m >= 1000.0 {
        format!("{:.2} km", distance_m / 1000.0)
    } else {
        format!("{} m", distance_m.round() as i32)
    }
}

/// `403 m SW`, or the kind while the Player's place is unknown.
fn describe(kind: &str, from: Option<(f32, f32)>) -> String {
    match from {
        Some((metres, bearing)) => format!(
            "{} {}",
            distance_text(metres),
            super::compass::cardinal(bearing)
        ),
        None => ui::declared_title(kind),
    }
}

/// A whole number with the typographic minus the prototype uses.
pub(super) fn signed(n: f32) -> String {
    let n = n.round() as i32;
    if n < 0 {
        format!("\u{2212}{}", n.unsigned_abs())
    } else {
        n.to_string()
    }
}

fn distances(spot: Res<Spot>, marks: Res<Marks>, mut texts: Query<(&Distance, &mut Text)>) {
    let Some((eye, _)) = spot.0 else {
        return;
    };
    for (Distance(id), mut text) in &mut texts {
        if let Some(w) = marks.get(*id) {
            let now = describe(&w.kind, Some(from_player(w, Vec2::new(eye.x, eye.z))));
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

/// Rebuilds the list when the marks move; focus stays on the Waypoint it was on.
fn rebuild(
    mut commands: Commands,
    marks: Res<Marks>,
    spot: Res<Spot>,
    mut images: ResMut<Assets<Image>>,
    (lists, fresh): (Query<Entity, With<ListBox>>, Query<(), Added<ListBox>>),
    (controls, mut focus): (Query<&WpControl>, ResMut<InputFocus>),
    mut built: Local<Option<u32>>,
) {
    if *built == Some(marks.revision()) && fresh.is_empty() {
        return;
    }
    let Ok(list) = lists.single() else {
        return;
    };
    let eye = spot.0.map(|(eye, _)| Vec2::new(eye.x, eye.z));
    let focused = match focus.get().and_then(|e| controls.get(e).ok()) {
        Some(WpControl::Select(id)) => Some(*id),
        _ => None,
    };
    commands
        .entity(list)
        .despawn_children()
        .with_children(|list| {
            if marks.list().is_empty() {
                ui::text(list, "None yet.", theme::TEXT_SMALL, theme::muted());
            }
            for w in marks.list() {
                let selected = marks.selected() == Some(w.id);
                let row = list
                    .spawn((
                        Button,
                        TabIndex(0),
                        WpControl::Select(w.id),
                        Hovered::default(),
                        theme::focus_ring(),
                        Node {
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(10.0),
                            padding: UiRect::axes(Val::Px(6.0), Val::Px(5.0)),
                            border_radius: BorderRadius::all(Val::Px(6.0)),
                            ..default()
                        },
                        BackgroundColor(if selected { ROW_SELECTED } else { Color::NONE }),
                    ))
                    .with_children(|button| {
                        badge(button, &mut images, &marks, &w.kind);
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
                                    Distance(w.id),
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
                    })
                    .id();
                if focused == Some(w.id) {
                    focus.set(row, FocusCause::Auto);
                }
            }
        });
    *built = Some(marks.revision());
}

//! Each category's page and what its rows are called.

use super::*;
use crate::client::menu_ui::Tab;
use crate::client::presentation::WindowMode;
use crate::client::theme;
use crate::server::player::SpeedLevel;
use bevy::input_focus::tab_navigation::TabIndex;

pub(in crate::client) fn action_name(action: Action) -> &'static str {
    use Action::*;
    match action {
        Forward => "Move forward",
        Backward => "Move backward",
        Left => "Move left",
        Right => "Move right",
        Ascend => "Ascend / jump",
        Descend => "Descend / crouch",
        Sprint => "Cycle sprint speed",
        Respawn => "Respawn",
        MovementMode => "Switch flight / physics",
        View => "Switch first / third person",
        ZoomIn => "Third-person zoom in",
        ZoomOut => "Third-person zoom out",
        Faster => "Increase flight speed",
        Slower => "Decrease flight speed",
        Remove => "Remove",
        Place => "Place / ignite",
        PlacementGhost => "Toggle placement ghost",
        Compass => "Toggle compass",
        Minimap => "Toggle minimap",
        Sample => "Sample",
        EmptyHand => "Empty hand",
        Shape => "Next Shape",
        Turn => "Turn Shape",
        Flip => "Flip Shape",
        CycleTool => "Take tool",
        Use => "Use",
        ClearTool => "Clear tool",
        Slot1 => "Quick slot 1",
        Slot2 => "Quick slot 2",
        Slot3 => "Quick slot 3",
        Slot4 => "Quick slot 4",
        Slot5 => "Quick slot 5",
        Slot6 => "Quick slot 6",
        Slot7 => "Quick slot 7",
        Slot8 => "Quick slot 8",
        Slot9 => "Quick slot 9",
        PreviousSlot => "Previous quick slot",
        NextSlot => "Next quick slot",
        Undo => "Undo",
        Redo => "Redo",
        Layout => "Layout mode and menu",
        LayoutMore => "Layout: more Voxels",
        LayoutLess => "Layout: fewer Voxels",
        LayoutReverse => "Layout: reverse the axis",
        LayoutActive => "Layout: next axis",
        TurnStructure => "Place Tool: turn the Structure",
        Structures => "Structure placement",
        Menu => "Open the Menu",
        Settings => "Open Settings",
        Inventory => "Open Inventory",
        Map => "Open the Map",
        SearchCatalogue => "Search the Catalogue",
        ReleaseCursor => "Release cursor",
        Help => "Open input settings",
        Diagnostics => "Diagnostics",
    }
}
pub(in crate::client) fn category(action: Action) -> &'static str {
    use Action::*;
    match action {
        Forward | Backward | Left | Right | Ascend | Descend | Sprint | Respawn | Faster
        | Slower | MovementMode | View | ZoomIn | ZoomOut => "Movement",
        Place | PlacementGhost | Remove | Sample | EmptyHand | Shape | Turn | Flip | CycleTool
        | Use | ClearTool | Layout | LayoutMore | LayoutLess | LayoutReverse | LayoutActive
        | TurnStructure | Structures => "Building",
        _ => "Interface",
    }
}
/// Shown in place of a Map or Compass setting's hint while the World's Game forbids them.
const UNAVAILABLE: &str = "Not available in this World.";
const BINDING_HELP: &str =
    "Select a key cap, then press the new key. Esc cancels; Delete clears it.";

/// The page under the sheet's title, as opposed to a dialog over it.
#[derive(Component)]
pub(in crate::client) struct Page;

pub(super) type RenderData<'w> = (
    Res<'w, crate::client::actions::Labels>,
    (Res<'w, Bindings>, Res<'w, Preferences>),
    Option<Res<'w, crate::client::map_cache::MapAllowed>>,
    WorldDistances<'w>,
);
pub(super) type RenderContext<'w, 's> = (
    (
        Res<'w, crate::client::edit::Hotbar>,
        Res<'w, crate::client::edit::Catalogue>,
        Res<'w, crate::client::edit::Notice>,
    ),
    Query<'w, 's, (), (With<Scalar>, With<bevy::ui::Pressed>)>,
);
type FocusTree<'w, 's> = (
    Res<'w, bevy::input_focus::InputFocus>,
    Query<'w, 's, &'static Children>,
    Query<'w, 's, &'static ChildOf>,
    Query<'w, 's, (), Focusable>,
);
type Focusable = (With<TabIndex>, Without<bevy::ui::InteractionDisabled>);
type Parts<'w, 's> = (
    Query<'w, 's, Entity, With<Panel>>,
    Query<'w, 's, Entity, With<Page>>,
    Query<'w, 's, Entity, With<crate::client::menu_ui::SettingsBody>>,
    Query<'w, 's, &'static ScrollPosition, With<SettingsScroll>>,
);
/// Each category's scroll, the category shown, and the place of the
/// focused control.
type Memory<'s> = (
    Local<'s, [Vec2; Tab::ORDER.len()]>,
    Local<'s, Option<Tab>>,
    Local<'s, Option<usize>>,
);

/// A page's controls in the order focus meets them.
fn controls(
    page: Entity,
    children: &Query<&Children>,
    focusable: &Query<(), Focusable>,
) -> Vec<Entity> {
    children
        .iter_descendants_depth_first(page)
        .filter(|e| focusable.contains(*e))
        .collect()
}

/// Rebuilds the open page when something on it changed. A change rebuilds
/// the control that made it, so focus returns to the control at the same
/// place.
pub(in crate::client) fn render(
    mut commands: Commands,
    mut settings: ResMut<Settings>,
    (panels, pages, body, scrolls): Parts,
    (labels, active, map_allowed, distances): RenderData,
    (mut positions, mut previous, mut remembered): Memory,
    (context, pressed): RenderContext,
    (focus, children, parents, focusable): FocusTree,
) {
    let map_changed = map_allowed.as_ref().is_some_and(|a| a.is_changed())
        || distances.0.as_ref().is_some_and(|d| d.is_changed());
    if !pressed.is_empty() || (!settings.dirty && !labels.is_changed() && !map_changed) {
        return;
    }
    if let Some(tab) = *previous
        && let Some(scroll) = scrolls.iter().next()
    {
        positions[scroll_slot(tab)] = scroll.0;
    }
    let focused = focus.get();
    let in_panel = focused.is_none_or(|f| {
        std::iter::once(f)
            .chain(parents.iter_ancestors(f))
            .any(|e| panels.contains(e))
    });
    if let Some(page) = pages.iter().next()
        && let Some(at) = focused.and_then(|f| {
            controls(page, &children, &focusable)
                .iter()
                .position(|e| *e == f)
        })
    {
        *remembered = Some(at);
    }
    // A category just opened focuses its first control.
    if *previous != Some(settings.tab) {
        *remembered = Some(0);
    }
    *previous = settings.open.then_some(settings.tab);
    settings.dirty = false;
    for entity in &panels {
        commands.entity(entity).despawn();
    }
    if !settings.open {
        return;
    }
    let Ok(body) = body.single() else {
        // The frame is built this frame; the page follows it next frame.
        settings.dirty = true;
        return;
    };
    let (bindings, p) = (&*active.0, &*active.1);
    let index = scroll_slot(settings.tab);
    let page = commands
        .spawn((
            Panel,
            Page,
            ChildOf(body),
            Node {
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn((
                SettingsScroll,
                ScrollArea,
                ScrollPosition(positions[index]),
                Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    min_width: Val::Px(0.0),
                    min_height: Val::Px(0.0),
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
            ))
            .with_children(|list| {
                if !settings.notice.is_empty() {
                    crate::client::menu_ui::text(list, &settings.notice, 12.0, theme::WARNING);
                }
                page(
                    list,
                    &settings,
                    (bindings, p),
                    &labels,
                    map_allowed.is_none_or(|a| a.0),
                    (&context.0, &context.1, &context.2),
                    game(&distances),
                );
            });
        })
        .id();
    if settings.modal() {
        modal(&mut commands, &settings);
    } else if !settings.capturing()
        && in_panel
        && let Some(at) = *remembered
    {
        commands.queue(move |world: &mut World| refocus(world, page, at));
    }
}

fn refocus(world: &mut World, page: Entity, at: usize) {
    let mut order = Vec::new();
    let mut stack = vec![page];
    while let Some(entity) = stack.pop() {
        if world.get::<TabIndex>(entity).is_some()
            && world.get::<bevy::ui::InteractionDisabled>(entity).is_none()
        {
            order.push(entity);
        }
        if let Some(children) = world.get::<Children>(entity) {
            stack.extend(children.iter().rev());
        }
    }
    if let Some(entity) = order.get(at).or(order.last()) {
        world
            .resource_mut::<bevy::input_focus::InputFocus>()
            .set(*entity, bevy::input_focus::FocusCause::Navigated);
    }
}

/// Where each category keeps its scroll position while another shows.
fn scroll_slot(tab: Tab) -> usize {
    Tab::ORDER.iter().position(|t| *t == tab).unwrap_or(0)
}

/// A selector over `choices`, on the one equal to `current`.
pub(super) fn select<T: PartialEq + Copy>(
    row: &mut ChildSpawnerCommands,
    current: T,
    choices: impl IntoIterator<Item = (T, String)>,
    operation: impl Fn(T) -> Operation,
) {
    let choices: Vec<_> = choices.into_iter().collect();
    let chosen = choices.iter().position(|(v, _)| *v == current).unwrap_or(0);
    choice(
        row,
        choices
            .into_iter()
            .map(|(value, label)| (label, operation(value)))
            .collect(),
        chosen,
    );
}

/// A row with a [`select`] whose chosen option's note, if it has one, the help strip shows after
/// that option's label.
pub(super) fn noted_select<T: PartialEq + Copy>(
    card: &mut ChildSpawnerCommands,
    title: &str,
    help: &str,
    current: T,
    choices: impl IntoIterator<Item = (T, String, Option<&'static str>)>,
    operation: impl Fn(T) -> Operation,
) {
    let choices: Vec<_> = choices.into_iter().collect();
    let note = choices
        .iter()
        .find(|(value, ..)| *value == current)
        .and_then(|(_, label, note)| Some((label.clone(), (*note)?)));
    noted_row(card, title, help, note, |row| {
        select(
            row,
            current,
            choices.into_iter().map(|(value, label, _)| (value, label)),
            operation,
        )
    });
}

pub(super) fn page(
    list: &mut ChildSpawnerCommands,
    settings: &Settings,
    (bindings, p): (&Bindings, &Preferences),
    labels: &crate::client::actions::Labels,
    map_allowed: bool,
    context: (
        &crate::client::edit::Hotbar,
        &crate::client::edit::Catalogue,
        &crate::client::edit::Notice,
    ),
    (game, world): (
        crate::server::quality::DeclaredStreaming,
        crate::world::WorldSettings,
    ),
) {
    match settings.tab {
        Tab::Behavior => behavior(list, settings, bindings, p, labels, context),
        Tab::Display => display(list, p, map_allowed),
        Tab::Distance => distance(list, p, (game, &world)),
        Tab::Audio => audio(list, p),
        Tab::Bindings => bindings_page(list, settings, bindings, labels, map_allowed),
        Tab::Controller => controller(list, settings, bindings, p, labels, map_allowed),
        Tab::Accessibility => accessibility(list, p),
        Tab::Catalogue | Tab::Recipes => {}
    }
}

pub(super) fn behavior(
    list: &mut ChildSpawnerCommands,
    settings: &Settings,
    bindings: &Bindings,
    p: &Preferences,
    labels: &crate::client::actions::Labels,
    context: (
        &crate::client::edit::Hotbar,
        &crate::client::edit::Catalogue,
        &crate::client::edit::Notice,
    ),
) {
    group(list, "MOVEMENT & BUILDING", |card| {
        noted_select(
            card,
            "Sprint speed",
            "Sprint cycles through these and keeps the one you stop on.",
            p.speed_level,
            [
                SpeedLevel::Walk,
                SpeedLevel::One,
                SpeedLevel::Two,
                SpeedLevel::Three,
            ]
            .map(|level| {
                let note = match level {
                    SpeedLevel::Walk => Some("no sprint."),
                    SpeedLevel::One => Some("a brisk run."),
                    SpeedLevel::Two => None,
                    SpeedLevel::Three => Some("the fastest; hard to stop on an edge."),
                };
                (level, level.label().to_owned(), note)
            }),
            Operation::Sprint,
        );
        noted_select(
            card,
            "Removal activation",
            "How Remove starts breaking a Voxel.",
            p.tap_remove,
            [
                (
                    false,
                    "Hold".into(),
                    Some("keep the binding down until the Voxel breaks."),
                ),
                (
                    true,
                    "Timed tap".into(),
                    Some("one press starts it; press again to stop."),
                ),
            ],
            Operation::Removal,
        );
        setting_row(
            card,
            "Repeat while held",
            "Keep placing or removing while the binding is held.",
            |row| toggle(row, p.repeat_held, Operation::Repeat),
        );
        setting_row(
            card,
            "Placement ghost",
            "Show a translucent preview of the Voxel where Place would put it.",
            |row| toggle(row, p.placement_ghost, Operation::Ghost),
        );
    });
    group(list, "MOUSE & QUICK SLOTS", |card| {
        setting_row(
            card,
            "Mouse sensitivity",
            "Relative to the default mouse feel.",
            |row| slider(row, Scalar::Mouse, p.sensitivity / 0.003),
        );
        setting_row(
            card,
            "Invert vertical look",
            "Moving the mouse up looks down.",
            |row| toggle(row, p.invert_y, Operation::Invert),
        );
        setting_row(
            card,
            "Scroll direction",
            "Which way the wheel steps through quick slots.",
            |row| {
                select(
                    row,
                    p.reverse_scroll,
                    [(false, "Standard".into()), (true, "Reversed".into())],
                    Operation::ScrollDirection,
                )
            },
        );
        setting_row(
            card,
            "Scroll sensitivity",
            "How far one wheel notch moves.",
            |row| slider(row, Scalar::Scroll, p.scroll_sensitivity),
        );
        setting_row(
            card,
            "Show quick reference",
            "List the main bindings in the corner while you play.",
            |row| toggle(row, settings.show_reference, Operation::Reference),
        );
    });
    if settings.show_reference {
        crate::client::menu_ui::paragraph(
            list,
            crate::client::edit::hud_text(context.0, &context.1.0, context.2, bindings, labels, p),
            theme::TEXT_SMALL,
            theme::muted(),
        );
    }
}

pub(super) fn accessibility(list: &mut ChildSpawnerCommands, p: &Preferences) {
    group(list, "COMFORT & READABILITY", |card| {
        setting_row(
            card,
            "Reduced motion",
            "Use immediate interface transitions.",
            |row| toggle(row, p.reduced_motion, Operation::ReducedMotion),
        );
        setting_row(
            card,
            "Stronger UI contrast",
            "Increase panel borders and secondary text contrast.",
            |row| toggle(row, p.stronger_contrast, Operation::StrongerContrast),
        );
        setting_row(
            card,
            "Simple menu background",
            "Replace the scenery with a solid background.",
            |row| toggle(row, p.solid_background, Operation::SolidBackground),
        );
        setting_row(
            card,
            "Text size",
            "100% is standard; 200% doubles the text in menus and the HUD.",
            |row| {
                select(
                    row,
                    p.text_scale,
                    crate::client::profile::TEXT_SCALES.map(|scale| (scale, format!("{scale}%"))),
                    Operation::TextScale,
                )
            },
        );
    });
}

pub(super) fn display(list: &mut ChildSpawnerCommands, p: &Preferences, map_allowed: bool) {
    let map_hint = |text: &'static str| if map_allowed { text } else { UNAVAILABLE };
    group(list, "WINDOW", |card| {
        noted_select(
            card,
            "Window mode",
            "Borderless fullscreen covers the screen without changing its mode, so switching away is instant. Fullscreen takes over the display.",
            p.window_mode,
            WindowMode::ALL.map(|mode| {
                let note = (mode == WindowMode::Fullscreen)
                    .then_some("switching to another app may flash the screen.");
                (mode, mode.label().into(), note)
            }),
            Operation::WindowMode,
        );
    });
    group(list, "FRAME PRESENTATION", |card| {
        noted_select(
            card,
            "Display sync",
            "Match frame presentation to your display. Synced is recommended and free of tearing; with sync off, frames may tear, so set a frame rate limit.",
            p.display_sync,
            [
                (true, "Display synced".into(), None),
                (
                    false,
                    "Sync off".into(),
                    Some("set a frame rate limit too."),
                ),
            ],
            Operation::Sync,
        );
        noted_select(
            card,
            "Frame rate limit",
            "An optional ceiling, independent of display sync.",
            p.fps_cap,
            [Some(60), Some(120), Some(144), Some(240), Some(1000), None].map(|cap| match cap {
                Some(n) => (cap, n.to_string(), None),
                None => (
                    cap,
                    "No limit".into(),
                    Some("the GPU runs flat out, even in menus."),
                ),
            }),
            Operation::Cap,
        );
    });
    group(list, "IMAGE QUALITY", |card| {
        noted_select(
            card,
            "Anti-aliasing",
            "Smooth voxel edges. MSAA costs more.",
            p.anti_aliasing,
            crate::client::anti_alias::CHOICES.map(|aa| {
                let note = (aa == crate::client::anti_alias::AntiAliasing::Off)
                    .then_some("sharpest and cheapest; edges shimmer.");
                (aa, aa.label().to_owned(), note)
            }),
            Operation::AntiAliasing,
        );
        noted_select(
            card,
            "Render scale",
            "Lower values reduce rendering work. Menus stay sharp.",
            p.render_scale,
            crate::client::render_scale::SCALES
                .iter()
                .rev()
                .map(|&scale| {
                    let note = (scale == 100).then_some("native resolution.");
                    (scale, format!("{scale}%"), note)
                }),
            Operation::Scale,
        );
    });
    group(list, "MAP AND COMPASS", |card| {
        setting_row(
            card,
            "Compass",
            map_hint("A strip along the top showing your heading."),
            |row| toggle(row, p.compass, Operation::Compass),
        );
        setting_row(
            card,
            "Minimap",
            map_hint("A small map in the corner, north up, centred on you."),
            |row| toggle(row, p.minimap, Operation::Minimap),
        );
        setting_row(
            card,
            "Minimap size",
            map_hint("Shown only while the minimap is on."),
            |row| {
                select(
                    row,
                    p.minimap_size,
                    crate::client::minimap::MinimapSize::ALL
                        .map(|size| (size, size.label().to_owned())),
                    Operation::MinimapSize,
                )
            },
        );
    });
}

/// The input columns a binding row shows.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Columns {
    Both,
    Controller,
}

pub(super) fn bindings_page(
    list: &mut ChildSpawnerCommands,
    settings: &Settings,
    bindings: &Bindings,
    labels: &crate::client::actions::Labels,
    map_allowed: bool,
) {
    list.spawn(Node {
        align_items: AlignItems::Center,
        column_gap: Val::Px(10.0),
        margin: UiRect::bottom(Val::Px(12.0)),
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|tabs| {
        let cap = |tabs: &mut ChildSpawnerCommands, key: &str| {
            tabs.spawn((
                BorderColor::all(theme::key_edge()),
                Node {
                    padding: UiRect::axes(Val::Px(5.0), Val::Px(2.0)),
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
        };
        cap(tabs, "Q");
        for category in Category::ALL {
            let tab = buttons::button(
                tabs,
                category.label(),
                Operation::Category(category),
                ButtonStyle::Tab {
                    selected: settings.category == category,
                },
            );
            tabs.commands()
                .entity(tab)
                .entry::<Node>()
                .and_modify(|mut node| node.padding.left = Val::Px(8.0))
                .and_modify(|mut node| node.padding.right = Val::Px(8.0));
        }
        cap(tabs, "E");
    });
    let mut entries: Vec<_> = bindings
        .keyboard
        .keys()
        .filter(|action| category(**action) == settings.category.label())
        .copied()
        .collect();
    entries.sort_by_key(|action| binding_order(*action));
    binding_rows(
        list,
        settings,
        bindings,
        entries,
        labels,
        (Columns::Both, map_allowed),
    );
}

/// One row per action: its name, then a key cap per input, and ↺ once it
/// differs from the default.
pub(super) fn binding_rows(
    list: &mut ChildSpawnerCommands,
    settings: &Settings,
    bindings: &Bindings,
    entries: Vec<Action>,
    labels: &crate::client::actions::Labels,
    (columns, map_allowed): (Columns, bool),
) {
    let cell = |caps: &mut ChildSpawnerCommands,
                content: &mut dyn FnMut(&mut ChildSpawnerCommands)| {
        caps.spawn(Node {
            min_width: Val::Px(150.0),
            column_gap: Val::Px(6.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|c| content(c));
    };
    list.spawn((
        Node {
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect::vertical(Val::Px(4.0)),
            border: UiRect::bottom(Val::Px(theme::RULE)),
            flex_shrink: 0.0,
            ..default()
        },
        BorderColor::all(theme::edge()),
    ))
    .with_children(|head| {
        head.spawn(Node::default());
        head.spawn(Node {
            column_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|caps| {
            let heading = |caps: &mut ChildSpawnerCommands, name: &str| {
                let name = name.to_owned();
                cell(caps, &mut |c| {
                    crate::client::menu_ui::text(c, name.clone(), 12.0, theme::muted())
                });
            };
            if columns == Columns::Both {
                heading(caps, "Keyboard");
            }
            heading(caps, "Controller");
            caps.spawn(Node {
                width: Val::Px(36.0),
                ..default()
            });
        });
    });
    let defaults = Bindings::default();
    let cap_label = |binding: &Option<Binding>| binding.map(|b| cap_text(labels.binding(b)));
    for action in entries {
        let keyboard = bindings.keyboard.get(&action).copied().unwrap_or_default();
        let pad = bindings.pad.get(&action).copied().unwrap_or_default();
        let waiting = settings
            .capture
            .filter(|(_, a, ..)| *a == action)
            .map(|(device, _, slot, _)| (device, slot));
        let help =
            if !map_allowed && matches!(action, Action::Map | Action::Compass | Action::Minimap) {
                UNAVAILABLE
            } else {
                BINDING_HELP
            };
        list.spawn((
            RowHelp {
                title: action_name(action).into(),
                text: help.into(),
                note: None,
            },
            Node {
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                column_gap: Val::Px(15.0),
                min_height: Val::Px(56.0),
                padding: UiRect::vertical(Val::Px(10.0)),
                border: UiRect::bottom(Val::Px(theme::RULE)),
                flex_shrink: 0.0,
                ..default()
            },
            BorderColor::all(theme::edge()),
        ))
        .with_children(|row| {
            row.spawn((
                Text::new(action_name(action)),
                theme::weighted(15.0, theme::LABEL),
                TextColor(theme::INK),
                Node {
                    flex_grow: 1.0,
                    flex_basis: Val::Px(0.0),
                    min_width: Val::Px(0.0),
                    ..default()
                },
                Pickable::IGNORE,
            ));
            // Gradients draw over a later sibling, so the overlay alone shows.
            let shown = if waiting.is_some() {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };
            row.spawn((
                shown,
                Node {
                    column_gap: Val::Px(8.0),
                    align_items: AlignItems::Center,
                    flex_shrink: 0.0,
                    ..default()
                },
            ))
            .with_children(|caps| {
                let column = |caps: &mut ChildSpawnerCommands,
                              device: Device,
                              slots: [Option<Binding>; 2]| {
                    cell(caps, &mut |c| {
                        for (slot, binding) in slots.iter().enumerate() {
                            if slot == 1 && binding.is_none() {
                                continue;
                            }
                            key_cap(
                                c,
                                cap_label(binding),
                                Operation::Capture(device, action, slot),
                            );
                        }
                    });
                };
                if columns == Columns::Both {
                    column(caps, Device::KeyboardMouse, keyboard);
                }
                column(caps, Device::Pad, pad);
                caps.spawn(Node {
                    width: Val::Px(36.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                })
                .with_children(|reset| {
                    if defaults.keyboard.get(&action) != Some(&keyboard)
                        || defaults.pad.get(&action) != Some(&pad)
                    {
                        buttons::button(
                            reset,
                            "↺",
                            Operation::ResetAction(action),
                            ButtonStyle::Small {
                                kind: crate::client::cinematic::Kind::Ghost,
                            },
                        );
                    }
                });
            });
            if let Some((device, slot)) = waiting {
                waiting_overlay(row, (device, action, slot));
            }
        });
    }
}

pub(super) fn controller(
    list: &mut ChildSpawnerCommands,
    settings: &Settings,
    bindings: &Bindings,
    p: &Preferences,
    labels: &crate::client::actions::Labels,
    map_allowed: bool,
) {
    let pad = &p.pad;
    group(list, "STICKS & BUTTONS", |card| {
        setting_row(
            card,
            "Look sensitivity",
            "How fast the right stick turns the view.",
            |row| slider(row, Scalar::PadLook, pad.look_sensitivity),
        );
        setting_row(
            card,
            "Invert vertical look",
            "Pushing the right stick up looks down.",
            |row| toggle(row, pad.invert_y, Operation::PadInvert),
        );
        setting_row(
            card,
            "Movement deadzone",
            "Ignore small movements caused by stick drift.",
            |row| slider(row, Scalar::MoveDeadzone, pad.move_deadzone),
        );
        setting_row(
            card,
            "Look deadzone",
            "Ignore small right-stick movements.",
            |row| slider(row, Scalar::LookDeadzone, pad.look_deadzone),
        );
        noted_select(
            card,
            "Crouch activation",
            "How the crouch button works.",
            pad.toggle_crouch,
            [
                (false, "Hold".into(), None),
                (
                    true,
                    "Toggle".into(),
                    Some("press once to crouch, again to stand."),
                ),
            ],
            Operation::PadCrouch,
        );
        setting_row(
            card,
            "Rumble on menu press",
            "A short rumble when a controller button presses something in a menu.",
            |row| toggle(row, pad.menu_rumble, Operation::PadMenuRumble),
        );
        disclosure(
            card,
            "Touch controls",
            settings.touch_open,
            Operation::TouchControls,
            |touch_rows| {
                let touch = &p.touch;
                setting_row(
                    touch_rows,
                    "Look sensitivity",
                    "How fast dragging turns the view.",
                    |row| slider(row, Scalar::TouchLook, touch.look_sensitivity),
                );
                setting_row(
                    touch_rows,
                    "Invert vertical look",
                    "Dragging up looks down.",
                    |row| toggle(row, touch.invert_y, Operation::TouchInvert),
                );
                setting_row(
                    touch_rows,
                    "Layout",
                    "Mirrored puts the movement stick on the right.",
                    |row| {
                        select(
                            row,
                            touch.mirrored,
                            [(false, "Normal".into()), (true, "Mirrored".into())],
                            Operation::TouchMirror,
                        )
                    },
                );
                setting_row(
                    touch_rows,
                    "Control size",
                    "The size of the on-screen buttons.",
                    |row| {
                        use crate::client::profile::TouchSize::*;
                        select(
                            row,
                            touch.size,
                            [
                                (Small, "Small".into()),
                                (Normal, "Normal".into()),
                                (Enlarged, "Enlarged".into()),
                            ],
                            Operation::TouchSize,
                        )
                    },
                );
                setting_row(
                    touch_rows,
                    "Control opacity",
                    "How solid the on-screen buttons look.",
                    |row| slider(row, Scalar::TouchOpacity, touch.opacity),
                );
            },
        );
    });
    group(list, "CONTROLLER BUTTONS", |buttons| {
        let mut entries: Vec<_> = bindings
            .pad
            .keys()
            .filter(|action| !matches!(action, Action::ReleaseCursor))
            .copied()
            .collect();
        entries.sort_by_key(|action| binding_order(*action));
        binding_rows(
            buttons,
            settings,
            bindings,
            entries,
            labels,
            (Columns::Controller, map_allowed),
        );
    });
}

pub(super) fn binding_order(action: Action) -> usize {
    use Action::*;
    [
        Forward,
        Backward,
        Left,
        Right,
        Ascend,
        Descend,
        Sprint,
        Respawn,
        MovementMode,
        Faster,
        Slower,
        View,
        ZoomIn,
        ZoomOut,
        Place,
        PlacementGhost,
        Compass,
        Minimap,
        Remove,
        Sample,
        EmptyHand,
        Turn,
        Flip,
        Shape,
        CycleTool,
        Use,
        ClearTool,
        Layout,
        LayoutMore,
        LayoutLess,
        LayoutReverse,
        LayoutActive,
        TurnStructure,
        Structures,
        Menu,
        Inventory,
        Map,
        SearchCatalogue,
        Help,
        Settings,
        ReleaseCursor,
        Undo,
        Redo,
        Diagnostics,
        Slot1,
        Slot2,
        Slot3,
        Slot4,
        Slot5,
        Slot6,
        Slot7,
        Slot8,
        Slot9,
        PreviousSlot,
        NextSlot,
    ]
    .iter()
    .position(|candidate| *candidate == action)
    .unwrap()
}
pub(super) fn binding_display(value: String) -> String {
    value
        .replace("ControlLeft", "Left Ctrl")
        .replace("ControlRight", "Right Ctrl")
        .replace("ShiftLeft", "Left Shift")
        .replace("ShiftRight", "Right Shift")
        .replace("AltLeft", "Left Alt")
        .replace("AltRight", "Right Alt")
        .replace("SuperLeft", "Left Super")
        .replace("SuperRight", "Right Super")
        .replace("BracketLeft", "[")
        .replace("BracketRight", "]")
}

fn audio(list: &mut ChildSpawnerCommands, p: &Preferences) {
    group(list, "Sound mix", |card| {
        for (name, hint, kind, value) in [
            (
                "Master volume",
                "Everything the game plays.",
                Scalar::MasterVolume,
                p.volumes.master,
            ),
            (
                "Effects",
                "All action sounds in the World. Each Sound effects row is relative to this.",
                Scalar::EffectsVolume,
                p.volumes.effects,
            ),
            (
                "Ambient",
                "Environmental beds around you.",
                Scalar::AmbientVolume,
                p.volumes.ambient,
            ),
            (
                "Music",
                "Occasional tracks, with silence between them.",
                Scalar::MusicVolume,
                p.volumes.music,
            ),
            (
                "Interface",
                "Menu clicks and other menu sounds, apart from Effects. 0 mutes them.",
                Scalar::InterfaceVolume,
                p.volumes.interface,
            ),
        ] {
            setting_row(card, name, hint, |row| slider(row, kind, f32::from(value)));
        }
    });
    noted_group(
        list,
        "Sound effects",
        Some("Each is relative to Effects; 0 mutes it."),
        |card| {
            use tesserae_core::audio::EffectGroup::*;
            for (name, hint, group) in [
                (
                    "Footsteps",
                    "Walking and running on each Material.",
                    Footsteps,
                ),
                ("Landings", "The impact when you land.", Landings),
                (
                    "Breaking & striking",
                    "Removing and striking Voxels.",
                    Breaking,
                ),
                ("Placing", "Voxels placed in the World.", Placing),
                ("Eating", "Bites while eating food.", Eating),
                ("Swimming", "Strokes while moving through water.", Swimming),
                (
                    "Floating water",
                    "Gentle water movement while resting at the surface.",
                    Floating,
                ),
                ("Pickups", "Items joining your Inventory.", Pickups),
                ("Tools", "Successful measurements and refused uses.", Tools),
                (
                    "Hurt & death",
                    "Feedback when you take damage or die.",
                    Vitals,
                ),
            ] {
                setting_row(card, name, hint, |row| {
                    slider(
                        row,
                        Scalar::DetailVolume(group),
                        f32::from(p.volumes.detail.get(group)),
                    )
                });
            }
        },
    );
}

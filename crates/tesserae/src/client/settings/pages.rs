//! Each category's page and what its rows are called.

use super::*;
use crate::client::theme;

pub(in crate::client) fn action_name(action: Action) -> &'static str {
    use Action::*;
    match action {
        Forward => "Move forward",
        Backward => "Move backward",
        Left => "Move left",
        Right => "Move right",
        Ascend => "Ascend / jump",
        Descend => "Descend / crouch",
        Sprint => "Sprint",
        Respawn => "Respawn",
        MovementMode => "Switch flight / physics",
        View => "Switch first / third person",
        ZoomIn => "Third-person zoom in",
        ZoomOut => "Third-person zoom out",
        Faster => "Increase flight speed",
        Slower => "Decrease flight speed",
        Remove => "Remove",
        Place => "Place / ignite",
        Sample => "Sample",
        EmptyHand => "Empty hand",
        Shape => "Next Shape",
        Turn => "Turn Shape",
        Flip => "Flip Shape",
        CycleTool => "Take tool",
        Use => "Use tool",
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
        Menu => "Open the Menu",
        Settings => "Open Settings",
        Inventory => "Open Inventory",
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
        Place | Remove | Sample | EmptyHand | Shape | Turn | Flip | CycleTool | Use | ClearTool
        | Layout | LayoutMore | LayoutLess | LayoutReverse | LayoutActive => "Building",
        _ => "Interface",
    }
}
pub(super) type RenderData<'w> = (
    Res<'w, crate::client::actions::Labels>,
    Res<'w, crate::client::menu_ui::Searches>,
    (Res<'w, Bindings>, Res<'w, Preferences>),
);
pub(super) type RenderContext<'w, 's> = (
    (
        Res<'w, crate::client::edit::Hotbar>,
        Res<'w, crate::client::edit::Catalogue>,
        Res<'w, crate::client::edit::Notice>,
    ),
    Query<'w, 's, (), (With<Scalar>, With<bevy::ui::Pressed>)>,
);
pub(in crate::client) fn render(
    mut commands: Commands,
    mut settings: ResMut<Settings>,
    (panels, body): (
        Query<Entity, With<Panel>>,
        Query<Entity, With<crate::client::menu_ui::SettingsBody>>,
    ),
    (labels, searches, active): RenderData,
    scrolls: Query<&ScrollPosition, With<SettingsScroll>>,
    (mut positions, mut previous, mut query): (
        Local<[Vec2; 4]>,
        Local<Option<crate::client::menu_ui::Tab>>,
        Local<String>,
    ),
    (context, pressed): RenderContext,
) {
    use crate::client::menu_ui::{self as ui, Tab};
    if !pressed.is_empty() || (!settings.dirty && !labels.is_changed()) {
        return;
    }
    if let Some(tab) = *previous {
        let index = match tab {
            Tab::Behavior => 0,
            Tab::Bindings => 1,
            Tab::Controller => 3,
            _ => 2,
        };
        if let Some(scroll) = scrolls.iter().next() {
            positions[index] = scroll.0;
        }
    }
    if *query != searches.bindings {
        positions[1] = Vec2::ZERO;
        *query = searches.bindings.clone();
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
    if settings.notice == "Applied" {
        crate::client::menu_ui::notify(
            &mut commands,
            "Settings applied",
            crate::client::menu_ui::Tone::Change,
        );
        settings.notice.clear();
    }
    let Some((bindings, p)) = &settings.draft else {
        return;
    };
    let changed = bindings != &*active.0 || p != &*active.1;
    let index = match settings.tab {
        Tab::Behavior => 0,
        Tab::Bindings => 1,
        Tab::Controller => 3,
        _ => 2,
    };
    commands
        .spawn((
            Panel,
            ChildOf(body),
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                ..default()
            },
        ))
        .with_children(|root| {
            if !settings.notice.is_empty() {
                ui::text(root, &settings.notice, 12.0, theme::WARNING);
            }
            root.spawn((
                SettingsScroll,
                ScrollArea,
                ScrollPosition(positions[index]),
                Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    flex_basis: Val::Px(0.0),
                    min_height: Val::Px(0.0),
                    padding: UiRect::axes(Val::Px(theme::SHEET_PAD_X), Val::Px(20.0)),
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
            ))
            .with_children(|list| {
                page(
                    list,
                    &settings,
                    &labels,
                    &searches,
                    (&context.0, &context.1, &context.2),
                );
            });
            footer(root, changed);
        });
    if settings.modal() || settings.capturing() {
        modal(&mut commands, &settings);
    }
}

pub(super) fn page(
    list: &mut ChildSpawnerCommands,
    settings: &Settings,
    labels: &crate::client::actions::Labels,
    searches: &crate::client::menu_ui::Searches,
    context: (
        &crate::client::edit::Hotbar,
        &crate::client::edit::Catalogue,
        &crate::client::edit::Notice,
    ),
) {
    use crate::client::menu_ui::Tab;
    let (bindings, p) = settings.draft.as_ref().unwrap();
    match settings.tab {
        Tab::Behavior => behavior(list, settings, bindings, p, labels, context),
        Tab::Display => display(list, p),
        Tab::Bindings => bindings_page(list, settings, bindings, labels, searches),
        Tab::Controller => controller(list, bindings, p, labels),
        Tab::Accessibility => accessibility(list, p),
        Tab::Catalogue => {}
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
        setting_row(
            card,
            "Sprint activation",
            if p.toggle_sprint {
                "Press once to enable sprint; press again to stop."
            } else {
                "Sprint while the binding is held."
            },
            |row| {
                segment(row, |buttons| {
                    option(buttons, "Hold", Operation::Sprint(false), !p.toggle_sprint);
                    option(buttons, "Toggle", Operation::Sprint(true), p.toggle_sprint);
                })
            },
        );
        setting_row(
            card,
            "Removal activation",
            if p.tap_remove {
                "Timed removal on one target. A second tap cancels."
            } else {
                "Hold on one target until removal completes."
            },
            |row| {
                segment(row, |buttons| {
                    option(buttons, "Hold", Operation::Removal(false), !p.tap_remove);
                    option(buttons, "Timed tap", Operation::Removal(true), p.tap_remove);
                })
            },
        );
        setting_row(
            card,
            "Repeat while held",
            "Keep placing solids, and removing one target after another, while held. Timed tap removal stays one per tap.",
            |row| switch(row, p.repeat_held, Operation::Repeat),
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
            "Reverse the mouse look direction.",
            |row| switch(row, p.invert_y, Operation::Invert),
        );
        setting_row(
            card,
            "Scroll direction",
            if p.reverse_scroll {
                "Wheel up selects the next quick slot."
            } else {
                "Wheel down selects the next quick slot."
            },
            |row| {
                segment(row, |buttons| {
                    option(
                        buttons,
                        "Standard",
                        Operation::ScrollDirection(false),
                        !p.reverse_scroll,
                    );
                    option(
                        buttons,
                        "Reversed",
                        Operation::ScrollDirection(true),
                        p.reverse_scroll,
                    );
                })
            },
        );
        setting_row(
            card,
            "Scroll sensitivity",
            "Higher values move through quick slots faster.",
            |row| slider(row, Scalar::Scroll, p.scroll_sensitivity),
        );
    });
    button(
        list,
        if settings.show_reference {
            "Hide quick reference"
        } else {
            "Show quick reference"
        }
        .into(),
        Operation::Reference,
    );
    if settings.show_reference {
        crate::client::menu_ui::text(
            list,
            crate::client::edit::hud_text(context.0, &context.1.0, context.2, bindings, labels, p),
            12.0,
            theme::MUTED,
        );
    }
}

pub(super) fn accessibility(list: &mut ChildSpawnerCommands, p: &Preferences) {
    group(list, "COMFORT & READABILITY", |card| {
        setting_row(
            card,
            "Reduced motion",
            "Use immediate interface transitions, and still the HUD's pulses and slides.",
            |row| {
                switch(
                    row,
                    p.reduced_motion,
                    Operation::ReducedMotion(!p.reduced_motion),
                )
            },
        );
        setting_row(
            card,
            "Simple menu background",
            "Show menus over a solid background instead of the dimmed World.",
            |row| {
                switch(
                    row,
                    p.solid_background,
                    Operation::SolidBackground(!p.solid_background),
                )
            },
        );
        setting_row(
            card,
            "Text size",
            "100% is standard; 200% doubles the text in menus and the HUD.",
            |row| {
                segment(row, |buttons| {
                    for scale in crate::client::profile::TEXT_SCALES {
                        option(
                            buttons,
                            format!("{scale}%"),
                            Operation::TextScale(scale),
                            p.text_scale == scale,
                        );
                    }
                })
            },
        );
    });
}

pub(super) fn display(list: &mut ChildSpawnerCommands, p: &Preferences) {
    use crate::client::menu_ui as ui;
    group(list, "PRESENTATION", |card| {
        setting_row(
            card,
            "Display sync",
            "Match frame presentation to your display.",
            |row| {
                segment(row, |buttons| {
                    option(
                        buttons,
                        "Display synced",
                        Operation::Sync(true),
                        p.display_sync,
                    );
                    option(buttons, "Sync off", Operation::Sync(false), !p.display_sync);
                })
            },
        );
        setting_row(
            card,
            "Anti-aliasing",
            "Smooths block edges. MSAA 4x also smooths the held item and costs more; Off is fastest.",
            |row| {
                segment(row, |buttons| {
                    for aa in crate::client::anti_alias::CHOICES {
                        option(
                            buttons,
                            aa.label(),
                            Operation::AntiAliasing(aa),
                            p.anti_aliasing == aa,
                        );
                    }
                })
            },
        );
        setting_row(
            card,
            "Render scale",
            "Draws the World at a fraction of the window and scales it up. Menus and text stay sharp.",
            |row| {
                segment(row, |buttons| {
                    for scale in crate::client::render_scale::SCALES {
                        option(
                            buttons,
                            format!("{scale}%"),
                            Operation::Scale(scale),
                            p.render_scale == scale,
                        );
                    }
                })
            },
        );
        card.spawn((
            BackgroundColor(theme::NOTICE),
            Node {
                padding: UiRect::axes(Val::Px(14.0), Val::Px(12.0)),
                margin: UiRect::top(Val::Px(15.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
        ))
        .with_children(|notice| {
            ui::text(
                notice,
                if p.display_sync {
                    "Recommended for normal play. Keeps presentation free of tearing."
                } else {
                    "Sync is off. Frames may tear; choose a limit below to reduce unnecessary work."
                },
                11.0,
                if p.display_sync {
                    theme::MUTED
                } else {
                    theme::WARNING
                },
            )
        });
        card.spawn((
            BorderColor::all(theme::EDGE),
            Node {
                flex_direction: FlexDirection::Column,
                padding: UiRect::vertical(Val::Px(17.0)),
                border: UiRect::bottom(Val::Px(1.0)),
                row_gap: Val::Px(4.0),
                ..default()
            },
        ))
        .with_children(|limit| {
            ui::weighted_text(limit, "Frame rate limit", 13.0, 550, theme::INK);
            ui::description(limit, "An optional ceiling, independent of display sync.");
            limit
                .spawn(Node {
                    column_gap: Val::Px(6.0),
                    row_gap: Val::Px(6.0),
                    flex_wrap: FlexWrap::Wrap,
                    margin: UiRect::top(Val::Px(8.0)),
                    ..default()
                })
                .with_children(|caps| {
                    for cap in [None, Some(60), Some(120), Some(144), Some(240), Some(1000)] {
                        let button = buttons::button(
                            caps,
                            cap.map_or("No limit".into(), |n| n.to_string()),
                            Operation::Cap(cap),
                            ButtonStyle::Compact {
                                selected: p.fps_cap == cap,
                                height_px: 35.0,
                                text_px: 11.0,
                            },
                        );
                        caps.commands().entity(button).insert(Node {
                            min_width: Val::Px(48.0),
                            min_height: Val::Px(35.0),
                            padding: UiRect::axes(Val::Px(13.0), Val::Px(9.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            border_radius: BorderRadius::all(Val::Px(5.0)),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        });
                    }
                });
        });
        card.spawn(Node {
            padding: UiRect::top(Val::Px(17.0)),
            column_gap: Val::Px(10.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|summary| {
            summary
                .spawn((
                    BackgroundColor(theme::CHOSEN),
                    BorderColor::all(theme::CHOSEN_EDGE),
                    Node {
                        width: Val::Px(29.0),
                        height: Val::Px(29.0),
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: BorderRadius::all(Val::Px(5.0)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                ))
                .with_children(|icon| {
                    ui::icon(
                        icon,
                        if p.display_sync {
                            ui::IconKind::Sync
                        } else {
                            ui::IconKind::Uncapped
                        },
                        theme::SAGE,
                        16.0,
                    )
                });
            summary
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(2.0),
                    ..default()
                })
                .with_children(|policy| {
                    ui::weighted_text(
                        policy,
                        if p.display_sync {
                            "Display synced"
                        } else if p.fps_cap.is_none() {
                            "Uncapped rendering"
                        } else {
                            "Sync off"
                        },
                        12.0,
                        550,
                        theme::INK,
                    );
                    ui::text(
                        policy,
                        p.fps_cap.map_or("No additional frame limit".into(), |n| {
                            format!("{n} FPS ceiling")
                        }),
                        11.0,
                        theme::MUTED,
                    );
                });
        });
    });
    ui::text(
        list,
        "View distance, textures, shadows, and simulation settings stay unchanged.",
        12.0,
        theme::MUTED,
    );
}

pub(super) fn bindings_page(
    list: &mut ChildSpawnerCommands,
    settings: &Settings,
    bindings: &Bindings,
    labels: &crate::client::actions::Labels,
    searches: &crate::client::menu_ui::Searches,
) {
    use crate::client::menu_ui as ui;
    let searching = !searches.bindings.trim().is_empty();
    if searching {
        ui::text(list, "Searching all categories", 11.0, theme::MUTED);
    } else {
        list.spawn((
            Node {
                border: UiRect::bottom(Val::Px(1.0)),
                column_gap: Val::Px(22.0),
                margin: UiRect::bottom(Val::Px(14.0)),
                ..default()
            },
            BorderColor::all(theme::EDGE),
        ))
        .with_children(|filters| {
            for category in [Category::Movement, Category::Building, Category::Interface] {
                let entity = buttons::button(
                    filters,
                    category.label(),
                    Operation::Category(category),
                    ButtonStyle::Tab {
                        selected: settings.category == category,
                    },
                );
                filters.commands().queue(move |world: &mut World| {
                    let children: Vec<_> = world
                        .get::<Children>(entity)
                        .map(|children| children.iter().collect())
                        .unwrap_or_default();
                    for child in children {
                        if let Some(mut font) = world.get_mut::<TextFont>(child) {
                            font.font_size = bevy::text::FontSize::Px(11.0);
                        }
                    }
                    if let Some(mut node) = world.get_mut::<Node>(entity) {
                        node.min_height = Val::Px(38.0);
                    }
                });
            }
        });
    }
    ui::text(
        list,
        "Select a binding to change it. Either alternative activates the action.",
        12.0,
        theme::MUTED,
    );
    binding_row(list, true, |row| {
        for label in ["ACTION", "PRIMARY", "ALTERNATIVE", ""] {
            ui::text(row, label, 9.0, theme::MUTED);
        }
    });
    let mut entries: Vec<_> = bindings
        .keyboard
        .iter()
        .filter(|(action, _)| {
            settings.binding_visible(**action, &searches.bindings, bindings, labels)
        })
        .collect();
    entries.sort_by_key(|(action, _)| binding_order(**action));
    if entries.is_empty() {
        ui::text(
            list,
            "No matches. Try another name or clear your search.",
            13.0,
            theme::INK,
        );
    }
    binding_rows(list, Device::KeyboardMouse, entries, labels, searching);
}
pub(super) fn binding_rows(
    list: &mut ChildSpawnerCommands,
    device: Device,
    entries: Vec<(&Action, &[Option<Binding>; 2])>,
    labels: &crate::client::actions::Labels,
    searching: bool,
) {
    use crate::client::menu_ui as ui;
    for (&action, alternatives) in entries {
        binding_row(list, false, |row| {
            row.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            })
            .with_children(|name| {
                ui::text(name, action_name(action), 12.0, theme::INK);
                if searching {
                    ui::text(name, category(action), 9.0, theme::MUTED);
                }
            });
            for (slot, binding) in alternatives.iter().enumerate() {
                row.spawn(Node {
                    column_gap: Val::Px(4.0),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|cell| {
                    let button = cell
                        .spawn((
                            bevy::ui_widgets::Button,
                            bevy::input_focus::tab_navigation::TabIndex(0),
                            Operation::Capture(device, action, slot),
                            ui::Styled {
                                selected: false,
                                tab: false,
                                primary: false,
                            },
                            Node {
                                flex_grow: 1.0,
                                min_width: Val::Px(0.0),
                                min_height: Val::Px(33.0),
                                padding: UiRect::axes(Val::Px(4.0), Val::Px(7.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                border_radius: BorderRadius::all(Val::Px(4.0)),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(theme::PANEL),
                            BorderColor::all(theme::EDGE),
                        ))
                        .with_child((
                            Text::new(binding.map_or("Add binding".into(), |b| {
                                binding_display(labels.binding(b))
                            })),
                            ui::font(10.0),
                            TextColor(if binding.is_some() {
                                theme::INK
                            } else {
                                theme::MUTED
                            }),
                            bevy::text::LineHeight::RelativeToFont(1.5),
                            Pickable::IGNORE,
                        ))
                        .id();
                    if binding.is_none() {
                        cell.commands()
                            .entity(button)
                            .insert(ui::EmptyBinding)
                            .with_child((
                                ui::EmptyBorder(button),
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(-1.0),
                                    right: Val::Px(-1.0),
                                    top: Val::Px(-1.0),
                                    bottom: Val::Px(-1.0),
                                    ..default()
                                },
                                Pickable::IGNORE,
                            ));
                    }
                    if binding.is_some() {
                        buttons::button(
                            cell,
                            "×",
                            Operation::Clear(device, action, slot),
                            ButtonStyle::Quiet { text_px: 12.0 },
                        );
                    }
                });
            }
            buttons::button(
                row,
                "↺",
                Operation::ResetAction(device, action),
                ButtonStyle::Quiet { text_px: 16.0 },
            );
        });
    }
}
pub(super) fn controller(
    list: &mut ChildSpawnerCommands,
    bindings: &Bindings,
    p: &Preferences,
    labels: &crate::client::actions::Labels,
) {
    use crate::client::menu_ui as ui;
    let pad = &p.pad;
    group(list, "STICKS", |card| {
        setting_row(
            card,
            "Look sensitivity",
            "Turn speed at full right-stick deflection.",
            |row| slider(row, Scalar::PadLook, pad.look_sensitivity),
        );
        setting_row(
            card,
            "Invert vertical look",
            "Reverse the right-stick look direction. Mouse look is separate.",
            |row| switch(row, pad.invert_y, Operation::PadInvert),
        );
        setting_row(
            card,
            "Movement deadzone",
            "Left-stick travel ignored as drift.",
            |row| slider(row, Scalar::MoveDeadzone, pad.move_deadzone),
        );
        setting_row(
            card,
            "Look deadzone",
            "Right-stick travel ignored as drift.",
            |row| slider(row, Scalar::LookDeadzone, pad.look_deadzone),
        );
        setting_row(
            card,
            "Crouch activation",
            if pad.toggle_crouch {
                "Press once to crouch on foot; press again to stand. Hold to descend in flight."
            } else {
                "Crouch or descend while the binding is held."
            },
            |row| {
                segment(row, |buttons| {
                    option(
                        buttons,
                        "Hold",
                        Operation::PadCrouch(false),
                        !pad.toggle_crouch,
                    );
                    option(
                        buttons,
                        "Toggle",
                        Operation::PadCrouch(true),
                        pad.toggle_crouch,
                    );
                })
            },
        );
    });
    group(list, "TOUCH", |card| {
        let touch = &p.touch;
        setting_row(
            card,
            "Look sensitivity",
            "How far a drag on the look side turns the view.",
            |row| slider(row, Scalar::TouchLook, touch.look_sensitivity),
        );
        setting_row(
            card,
            "Invert vertical look",
            "Reverse the touch look direction. Mouse look is separate.",
            |row| switch(row, touch.invert_y, Operation::TouchInvert),
        );
        setting_row(
            card,
            "Layout",
            "Which hand moves. Mirroring also swaps Place and Remove.",
            |row| {
                segment(row, |buttons| {
                    option(
                        buttons,
                        "Normal",
                        Operation::TouchMirror(false),
                        !touch.mirrored,
                    );
                    option(
                        buttons,
                        "Mirrored",
                        Operation::TouchMirror(true),
                        touch.mirrored,
                    );
                })
            },
        );
        setting_row(
            card,
            "Control size",
            "Every control stays at least 44 pixels.",
            |row| {
                segment(row, |buttons| {
                    use crate::client::profile::TouchSize::*;
                    for (label, size) in
                        [("Small", Small), ("Normal", Normal), ("Enlarged", Enlarged)]
                    {
                        option(
                            buttons,
                            label,
                            Operation::TouchSize(size),
                            touch.size == size,
                        );
                    }
                })
            },
        );
        setting_row(
            card,
            "Control opacity",
            "How solid the on-screen controls are.",
            |row| slider(row, Scalar::TouchOpacity, touch.opacity),
        );
    });
    use bevy::input::gamepad::GamepadButton::{East, LeftTrigger, RightTrigger, South, Start};
    ui::text(
        list,
        format!(
            "The left stick moves and the right stick looks. In menus the D-pad or left stick \
             moves focus, {} selects, {} goes back, {} and {} switch pages, and {} closes.",
            labels.pad(South),
            labels.pad(East),
            labels.pad(LeftTrigger),
            labels.pad(RightTrigger),
            labels.pad(Start),
        ),
        12.0,
        theme::MUTED,
    );
    binding_row(list, true, |row| {
        for label in ["ACTION", "PRIMARY", "ALTERNATIVE", ""] {
            ui::text(row, label, 9.0, theme::MUTED);
        }
    });
    let mut entries: Vec<_> = bindings
        .pad
        .iter()
        .filter(|(action, _)| !matches!(action, Action::ReleaseCursor))
        .collect();
    entries.sort_by_key(|(action, _)| binding_order(**action));
    binding_rows(list, Device::Pad, entries, labels, true);
}
pub(super) fn binding_row(
    parent: &mut ChildSpawnerCommands,
    heading: bool,
    content: impl FnOnce(&mut ChildSpawnerCommands),
) {
    parent
        .spawn((
            Node {
                display: Display::Grid,
                grid_template_columns: vec![
                    GridTrack::flex(1.0),
                    GridTrack::px(150.0),
                    GridTrack::px(150.0),
                    GridTrack::px(28.0),
                ],
                column_gap: Val::Px(12.0),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(10.0)),
                min_height: Val::Px(if heading { 32.0 } else { 55.0 }),
                margin: UiRect::top(Val::Px(5.0)),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(if heading { Color::NONE } else { theme::RAISED }),
        ))
        .with_children(content);
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
        Menu,
        Inventory,
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
}

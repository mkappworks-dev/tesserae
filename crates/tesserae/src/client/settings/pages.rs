//! Each category's page and what its rows are called.

use super::*;
use crate::client::theme;
use crate::server::player::SpeedLevel;

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
        Local<[Vec2; crate::client::menu_ui::Tab::ORDER.len()]>,
        Local<Option<crate::client::menu_ui::Tab>>,
        Local<String>,
    ),
    (context, pressed): RenderContext,
) {
    use crate::client::menu_ui::{self as ui, Tab};
    if !pressed.is_empty() || (!settings.dirty && !labels.is_changed()) {
        return;
    }
    if let Some(tab) = *previous
        && let Some(scroll) = scrolls.iter().next()
    {
        positions[scroll_slot(tab)] = scroll.0;
    }
    if *query != searches.bindings {
        positions[scroll_slot(Tab::Bindings)] = Vec2::ZERO;
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
    let index = scroll_slot(settings.tab);
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
                    padding: UiRect::new(
                        Val::Px(theme::SHEET_PAD_X),
                        Val::Px(theme::SHEET_PAD_X),
                        // The search field above the body already leaves this gap.
                        Val::Px(if settings.tab == Tab::Bindings {
                            0.0
                        } else {
                            20.0
                        }),
                        Val::Px(20.0),
                    ),
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

/// Where each category keeps its scroll position while another shows.
fn scroll_slot(tab: crate::client::menu_ui::Tab) -> usize {
    use crate::client::menu_ui::Tab;
    Tab::ORDER.iter().position(|t| *t == tab).unwrap_or(0)
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
        Tab::Audio => audio(list, settings, p),
        Tab::Bindings => bindings_page(list, settings, bindings, labels, searches),
        Tab::Controller => controller(list, settings, bindings, p, labels),
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
            "Sprint speed",
            "Sprint cycles through these and keeps the one you stop on.",
            |row| {
                segment(row, |buttons| {
                    for level in [
                        SpeedLevel::Walk,
                        SpeedLevel::One,
                        SpeedLevel::Two,
                        SpeedLevel::Three,
                    ] {
                        option(
                            buttons,
                            level.label(),
                            Operation::Sprint(level),
                            p.speed_level == level,
                        );
                    }
                })
            },
        );
        setting_row(card, "Removal activation", "", |row| {
            segment(row, |buttons| {
                option(buttons, "Hold", Operation::Removal(false), !p.tap_remove);
                option(buttons, "Timed tap", Operation::Removal(true), p.tap_remove);
            })
        });
        setting_row(
            card,
            "Repeat while held",
            "Keep placing or removing while the binding is held.",
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
        setting_row(card, "Invert vertical look", "", |row| {
            switch(row, p.invert_y, Operation::Invert)
        });
        setting_row(card, "Scroll direction", "", |row| {
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
        });
        setting_row(card, "Scroll sensitivity", "", |row| {
            slider(row, Scalar::Scroll, p.scroll_sensitivity)
        });
        setting_row(card, "Show quick reference", "", |row| {
            switch(row, settings.show_reference, Operation::Reference)
        });
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
            "Stronger UI contrast",
            "Increase panel borders and secondary text contrast.",
            |row| {
                switch(
                    row,
                    p.stronger_contrast,
                    Operation::StrongerContrast(!p.stronger_contrast),
                )
            },
        );
        setting_row(
            card,
            "Simple menu background",
            "Replace the scenery with a solid background.",
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
    use crate::client::cinematic as cine;
    use crate::client::menu_ui as ui;
    group(list, "FRAME PRESENTATION", |card| {
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
        card.spawn(Node {
            flex_direction: FlexDirection::Column,
            margin: UiRect::new(Val::Px(0.0), Val::Px(0.0), Val::Px(14.0), Val::Px(19.0)),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|notice| {
            cine::notice(
                notice,
                if p.display_sync {
                    "Recommended for normal play. Keeps presentation free of tearing."
                } else {
                    "Sync is off. Frames may tear; choose a limit below to reduce unnecessary work."
                },
            )
        });
        setting_row(
            card,
            "Frame rate limit",
            "An optional ceiling, independent of display sync.",
            |row| {
                segment(row, |buttons| {
                    for cap in [Some(60), Some(120), Some(144), Some(240), Some(1000), None] {
                        option(
                            buttons,
                            cap.map_or("No limit".into(), |n: u32| n.to_string()),
                            Operation::Cap(cap),
                            p.fps_cap == cap,
                        );
                    }
                })
            },
        );
        let pacing = if p.display_sync {
            "Display synced"
        } else if p.fps_cap.is_none() {
            "Uncapped rendering"
        } else {
            "Sync off"
        };
        let ceiling = p.fps_cap.map_or("No additional frame limit".into(), |n| {
            format!("{n} FPS ceiling")
        });
        setting_row(card, pacing, &ceiling, |row| {
            cine::pill(row, "Frame pacing")
        });
    });
    group(list, "IMAGE QUALITY", |card| {
        setting_row(
            card,
            "Anti-aliasing",
            "Smooth voxel edges. MSAA costs more.",
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
            "Lower values reduce rendering work. Menus stay sharp.",
            |row| {
                segment(row, |buttons| {
                    for scale in crate::client::render_scale::SCALES.iter().rev() {
                        option(
                            buttons,
                            format!("{scale}%"),
                            Operation::Scale(*scale),
                            p.render_scale == *scale,
                        );
                    }
                })
            },
        );
    });
    ui::paragraph(
        list,
        "View distance, textures, shadows, and simulation settings stay unchanged.",
        theme::TEXT_SMALL,
        theme::muted(),
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
        list.spawn(Node {
            margin: UiRect::bottom(Val::Px(14.0)),
            ..default()
        })
        .with_children(|line| {
            ui::paragraph(
                line,
                "Searching all categories",
                theme::TEXT_SMALL,
                theme::muted(),
            )
        });
    } else {
        list.spawn((
            Node {
                border: UiRect::bottom(Val::Px(theme::RULE)),
                column_gap: Val::Px(15.0),
                margin: UiRect::bottom(Val::Px(17.0)),
                flex_shrink: 0.0,
                ..default()
            },
            BorderColor::all(theme::edge()),
        ))
        .with_children(|filters| {
            for category in [Category::Movement, Category::Building, Category::Interface] {
                buttons::button(
                    filters,
                    category.label(),
                    Operation::Category(category),
                    ButtonStyle::Tab {
                        selected: settings.category == category,
                    },
                );
            }
        });
    }
    list.spawn(Node {
        justify_content: JustifyContent::SpaceBetween,
        align_items: AlignItems::Center,
        column_gap: Val::Px(10.0),
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|intro| {
        ui::paragraph(
            intro,
            "Keyboard & mouse · primary and alternative",
            theme::TEXT_SMALL,
            theme::muted(),
        );
        crate::client::cinematic::pill(intro, "Select to rebind");
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
        list.spawn(Node {
            justify_content: JustifyContent::Center,
            padding: UiRect::axes(Val::Px(10.0), Val::Px(35.0)),
            ..default()
        })
        .with_children(|empty| {
            ui::paragraph(
                empty,
                "No matches. Try another name or clear your search.",
                theme::TEXT_BODY,
                theme::INK,
            )
        });
    }
    binding_rows(list, Device::KeyboardMouse, entries, labels, searching);
}

/// One row per action: its name, then a button per binding, and ↺ once it
/// differs from the default. A controller row offers its alternative only
/// once one is bound, as the prototype shows one button per action.
pub(super) fn binding_rows(
    list: &mut ChildSpawnerCommands,
    device: Device,
    entries: Vec<(&Action, &[Option<Binding>; 2])>,
    labels: &crate::client::actions::Labels,
    searching: bool,
) {
    use crate::client::cinematic::Kind;
    use crate::client::menu_ui as ui;
    let defaults = Bindings::default();
    for (&action, alternatives) in entries {
        binding_row(list, |row| {
            row.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(5.0),
                min_width: Val::Px(0.0),
                ..default()
            })
            .with_children(|name| {
                ui::weighted_text(
                    name,
                    action_name(action),
                    theme::TEXT_BODY,
                    theme::LABEL,
                    theme::INK,
                );
                if searching && device == Device::KeyboardMouse {
                    ui::description(name, category(action));
                }
            });
            row.spawn(Node {
                column_gap: Val::Px(10.0),
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                ..default()
            })
            .with_children(|cells| {
                for (slot, binding) in alternatives.iter().enumerate() {
                    if device == Device::Pad && slot == 1 && binding.is_none() {
                        continue;
                    }
                    buttons::button(
                        cells,
                        binding
                            .map_or("Add binding".into(), |b| binding_display(labels.binding(b))),
                        Operation::Capture(device, action, slot),
                        ButtonStyle::Cinematic {
                            kind: if binding.is_some() {
                                Kind::Plain
                            } else {
                                Kind::Ghost
                            },
                            align: Align::Centre,
                        },
                    );
                }
                if defaults.map(device).get(&action) != Some(alternatives) {
                    buttons::button(
                        cells,
                        "↺",
                        Operation::ResetAction(device, action),
                        ButtonStyle::Small { kind: Kind::Ghost },
                    );
                }
            });
        });
    }
}
pub(super) fn controller(
    list: &mut ChildSpawnerCommands,
    settings: &Settings,
    bindings: &Bindings,
    p: &Preferences,
    labels: &crate::client::actions::Labels,
) {
    use crate::client::menu_ui as ui;
    let pad = &p.pad;
    group(list, "STICKS & BUTTONS", |card| {
        setting_row(card, "Look sensitivity", "", |row| {
            slider(row, Scalar::PadLook, pad.look_sensitivity)
        });
        setting_row(card, "Invert vertical look", "", |row| {
            switch(row, pad.invert_y, Operation::PadInvert)
        });
        setting_row(
            card,
            "Movement deadzone",
            "Ignore small movements caused by stick drift.",
            |row| slider(row, Scalar::MoveDeadzone, pad.move_deadzone),
        );
        setting_row(card, "Look deadzone", "", |row| {
            slider(row, Scalar::LookDeadzone, pad.look_deadzone)
        });
        setting_row(card, "Crouch activation", "", |row| {
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
        });
        disclosure(
            card,
            "Touch controls",
            settings.touch_open,
            Operation::TouchControls,
            |touch_rows| {
                let touch = &p.touch;
                setting_row(touch_rows, "Look sensitivity", "", |row| {
                    slider(row, Scalar::TouchLook, touch.look_sensitivity)
                });
                setting_row(touch_rows, "Invert vertical look", "", |row| {
                    switch(row, touch.invert_y, Operation::TouchInvert)
                });
                setting_row(touch_rows, "Layout", "", |row| {
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
                });
                setting_row(touch_rows, "Control size", "", |row| {
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
                });
                setting_row(touch_rows, "Control opacity", "", |row| {
                    slider(row, Scalar::TouchOpacity, touch.opacity)
                });
            },
        );
    });
    use bevy::input::gamepad::GamepadButton::{East, LeftTrigger, RightTrigger, South, Start};
    group(list, "CONTROLLER BUTTONS", |buttons| {
        ui::paragraph(
            buttons,
            format!(
                "The left stick moves and the right stick looks. In menus the D-pad or left \
                 stick moves focus, {} selects, {} goes back, {} and {} switch pages, and {} \
                 closes.",
                labels.pad(South),
                labels.pad(East),
                labels.pad(LeftTrigger),
                labels.pad(RightTrigger),
                labels.pad(Start),
            ),
            theme::TEXT_SMALL,
            theme::muted(),
        );
        let mut entries: Vec<_> = bindings
            .pad
            .iter()
            .filter(|(action, _)| !matches!(action, Action::ReleaseCursor))
            .collect();
        entries.sort_by_key(|(action, _)| binding_order(**action));
        binding_rows(buttons, Device::Pad, entries, labels, false);
    });
}

pub(super) fn binding_row(
    parent: &mut ChildSpawnerCommands,
    content: impl FnOnce(&mut ChildSpawnerCommands),
) {
    parent
        .spawn((
            Node {
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                column_gap: Val::Px(15.0),
                padding: UiRect::vertical(Val::Px(16.0)),
                border: UiRect::bottom(Val::Px(theme::RULE)),
                flex_shrink: 0.0,
                ..default()
            },
            BorderColor::all(theme::edge()),
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
        .replace("BracketLeft", "[")
        .replace("BracketRight", "]")
}

fn audio(list: &mut ChildSpawnerCommands, settings: &Settings, p: &Preferences) {
    group(list, "Sound mix", |card| {
        for (name, hint, kind, value) in [
            (
                "Master volume",
                "Everything the World plays.",
                Scalar::MasterVolume,
                p.volumes.master,
            ),
            (
                "Effects",
                "All action sounds, including the fine controls below.",
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
        ] {
            setting_row(card, name, hint, |row| slider(row, kind, f32::from(value)));
        }
    });
    disclosure(
        list,
        "Fine-tune sound effects",
        settings.effects_open,
        Operation::EffectControls,
        |card| {
            crate::client::menu_ui::description(
                card,
                "Relative to Effects. Set any sound to 0 to mute it.",
            );
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
    crate::client::menu_ui::description(
        list,
        "Apply changes, then resume to hear your mix. World audio rests while Settings is open.",
    );
}

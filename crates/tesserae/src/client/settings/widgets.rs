//! The controls a Settings page is built from: rows, switches, sliders,
//! segments, the footer and the confirming dialog.

use super::*;

pub(super) fn button(parent: &mut ChildSpawnerCommands, text: String, operation: Operation) {
    buttons::button(parent, text, operation, ButtonStyle::Plain);
}
pub(super) fn setting_row(
    parent: &mut ChildSpawnerCommands,
    title: &str,
    description: &str,
    last: bool,
    choices: impl FnOnce(&mut ChildSpawnerCommands),
) {
    use crate::client::menu_ui as ui;
    parent
        .spawn((
            Node {
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: Val::Px(18.0),
                padding: UiRect {
                    top: Val::Px(17.0),
                    bottom: Val::Px(if last { 1.0 } else { 17.0 }),
                    ..default()
                },
                border: UiRect::bottom(Val::Px(if last { 0.0 } else { 1.0 })),
                flex_shrink: 0.0,
                ..default()
            },
            BorderColor::all(ui::EDGE),
        ))
        .with_children(|row| {
            row.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                max_width: Val::Px(340.0),
                min_width: Val::Px(0.0),
                flex_shrink: 1.0,
                ..default()
            })
            .with_children(|label| {
                ui::weighted_text(label, title, 13.0, 550, ui::INK);
                ui::description(label, description);
            });
            row.spawn(Node {
                flex_shrink: 0.0,
                ..default()
            })
            .with_children(choices);
        });
}
pub(super) fn option(
    parent: &mut ChildSpawnerCommands,
    label: impl Into<String>,
    operation: Operation,
    selected: bool,
) {
    buttons::button(
        parent,
        label,
        operation,
        ButtonStyle::Compact {
            selected,
            height_px: 31.0,
            text_px: 11.0,
        },
    );
}
#[derive(Component, Clone, Copy)]
pub(in crate::client) enum Scalar {
    Mouse,
    Scroll,
    PadLook,
    TouchLook,
    TouchOpacity,
    MoveDeadzone,
    LookDeadzone,
}
impl Scalar {
    fn range(self) -> std::ops::RangeInclusive<f32> {
        match self {
            Self::Mouse | Self::Scroll => 0.25..=3.0,
            Self::PadLook | Self::TouchLook => crate::client::profile::PAD_SENSITIVITY,
            Self::TouchOpacity => crate::client::profile::TOUCH_OPACITY,
            Self::MoveDeadzone | Self::LookDeadzone => crate::client::profile::PAD_DEADZONE,
        }
    }
    fn step(self) -> f32 {
        match self {
            Self::MoveDeadzone | Self::LookDeadzone | Self::TouchOpacity => 0.01,
            _ => 0.05,
        }
    }
    fn display(self, value: f32) -> String {
        match self {
            Self::MoveDeadzone | Self::LookDeadzone | Self::TouchOpacity => {
                format!("{:.0}%", value * 100.0)
            }
            _ => format!("{value:.2}×"),
        }
    }
    fn fraction(self, value: f32) -> f32 {
        let range = self.range();
        ((value - range.start()) / (range.end() - range.start())).clamp(0.0, 1.0)
    }
}
#[derive(Component)]
pub(super) struct ScalarThumb(Entity, Scalar);
#[derive(Component)]
pub(super) struct ScalarLabel(Entity, Scalar);
pub(super) fn slider(parent: &mut ChildSpawnerCommands, kind: Scalar, value: f32) {
    use crate::client::menu_ui as ui;
    use bevy::ui_widgets::{Slider, SliderRange, SliderStep, SliderThumb, SliderValue};
    parent
        .spawn(Node {
            width: Val::Px(206.0),
            align_items: AlignItems::Center,
            column_gap: Val::Px(12.0),
            ..default()
        })
        .with_children(|row| {
            let slider = row
                .spawn((
                    kind,
                    Slider::default(),
                    SliderRange::new(*kind.range().start(), *kind.range().end()),
                    SliderValue(value),
                    SliderStep(kind.step()),
                    bevy::input_focus::tab_navigation::TabIndex(0),
                    Node {
                        width: Val::Px(135.0),
                        height: Val::Px(22.0),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                ))
                .id();
            row.commands().entity(slider).with_children(|track| {
                track.spawn((
                    BackgroundColor(ui::EDGE),
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(4.0),
                        border_radius: BorderRadius::all(Val::Px(2.0)),
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
                track.spawn((
                    SliderThumb,
                    ScalarThumb(slider, kind),
                    BackgroundColor(ui::LEAF),
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Px(12.0),
                        height: Val::Px(12.0),
                        border_radius: BorderRadius::all(Val::Px(6.0)),
                        left: Val::Px(kind.fraction(value) * 123.0),
                        ..default()
                    },
                ));
            });
            row.spawn((
                ScalarLabel(slider, kind),
                Text::new(kind.display(value)),
                TextFont {
                    font: bevy::text::FontSource::family("Aileron"),
                    ..ui::font(11.0)
                },
                TextColor(ui::INK),
                Pickable::IGNORE,
            ));
        });
}
pub(super) fn scalar_changed(
    event: On<bevy::ui_widgets::ValueChange<f32>>,
    values: Query<&Scalar>,
    mut commands: Commands,
    mut settings: ResMut<Settings>,
    status: Option<Res<crate::server::persistence::SaveStatus>>,
) {
    if !settings.open
        || settings.modal()
        || settings.capturing()
        || status.is_some_and(|s| s.modal())
    {
        return;
    }
    let Ok(kind) = values.get(event.source) else {
        return;
    };
    let Some((_, preferences)) = settings.draft.as_mut() else {
        return;
    };
    let range = kind.range();
    let value = event.value.clamp(*range.start(), *range.end());
    commands
        .entity(event.source)
        .insert(bevy::ui_widgets::SliderValue(value));
    match kind {
        Scalar::Mouse => preferences.sensitivity = value * 0.003,
        Scalar::Scroll => preferences.scroll_sensitivity = value,
        Scalar::PadLook => preferences.pad.look_sensitivity = value,
        Scalar::TouchLook => preferences.touch.look_sensitivity = value,
        Scalar::TouchOpacity => preferences.touch.opacity = value,
        Scalar::MoveDeadzone => preferences.pad.move_deadzone = value,
        Scalar::LookDeadzone => preferences.pad.look_deadzone = value,
    }
    settings.dirty = true;
}
pub(super) fn slider_visuals(
    values: Query<&bevy::ui_widgets::SliderValue>,
    mut thumbs: Query<(&ScalarThumb, &mut Node, &mut BackgroundColor)>,
    mut labels: Query<(&ScalarLabel, &mut Text)>,
    focus: Option<Res<bevy::input_focus::InputFocus>>,
    visible: Option<Res<bevy::input_focus::InputFocusVisible>>,
) {
    let focused = crate::client::menu_ui::visible_focus(focus.as_deref(), visible.as_deref());
    for (thumb, mut node, mut background) in &mut thumbs {
        if let Ok(value) = values.get(thumb.0) {
            node.left = Val::Px(thumb.1.fraction(value.0) * 123.0);
        }
        let colour = if focused == Some(thumb.0) {
            crate::client::menu_ui::INK
        } else {
            crate::client::menu_ui::LEAF
        };
        if background.0 != colour {
            background.0 = colour;
        }
    }
    for (label, mut text) in &mut labels {
        if let Ok(value) = values.get(label.0) {
            text.set_if_neq(Text::new(label.1.display(value.0)));
        }
    }
}
pub(super) fn group(
    parent: &mut ChildSpawnerCommands,
    title: &str,
    content: impl FnOnce(&mut ChildSpawnerCommands),
) {
    use crate::client::menu_ui as ui;
    parent
        .spawn((
            BackgroundColor(ui::CARD),
            BorderColor::all(ui::EDGE),
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(18.0)),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                margin: UiRect::bottom(Val::Px(14.0)),
                flex_direction: FlexDirection::Column,
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|card| {
            card.spawn((
                Text::new(title),
                TextFont {
                    weight: bevy::text::FontWeight(550),
                    ..ui::font(10.0)
                },
                bevy::text::LetterSpacing::Px(1.5),
                TextColor(ui::MUTED),
                bevy::text::LineHeight::RelativeToFont(1.5),
                Node {
                    margin: UiRect::bottom(Val::Px(3.0)),
                    ..default()
                },
                Pickable::IGNORE,
            ));
            content(card);
        });
}
pub(super) fn segment(
    parent: &mut ChildSpawnerCommands,
    choices: impl FnOnce(&mut ChildSpawnerCommands),
) {
    use crate::client::menu_ui as ui;
    parent
        .spawn((
            ui::Segment,
            BackgroundColor(ui::BG),
            BorderColor::all(ui::EDGE),
            Node {
                padding: UiRect::all(Val::Px(3.0)),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                column_gap: Val::Px(2.0),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(choices);
}
pub(super) fn switch(parent: &mut ChildSpawnerCommands, on: bool, operation: Operation) {
    use crate::client::menu_ui as ui;
    parent
        .spawn((
            bevy::ui_widgets::Button,
            bevy::input_focus::tab_navigation::TabIndex(0),
            operation,
            Node {
                min_height: Val::Px(40.0),
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                ..default()
            },
        ))
        .with_children(|button| {
            ui::text(button, if on { "On" } else { "Off" }, 11.0, ui::INK);
            button
                .spawn((
                    BackgroundColor(if on { ui::LEAF } else { ui::CARD }),
                    BorderColor::all(if on { ui::LEAF } else { ui::EDGE }),
                    Node {
                        width: Val::Px(32.0),
                        height: Val::Px(18.0),
                        padding: UiRect::all(Val::Px(2.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: BorderRadius::all(Val::Px(12.0)),
                        align_items: AlignItems::Center,
                        justify_content: if on {
                            JustifyContent::FlexEnd
                        } else {
                            JustifyContent::FlexStart
                        },
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_child((
                    BackgroundColor(if on { ui::BG } else { ui::MUTED }),
                    Node {
                        width: Val::Px(12.0),
                        height: Val::Px(12.0),
                        border_radius: BorderRadius::all(Val::Px(6.0)),
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
        });
}
pub(super) fn footer(parent: &mut ChildSpawnerCommands, changed: bool) {
    use crate::client::cinematic::{self as cine, Kind};
    use crate::client::menu_ui as ui;
    parent
        .spawn((
            BackgroundColor(cine::SHEET_FOOTER),
            BorderColor::all(cine::EDGE.with_alpha(0.5)),
            Node {
                padding: UiRect::axes(Val::Px(26.0), Val::Px(14.0)),
                border: UiRect::top(Val::Px(1.0)),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|footer| {
            buttons::button(
                footer,
                "Back",
                Operation::Back,
                ButtonStyle::Cinematic {
                    kind: Kind::Ghost,
                    align: Align::Centre,
                },
            );
            footer
                .spawn(Node {
                    column_gap: Val::Px(10.0),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|actions| {
                    ui::text(
                        actions,
                        if changed {
                            "Unapplied changes"
                        } else {
                            "All changes applied"
                        },
                        12.0,
                        if changed { cine::WARNING } else { cine::MUTED },
                    );
                    buttons::button(
                        actions,
                        "Restore defaults",
                        Operation::Reset,
                        ButtonStyle::Cinematic {
                            kind: Kind::Ghost,
                            align: Align::Centre,
                        },
                    );
                    let apply = buttons::button(
                        actions,
                        "Apply changes",
                        Operation::Apply,
                        ButtonStyle::Cinematic {
                            kind: Kind::Primary,
                            align: Align::Centre,
                        },
                    );
                    if !changed {
                        actions
                            .commands()
                            .entity(apply)
                            .insert(bevy::ui::InteractionDisabled);
                    }
                });
        });
}

#[derive(Component)]
pub(super) struct ModalFocus;
pub(super) fn capture_focus(
    settings: Res<Settings>,
    anchors: Query<Entity, With<ModalFocus>>,
    mut focus: ResMut<bevy::input_focus::InputFocus>,
    status: Option<Res<crate::server::persistence::SaveStatus>>,
) {
    if settings.capturing()
        && !status.is_some_and(|s| s.modal())
        && let Some(entity) = anchors.iter().next()
    {
        focus.set(entity, bevy::input_focus::FocusCause::Auto);
    }
}
pub(super) fn modal(commands: &mut Commands, settings: &Settings) {
    use crate::client::menu_ui as ui;
    let mut first = None;
    let mut anchor = None;
    commands.spawn((Panel,GlobalZIndex(80),BackgroundColor(ui::BG.with_alpha(0.85)),Node {position_type:PositionType::Absolute,left:Val::Px(0.0),right:Val::Px(0.0),top:Val::Px(0.0),bottom:Val::Px(0.0),align_items:AlignItems::Center,justify_content:JustifyContent::Center,..default()})).with_children(|shade| {
        let dialog=shade.spawn((ModalFocus,bevy::input_focus::tab_navigation::TabGroup {modal:true,..default()},BackgroundColor(ui::CARD),BorderColor::all(ui::EDGE),Node {width:Val::Px(390.0),max_width:Val::Percent(92.0),padding:UiRect::all(Val::Px(26.0)),border:UiRect::all(Val::Px(1.0)),border_radius:BorderRadius::all(Val::Px(10.0)),flex_direction:FlexDirection::Column,row_gap:Val::Px(10.0),..default()})).with_children(|dialog| {
            if let Some((device,action,slot,_))=settings.capture {
                let (prompt,cancel)=match device {Device::KeyboardMouse=>("Press a key or mouse button","Esc cancels. The opening click is ignored."),Device::Pad=>("Press a controller button","Menu or Esc cancels. The opening press is ignored.")};
                ui::weighted_text(dialog,"Change binding",19.0,620,ui::INK);
                ui::paragraph(dialog,format!("{} · {}",action_name(action),if slot==0{"Primary"}else{"Alternative"}),12.0,ui::MUTED);
                dialog.spawn((BackgroundColor(Color::srgb(0.069,0.118,0.081)),BorderColor::all(ui::LEAF),Node {padding:UiRect::all(Val::Px(28.0)),margin:UiRect::vertical(Val::Px(5.0)),border:UiRect::all(Val::Px(1.0)),border_radius:BorderRadius::all(Val::Px(6.0)),justify_content:JustifyContent::Center,..default()})).with_children(|well|ui::technical_text(well,prompt,12.0,ui::LEAF));
                ui::paragraph(dialog,cancel,12.0,ui::MUTED);
                dialog.spawn(Node {justify_content:JustifyContent::FlexEnd,margin:UiRect::top(Val::Px(10.0)),..default()}).with_children(|actions|{buttons::button(actions, "Cancel", Operation::Cancel, ButtonStyle::Plain);});
            }else {
                let (title,description)=if settings.unsaved {("Keep your changes?","You have unapplied behavior, binding or display changes.".into())}else if settings.resetting {("Restore defaults?","This resets the controls and display draft. Your saved preferences stay unchanged until you apply.".into())}else{let names=settings.conflict.as_ref().map(|(_,_,conflicts)|conflicts.iter().map(|(action,_)|action_name(*action)).collect::<Vec<_>>().join(", ")).unwrap_or_default();("Binding already in use",format!("This binding is assigned to {names}. Replace it?"))};
                ui::weighted_text(dialog,title,19.0,620,ui::INK);ui::paragraph(dialog,description,12.0,ui::MUTED);
                dialog.spawn(Node {justify_content:JustifyContent::FlexEnd,column_gap:Val::Px(7.0),row_gap:Val::Px(7.0),flex_wrap:FlexWrap::Wrap,margin:UiRect::top(Val::Px(10.0)),..default()}).with_children(|actions| {
                    if settings.unsaved {first=Some(buttons::button(actions, "Stay", Operation::Stay, ButtonStyle::Plain));let discard=buttons::button(actions, "Discard", Operation::Discard, ButtonStyle::Plain);actions.commands().entity(discard).insert(ui::Danger);buttons::button(actions, "Save changes", Operation::Apply, ButtonStyle::Primary);}
                    else if settings.resetting {first=Some(buttons::button(actions, "Cancel", Operation::CancelReset, ButtonStyle::Plain));let restore=buttons::button(actions, "Restore defaults", Operation::ConfirmReset, ButtonStyle::Primary);actions.commands().entity(restore).insert(ui::Danger);}
                    else {first=Some(buttons::button(actions, "Keep existing", Operation::CancelConflict, ButtonStyle::Plain));buttons::button(actions, "Replace binding", Operation::Replace, ButtonStyle::Primary);}
                });
            }
        }).id();anchor=Some(dialog);
    });
    let focused = if settings.capturing() { anchor } else { first };
    if let Some(entity) = focused {
        commands.queue(move |world: &mut World| {
            world
                .resource_mut::<bevy::input_focus::InputFocus>()
                .set(entity, bevy::input_focus::FocusCause::Auto)
        });
    }
}

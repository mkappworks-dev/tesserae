//! Draft preferences become active only after a successful Apply.
use super::{
    actions::{Action, Binding, Bindings, Modifier, Trigger},
    profile::{Preferences, Store},
};
use bevy::{
    prelude::*,
    ui_widgets::{Activate, ScrollArea},
};

type Conflict = (Action, [Option<Binding>; 2], Vec<(Action, usize)>);

#[derive(Resource, Default)]
pub(super) struct Settings {
    pub open: bool,
    draft: Option<(Bindings, Preferences)>,
    capture: Option<(Action, usize, bool)>,
    conflict: Option<Conflict>,
    unsaved: bool,
    notice: String,
    pub(super) dirty: bool,
    pub(super) tab: super::menu_ui::Tab,
    last_controls: super::menu_ui::Tab,
    pending: Option<super::menu_ui::Tab>,
}
impl Settings {
    pub(super) fn modal(&self) -> bool {
        self.unsaved || self.conflict.is_some()
    }
    pub fn capturing(&self) -> bool {
        self.capture.is_some()
    }
}

#[derive(Component)]
pub(super) struct Panel;
#[derive(Component)]
pub(super) struct SettingsScroll;
#[derive(Component, Clone, Copy)]
pub(super) enum Operation {
    Navigate(super::menu_ui::Tab),
    Apply,
    Cancel,
    Back,
    Discard,
    Stay,
    Reset,
    ResetAction(Action),
    Capture(Action, usize),
    Clear(Action, usize),
    Replace,
    CancelConflict,
    Sprint(bool),
    Removal(bool),
    Invert,
    Sensitivity(f32),
    ScrollDirection(bool),
    ScrollSensitivity(f32),
    Sync(bool),
    Cap(Option<u32>),
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Settings>()
        .init_resource::<super::menu::Menu>()
        .init_resource::<bevy::input_focus::InputFocus>()
        .add_observer(activate)
        .add_systems(Startup, launch)
        .add_systems(
            Update,
            open_shortcut
                .after(super::menu::toggle)
                .before(capture)
                .before(super::input::resolve),
        )
        .add_systems(
            Update,
            (capture, render).chain().before(super::input::resolve),
        );
}
fn button(parent: &mut ChildSpawnerCommands, text: String, operation: Operation) {
    super::menu_ui::button(parent, text, operation, false, false, false);
}
fn launch(mut settings: ResMut<Settings>, bindings: Res<Bindings>, preferences: Res<Preferences>) {
    if std::env::var_os("TESSERAE_SCREENSHOT_SETTINGS").is_some()
        || std::env::var_os("TESSERAE_SCREENSHOT_HELP").is_some()
    {
        settings.tab = match std::env::var("TESSERAE_SCREENSHOT_SETTINGS_TAB").as_deref() {
            Ok("bindings") => super::menu_ui::Tab::Bindings,
            Ok("display") => super::menu_ui::Tab::Display,
            _ => super::menu_ui::Tab::Behavior,
        };
        settings.open = true;
        settings.dirty = true;
        settings.draft = Some((bindings.clone(), preferences.clone()));
    }
}

fn open_shortcut(
    actions: Res<super::actions::Actions>,
    mut settings: ResMut<Settings>,
    mut menu: ResMut<super::menu::Menu>,
    bindings: Res<Bindings>,
    preferences: Res<Preferences>,
    ui: Res<super::input::UiInput>,
    status: Res<crate::server::persistence::SaveStatus>,
    mut cursor: Query<&mut bevy::window::CursorOptions, With<bevy::window::PrimaryWindow>>,
) {
    if !actions.started(Action::Help) || ui.text_entry || status.modal() || settings.open {
        return;
    }
    settings.open = true;
    settings.tab = settings.last_controls;
    settings.draft = Some((bindings.clone(), preferences.clone()));
    settings.dirty = true;
    menu.open = false;
    for mut cursor in &mut cursor {
        cursor.grab_mode = bevy::window::CursorGrabMode::None;
        cursor.visible = true;
    }
}

fn apply(
    settings: &mut Settings,
    store: &mut Store,
    bindings: &mut Bindings,
    preferences: &mut Preferences,
) -> bool {
    let Some((draft_bindings, draft_preferences)) = &settings.draft else {
        return false;
    };
    if store.save(draft_bindings, draft_preferences).is_err() {
        settings.notice = store.error.clone().unwrap_or_default();
        return false;
    }
    *bindings = draft_bindings.clone();
    *preferences = draft_preferences.clone();
    settings.unsaved = false;
    settings.notice = "Applied".into();
    true
}
fn close(settings: &mut Settings) {
    settings.open = false;
    settings.draft = None;
    settings.capture = None;
    settings.conflict = None;
    settings.unsaved = false;
    settings.pending = None;
}
fn propose(settings: &mut Settings, action: Action, proposed: [Option<Binding>; 2]) {
    let draft = &mut settings.draft.as_mut().unwrap().0;
    let conflicts: Vec<_> = draft
        .0
        .iter()
        .filter(|(other, _)| **other != action)
        .flat_map(|(&other, alternatives)| {
            alternatives
                .iter()
                .enumerate()
                .filter_map(move |(index, binding)| {
                    binding
                        .filter(|binding| {
                            proposed.iter().flatten().any(|new| {
                                new.trigger == binding.trigger
                                    && new.modifier.overlaps(binding.modifier)
                            })
                        })
                        .map(|_| (other, index))
                })
        })
        .collect();
    if conflicts.is_empty() {
        draft.0.insert(action, proposed);
    } else {
        settings.conflict = Some((action, proposed, conflicts));
    }
}

fn activate(
    event: On<Activate>,
    operations: Query<&Operation>,
    mut settings: ResMut<Settings>,
    mut store: ResMut<Store>,
    mut bindings: ResMut<Bindings>,
    mut preferences: ResMut<Preferences>,
    save: Option<Res<crate::server::persistence::SaveStatus>>,
    mut menu: ResMut<super::menu::Menu>,
    mut focus: ResMut<bevy::input_focus::InputFocus>,
) {
    if save.is_some_and(|status| status.modal()) {
        return;
    }
    let Ok(operation) = operations.get(event.entity) else {
        return;
    };
    if settings.capture.is_some() {
        if matches!(operation, Operation::Cancel) {
            settings.capture = None;
            settings.notice.clear();
            settings.dirty = true;
        }
        return;
    }
    if settings.conflict.is_some()
        && !matches!(operation, Operation::Replace | Operation::CancelConflict)
    {
        return;
    }
    if settings.unsaved
        && !matches!(
            operation,
            Operation::Apply | Operation::Discard | Operation::Stay
        )
    {
        return;
    }
    settings.dirty = true;
    match *operation {
        Operation::Navigate(tab) => {
            if tab == super::menu_ui::Tab::Catalogue {
                if settings
                    .draft
                    .as_ref()
                    .is_some_and(|(b, p)| b != &*bindings || p != &*preferences)
                {
                    settings.unsaved = true;
                    settings.pending = Some(tab);
                } else {
                    close(&mut settings);
                    menu.open = true;
                }
            } else {
                if !settings.open {
                    settings.draft = Some((bindings.clone(), preferences.clone()));
                }
                settings.open = true;
                settings.tab = tab;
                if tab != super::menu_ui::Tab::Display {
                    settings.last_controls = tab;
                }
                menu.open = false;
            }
            focus.clear();
        }
        Operation::Apply => {
            let exiting = settings.unsaved;
            if apply(&mut settings, &mut store, &mut bindings, &mut preferences) && exiting {
                let target = settings.pending.take();
                close(&mut settings);
                menu.open = target == Some(super::menu_ui::Tab::Catalogue);
            }
        }
        Operation::Cancel | Operation::Discard => {
            let target = settings.pending.take();
            close(&mut settings);
            menu.open = target == Some(super::menu_ui::Tab::Catalogue);
        }
        Operation::Back => {
            settings.pending = None;
            if !settings.open {
                menu.open = false;
                return;
            }
            if settings
                .draft
                .as_ref()
                .is_some_and(|(b, p)| b != &*bindings || p != &*preferences)
            {
                settings.unsaved = true;
            } else {
                close(&mut settings);
            }
        }
        Operation::Stay => {
            settings.unsaved = false;
            settings.pending = None;
        }
        Operation::Capture(action, slot) => {
            focus.clear();
            settings.capture = Some((action, slot, false));
            settings.notice =
                "Release inputs, then press a key or mouse button. Escape cancels.".into();
        }
        Operation::CancelConflict => {
            settings.conflict = None;
        }
        Operation::Replace => {
            if let Some((action, proposed, conflicts)) = settings.conflict.take() {
                let draft = &mut settings.draft.as_mut().unwrap().0;
                for (other, index) in conflicts {
                    draft.0.get_mut(&other).unwrap()[index] = None;
                }
                draft.0.insert(action, proposed);
            }
        }
        Operation::ResetAction(action) => {
            propose(&mut settings, action, Bindings::default().0[&action])
        }
        operation => {
            let Some((draft, p)) = settings.draft.as_mut() else {
                return;
            };
            match operation {
                Operation::Reset => {
                    *draft = Bindings::default();
                    *p = Preferences::default();
                }
                Operation::Clear(action, slot) => draft.0.get_mut(&action).unwrap()[slot] = None,
                Operation::Sprint(value) => p.toggle_sprint = value,
                Operation::Removal(value) => p.tap_remove = value,
                Operation::Invert => p.invert_y = !p.invert_y,
                Operation::Sensitivity(value) => p.sensitivity = value,
                Operation::ScrollDirection(value) => p.reverse_scroll = value,
                Operation::ScrollSensitivity(value) => p.scroll_sensitivity = value,
                Operation::Sync(value) => p.display_sync = value,
                Operation::Cap(value) => p.fps_cap = value,
                _ => {}
            }
        }
    }
}
pub(super) fn capture(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut settings: ResMut<Settings>,
    active: (Res<Bindings>, Res<Preferences>),
    mut actions: ResMut<super::actions::Actions>,
    save: Option<Res<crate::server::persistence::SaveStatus>>,
) {
    if save.is_some_and(|status| status.modal()) {
        if settings.capture.take().is_some() {
            settings.notice = "Binding capture cancelled while the save dialog is open.".into();
            settings.dirty = true;
        }
        return;
    }
    if !settings.open {
        return;
    }
    if actions.cancel_requested() && keys.just_pressed(KeyCode::Escape) {
        actions.consume_cancel();
        settings.dirty = true;
        if settings.capture.take().is_some() || settings.conflict.take().is_some() {
            settings.notice.clear();
            return;
        }
        if settings.capture.is_none() {
            if settings
                .draft
                .as_ref()
                .is_some_and(|(b, p)| b != &*active.0 || p != &*active.1)
            {
                settings.unsaved = true;
            } else {
                close(&mut settings);
            }
        }
        settings.conflict = None;
        return;
    }
    let Some((action, slot, armed)) = settings.capture else {
        return;
    };
    if !armed {
        if keys.get_pressed().next().is_none() && mouse.get_pressed().next().is_none() {
            settings.capture = Some((action, slot, true));
        }
        return;
    }
    let modifier_key = |key: KeyCode| {
        matches!(
            key,
            KeyCode::ShiftLeft
                | KeyCode::ShiftRight
                | KeyCode::ControlLeft
                | KeyCode::ControlRight
                | KeyCode::SuperLeft
                | KeyCode::SuperRight
                | KeyCode::AltLeft
                | KeyCode::AltRight
        )
    };
    let mut triggers: Vec<_> = keys
        .get_just_pressed()
        .filter(|key| !modifier_key(**key))
        .copied()
        .map(Trigger::Key)
        .chain(mouse.get_just_pressed().copied().map(Trigger::Mouse))
        .collect();
    if triggers.is_empty() {
        triggers.extend(
            keys.get_just_released()
                .filter(|key| modifier_key(**key))
                .copied()
                .map(Trigger::Key),
        );
    }
    triggers.sort_by_key(|trigger| format!("{trigger:?}"));
    let Some(trigger) = triggers.first().copied() else {
        return;
    };
    let primary = keys.any_pressed([
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ]);
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let alt = keys.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]);
    let modifier = match (primary, shift, alt) {
        (true, true, true) => Modifier::PrimaryAltShift,
        (true, false, true) => Modifier::PrimaryAlt,
        (true, true, false) => Modifier::PrimaryShift,
        (true, false, false) => Modifier::Primary,
        (false, true, true) => Modifier::AltShift,
        (false, false, true) => Modifier::Alt,
        (false, true, false) => Modifier::Shift,
        _ => Modifier::None,
    };
    let binding = Binding { trigger, modifier };
    let mut proposed = settings.draft.as_ref().unwrap().0.0[&action];
    for (index, alternative) in proposed.iter_mut().enumerate() {
        if index != slot
            && alternative.is_some_and(|other| {
                other.trigger == binding.trigger && other.modifier.overlaps(binding.modifier)
            })
        {
            *alternative = None;
        }
    }
    proposed[slot] = Some(binding);
    propose(&mut settings, action, proposed);
    settings.capture = None;
    settings.notice.clear();
    settings.dirty = true;
    // Keep capture independent of gameplay action bindings, including a remapped menu key.
}
fn setting_row(
    parent: &mut ChildSpawnerCommands,
    title: &str,
    description: &str,
    choices: impl FnOnce(&mut ChildSpawnerCommands),
) {
    use super::menu_ui as ui;
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(9.0),
                padding: UiRect::all(Val::Px(16.0)),
                border: UiRect::bottom(Val::Px(1.0)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(ui::CARD),
            BorderColor::all(ui::EDGE),
        ))
        .with_children(|row| {
            ui::text(row, title, 15.0, ui::INK);
            ui::text(row, description, 12.0, ui::MUTED);
            row.spawn(Node {
                column_gap: Val::Px(6.0),
                row_gap: Val::Px(6.0),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            })
            .with_children(choices);
        });
}
fn option(
    parent: &mut ChildSpawnerCommands,
    label: impl Into<String>,
    operation: Operation,
    selected: bool,
) {
    super::menu_ui::button(parent, label, operation, selected, false, false);
}
fn action_name(action: Action) -> &'static str {
    use Action::*;
    match action {
        Forward => "Move forward",
        Backward => "Move backward",
        Left => "Move left",
        Right => "Move right",
        Ascend => "Ascend",
        Descend => "Descend",
        Sprint => "Sprint",
        Respawn => "Respawn",
        Faster => "Increase flight speed",
        Slower => "Decrease flight speed",
        Remove => "Remove",
        Place => "Place / ignite",
        Sample => "Sample",
        EmptyHand => "Empty hand",
        Shape => "Next Shape",
        Turn => "Turn Shape",
        Flip => "Flip Shape",
        Slot1 => "Quick slot 1",
        Slot2 => "Quick slot 2",
        Slot3 => "Quick slot 3",
        Slot4 => "Quick slot 4",
        Slot5 => "Quick slot 5",
        Slot6 => "Quick slot 6",
        Slot7 => "Quick slot 7",
        Slot8 => "Quick slot 8",
        Slot9 => "Quick slot 9",
        Undo => "Undo",
        Redo => "Redo",
        Catalogue => "Open Catalogue",
        ReleaseCursor => "Release cursor",
        Help => "Open input settings",
        Diagnostics => "Diagnostics",
    }
}
fn category(action: Action) -> &'static str {
    use Action::*;
    match action {
        Forward | Backward | Left | Right | Ascend | Descend | Sprint | Respawn | Faster
        | Slower => "Movement",
        Place | Remove | Sample | EmptyHand | Shape | Turn | Flip | Undo | Redo => "Building",
        _ => "Interface",
    }
}
pub(super) fn render(
    mut commands: Commands,
    mut settings: ResMut<Settings>,
    panels: Query<Entity, With<Panel>>,
    labels: Res<super::actions::Labels>,
    searches: Res<super::menu_ui::Searches>,
    active: (Res<Bindings>, Res<Preferences>),
    scrolls: Query<&ScrollPosition, With<SettingsScroll>>,
    mut positions: Local<[Vec2; 3]>,
    mut previous: Local<Option<super::menu_ui::Tab>>,
    context: (
        Res<super::edit::Hotbar>,
        Res<super::edit::Catalogue>,
        Res<super::edit::Notice>,
    ),
) {
    use super::menu_ui::{self as ui, Tab};
    if !settings.dirty && !labels.is_changed() {
        return;
    }
    if let Some(tab) = *previous {
        let index = match tab {
            Tab::Behavior => 0,
            Tab::Bindings => 1,
            _ => 2,
        };
        if let Some(scroll) = scrolls.iter().next() {
            positions[index] = scroll.0;
        }
    }
    *previous = settings.open.then_some(settings.tab);
    settings.dirty = false;
    for entity in &panels {
        commands.entity(entity).despawn();
    }
    if !settings.open {
        return;
    }
    let Some((bindings, p)) = &settings.draft else {
        return;
    };
    let changed = bindings != &*active.0 || p != &*active.1;
    let index = match settings.tab {
        Tab::Behavior => 0,
        Tab::Bindings => 1,
        _ => 2,
    };
    commands.spawn((Panel,GlobalZIndex(20),Node {position_type:PositionType::Absolute,left:Val::Percent(5.0),right:Val::Percent(5.0),top:Val::Px(if settings.tab==Tab::Bindings {246.0}else{192.0}),bottom:Val::Px(18.0),flex_direction:FlexDirection::Column,row_gap:Val::Px(12.0),..default()})).with_children(|root| {
        if !settings.notice.is_empty() {ui::text(root,&settings.notice,12.0,ui::LEAF);}
        if settings.capture.is_some() {
            ui::text(root,"Change binding",22.0,ui::INK);
            ui::text(root,"Release inputs, then press a key or mouse button. Escape cancels.",14.0,ui::MUTED);
            button(root,"Cancel binding".into(),Operation::Cancel);return;
        }
        if settings.unsaved {
            ui::text(root,"Keep your changes?",22.0,ui::INK);
            ui::text(root,"Apply saves your preferences. Discard leaves without saving.",14.0,ui::MUTED);
            root.spawn(Node {column_gap:Val::Px(8.0),..default()}).with_children(|row| {button(row,"Stay".into(),Operation::Stay);button(row,"Discard".into(),Operation::Discard);button(row,"Apply and continue".into(),Operation::Apply);});return;
        }
        if let Some((_,_,conflicts))=&settings.conflict {
            ui::text(root,"Binding already in use",22.0,ui::INK);
            let names:Vec<_>=conflicts.iter().map(|(action,_)|action_name(*action)).collect();
            ui::text(root,format!("Replace bindings assigned to {}?",names.join(", ")),14.0,ui::MUTED);
            root.spawn(Node {column_gap:Val::Px(8.0),..default()}).with_children(|row| {button(row,"Keep existing".into(),Operation::CancelConflict);button(row,"Replace binding".into(),Operation::Replace);});return;
        }
        root.spawn((SettingsScroll,ScrollArea,ScrollPosition(positions[index]),Node {flex_direction:FlexDirection::Column,flex_grow:1.0,min_height:Val::Px(0.0),overflow:Overflow::scroll_y(),row_gap:Val::Px(2.0),..default()})).with_children(|list| {
            match settings.tab {
                Tab::Behavior=>{
                    ui::text(list,super::edit::hud_text(&context.0,&context.1.0,&context.2,bindings,&labels,p),12.0,ui::MUTED);
                    setting_row(list,"Sprint activation",if p.toggle_sprint {"Press once to sprint; press again to stop."}else{"Sprint while the binding is held."},|row| {option(row,"Hold",Operation::Sprint(false),!p.toggle_sprint);option(row,"Toggle",Operation::Sprint(true),p.toggle_sprint);});
                    setting_row(list,"Removal activation",if p.tap_remove {"Timed removal on one target. A second tap cancels."}else{"Hold on one target until removal completes."},|row| {option(row,"Hold",Operation::Removal(false),!p.tap_remove);option(row,"Timed tap",Operation::Removal(true),p.tap_remove);});
                    setting_row(list,"Mouse sensitivity","Relative to the default mouse feel.",|row| {for value in [0.00075,0.0015,0.003,0.006,0.012] {option(row,format!("{:.2}x",value/0.003),Operation::Sensitivity(value),p.sensitivity==value);}});
                    setting_row(list,"Invert vertical look","Reverse the vertical mouse direction.",|row| {option(row,if p.invert_y {"On"}else{"Off"},Operation::Invert,p.invert_y);});
                    setting_row(list,"Scroll direction","Select quick slots with the scroll wheel.",|row| {option(row,"Standard",Operation::ScrollDirection(false),!p.reverse_scroll);option(row,"Reversed",Operation::ScrollDirection(true),p.reverse_scroll);});
                    setting_row(list,"Scroll sensitivity","Higher values move through quick slots faster.",|row| {for value in [0.5,1.0,2.0] {option(row,format!("{value:.1}x"),Operation::ScrollSensitivity(value),p.scroll_sensitivity==value);}});
                    setting_row(list,"Using the Catalogue","Click a Material to hold it. Drag to a quick slot, or press its number while pointing at an item. Shapes and Materials come from your Game.",|row| {ui::text(row,format!("{}: Catalogue   {}: input settings   {}: release cursor",bindings.label(Action::Catalogue,&labels),bindings.label(Action::Help,&labels),bindings.label(Action::ReleaseCursor,&labels)),12.0,ui::MUTED);});
                }
                Tab::Display=>{
                    setting_row(list,"Display sync","Match frame presentation to your display.",|row| {option(row,"Display synced",Operation::Sync(true),p.display_sync);option(row,"Sync off",Operation::Sync(false),!p.display_sync);});
                    ui::text(list,if p.display_sync {"Recommended for normal play. Avoids tearing when supported by your display."}else{"Sync is off. Frames may tear; a frame limit can reduce unnecessary work."},12.0,ui::MUTED);
                    setting_row(list,"Frame rate limit","An optional ceiling, independent of display sync.",|row| {for cap in [None,Some(60),Some(120),Some(144),Some(240),Some(1000)] {option(row,cap.map_or("No limit".into(),|n|format!("{n} FPS")),Operation::Cap(cap),p.fps_cap==cap);}});
                    ui::text(list,"Visual quality and simulation settings stay unchanged.",12.0,ui::MUTED);
                }
                Tab::Bindings=>{
                    let mut entries:Vec<_>=bindings.0.iter().filter(|(action,_)|ui::matches(&searches.bindings,&format!("{} {} {}",action_name(**action),category(**action),bindings.label(**action,&labels)))).collect();
                    entries.sort_by_key(|(action,_)|(category(**action),action_name(**action)));
                    ui::text(list,format!("{} actions",entries.len()),12.0,ui::MUTED);
                    if entries.is_empty() {ui::text(list,"No matches. Try another action or key, or clear your search.",15.0,ui::INK);}
                    for (&action,alternatives) in entries {
                        list.spawn((Node {padding:UiRect::all(Val::Px(12.0)),flex_direction:FlexDirection::Column,row_gap:Val::Px(8.0),flex_shrink:0.0,..default()},BackgroundColor(ui::CARD))).with_children(|row| {
                            row.spawn(Node {column_gap:Val::Px(12.0),align_items:AlignItems::Center,..default()}).with_children(|heading| {ui::text(heading,action_name(action),14.0,ui::INK);ui::text(heading,category(action),11.0,ui::MUTED);});
                            row.spawn(Node {column_gap:Val::Px(8.0),row_gap:Val::Px(6.0),flex_wrap:FlexWrap::Wrap,..default()}).with_children(|choices| {
                                for (slot,binding) in alternatives.iter().enumerate() {
                                    choices.spawn(Node {column_gap:Val::Px(5.0),align_items:AlignItems::Center,..default()}).with_children(|alternative| {
                                        ui::text(alternative,if slot==0 {"Primary"}else{"Alternative"},11.0,ui::MUTED);
                                        button(alternative,binding.map_or("Add binding".into(),|b|labels.binding(b)),Operation::Capture(action,slot));
                                        if binding.is_some() {button(alternative,"Clear".into(),Operation::Clear(action,slot));}
                                    });
                                }
                                button(choices,"Restore".into(),Operation::ResetAction(action));
                            });
                        });
                    }
                }
                Tab::Catalogue=>{}
            }
        });
        root.spawn((Node {flex_shrink:0.0,padding:UiRect::top(Val::Px(12.0)),border:UiRect::top(Val::Px(1.0)),justify_content:JustifyContent::SpaceBetween,column_gap:Val::Px(8.0),flex_wrap:FlexWrap::Wrap,row_gap:Val::Px(8.0),..default()},BorderColor::all(ui::EDGE))).with_children(|footer| {
            footer.spawn(Node {flex_direction:FlexDirection::Column,row_gap:Val::Px(5.0),..default()}).with_children(|status| {ui::text(status,if changed {"Changes not applied"}else{"No pending changes"},12.0,if changed {ui::LEAF}else{ui::MUTED});button(status,"Restore defaults".into(),Operation::Reset);});
            footer.spawn(Node {column_gap:Val::Px(8.0),align_items:AlignItems::Center,..default()}).with_children(|actions| {button(actions,"Cancel".into(),Operation::Cancel);ui::button(actions,"Apply changes",Operation::Apply,false,false,true);});
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<super::super::actions::Actions>()
            .init_resource::<Bindings>()
            .init_resource::<Preferences>()
            .init_resource::<Settings>()
            .init_resource::<super::super::menu::Menu>()
            .init_resource::<bevy::input_focus::InputFocus>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .insert_resource(Store {
                path: std::env::temp_dir().join(format!(
                    "tesserae-settings-{}-{}.json",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                )),
                error: None,
            })
            .add_observer(activate)
            .add_systems(Update, capture);
        app
    }
    fn activate_operation(app: &mut App, operation: Operation) {
        let entity = app.world_mut().spawn(operation).id();
        app.world_mut().trigger(Activate { entity });
    }
    #[test]
    fn save_modals_cancel_capture_and_block_settings_activation() {
        let mut app = app();
        app.init_resource::<crate::server::persistence::SaveStatus>();
        activate_operation(
            &mut app,
            Operation::Navigate(super::super::menu_ui::Tab::Behavior),
        );
        activate_operation(&mut app, Operation::Capture(Action::Forward, 0));
        app.world_mut()
            .resource_mut::<crate::server::persistence::SaveStatus>()
            .quitting = true;
        app.update();
        assert!(app.world().resource::<Settings>().capture.is_none());
        activate_operation(&mut app, Operation::Invert);
        assert!(
            !app.world()
                .resource::<Settings>()
                .draft
                .as_ref()
                .unwrap()
                .1
                .invert_y
        );
        assert!(app.world().resource::<Settings>().open);
    }

    #[test]
    fn failed_apply_keeps_active_preferences_and_the_draft() {
        let mut app = app();
        let parent = app.world().resource::<Store>().path.clone();
        std::fs::write(&parent, b"not a directory").unwrap();
        app.world_mut().resource_mut::<Store>().path = parent.join("profile.json");
        activate_operation(
            &mut app,
            Operation::Navigate(super::super::menu_ui::Tab::Behavior),
        );
        activate_operation(&mut app, Operation::Invert);
        activate_operation(&mut app, Operation::Apply);
        assert!(!app.world().resource::<Preferences>().invert_y);
        assert!(
            app.world()
                .resource::<Settings>()
                .draft
                .as_ref()
                .unwrap()
                .1
                .invert_y
        );
        assert!(app.world().resource::<Settings>().open);
        assert!(app.world().resource::<Store>().error.is_some());
        std::fs::remove_file(parent).unwrap();
    }

    #[test]
    fn draft_apply_cancel_and_unsaved_exit_are_distinct() {
        let mut app = app();
        activate_operation(
            &mut app,
            Operation::Navigate(super::super::menu_ui::Tab::Behavior),
        );
        activate_operation(&mut app, Operation::Invert);
        assert!(!app.world().resource::<Preferences>().invert_y);
        activate_operation(&mut app, Operation::Back);
        assert!(app.world().resource::<Settings>().unsaved);
        activate_operation(&mut app, Operation::Stay);
        assert!(app.world().resource::<Settings>().open);
        activate_operation(&mut app, Operation::Apply);
        assert!(app.world().resource::<Preferences>().invert_y);
        activate_operation(&mut app, Operation::Invert);
        activate_operation(&mut app, Operation::Cancel);
        assert!(app.world().resource::<Preferences>().invert_y);
        let path = &app.world().resource::<Store>().path;
        assert!(Store::load(path.clone()).2.invert_y);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn capture_waits_for_opening_release_and_conflicts_require_a_choice() {
        let mut app = app();
        activate_operation(
            &mut app,
            Operation::Navigate(super::super::menu_ui::Tab::Behavior),
        );
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        activate_operation(&mut app, Operation::Capture(Action::Forward, 0));
        app.update();
        assert!(!app.world().resource::<Settings>().capture.unwrap().2);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        app.update();
        assert!(app.world().resource::<Settings>().capture.unwrap().2);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyP);
        app.update();
        assert!(app.world().resource::<Settings>().conflict.is_some());
        assert_eq!(
            app.world().resource::<Bindings>().0[&Action::Sample],
            Bindings::default().0[&Action::Sample]
        );
        activate_operation(&mut app, Operation::Replace);
        assert!(
            app.world()
                .resource::<Settings>()
                .draft
                .as_ref()
                .unwrap()
                .0
                .0[&Action::Sample][0]
                .is_none()
        );
        assert_eq!(
            app.world().resource::<Bindings>().0[&Action::Forward],
            Bindings::default().0[&Action::Forward]
        );
        activate_operation(&mut app, Operation::ResetAction(Action::Sample));
        assert!(app.world().resource::<Settings>().conflict.is_some());
        activate_operation(&mut app, Operation::CancelConflict);
        assert!(
            app.world()
                .resource::<Settings>()
                .draft
                .as_ref()
                .unwrap()
                .0
                .0[&Action::Sample][0]
                .is_none()
        );
        activate_operation(&mut app, Operation::ResetAction(Action::Sample));
        activate_operation(&mut app, Operation::Replace);
        let draft = &app.world().resource::<Settings>().draft.as_ref().unwrap().0;
        assert!(draft.0[&Action::Forward][0].is_none());
        assert_eq!(
            draft.0[&Action::Sample],
            Bindings::default().0[&Action::Sample]
        );
    }
}

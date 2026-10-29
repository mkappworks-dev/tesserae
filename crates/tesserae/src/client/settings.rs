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
    show_reference: bool,
    resetting: bool,
    category: Category,
    notice: String,
    pub(super) dirty: bool,
    pub(super) tab: super::menu_ui::Tab,
    last_controls: super::menu_ui::Tab,
    pending: Option<super::menu_ui::Tab>,
}
impl Settings {
    pub(super) fn binding_visible(
        &self,
        action: Action,
        query: &str,
        bindings: &Bindings,
        labels: &super::actions::Labels,
    ) -> bool {
        (!query.trim().is_empty() || category(action) == self.category.label())
            && super::menu_ui::matches(
                query,
                &format!(
                    "{} {} {} {}",
                    action_name(action),
                    category(action),
                    bindings.label(action, labels),
                    binding_display(bindings.label(action, labels))
                ),
            )
    }

    pub(super) fn changed(&self, bindings: &Bindings, preferences: &Preferences) -> bool {
        self.draft
            .as_ref()
            .is_some_and(|(b, p)| b != bindings || p != preferences)
    }
    pub(super) fn modal(&self) -> bool {
        self.unsaved || self.conflict.is_some() || self.resetting
    }
    pub fn capturing(&self) -> bool {
        self.capture.is_some()
    }
}

#[derive(Component)]
pub(super) struct Panel;
#[derive(Component)]
pub(super) struct SettingsScroll;
#[derive(Default, Clone, Copy, PartialEq)]
pub(super) enum Category {
    #[default]
    Movement,
    Building,
    Interface,
}
impl Category {
    fn label(self) -> &'static str {
        match self {
            Self::Movement => "Movement",
            Self::Building => "Building",
            Self::Interface => "Interface",
        }
    }
}
#[derive(Component, Clone, Copy)]
pub(super) enum Operation {
    Navigate(super::menu_ui::Tab),
    Category(Category),
    Apply,
    Cancel,
    Back,
    Discard,
    Stay,
    Reset,
    ConfirmReset,
    CancelReset,
    Reference,
    ResetAction(Action),
    Capture(Action, usize),
    Clear(Action, usize),
    Replace,
    CancelConflict,
    Sprint(bool),
    Removal(bool),
    Invert,
    ScrollDirection(bool),
    Sync(bool),
    Cap(Option<u32>),
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Settings>()
        .init_resource::<super::menu::Menu>()
        .init_resource::<bevy::input_focus::InputFocus>()
        .add_observer(activate)
        .add_observer(scalar_changed)
        .add_systems(
            PreUpdate,
            capture_focus.before(bevy::input_focus::InputFocusSystems::Dispatch),
        )
        .add_systems(Update, slider_visuals)
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
fn launch(
    mut settings: ResMut<Settings>,
    bindings: Res<Bindings>,
    preferences: Res<Preferences>,
    mut searches: ResMut<super::menu_ui::Searches>,
) {
    if std::env::var_os("TESSERAE_SCREENSHOT").is_some()
        && let Ok(query) = std::env::var("TESSERAE_SCREENSHOT_SEARCH")
    {
        searches.catalogue = query.clone();
        searches.bindings = query;
    }
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
        if std::env::var_os("TESSERAE_SCREENSHOT").is_some() {
            match std::env::var("TESSERAE_SCREENSHOT_SETTINGS_DIALOG").as_deref() {
                Ok("capture") => {
                    settings.tab = super::menu_ui::Tab::Bindings;
                    settings.capture = Some((Action::Forward, 0, true));
                }
                Ok("reset") => settings.resetting = true,
                Ok("unsaved") => {
                    settings.draft.as_mut().unwrap().1.invert_y = !preferences.invert_y;
                    settings.unsaved = true;
                }
                _ => {}
            }
        }
    }
}

fn open_shortcut(
    actions: Res<super::actions::Actions>,
    mut settings: ResMut<Settings>,
    mut menu: ResMut<super::menu::Menu>,
    (bindings, preferences, store): (Res<Bindings>, Res<Preferences>, Res<Store>),
    (ui, status, ownership): (
        Res<super::input::UiInput>,
        Res<crate::server::persistence::SaveStatus>,
        Res<super::input::Ownership>,
    ),
    window: Single<&Window, With<bevy::window::PrimaryWindow>>,
    mut cursor: Query<&mut bevy::window::CursorOptions, With<bevy::window::PrimaryWindow>>,
) {
    if !window.focused
        || matches!(
            ownership.owner,
            super::input::Owner::Unfocused | super::input::Owner::TextEntry
        )
        || !actions.started(Action::Help)
        || ui.text_entry
        || status.modal()
        || settings.open
    {
        return;
    }
    settings.open = true;
    settings.notice = store.error.clone().unwrap_or_default();
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
    settings.resetting = false;
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
    (save, mut menu, mut focus): (
        Option<Res<crate::server::persistence::SaveStatus>>,
        ResMut<super::menu::Menu>,
        ResMut<bevy::input_focus::InputFocus>,
    ),
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
    if settings.resetting && !matches!(operation, Operation::ConfirmReset | Operation::CancelReset)
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
                    settings.notice = store.error.clone().unwrap_or_default();
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
        Operation::Discard => {
            let target = settings.pending.take();
            close(&mut settings);
            menu.open = target == Some(super::menu_ui::Tab::Catalogue);
        }
        Operation::Cancel | Operation::Back => {
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
        Operation::Category(category) => settings.category = category,
        Operation::Reset => settings.resetting = true,
        Operation::CancelReset => settings.resetting = false,
        Operation::Reference => settings.show_reference = !settings.show_reference,
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
            if matches!(operation, Operation::ConfirmReset) {
                settings.resetting = false;
            }
            let Some((draft, p)) = settings.draft.as_mut() else {
                return;
            };
            match operation {
                Operation::ConfirmReset => {
                    *draft = Bindings::default();
                    *p = Preferences::default();
                }
                Operation::Clear(action, slot) => draft.0.get_mut(&action).unwrap()[slot] = None,
                Operation::Sprint(value) => p.toggle_sprint = value,
                Operation::Removal(value) => p.tap_remove = value,
                Operation::Invert => p.invert_y = !p.invert_y,
                Operation::ScrollDirection(value) => p.reverse_scroll = value,
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
    (ui, focus): (
        Option<Res<super::input::UiInput>>,
        Option<Res<bevy::input_focus::InputFocus>>,
    ),
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
    let close_shortcut = actions.started(Action::Catalogue)
        && !settings.modal()
        && !settings.capturing()
        && !ui.as_ref().is_some_and(|ui| ui.text_entry)
        && !focus.is_some_and(|focus| focus.get().is_some());
    let cancel_shortcut = actions.cancel_requested()
        && (keys.just_pressed(KeyCode::Escape) || !ui.as_ref().is_some_and(|ui| ui.text_entry));
    if cancel_shortcut || close_shortcut {
        actions.consume_cancel();
        settings.dirty = true;
        if settings.unsaved {
            settings.unsaved = false;
            settings.pending = None;
            return;
        }
        if settings.resetting {
            settings.resetting = false;
            return;
        }
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
    last: bool,
    choices: impl FnOnce(&mut ChildSpawnerCommands),
) {
    use super::menu_ui as ui;
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
fn option(
    parent: &mut ChildSpawnerCommands,
    label: impl Into<String>,
    operation: Operation,
    selected: bool,
) {
    super::menu_ui::compact_button(parent, label, operation, selected, 31.0, 11.0);
}
pub(super) fn action_name(action: Action) -> &'static str {
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
pub(super) fn category(action: Action) -> &'static str {
    use Action::*;
    match action {
        Forward | Backward | Left | Right | Ascend | Descend | Sprint | Respawn | Faster
        | Slower | MovementMode => "Movement",
        Place | Remove | Sample | EmptyHand | Shape | Turn | Flip => "Building",
        _ => "Interface",
    }
}
type RenderData<'w> = (
    Res<'w, super::actions::Labels>,
    Res<'w, super::menu_ui::Searches>,
    (Res<'w, Bindings>, Res<'w, Preferences>),
);
type RenderContext<'w, 's> = (
    (
        Res<'w, super::edit::Hotbar>,
        Res<'w, super::edit::Catalogue>,
        Res<'w, super::edit::Notice>,
    ),
    Query<'w, 's, (), (With<Scalar>, With<bevy::ui::Pressed>)>,
);
pub(super) fn render(
    mut commands: Commands,
    mut settings: ResMut<Settings>,
    panels: Query<Entity, With<Panel>>,
    (labels, searches, active): RenderData,
    scrolls: Query<&ScrollPosition, With<SettingsScroll>>,
    (mut positions, mut previous, mut query): (
        Local<[Vec2; 3]>,
        Local<Option<super::menu_ui::Tab>>,
        Local<String>,
    ),
    (context, pressed): RenderContext,
) {
    use super::menu_ui::{self as ui, Tab};
    if !pressed.is_empty() || (!settings.dirty && !labels.is_changed()) {
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
    if settings.notice == "Applied" {
        super::menu_ui::notify(&mut commands, "Settings applied");
        settings.notice.clear();
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
    commands
        .spawn((
            Panel,
            GlobalZIndex(20),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(5.0),
                right: Val::Percent(5.0),
                top: Val::Px(if settings.tab == Tab::Bindings {
                    295.0
                } else {
                    235.0
                }),
                bottom: Val::Px(20.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(0.0),
                ..default()
            },
        ))
        .with_children(|root| {
            if !settings.notice.is_empty() {
                ui::text(
                    root,
                    &settings.notice,
                    12.0,
                    Color::srgb(254.0 / 255.0, 154.0 / 255.0, 0.0),
                );
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
                    padding: UiRect {
                        top: Val::Px(2.0),
                        right: Val::Px(5.0),
                        ..default()
                    },
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

fn page(
    list: &mut ChildSpawnerCommands,
    settings: &Settings,
    labels: &super::actions::Labels,
    searches: &super::menu_ui::Searches,
    context: (
        &super::edit::Hotbar,
        &super::edit::Catalogue,
        &super::edit::Notice,
    ),
) {
    use super::menu_ui::Tab;
    let (bindings, p) = settings.draft.as_ref().unwrap();
    match settings.tab {
        Tab::Behavior => behavior(list, settings, bindings, p, labels, context),
        Tab::Display => display(list, p),
        Tab::Bindings => bindings_page(list, settings, bindings, labels, searches),
        Tab::Catalogue => {}
    }
}

#[derive(Component, Clone, Copy)]
pub(super) enum Scalar {
    Mouse,
    Scroll,
}
#[derive(Component)]
struct ScalarThumb(Entity);
#[derive(Component)]
struct ScalarLabel(Entity);
fn slider(parent: &mut ChildSpawnerCommands, kind: Scalar, value: f32) {
    use super::menu_ui as ui;
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
                    SliderRange::new(0.25, 3.0),
                    SliderValue(value),
                    SliderStep(0.05),
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
                    ScalarThumb(slider),
                    BackgroundColor(ui::LEAF),
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Px(12.0),
                        height: Val::Px(12.0),
                        border_radius: BorderRadius::all(Val::Px(6.0)),
                        left: Val::Px(((value - 0.25) / 2.75).clamp(0.0, 1.0) * 123.0),
                        ..default()
                    },
                ));
            });
            row.spawn((
                ScalarLabel(slider),
                Text::new(format!("{value:.2}×")),
                TextFont {
                    font: bevy::text::FontSource::family("Aileron"),
                    ..ui::font(11.0)
                },
                TextColor(ui::INK),
                Pickable::IGNORE,
            ));
        });
}
fn scalar_changed(
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
    let value = event.value.clamp(0.25, 3.0);
    commands
        .entity(event.source)
        .insert(bevy::ui_widgets::SliderValue(value));
    match kind {
        Scalar::Mouse => preferences.sensitivity = value * 0.003,
        Scalar::Scroll => preferences.scroll_sensitivity = value,
    }
    settings.dirty = true;
}
fn slider_visuals(
    values: Query<&bevy::ui_widgets::SliderValue>,
    mut thumbs: Query<(&ScalarThumb, &mut Node)>,
    mut labels: Query<(&ScalarLabel, &mut Text)>,
) {
    for (thumb, mut node) in &mut thumbs {
        if let Ok(value) = values.get(thumb.0) {
            node.left = Val::Px(((value.0 - 0.25) / 2.75).clamp(0.0, 1.0) * 123.0);
        }
    }
    for (label, mut text) in &mut labels {
        if let Ok(value) = values.get(label.0) {
            text.set_if_neq(Text::new(format!("{:.2}×", value.0)));
        }
    }
}
fn group(
    parent: &mut ChildSpawnerCommands,
    title: &str,
    content: impl FnOnce(&mut ChildSpawnerCommands),
) {
    use super::menu_ui as ui;
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
fn segment(parent: &mut ChildSpawnerCommands, choices: impl FnOnce(&mut ChildSpawnerCommands)) {
    use super::menu_ui as ui;
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
fn switch(parent: &mut ChildSpawnerCommands, on: bool) {
    use super::menu_ui as ui;
    parent
        .spawn((
            bevy::ui_widgets::Button,
            Operation::Invert,
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
fn behavior(
    list: &mut ChildSpawnerCommands,
    settings: &Settings,
    bindings: &Bindings,
    p: &Preferences,
    labels: &super::actions::Labels,
    context: (
        &super::edit::Hotbar,
        &super::edit::Catalogue,
        &super::edit::Notice,
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
            false,
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
            true,
            |row| {
                segment(row, |buttons| {
                    option(buttons, "Hold", Operation::Removal(false), !p.tap_remove);
                    option(buttons, "Timed tap", Operation::Removal(true), p.tap_remove);
                })
            },
        );
    });
    group(list, "MOUSE & QUICK SLOTS", |card| {
        setting_row(
            card,
            "Mouse sensitivity",
            "Relative to the default mouse feel.",
            false,
            |row| slider(row, Scalar::Mouse, p.sensitivity / 0.003),
        );
        setting_row(
            card,
            "Invert vertical look",
            "Reverse the mouse look direction.",
            false,
            |row| switch(row, p.invert_y),
        );
        setting_row(
            card,
            "Scroll direction",
            if p.reverse_scroll {
                "Wheel up selects the next quick slot."
            } else {
                "Wheel down selects the next quick slot."
            },
            false,
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
            true,
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
        super::menu_ui::text(
            list,
            super::edit::hud_text(context.0, &context.1.0, context.2, bindings, labels, p),
            12.0,
            super::menu_ui::MUTED,
        );
    }
}

fn display(list: &mut ChildSpawnerCommands, p: &Preferences) {
    use super::menu_ui as ui;
    group(list, "PRESENTATION", |card| {
        setting_row(
            card,
            "Display sync",
            "Match frame presentation to your display.",
            false,
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
        card.spawn((
            BackgroundColor(Color::srgb(0.09, 0.09, 0.09)),
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
                    ui::MUTED
                } else {
                    Color::srgb(254.0 / 255.0, 154.0 / 255.0, 0.0)
                },
            )
        });
        card.spawn((
            BorderColor::all(ui::EDGE),
            Node {
                flex_direction: FlexDirection::Column,
                padding: UiRect::vertical(Val::Px(17.0)),
                border: UiRect::bottom(Val::Px(1.0)),
                row_gap: Val::Px(4.0),
                ..default()
            },
        ))
        .with_children(|limit| {
            ui::weighted_text(limit, "Frame rate limit", 13.0, 550, ui::INK);
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
                        let button = ui::compact_button(
                            caps,
                            cap.map_or("No limit".into(), |n| n.to_string()),
                            Operation::Cap(cap),
                            p.fps_cap == cap,
                            35.0,
                            11.0,
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
                    BackgroundColor(Color::srgb(0.069, 0.118, 0.081)),
                    BorderColor::all(Color::srgb(0.1735, 0.3373, 0.2126)),
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
                        ui::LEAF,
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
                        ui::INK,
                    );
                    ui::text(
                        policy,
                        p.fps_cap.map_or("No additional frame limit".into(), |n| {
                            format!("{n} FPS ceiling")
                        }),
                        11.0,
                        ui::MUTED,
                    );
                });
        });
    });
    ui::text(
        list,
        "View distance, textures, shadows, and simulation settings stay unchanged.",
        12.0,
        ui::MUTED,
    );
}

fn bindings_page(
    list: &mut ChildSpawnerCommands,
    settings: &Settings,
    bindings: &Bindings,
    labels: &super::actions::Labels,
    searches: &super::menu_ui::Searches,
) {
    use super::menu_ui as ui;
    let searching = !searches.bindings.trim().is_empty();
    if searching {
        ui::text(list, "Searching all categories", 11.0, ui::MUTED);
    } else {
        list.spawn((
            Node {
                border: UiRect::bottom(Val::Px(1.0)),
                column_gap: Val::Px(22.0),
                margin: UiRect::bottom(Val::Px(14.0)),
                ..default()
            },
            BorderColor::all(ui::EDGE),
        ))
        .with_children(|filters| {
            for category in [Category::Movement, Category::Building, Category::Interface] {
                let entity = ui::button(
                    filters,
                    category.label(),
                    Operation::Category(category),
                    settings.category == category,
                    true,
                    false,
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
        ui::MUTED,
    );
    binding_row(list, true, |row| {
        for label in ["ACTION", "PRIMARY", "ALTERNATIVE", ""] {
            ui::text(row, label, 9.0, ui::MUTED);
        }
    });
    let mut entries: Vec<_> = bindings
        .0
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
            ui::INK,
        );
    }
    for (&action, alternatives) in entries {
        binding_row(list, false, |row| {
            row.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            })
            .with_children(|name| {
                ui::text(name, action_name(action), 12.0, ui::INK);
                if searching {
                    ui::text(name, category(action), 9.0, ui::MUTED);
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
                            Operation::Capture(action, slot),
                            ui::BindingControl,
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
                            BackgroundColor(ui::BG),
                            BorderColor::all(ui::EDGE),
                        ))
                        .with_child((
                            Text::new(binding.map_or("Add binding".into(), |b| {
                                binding_display(labels.binding(b))
                            })),
                            TextFont {
                                font: bevy::text::FontSource::family("Aileron"),
                                ..ui::font(10.0)
                            },
                            TextColor(if binding.is_some() {
                                ui::INK
                            } else {
                                ui::MUTED
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
                        ui::quiet_button(cell, "×", Operation::Clear(action, slot), 12.0);
                    }
                });
            }
            ui::quiet_button(row, "↺", Operation::ResetAction(action), 16.0);
        });
    }
}
fn binding_row(
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
            BackgroundColor(if heading {
                Color::NONE
            } else {
                super::menu_ui::CARD
            }),
        ))
        .with_children(content);
}

fn footer(parent: &mut ChildSpawnerCommands, changed: bool) {
    use super::menu_ui as ui;
    parent
        .spawn((
            Node {
                min_height: Val::Px(73.0),
                margin: UiRect::top(Val::Auto),
                padding: UiRect::top(Val::Px(16.0)),
                border: UiRect::top(Val::Px(1.0)),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                flex_shrink: 0.0,
                ..default()
            },
            BorderColor::all(ui::EDGE),
        ))
        .with_children(|footer| {
            footer
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(|left| {
                    left.spawn(Node {
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(7.0),
                        ..default()
                    })
                    .with_children(|status| {
                        let color = if changed {
                            Color::srgb(254.0 / 255.0, 154.0 / 255.0, 0.0)
                        } else {
                            ui::MUTED
                        };
                        status.spawn((
                            BackgroundColor(color),
                            Node {
                                width: Val::Px(5.0),
                                height: Val::Px(5.0),
                                border_radius: BorderRadius::all(Val::Px(3.0)),
                                ..default()
                            },
                            Pickable::IGNORE,
                        ));
                        ui::text(
                            status,
                            if changed {
                                "Changes not applied"
                            } else {
                                "No pending changes"
                            },
                            11.0,
                            color,
                        );
                    });
                    ui::button(
                        left,
                        "Restore defaults",
                        Operation::Reset,
                        false,
                        false,
                        false,
                    );
                });
            footer
                .spawn(Node {
                    column_gap: Val::Px(8.0),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|actions| {
                    let cancel =
                        ui::button(actions, "Cancel", Operation::Cancel, false, false, false);
                    actions.commands().entity(cancel).insert(Node {
                        min_height: Val::Px(39.0),
                        padding: UiRect::axes(Val::Px(16.0), Val::Px(9.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: BorderRadius::all(Val::Px(6.0)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    });
                    let apply = ui::button(
                        actions,
                        "Apply changes",
                        Operation::Apply,
                        false,
                        false,
                        changed,
                    );
                    actions.commands().entity(apply).insert(Node {
                        min_width: Val::Px(136.0),
                        min_height: Val::Px(39.0),
                        padding: UiRect::axes(Val::Px(16.0), Val::Px(9.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: BorderRadius::all(Val::Px(6.0)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    });
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
struct ModalFocus;
fn capture_focus(
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
fn modal(commands: &mut Commands, settings: &Settings) {
    use super::menu_ui as ui;
    let mut first = None;
    let mut anchor = None;
    commands.spawn((Panel,GlobalZIndex(80),BackgroundColor(ui::BG.with_alpha(0.85)),Node {position_type:PositionType::Absolute,left:Val::Px(0.0),right:Val::Px(0.0),top:Val::Px(0.0),bottom:Val::Px(0.0),align_items:AlignItems::Center,justify_content:JustifyContent::Center,..default()})).with_children(|shade| {
        let dialog=shade.spawn((ModalFocus,bevy::input_focus::tab_navigation::TabGroup {modal:true,..default()},BackgroundColor(ui::CARD),BorderColor::all(ui::EDGE),Node {width:Val::Px(390.0),max_width:Val::Percent(92.0),padding:UiRect::all(Val::Px(26.0)),border:UiRect::all(Val::Px(1.0)),border_radius:BorderRadius::all(Val::Px(10.0)),flex_direction:FlexDirection::Column,row_gap:Val::Px(10.0),..default()})).with_children(|dialog| {
            if let Some((action,slot,_))=settings.capture {
                ui::weighted_text(dialog,"Change binding",19.0,620,ui::INK);
                ui::paragraph(dialog,format!("{} · {}",action_name(action),if slot==0{"Primary"}else{"Alternative"}),12.0,ui::MUTED);
                dialog.spawn((BackgroundColor(Color::srgb(0.069,0.118,0.081)),BorderColor::all(ui::LEAF),Node {padding:UiRect::all(Val::Px(28.0)),margin:UiRect::vertical(Val::Px(5.0)),border:UiRect::all(Val::Px(1.0)),border_radius:BorderRadius::all(Val::Px(6.0)),justify_content:JustifyContent::Center,..default()})).with_children(|well|ui::technical_text(well,"Press a key or mouse button",12.0,ui::LEAF));
                ui::paragraph(dialog,"Esc cancels. The opening click is ignored.",12.0,ui::MUTED);
                dialog.spawn(Node {justify_content:JustifyContent::FlexEnd,margin:UiRect::top(Val::Px(10.0)),..default()}).with_children(|actions|{ui::button(actions,"Cancel",Operation::Cancel,false,false,false);});
            }else {
                let (title,description)=if settings.unsaved {("Keep your changes?","You have unapplied behavior, binding or display changes.".into())}else if settings.resetting {("Restore defaults?","This resets the controls and display draft. Your saved preferences stay unchanged until you apply.".into())}else{let names=settings.conflict.as_ref().map(|(_,_,conflicts)|conflicts.iter().map(|(action,_)|action_name(*action)).collect::<Vec<_>>().join(", ")).unwrap_or_default();("Binding already in use",format!("This binding is assigned to {names}. Replace it?"))};
                ui::weighted_text(dialog,title,19.0,620,ui::INK);ui::paragraph(dialog,description,12.0,ui::MUTED);
                dialog.spawn(Node {justify_content:JustifyContent::FlexEnd,column_gap:Val::Px(7.0),row_gap:Val::Px(7.0),flex_wrap:FlexWrap::Wrap,margin:UiRect::top(Val::Px(10.0)),..default()}).with_children(|actions| {
                    if settings.unsaved {first=Some(ui::button(actions,"Stay",Operation::Stay,false,false,false));ui::button(actions,"Discard",Operation::Discard,false,false,false);ui::button(actions,"Save changes",Operation::Apply,false,false,true);}
                    else if settings.resetting {first=Some(ui::button(actions,"Cancel",Operation::CancelReset,false,false,false));ui::button(actions,"Restore defaults",Operation::ConfirmReset,false,false,true);}
                    else {first=Some(ui::button(actions,"Keep existing",Operation::CancelConflict,false,false,false));ui::button(actions,"Replace binding",Operation::Replace,false,false,true);}
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

fn binding_order(action: Action) -> usize {
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
        Faster,
        Slower,
        Place,
        Remove,
        Sample,
        EmptyHand,
        Turn,
        Flip,
        Shape,
        Catalogue,
        Help,
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
    ]
    .iter()
    .position(|candidate| *candidate == action)
    .unwrap()
}
fn binding_display(value: String) -> String {
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
            .add_observer(scalar_changed)
            .add_systems(
                PreUpdate,
                capture_focus.before(bevy::input_focus::InputFocusSystems::Dispatch),
            )
            .add_systems(Update, slider_visuals)
            .add_systems(Update, capture);
        app
    }
    fn activate_operation(app: &mut App, operation: Operation) {
        let entity = app.world_mut().spawn(operation).id();
        app.world_mut().trigger(Activate { entity });
    }
    #[test]
    fn sensitivity_slider_changes_only_the_draft_and_keeps_its_drag_entity() {
        use bevy::ui_widgets::{SliderValue, ValueChange};
        let mut app = app();
        app.add_observer(scalar_changed);
        activate_operation(
            &mut app,
            Operation::Navigate(super::super::menu_ui::Tab::Behavior),
        );
        let source = app
            .world_mut()
            .spawn((Scalar::Mouse, SliderValue(1.0), bevy::ui::Pressed))
            .id();
        app.world_mut().trigger(ValueChange {
            source,
            value: 2.0_f32,
            is_final: false,
        });
        app.update();
        assert_eq!(app.world().get::<SliderValue>(source).unwrap().0, 2.0);
        assert_eq!(
            app.world()
                .resource::<Settings>()
                .draft
                .as_ref()
                .unwrap()
                .1
                .sensitivity,
            0.006
        );
        assert_eq!(app.world().resource::<Preferences>().sensitivity, 0.003);
        assert!(app.world().get::<bevy::ui::Pressed>(source).is_some());
        activate_operation(&mut app, Operation::Cancel);
        assert_eq!(app.world().resource::<Preferences>().sensitivity, 0.003);
    }
    #[test]
    fn searching_bindings_renders_actual_mouse_actions_and_restore_requires_confirmation() {
        use super::super::menu_ui::{Searches, Tab};
        let mut app = app();
        assert!(app.world().resource::<Settings>().binding_visible(
            Action::Descend,
            "Left Ctrl",
            app.world().resource::<Bindings>(),
            &super::super::actions::Labels::default()
        ));
        app.init_resource::<super::super::actions::Labels>()
            .init_resource::<Searches>()
            .init_resource::<super::super::edit::Hotbar>()
            .init_resource::<super::super::edit::Catalogue>()
            .init_resource::<super::super::edit::Notice>()
            .init_resource::<Assets<Image>>()
            .init_resource::<Assets<Font>>()
            .init_resource::<super::super::input::UiInput>()
            .add_plugins(super::super::menu_ui::plugin)
            .add_systems(Update, render);
        activate_operation(&mut app, Operation::Navigate(Tab::Bindings));
        app.world_mut().resource_mut::<Searches>().bindings = "mouse".into();
        app.update();
        let captures: Vec<_> = app
            .world_mut()
            .query::<&Operation>()
            .iter(app.world())
            .filter_map(|operation| {
                if let Operation::Capture(action, _) = operation {
                    Some(*action)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(captures.len(), 6);
        assert!(
            app.world_mut()
                .query::<(&Text, &TextColor)>()
                .iter(app.world())
                .filter(|(text, _)| text.0 == "Add binding")
                .all(|(_, color)| color.0 == super::super::menu_ui::MUTED)
        );
        assert!(
            app.world_mut()
                .query::<&Text>()
                .iter(app.world())
                .any(|text| text.0 == "3 actions")
        );
        assert!(
            captures.contains(&Action::Place)
                && captures.contains(&Action::Remove)
                && captures.contains(&Action::Sample)
        );
        activate_operation(&mut app, Operation::Invert);
        activate_operation(&mut app, Operation::Reset);
        activate_operation(&mut app, Operation::Navigate(Tab::Behavior));
        assert_eq!(app.world().resource::<Settings>().tab, Tab::Bindings);
        activate_operation(&mut app, Operation::CancelReset);
        assert!(
            app.world()
                .resource::<Settings>()
                .draft
                .as_ref()
                .unwrap()
                .1
                .invert_y
        );
        activate_operation(&mut app, Operation::Reset);
        activate_operation(&mut app, Operation::ConfirmReset);
        assert!(
            !app.world()
                .resource::<Settings>()
                .draft
                .as_ref()
                .unwrap()
                .1
                .invert_y
        );
    }
    #[test]
    fn tab_navigation_keeps_the_draft_and_requires_a_choice_before_leaving() {
        use super::super::menu_ui::Tab;
        let mut app = app();
        activate_operation(&mut app, Operation::Navigate(Tab::Behavior));
        activate_operation(&mut app, Operation::Invert);
        activate_operation(&mut app, Operation::Navigate(Tab::Bindings));
        activate_operation(&mut app, Operation::Navigate(Tab::Display));
        assert!(
            app.world()
                .resource::<Settings>()
                .draft
                .as_ref()
                .unwrap()
                .1
                .invert_y
        );
        assert!(!app.world().resource::<Preferences>().invert_y);
        activate_operation(&mut app, Operation::Navigate(Tab::Catalogue));
        assert!(app.world().resource::<Settings>().unsaved);
        assert!(!app.world().resource::<super::super::menu::Menu>().open);
        activate_operation(&mut app, Operation::Navigate(Tab::Behavior));
        assert_eq!(app.world().resource::<Settings>().tab, Tab::Display);
        activate_operation(&mut app, Operation::Stay);
        activate_operation(&mut app, Operation::Navigate(Tab::Catalogue));
        activate_operation(&mut app, Operation::Discard);
        assert!(!app.world().resource::<Settings>().open);
        assert!(app.world().resource::<super::super::menu::Menu>().open);
        assert!(!app.world().resource::<Preferences>().invert_y);
    }
    #[test]
    fn input_shortcut_opens_the_last_controls_page_and_cancels_gameplay_in_that_frame() {
        use super::super::menu_ui::Tab;
        use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
        let mut app = app();
        app.init_resource::<crate::server::persistence::SaveStatus>()
            .init_resource::<super::super::input::UiInput>()
            .insert_resource(super::super::input::Ownership::playing())
            .add_plugins(super::super::actions::test_plugin)
            .add_systems(
                Update,
                (open_shortcut, super::super::input::resolve).chain(),
            );
        app.world_mut().resource_mut::<Settings>().last_controls = Tab::Bindings;
        app.world_mut().spawn((
            PrimaryWindow,
            Window {
                focused: true,
                ..default()
            },
            CursorOptions {
                grab_mode: CursorGrabMode::Locked,
                ..default()
            },
        ));
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyW);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyI);
        app.update();
        assert_eq!(app.world().resource::<Settings>().tab, Tab::Bindings);
        assert!(app.world().resource::<Settings>().open);
        assert!(
            !app.world()
                .resource::<super::super::actions::Actions>()
                .held(Action::Forward)
        );
        assert_eq!(
            app.world()
                .resource::<super::super::input::Ownership>()
                .owner,
            super::super::input::Owner::Settings
        );
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
        activate_operation(&mut app, Operation::Back);
        activate_operation(&mut app, Operation::Apply);
        assert!(app.world().resource::<Settings>().unsaved);
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
    fn tab_focus_navigation_does_not_request_a_menu_exit() {
        use super::super::menu_ui::Tab;
        for (text_entry, focused, key) in [
            (false, true, KeyCode::Tab),
            (true, false, KeyCode::Tab),
            (true, false, KeyCode::KeyC),
        ] {
            let mut app = app();
            app.add_plugins(super::super::actions::test_plugin)
                .insert_resource(super::super::input::UiInput {
                    text_entry,
                    ..default()
                });
            activate_operation(&mut app, Operation::Navigate(Tab::Bindings));
            activate_operation(&mut app, Operation::Invert);
            let control = app.world_mut().spawn_empty().id();
            if focused {
                app.world_mut()
                    .resource_mut::<bevy::input_focus::InputFocus>()
                    .set(control, bevy::input_focus::FocusCause::Navigated);
            }
            app.world_mut().resource_mut::<Bindings>().0.insert(
                Action::ReleaseCursor,
                [
                    Some(Binding {
                        trigger: Trigger::Key(KeyCode::KeyC),
                        modifier: Modifier::None,
                    }),
                    None,
                ],
            );
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(key);
            app.update();
            assert!(app.world().resource::<Settings>().open);
            assert!(!app.world().resource::<Settings>().unsaved);
        }
    }
    #[test]
    fn dirty_settings_prompt_on_every_exit_and_save_before_closing() {
        use super::super::menu_ui::Tab;
        for tab in [Tab::Behavior, Tab::Bindings, Tab::Display] {
            for exit in 0..5 {
                let mut app = app();
                app.add_plugins(super::super::actions::test_plugin);
                activate_operation(&mut app, Operation::Navigate(tab));
                activate_operation(&mut app, Operation::Invert);
                match exit {
                    0 => activate_operation(&mut app, Operation::Back),
                    1 => activate_operation(&mut app, Operation::Cancel),
                    2 => activate_operation(&mut app, Operation::Navigate(Tab::Catalogue)),
                    _ => {
                        app.world_mut()
                            .resource_mut::<ButtonInput<KeyCode>>()
                            .press(if exit == 3 {
                                KeyCode::Escape
                            } else {
                                KeyCode::Tab
                            });
                        app.update();
                    }
                }
                assert!(
                    app.world().resource::<Settings>().unsaved,
                    "{tab:?}, exit {exit}"
                );
                assert!(app.world().resource::<Settings>().open);
                assert!(!app.world().resource::<Preferences>().invert_y);
                activate_operation(&mut app, Operation::Apply);
                assert!(!app.world().resource::<Settings>().open);
                assert!(app.world().resource::<Preferences>().invert_y);
                assert_eq!(
                    app.world().resource::<super::super::menu::Menu>().open,
                    exit == 2
                );
                let path = app.world().resource::<Store>().path.clone();
                assert!(Store::load(path.clone()).2.invert_y);
                std::fs::remove_file(path).unwrap();
            }
        }
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
        assert!(app.world().resource::<Settings>().unsaved);
        activate_operation(&mut app, Operation::Discard);
        assert!(!app.world().resource::<Settings>().open);
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

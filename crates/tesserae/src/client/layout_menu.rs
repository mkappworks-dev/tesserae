//! The Layout menu: a panel docked over the live game, so the preview stays
//! drawn behind it. Its choices apply as they are made and are saved when it
//! closes.
//!
//! It is its own input context beside the Catalogue and the settings: the
//! cursor is released and the pad navigates it, and gameplay input waits.
use super::{
    actions::{Action, Actions},
    layout::{Direction, Form, Kind, LayoutMode, LayoutPreferences},
    menu_ui as ui,
    profile::{Preferences, Store},
};
use crate::world::WorldSettings;
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use tesserae_core::layout::MAX_LAYOUT_VOXELS;

const CARD_WIDTH: f32 = 340.0;
const STATUS_WIDTH: f32 = 300.0;

#[derive(Resource, Default)]
pub(super) struct LayoutMenu {
    pub(super) open: bool,
    /// Digits typed into an axis count, and which axis.
    typed: Option<(usize, String)>,
    notice: String,
    dirty: bool,
    /// What the panel is now: the menu, the status card, or nothing.
    shown: Option<bool>,
    /// The layout as it was when the menu opened, so closing saves only a change.
    opened_with: Option<LayoutPreferences>,
}

impl LayoutMenu {
    /// Open from the first frame, for a capture of the panel.
    pub(super) fn opened() -> Self {
        Self {
            open: true,
            dirty: true,
            ..default()
        }
    }
}

#[derive(Component, Clone, Copy, PartialEq, Debug)]
enum Control {
    Kind(Kind),
    Form(Form),
    Direction(usize, Direction),
    Type(usize),
    Count(usize, i32),
    Gap(i32),
    Close,
    Exit,
}

#[derive(Component)]
struct Panel;

/// The text that says what the layout is and what the aim would lay.
#[derive(Component)]
struct Info;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<LayoutMenu>()
        .add_observer(activate)
        .add_systems(
            Update,
            (
                toggle
                    .after(super::menu::toggle)
                    .before(super::input::resolve),
                (type_count, render, update_info, persist)
                    .chain()
                    .after(toggle),
            ),
        );
}

type Blockers<'w> = (
    Res<'w, super::menu::Menu>,
    Option<Res<'w, super::settings::Settings>>,
    Option<Res<'w, super::input::UiInput>>,
    Option<Res<'w, crate::server::persistence::SaveStatus>>,
);

/// The Layout key turns the mode on and opens the menu, and thereafter opens
/// and closes it; Escape closes it.
pub(super) fn toggle(
    actions: Res<Actions>,
    mut menu: ResMut<LayoutMenu>,
    mut mode: ResMut<LayoutMode>,
    preferences: Res<Preferences>,
    (catalogue, settings, ui, save): Blockers,
    window: Single<&Window, With<PrimaryWindow>>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if !window.focused
        || catalogue.open
        || settings.is_some_and(|settings| settings.open)
        || ui.is_some_and(|ui| ui.text_entry)
        || save.is_some_and(|save| save.modal())
    {
        return;
    }
    let key = actions.started(Action::Layout);
    let escape = menu.open && actions.cancel_requested();
    if !key && !escape {
        return;
    }
    if menu.open {
        close(&mut menu, &mut cursor, key);
    } else if key {
        mode.on = true;
        menu.open = true;
        menu.dirty = true;
        menu.typed = None;
        menu.notice.clear();
        menu.opened_with = Some(preferences.layout.clone());
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}

/// Closes the menu: the Layout key goes back to play, Escape leaves the cursor
/// free as it does from the Catalogue.
fn close(menu: &mut LayoutMenu, cursor: &mut CursorOptions, grab: bool) {
    menu.open = false;
    menu.typed = None;
    cursor.grab_mode = if grab {
        CursorGrabMode::Locked
    } else {
        CursorGrabMode::None
    };
    cursor.visible = !grab;
}

#[allow(clippy::too_many_arguments)]
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    mut menu: ResMut<LayoutMenu>,
    mut mode: ResMut<LayoutMode>,
    mut preferences: ResMut<Preferences>,
    mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let Ok(&control) = controls.get(event.entity) else {
        return;
    };
    if !menu.open {
        return;
    }
    let mut layout = preferences.layout.clone();
    menu.notice.clear();
    match control {
        Control::Kind(kind) => layout.kind = kind,
        Control::Form(form) => {
            layout.form = form;
            layout.fit();
            mode.active = mode.active.min(form.axes() - 1);
        }
        Control::Direction(axis, direction) => layout.set_direction(axis, direction),
        Control::Type(axis) => {
            mode.active = axis;
            menu.typed = None;
        }
        Control::Count(axis, by) => {
            mode.active = axis;
            menu.typed = None;
            if layout.bump(axis, by) {
                menu.notice = format!("Most is {MAX_LAYOUT_VOXELS} Voxels in all.");
            }
        }
        Control::Gap(by) => layout.set_gap(by),
        Control::Close | Control::Exit => {
            if control == Control::Exit {
                mode.on = false;
            }
            for mut cursor in &mut cursor {
                close(&mut menu, &mut cursor, true);
            }
        }
    }
    menu.dirty = true;
    if layout != preferences.layout {
        preferences.layout = layout;
    }
}

const DIGITS: [(KeyCode, u32); 20] = [
    (KeyCode::Digit0, 0),
    (KeyCode::Digit1, 1),
    (KeyCode::Digit2, 2),
    (KeyCode::Digit3, 3),
    (KeyCode::Digit4, 4),
    (KeyCode::Digit5, 5),
    (KeyCode::Digit6, 6),
    (KeyCode::Digit7, 7),
    (KeyCode::Digit8, 8),
    (KeyCode::Digit9, 9),
    (KeyCode::Numpad0, 0),
    (KeyCode::Numpad1, 1),
    (KeyCode::Numpad2, 2),
    (KeyCode::Numpad3, 3),
    (KeyCode::Numpad4, 4),
    (KeyCode::Numpad5, 5),
    (KeyCode::Numpad6, 6),
    (KeyCode::Numpad7, 7),
    (KeyCode::Numpad8, 8),
    (KeyCode::Numpad9, 9),
];

/// Types a count into the active axis: digits append, Backspace removes one and
/// Enter ends it. A pad has the steppers instead.
fn type_count(
    keys: Res<ButtonInput<KeyCode>>,
    mode: Res<LayoutMode>,
    mut menu: ResMut<LayoutMenu>,
    mut preferences: ResMut<Preferences>,
) {
    if !menu.open {
        return;
    }
    let axis = mode.active.min(preferences.layout.form.axes() - 1);
    if keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter]) {
        menu.typed = None;
        menu.dirty = true;
        return;
    }
    let mut digits = menu
        .typed
        .as_ref()
        .filter(|(typed_axis, _)| *typed_axis == axis)
        .map(|(_, digits)| digits.clone())
        .unwrap_or_default();
    let mut changed = keys.just_pressed(KeyCode::Backspace) && digits.pop().is_some();
    for (key, digit) in DIGITS {
        if keys.just_pressed(key) && digits.len() < 3 {
            digits.push(char::from_digit(digit, 10).expect("a digit"));
            changed = true;
        }
    }
    if !changed {
        return;
    }
    menu.typed = Some((axis, digits.clone()));
    menu.dirty = true;
    menu.notice.clear();
    if let Ok(count) = digits.parse::<u32>()
        && count > 0
    {
        let mut layout = preferences.layout.clone();
        if layout.set_count(axis, count) {
            menu.notice = format!("Most is {MAX_LAYOUT_VOXELS} Voxels in all.");
        } else if layout != preferences.layout {
            preferences.layout = layout;
        }
    }
}

/// Saves the layout when the menu closes having changed it.
fn persist(
    mut menu: ResMut<LayoutMenu>,
    preferences: Res<Preferences>,
    bindings: Res<super::actions::Bindings>,
    store: Option<ResMut<Store>>,
    mut commands: Commands,
) {
    if menu.open {
        return;
    }
    let Some(opened_with) = menu.opened_with.take() else {
        return;
    };
    if opened_with == preferences.layout {
        return;
    }
    if let Some(mut store) = store
        && let Err(error) = store.save(&bindings, &preferences)
    {
        warn!("Could not save the layout: {error}");
        ui::notify(&mut commands, "Could not save the layout", ui::Tone::Alert);
    }
}

fn row(parent: &mut ChildSpawnerCommands, children: impl FnOnce(&mut ChildSpawnerCommands)) {
    parent
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(5.0),
            row_gap: Val::Px(5.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(children);
}

fn button(parent: &mut ChildSpawnerCommands, label: &str, control: Control, selected: bool) {
    ui::compact_button(parent, label, control, selected, 28.0, 11.0);
}

fn gap_description(gap: u32, voxel_size: f32) -> String {
    format!(
        "{gap} Voxels, {:.1} m, between neighbours.",
        gap as f32 * voxel_size
    )
}

fn count_description(layout: &LayoutPreferences, typed: Option<&str>) -> String {
    let all = format!(
        "{} Voxels in all. Most is {MAX_LAYOUT_VOXELS}.",
        layout.total(layout.counts)
    );
    match typed {
        Some(digits) => format!("Typing {digits}_ for this axis. {all}"),
        None => all,
    }
}

/// What each frame says about the layout, wherever the card is.
#[allow(clippy::too_many_arguments)]
fn update_info(
    mode: Res<LayoutMode>,
    preferences: Res<Preferences>,
    hotbar: Res<super::edit::Hotbar>,
    camera: Single<&GlobalTransform, With<super::camera::FlyCamera>>,
    preview: Res<super::layout::Preview>,
    (bindings, labels): (Res<super::actions::Bindings>, Res<super::actions::Labels>),
    time: Res<Time>,
    mut text: Query<&mut Text, With<Info>>,
) {
    let shown = super::layout::summary(
        &mode,
        &preferences.layout,
        &hotbar,
        &camera,
        &preview,
        &bindings,
        &labels,
        time.elapsed_secs_f64(),
    );
    for mut text in &mut text {
        if text.0 != shown {
            text.0 = shown.clone();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render(
    mut commands: Commands,
    mut menu: ResMut<LayoutMenu>,
    panels: Query<Entity, With<Panel>>,
    controls: Query<&Control>,
    preferences: Res<Preferences>,
    mode: Res<LayoutMode>,
    world: Res<WorldSettings>,
    focus: Res<InputFocus>,
) {
    let want = if menu.open {
        Some(true)
    } else if mode.on {
        Some(false)
    } else {
        None
    };
    let rebuild =
        want != menu.shown || (want == Some(true) && (menu.dirty || preferences.is_changed()));
    if !rebuild {
        return;
    }
    menu.dirty = false;
    menu.shown = want;
    let focused = focus
        .get()
        .and_then(|entity| controls.get(entity).ok())
        .copied();
    for panel in &panels {
        commands.entity(panel).despawn();
    }
    let Some(open) = want else {
        return;
    };
    let layout = &preferences.layout;
    // A card as tall as its content, at the right edge and halfway down, so it
    // never covers the crosshair or the preview.
    commands
        .spawn((
            Panel,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(16.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|wrapper| {
            wrapper
                .spawn((
                    Node {
                        width: Val::Px(if open { CARD_WIDTH } else { STATUS_WIDTH }),
                        max_height: Val::Percent(92.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(8.0),
                        padding: UiRect::all(Val::Px(14.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: BorderRadius::all(Val::Px(8.0)),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(17.0 / 255.0, 17.0 / 255.0, 17.0 / 255.0, 0.94)),
                    BorderColor::all(ui::EDGE),
                ))
                .with_children(|card| {
                    card.spawn(Node {
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|header| {
                        ui::text(header, "Layout", 16.0, ui::INK);
                        if open {
                            button(header, "×", Control::Close, false);
                        }
                    });
                    let info = |card: &mut ChildSpawnerCommands| {
                        card.spawn((
                            Info,
                            Text::new(""),
                            ui::font(12.0),
                            bevy::text::LineHeight::RelativeToFont(1.5),
                            TextColor(ui::LEAF),
                            Pickable::IGNORE,
                        ));
                    };
                    if !open {
                        info(card);
                        return;
                    }
                    ui::eyebrow(card, "MODE");
                    row(card, |r| {
                        for (kind, name) in [
                            (Kind::Directions, "Direction and count"),
                            (Kind::Between, "Point to point"),
                        ] {
                            button(r, name, Control::Kind(kind), layout.kind == kind);
                        }
                    });
                    ui::eyebrow(card, "SHAPE");
                    row(card, |r| {
                        for (form, name) in [
                            (Form::Line, "Line"),
                            (Form::Rectangle, "Rectangle"),
                            (Form::Box, "Box"),
                        ] {
                            button(r, name, Control::Form(form), layout.form == form);
                        }
                    });
                    if layout.kind == Kind::Directions {
                        for (axis, name) in ["First axis", "Second axis", "Third axis"]
                            .into_iter()
                            .enumerate()
                            .take(layout.form.axes())
                        {
                            ui::eyebrow(card, &name.to_uppercase());
                            row(card, |r| {
                                for direction in Direction::ALL {
                                    button(
                                        r,
                                        direction.label(),
                                        Control::Direction(axis, direction),
                                        layout.directions[axis] == direction,
                                    );
                                }
                            });
                            row(card, |r| {
                                button(r, "Type", Control::Type(axis), mode.active == axis);
                                button(r, "−", Control::Count(axis, -1), false);
                                ui::text(r, layout.counts[axis].to_string(), 13.0, ui::INK);
                                button(r, "+", Control::Count(axis, 1), false);
                            });
                            let typed = menu
                                .typed
                                .as_ref()
                                .filter(|(typed_axis, _)| *typed_axis == axis)
                                .map(|(_, digits)| digits.as_str());
                            ui::description(card, &count_description(layout, typed));
                        }
                    }
                    ui::eyebrow(card, "GAP");
                    row(card, |r| {
                        button(r, "−", Control::Gap(-1), false);
                        ui::text(r, layout.gap.to_string(), 13.0, ui::INK);
                        button(r, "+", Control::Gap(1), false);
                    });
                    ui::description(card, &gap_description(layout.gap, world.voxel_size));
                    if !menu.notice.is_empty() {
                        ui::text(card, menu.notice.clone(), 11.0, ui::AMBER);
                    }
                    ui::eyebrow(card, "NOW");
                    info(card);
                    button(card, "Exit layout mode", Control::Exit, false);
                });
        });
    // The rebuilt panel keeps focus where it was, or starts at the top.
    let target = focused.unwrap_or(Control::Kind(layout.kind));
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::ui_widgets::Activate;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(super::super::actions::test_plugin);
        app.init_resource::<LayoutMenu>()
            .init_resource::<LayoutMode>()
            .init_resource::<super::super::menu::Menu>()
            .init_resource::<WorldSettings>()
            .init_resource::<InputFocus>()
            .init_resource::<super::super::input::Ownership>()
            .insert_resource(Store {
                path: std::env::temp_dir().join(format!(
                    "tesserae-layout-menu-{}-{}.json",
                    std::process::id(),
                    {
                        static TESTS: std::sync::atomic::AtomicU64 =
                            std::sync::atomic::AtomicU64::new(0);
                        TESTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                    }
                )),
                error: None,
            })
            .add_observer(activate);
        app.world_mut().spawn((
            Window {
                focused: true,
                ..default()
            },
            PrimaryWindow,
            CursorOptions {
                grab_mode: CursorGrabMode::Locked,
                visible: false,
                ..default()
            },
        ));
        app
    }

    fn menu(app: &App) -> &LayoutMenu {
        app.world().resource::<LayoutMenu>()
    }

    fn layout(app: &App) -> LayoutPreferences {
        app.world().resource::<Preferences>().layout.clone()
    }

    fn cursor(app: &mut App) -> (CursorGrabMode, bool) {
        let mut query = app.world_mut().query::<&CursorOptions>();
        let cursor = query.single(app.world()).unwrap();
        (cursor.grab_mode, cursor.visible)
    }

    /// One frame of input, then the toggle.
    fn press(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.world_mut().run_schedule(PreUpdate);
        app.world_mut().run_system_once(toggle).unwrap();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(key);
        keys.clear();
        app.world_mut().run_schedule(PreUpdate);
    }

    fn open(app: &mut App) {
        press(app, KeyCode::KeyL);
        assert!(menu(app).open);
    }

    fn activate_control(app: &mut App, control: Control) {
        let entity = app.world_mut().spawn(control).id();
        app.world_mut().trigger(Activate { entity });
    }

    #[test]
    fn the_layout_key_turns_the_mode_on_opens_the_menu_and_frees_the_cursor() {
        let mut app = app();
        assert_eq!(cursor(&mut app), (CursorGrabMode::Locked, false));
        open(&mut app);
        assert!(app.world().resource::<LayoutMode>().on);
        assert_eq!(cursor(&mut app), (CursorGrabMode::None, true));
    }

    #[test]
    fn the_key_again_closes_it_back_into_play_and_keeps_the_mode() {
        let mut app = app();
        open(&mut app);
        press(&mut app, KeyCode::KeyL);
        assert!(!menu(&app).open);
        assert!(app.world().resource::<LayoutMode>().on);
        assert_eq!(cursor(&mut app), (CursorGrabMode::Locked, false));
        press(&mut app, KeyCode::KeyL);
        assert!(menu(&app).open, "and it opens again while the mode is on");
    }

    #[test]
    fn escape_closes_it_and_leaves_the_cursor_free_as_the_catalogue_does() {
        let mut app = app();
        open(&mut app);
        press(&mut app, KeyCode::Escape);
        assert!(!menu(&app).open);
        assert!(app.world().resource::<LayoutMode>().on);
        assert_eq!(cursor(&mut app), (CursorGrabMode::None, true));
    }

    #[test]
    fn it_stays_shut_while_another_menu_or_a_text_field_has_the_input() {
        for blocker in 0..3 {
            let mut app = app();
            match blocker {
                0 => {
                    app.world_mut()
                        .resource_mut::<super::super::menu::Menu>()
                        .open = true
                }
                1 => {
                    app.init_resource::<super::super::input::UiInput>();
                    app.world_mut()
                        .resource_mut::<super::super::input::UiInput>()
                        .text_entry = true;
                }
                _ => {
                    app.init_resource::<super::super::settings::Settings>();
                    app.world_mut()
                        .resource_mut::<super::super::settings::Settings>()
                        .open = true;
                }
            }
            press(&mut app, KeyCode::KeyL);
            assert!(
                !menu(&app).open && !app.world().resource::<LayoutMode>().on,
                "{blocker}"
            );
        }
    }

    #[test]
    fn the_controls_change_the_layout_as_they_are_pressed() {
        let mut app = app();
        open(&mut app);
        activate_control(&mut app, Control::Kind(Kind::Between));
        activate_control(&mut app, Control::Form(Form::Box));
        activate_control(&mut app, Control::Gap(3));
        activate_control(&mut app, Control::Count(0, 2));
        // The second axis takes X, so the first trades to Z.
        activate_control(&mut app, Control::Direction(1, Direction::NegX));
        let saved = layout(&app);
        assert_eq!(saved.kind, Kind::Between);
        assert_eq!(saved.form, Form::Box);
        assert_eq!(saved.gap, 3);
        assert_eq!(saved.counts, [6, 3, 2]);
        assert_eq!(
            saved.directions,
            [Direction::PosZ, Direction::NegX, Direction::Up]
        );
        assert!(saved.valid());
    }

    #[test]
    fn a_count_past_the_cap_is_refused_with_a_note() {
        let mut app = app();
        open(&mut app);
        activate_control(&mut app, Control::Form(Form::Rectangle));
        app.world_mut().resource_mut::<Preferences>().layout.counts = [16, 16, 2];
        activate_control(&mut app, Control::Count(0, 1));
        assert_eq!(layout(&app).counts, [16, 16, 2]);
        assert!(!menu(&app).notice.is_empty());
    }

    #[test]
    fn a_shape_change_keeps_the_active_axis_in_range() {
        let mut app = app();
        open(&mut app);
        activate_control(&mut app, Control::Form(Form::Box));
        activate_control(&mut app, Control::Type(2));
        assert_eq!(app.world().resource::<LayoutMode>().active, 2);
        activate_control(&mut app, Control::Form(Form::Line));
        assert_eq!(app.world().resource::<LayoutMode>().active, 0);
    }

    #[test]
    fn close_goes_back_into_play_and_exit_also_turns_the_mode_off() {
        let mut app = app();
        open(&mut app);
        activate_control(&mut app, Control::Close);
        assert!(!menu(&app).open && app.world().resource::<LayoutMode>().on);
        assert_eq!(cursor(&mut app), (CursorGrabMode::Locked, false));
        open_again(&mut app);
        activate_control(&mut app, Control::Exit);
        assert!(!menu(&app).open && !app.world().resource::<LayoutMode>().on);
    }

    fn open_again(app: &mut App) {
        press(app, KeyCode::KeyL);
        assert!(menu(app).open);
    }

    #[test]
    fn digits_type_a_count_into_the_active_axis_and_the_cap_refuses_a_big_one() {
        let mut app = app();
        open(&mut app);
        activate_control(&mut app, Control::Type(0));
        let key = |app: &mut App, key| {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(key);
            app.world_mut().run_system_once(type_count).unwrap();
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.release(key);
            keys.clear();
        };
        key(&mut app, KeyCode::Digit1);
        key(&mut app, KeyCode::Digit2);
        assert_eq!(layout(&app).counts[0], 12);
        key(&mut app, KeyCode::Backspace);
        assert_eq!(layout(&app).counts[0], 1);
        key(&mut app, KeyCode::Enter);
        activate_control(&mut app, Control::Form(Form::Rectangle));
        app.world_mut().resource_mut::<Preferences>().layout.counts = [1, 16, 2];
        key(&mut app, KeyCode::Digit1);
        key(&mut app, KeyCode::Digit7);
        assert_eq!(layout(&app).counts[0], 1, "17 by 16 passes the cap");
        assert!(!menu(&app).notice.is_empty());
    }

    #[test]
    fn closing_having_changed_the_layout_saves_it_and_closing_unchanged_does_not() {
        let mut app = app();
        let path = app.world().resource::<Store>().path.clone();
        open(&mut app);
        app.world_mut().run_system_once(persist).unwrap();
        press(&mut app, KeyCode::KeyL);
        app.world_mut().run_system_once(persist).unwrap();
        assert!(!path.exists(), "nothing changed, nothing written");
        open_again(&mut app);
        activate_control(&mut app, Control::Gap(2));
        press(&mut app, KeyCode::KeyL);
        app.world_mut().run_system_once(persist).unwrap();
        let (_, _, saved) = Store::load(path.clone());
        assert_eq!(saved.layout.gap, 2);
        std::fs::remove_file(path).unwrap();
    }

    fn render_app() -> App {
        let mut app = app();
        app.init_resource::<super::super::actions::Labels>()
            .init_resource::<super::super::menu_ui::Searches>()
            .init_resource::<super::super::edit::Hotbar>()
            .init_resource::<super::super::edit::Notice>()
            .init_resource::<super::super::input::UiInput>()
            .init_resource::<Assets<Image>>()
            .init_resource::<Assets<Font>>()
            .add_plugins(super::super::menu_ui::plugin);
        app
    }

    fn controls(app: &mut App) -> Vec<Control> {
        app.world_mut().run_system_once(render).unwrap();
        app.world_mut().flush();
        app.world_mut()
            .query_filtered::<&Control, With<bevy::ui_widgets::Button>>()
            .iter(app.world())
            .copied()
            .collect()
    }

    fn panels(app: &mut App) -> (usize, usize) {
        let panels = app.world_mut().query::<&Panel>().iter(app.world()).count();
        let infos = app.world_mut().query::<&Info>().iter(app.world()).count();
        (panels, infos)
    }

    #[test]
    fn closed_with_the_mode_on_a_status_card_stays_with_no_controls_and_goes_with_the_mode() {
        let mut app = render_app();
        open(&mut app);
        assert!(!controls(&mut app).is_empty());
        press(&mut app, KeyCode::KeyL);
        assert!(
            controls(&mut app).is_empty(),
            "the card has nothing to press"
        );
        assert_eq!(
            panels(&mut app),
            (1, 1),
            "but it still says what the aim does"
        );
        press(&mut app, KeyCode::KeyL);
        activate_control(&mut app, Control::Exit);
        controls(&mut app);
        assert_eq!(
            panels(&mut app),
            (0, 0),
            "the mode is off, so nothing is shown"
        );
    }

    #[test]
    fn the_menu_holds_the_information_that_used_to_sit_in_the_corner() {
        let mut app = render_app();
        open(&mut app);
        controls(&mut app);
        assert_eq!(panels(&mut app), (1, 1));
    }

    #[test]
    fn a_rebuild_keeps_focus_on_the_same_control_and_closing_takes_the_controls_away() {
        let mut app = render_app();
        open(&mut app);
        controls(&mut app);
        let target = app
            .world_mut()
            .query_filtered::<(Entity, &Control), With<bevy::ui_widgets::Button>>()
            .iter(app.world())
            .find(|(_, control)| **control == Control::Gap(1))
            .map(|(entity, _)| entity)
            .unwrap();
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(target, FocusCause::Auto);
        activate_control(&mut app, Control::Gap(1));
        controls(&mut app);
        let focused = app.world().resource::<InputFocus>().get().unwrap();
        assert_ne!(focused, target, "the panel was rebuilt");
        assert_eq!(app.world().get::<Control>(focused), Some(&Control::Gap(1)));
        press(&mut app, KeyCode::KeyL);
        assert!(controls(&mut app).is_empty(), "closed, the controls go");
    }

    #[test]
    fn the_panel_offers_every_control_for_the_shape_and_hides_those_the_mode_does_not_use() {
        let mut app = render_app();
        open(&mut app);
        let shown = controls(&mut app);
        for expected in [
            Control::Kind(Kind::Directions),
            Control::Kind(Kind::Between),
            Control::Form(Form::Line),
            Control::Form(Form::Rectangle),
            Control::Form(Form::Box),
            Control::Direction(0, Direction::PosX),
            Control::Direction(0, Direction::Down),
            Control::Type(0),
            Control::Count(0, -1),
            Control::Count(0, 1),
            Control::Gap(-1),
            Control::Gap(1),
            Control::Close,
            Control::Exit,
        ] {
            assert!(shown.contains(&expected), "{expected:?}");
        }
        assert!(
            !shown.contains(&Control::Count(1, 1)),
            "a line has one axis"
        );
        activate_control(&mut app, Control::Kind(Kind::Between));
        let shown = controls(&mut app);
        assert!(
            shown.contains(&Control::Kind(Kind::Directions)),
            "the way back stays"
        );
        assert!(!shown.iter().any(|c| matches!(
            c,
            Control::Direction(..) | Control::Count(..) | Control::Type(_)
        )));
    }
}

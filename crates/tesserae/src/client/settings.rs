//! Every change applies at once and is saved; a display change asks to be
//! kept before it stays.

mod distance;
mod operations;
mod pages;
mod widgets;

use super::{
    actions::{Action, Binding, Bindings, Device, Modifier, Trigger, Wheel},
    profile::{Preferences, Store},
};
use crate::client::buttons::{self, ButtonStyle};
use crate::session::SessionApp;
use bevy::{
    prelude::*,
    ui_widgets::{Activate, ScrollArea},
};

pub(super) use distance::*;
pub(super) use operations::*;
pub(super) use pages::*;
pub(super) use widgets::*;

type Conflict = (Action, [Option<Binding>; 2], Vec<(Action, usize)>);

#[derive(Resource, Default)]
pub(super) struct Settings {
    pub open: bool,
    capture: Option<(Device, Action, usize, bool)>,
    conflict: Option<Conflict>,
    keep: Option<Keep>,
    /// Changed preferences or bindings wait for the frame's end to be saved,
    /// so a dragged slider writes the file once.
    unsaved: bool,
    show_reference: bool,
    /// Whether Controller's Touch controls are expanded.
    touch_open: bool,
    resetting: bool,
    category: Category,
    notice: String,
    pub(super) dirty: bool,
    pub(super) tab: super::menu_ui::Tab,
    last_controls: super::menu_ui::Tab,
    /// The category F10 reopens.
    last: super::menu_ui::Tab,
}
impl Settings {
    pub(super) fn modal(&self) -> bool {
        self.conflict.is_some() || self.resetting || self.keep.is_some()
    }
    pub fn capturing(&self) -> bool {
        self.capture.is_some()
    }
}

/// The display preferences a change may need to undo, and how long is left
/// to keep the change.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) struct Keep {
    previous: Display,
    left_s: f32,
}

pub(super) const KEEP_S: f32 = 10.0;

/// The preferences a bad value of could leave the screen unusable.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) struct Display {
    window_mode: super::presentation::WindowMode,
    sync: bool,
    cap: Option<u32>,
    anti_aliasing: super::anti_alias::AntiAliasing,
    scale: u32,
}
impl Display {
    fn of(p: &Preferences) -> Self {
        Self {
            window_mode: p.window_mode,
            sync: p.display_sync,
            cap: p.fps_cap,
            anti_aliasing: p.anti_aliasing,
            scale: p.render_scale,
        }
    }
    fn restore(self, p: &mut Preferences) {
        p.window_mode = self.window_mode;
        p.display_sync = self.sync;
        p.fps_cap = self.cap;
        p.anti_aliasing = self.anti_aliasing;
        p.render_scale = self.scale;
    }
}

#[derive(Component)]
pub(super) struct Panel;
#[derive(Component)]
pub(super) struct SettingsScroll;
#[derive(Default, Clone, Copy, PartialEq, Debug)]
pub(super) enum Category {
    #[default]
    Movement,
    Building,
    Interface,
}
impl Category {
    const ALL: [Category; 3] = [Self::Movement, Self::Building, Self::Interface];
    fn label(self) -> &'static str {
        match self {
            Self::Movement => "Movement",
            Self::Building => "Building",
            Self::Interface => "Interface",
        }
    }
}
#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub(super) enum Operation {
    Navigate(super::menu_ui::Tab),
    Category(Category),
    /// Q and E: the previous or next binding category.
    CategoryStep(i8),
    Cancel,
    Back,
    Reset,
    ConfirmReset,
    CancelReset,
    KeepDisplay,
    RevertDisplay,
    Reference(bool),
    TouchControls,
    ResetAction(Action),
    Capture(Device, Action, usize),
    Clear(Device, Action, usize),
    Replace,
    CancelConflict,
    Sprint(crate::server::player::SpeedLevel),
    Removal(bool),
    Repeat(bool),
    Ghost(bool),
    Compass(bool),
    Minimap(bool),
    MinimapSize(super::minimap::MinimapSize),
    Invert(bool),
    PadInvert(bool),
    PadCrouch(bool),
    PadMenuRumble(bool),
    TouchInvert(bool),
    TouchMirror(bool),
    TouchSize(super::profile::TouchSize),
    ScrollDirection(bool),
    WindowMode(super::presentation::WindowMode),
    Sync(bool),
    Cap(Option<u32>),
    AntiAliasing(super::anti_alias::AntiAliasing),
    Scale(u32),
    AmbientOcclusion(bool),
    WaterRefraction(bool),
    TextScale(u32),
    ReducedMotion(bool),
    SolidBackground(bool),
    StrongerContrast(bool),
    Distance(super::quality::DistancePreset),
}

pub(super) fn plugin(app: &mut App) {
    super::frame::plugin(app);
    app.init_session_state::<Settings>()
        .init_session_state::<super::menu::Menu>()
        .init_resource::<bevy::input_focus::InputFocus>()
        .add_observer(activate)
        .add_observer(step)
        .add_observer(run)
        .add_observer(scalar_changed)
        .add_systems(
            Update,
            (choice_visuals, cap_visuals, show_help.after(render)).run_if(crate::session::in_menus),
        )
        .add_systems(
            PreUpdate,
            capture_focus
                .before(bevy::input_focus::InputFocusSystems::Dispatch)
                .run_if(crate::session::in_menus),
        )
        .add_systems(Update, slider_visuals.run_if(crate::session::in_menus))
        .add_session_systems(Startup, launch)
        .add_session_systems(
            Update,
            open_shortcut
                .after(super::menu::toggle)
                .before(capture)
                .before(super::input::resolve),
        )
        .add_systems(
            Update,
            (count_down, capture, render)
                .chain()
                .before(super::input::resolve)
                .run_if(crate::session::in_menus),
        )
        .add_systems(Last, persist.run_if(crate::session::in_menus));
}
fn launch(mut settings: ResMut<Settings>, preferences: Res<Preferences>) {
    if std::env::var_os("TESSERAE_SCREENSHOT_SETTINGS").is_some()
        || std::env::var_os("TESSERAE_SCREENSHOT_HELP").is_some()
    {
        settings.tab = match std::env::var("TESSERAE_SCREENSHOT_SETTINGS_TAB").as_deref() {
            Ok("bindings") => super::menu_ui::Tab::Bindings,
            Ok("controller") => super::menu_ui::Tab::Controller,
            Ok("audio") => super::menu_ui::Tab::Audio,
            Ok("display") => super::menu_ui::Tab::Display,
            Ok("distance") => super::menu_ui::Tab::Distance,
            Ok("accessibility") => super::menu_ui::Tab::Accessibility,
            _ => super::menu_ui::Tab::Behavior,
        };
        settings.open = true;
        settings.dirty = true;
        if std::env::var_os("TESSERAE_SCREENSHOT").is_some() {
            match std::env::var("TESSERAE_SCREENSHOT_SETTINGS_DIALOG").as_deref() {
                Ok("capture") => {
                    settings.tab = super::menu_ui::Tab::Bindings;
                    settings.capture = Some((Device::KeyboardMouse, Action::Forward, 0, true));
                }
                Ok("reset") => settings.resetting = true,
                Ok("keep") => {
                    settings.keep = Some(Keep {
                        previous: Display::of(&preferences),
                        left_s: KEEP_S,
                    })
                }
                _ => {}
            }
        }
    }
}

/// Opens Settings at the category used last.
#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct Opener<'w, 's> {
    settings: ResMut<'w, Settings>,
    store: Res<'w, Store>,
    cursor:
        Query<'w, 's, &'static mut bevy::window::CursorOptions, With<bevy::window::PrimaryWindow>>,
}

impl Opener<'_, '_> {
    /// At the category used last; opening Settings again while they show
    /// does nothing.
    pub(super) fn open(&mut self) {
        if self.settings.open {
            return;
        }
        let last = self.settings.last;
        self.open_at(last);
    }

    fn open_at(&mut self, tab: super::menu_ui::Tab) {
        let settings = &mut *self.settings;
        settings.open = true;
        settings.notice = self.store.error.clone().unwrap_or_default();
        settings.tab = tab;
        settings.dirty = true;
        for mut cursor in &mut self.cursor {
            cursor.grab_mode = bevy::window::CursorGrabMode::None;
            cursor.visible = true;
        }
    }
}

type Gates<'w> = (
    Res<'w, super::input::UiInput>,
    Res<'w, crate::server::persistence::SaveStatus>,
    Res<'w, super::input::Ownership>,
    Option<Res<'w, super::inventory_window::InventoryWindow>>,
    Option<Res<'w, super::map_window::MapWindow>>,
);

pub(super) fn open_shortcut(
    actions: Res<super::actions::Actions>,
    mut opener: Opener,
    (ui, status, ownership, inventory, map): Gates,
    window: Single<&Window, With<bevy::window::PrimaryWindow>>,
) {
    if !window.focused
        || matches!(
            ownership.owner,
            super::input::Owner::Unfocused
                | super::input::Owner::TextEntry
                | super::input::Owner::Dead
        )
        || ui.text_entry
        || status.modal()
        || opener.settings.open
        || inventory.is_some_and(|window| window.open)
        || map.is_some_and(|window| window.open)
    {
        return;
    }
    if actions.started(Action::Settings) {
        opener.open();
    } else if actions.started(Action::Help) {
        let tab = opener.settings.last_controls;
        opener.open_at(tab);
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(super) fn observe_operations_for_test(app: &mut App) {
    app.add_observer(activate)
        .add_observer(step)
        .add_observer(run)
        .add_observer(scalar_changed);
}

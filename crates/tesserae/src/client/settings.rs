//! Draft preferences become active only after a successful Apply.

mod drafts;
mod pages;
mod widgets;

use super::{
    actions::{Action, Binding, Bindings, Device, Modifier, Trigger, Wheel},
    profile::{Preferences, Store},
};
use crate::client::buttons::{self, Align, ButtonStyle};
use crate::session::SessionApp;
use bevy::{
    prelude::*,
    ui_widgets::{Activate, ScrollArea},
};

pub(super) use drafts::*;
pub(super) use pages::*;
pub(super) use widgets::*;

type Conflict = (Action, [Option<Binding>; 2], Vec<(Action, usize)>);

#[derive(Resource, Default)]
pub(super) struct Settings {
    pub open: bool,
    draft: Option<(Bindings, Preferences)>,
    capture: Option<(Device, Action, usize, bool)>,
    conflict: Option<Conflict>,
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
                    bindings.label_for(Device::KeyboardMouse, action, labels),
                    binding_display(bindings.label_for(Device::KeyboardMouse, action, labels))
                ),
            )
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
    TouchControls,
    ResetAction(Device, Action),
    Capture(Device, Action, usize),
    Clear(Device, Action, usize),
    Replace,
    CancelConflict,
    Sprint(bool),
    Removal(bool),
    Repeat,
    Invert,
    PadInvert,
    PadCrouch(bool),
    TouchInvert,
    TouchMirror(bool),
    TouchSize(super::profile::TouchSize),
    ScrollDirection(bool),
    Sync(bool),
    Cap(Option<u32>),
    AntiAliasing(super::anti_alias::AntiAliasing),
    Scale(u32),
    TextScale(u32),
    ReducedMotion(bool),
    SolidBackground(bool),
}

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<Settings>()
        .init_session_state::<super::menu::Menu>()
        .init_resource::<bevy::input_focus::InputFocus>()
        .add_observer(activate)
        .add_observer(scalar_changed)
        .add_observer(toggle_select)
        .add_systems(
            Update,
            (
                close_selects.before(super::menu::toggle).before(capture),
                place_selects,
                select_visuals,
                checkbox_visuals,
            )
                .run_if(crate::session::in_menus),
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
            (capture, render)
                .chain()
                .before(super::input::resolve)
                .run_if(crate::session::in_menus),
        );
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
            Ok("controller") => super::menu_ui::Tab::Controller,
            Ok("display") => super::menu_ui::Tab::Display,
            Ok("accessibility") => super::menu_ui::Tab::Accessibility,
            _ => super::menu_ui::Tab::Behavior,
        };
        settings.open = true;
        settings.dirty = true;
        settings.draft = Some((bindings.clone(), preferences.clone()));
        if std::env::var_os("TESSERAE_SCREENSHOT").is_some() {
            match std::env::var("TESSERAE_SCREENSHOT_SETTINGS_DIALOG").as_deref() {
                Ok("capture") => {
                    settings.tab = super::menu_ui::Tab::Bindings;
                    settings.capture = Some((Device::KeyboardMouse, Action::Forward, 0, true));
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

/// Opens Settings at the category used last, drafting what is applied now.
#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct Opener<'w, 's> {
    settings: ResMut<'w, Settings>,
    bindings: Res<'w, Bindings>,
    preferences: Res<'w, Preferences>,
    store: Res<'w, Store>,
    cursor:
        Query<'w, 's, &'static mut bevy::window::CursorOptions, With<bevy::window::PrimaryWindow>>,
}

impl Opener<'_, '_> {
    /// At the category used last.
    /// Opening Settings again while they show, from the header's button,
    /// keeps the draft.
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
        settings.draft = Some((self.bindings.clone(), self.preferences.clone()));
        settings.dirty = true;
        for mut cursor in &mut self.cursor {
            cursor.grab_mode = bevy::window::CursorGrabMode::None;
            cursor.visible = true;
        }
    }
}

pub(super) fn open_shortcut(
    actions: Res<super::actions::Actions>,
    mut opener: Opener,
    (ui, status, ownership, inventory): (
        Res<super::input::UiInput>,
        Res<crate::server::persistence::SaveStatus>,
        Res<super::input::Ownership>,
        Option<Res<super::inventory_window::InventoryWindow>>,
    ),
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

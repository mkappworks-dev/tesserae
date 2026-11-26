//! Edit World: renaming, game mode and Daylight cycle changes waiting for the
//! next open, and deleting.

use super::*;

/// The acts on Edit World and its Delete prompt.
pub(super) fn on_edit_world(l: &mut Launcher, cx: &mut Acting, act: Act) {
    match act {
        Act::Edit(path) => {
            let Some(entry) = l.entry(&path) else {
                return;
            };
            let name = entry.name().unwrap_or_default().to_owned();
            let chosen = entry.details().map_or_else(Modes::default, modes);
            let on = entry.details().is_none_or(cycle);
            l.rename = name;
            l.modes = chosen;
            l.cycle = on;
            l.selected = Some(path.clone());
            l.show(Screen::Edit(path));
        }
        Act::Delete => {
            if let Screen::Edit(path) = l.screen.clone() {
                l.from_list = false;
                l.show(Screen::Delete(path));
            }
        }
        Act::EditHighlighted | Act::DeleteHighlighted if l.screen == Screen::Worlds => {
            if let Some(path) = l.selected.clone() {
                let act = if act == Act::EditHighlighted {
                    Act::Edit(path)
                } else {
                    Act::DeleteWorld(path)
                };
                on_edit_world(l, cx, act);
            }
        }
        Act::DeleteWorld(path) => {
            l.selected = Some(path.clone());
            l.from_list = true;
            l.show(Screen::Delete(path));
        }
        Act::ShowFolder => {
            if let Some(path) = l.selected.clone() {
                reveal(&path);
            }
        }
        Act::CopySeed => {
            if let Some(seed) = l.selected().and_then(Entry::details).map(|d| d.seed) {
                copy(cx.commands, cx.clipboard.as_deref_mut(), seed);
            }
        }
        Act::EditMode(mode) => {
            l.modes = Modes {
                default: mode,
                own: mode,
            };
            l.refocus = Some(Act::EditMode(mode));
            l.version += 1;
        }
        Act::EditCycle(on) => {
            l.cycle = on;
            l.refocus = Some(Act::EditCycle(on));
            l.version += 1;
        }
        Act::SaveName => {
            let Screen::Edit(path) = l.screen.clone() else {
                return;
            };
            match save(l, &path) {
                Ok(()) => {
                    l.show(Screen::Worlds);
                    l.refresh(cx.worlds);
                }
                Err(e) => {
                    l.problem = Some(e);
                    l.version += 1;
                }
            }
        }
        Act::DontSave | Act::SaveAndContinue => {
            let Screen::SaveFirst(path) = l.screen.clone() else {
                return;
            };
            l.show(Screen::Edit(path.clone()));
            if act == Act::SaveAndContinue {
                if let Err(e) = save(l, &path) {
                    l.problem = Some(e);
                    return;
                }
                l.refresh(cx.worlds);
            }
            recreate(l, cx);
        }
        Act::DiscardEdits => {
            if matches!(l.screen, Screen::DiscardEdits(_)) {
                l.show(Screen::Worlds);
            }
        }
        Act::ConfirmDelete => {
            let Screen::Delete(path) = l.screen.clone() else {
                return;
            };
            let result = library::ensure_closed(&path)
                .map_err(describe)
                .and_then(|()| (cx.remover.0)(&path));
            match result {
                Ok(()) => {
                    l.selected = None;
                    l.show(Screen::Worlds);
                    l.refresh(cx.worlds);
                }
                Err(e) => {
                    l.problem = Some(e);
                    l.version += 1;
                }
            }
        }
        _ => {}
    }
}

/// Writes Edit World's name, mode and Daylight cycle to the World's save.
fn save(l: &Launcher, path: &Path) -> Result<(), String> {
    let details = l.entry(path).and_then(Entry::details);
    let current = details.map(saved_modes);
    let was_cycle = details.map_or(l.cycle, |d| d.daylight_cycle);
    let name = library::valid_name(&l.rename).map_err(|_| too_long())?;
    library::rename(path, &name).map_err(describe)?;
    // Applied by the Server when the World next opens; nothing changes in play.
    let (now, was) = (l.modes, current.unwrap_or(l.modes));
    let changes = library::ModeChanges {
        default: (now.default != was.default).then_some(now.default),
        own: (now.own != was.own).then_some(now.own),
        daylight_cycle: (l.cycle != was_cycle).then_some(l.cycle),
    };
    library::request_modes(path, changes).map_err(describe)
}

/// Whether Edit World holds a name, mode or Daylight cycle not yet saved.
pub(super) fn unsaved(l: &Launcher) -> bool {
    let Screen::Edit(path) = &l.screen else {
        return false;
    };
    let Some(entry) = l.entry(path) else {
        return false;
    };
    let named = entry.name().unwrap_or_default() != l.rename.trim();
    named
        || entry
            .details()
            .is_some_and(|d| modes(d) != l.modes || cycle(d) != l.cycle)
}

/// A World's default game mode, for Players new to it, and the player's own.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Modes {
    pub(super) default: GameMode,
    pub(super) own: GameMode,
}

/// The modes as a World's save holds them.
pub(super) fn saved_modes(details: &Sidecar) -> Modes {
    Modes {
        default: details.game_mode,
        own: details.player_mode.unwrap_or(details.game_mode),
    }
}

/// The same once any requested change has applied.
pub(super) fn modes(details: &Sidecar) -> Modes {
    let saved = saved_modes(details);
    Modes {
        default: details.mode_changes.default.unwrap_or(saved.default),
        own: details.mode_changes.own.unwrap_or(saved.own),
    }
}

/// A World's Daylight cycle once any requested change has applied.
pub(super) fn cycle(details: &Sidecar) -> bool {
    details
        .mode_changes
        .daylight_cycle
        .unwrap_or(details.daylight_cycle)
}

pub(super) fn describe(error: tesserae_core::save::Error) -> String {
    match error {
        tesserae_core::save::Error::Busy => "This World is open in another Tesserae.".into(),
        e => e.to_string(),
    }
}

pub(super) fn copy(
    commands: &mut Commands,
    clipboard: Option<&mut bevy::clipboard::Clipboard>,
    seed: u64,
) {
    let copied = clipboard.is_some_and(|c| c.set_text(seed.to_string()).is_ok());
    if copied {
        ui::notify(commands, "Seed copied", Tone::Change);
    } else {
        ui::notify(commands, "Could not copy the seed", Tone::Alert);
    }
}

pub(super) fn reveal(path: &Path) {
    let program = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(windows) {
        "explorer"
    } else {
        "xdg-open"
    };
    if let Err(e) = std::process::Command::new(program).arg(path).spawn() {
        warn!("could not show {}: {e}", path.display());
    }
}

//! Opening a World: through Trust when its Game or Mods need it, then the
//! Opening screen that hands it to the loader.

use super::*;

/// The acts that open a World.
pub(super) fn on_open(l: &mut Launcher, cx: &mut Acting, act: Act) {
    match act {
        Act::Continue => {
            if let Some(path) = l.last_played().map(|e| e.path.clone()) {
                play(l, cx.installed, cx.relaunch, path);
            }
        }
        Act::Play(path) => play(l, cx.installed, cx.relaunch, path),
        Act::Trust => {
            let Screen::Trust(next) = l.screen.clone() else {
                return;
            };
            if let Err(e) = cx.installed.grant_all() {
                l.problem = Some(format!("Could not record Trust: {e}"));
                l.version += 1;
                return;
            }
            if !cx.installed.untrusted.is_empty() {
                l.version += 1;
                return;
            }
            match next {
                Next::Open(path) => open(l, cx.relaunch, path),
                Next::Create => create(l, cx.installed, cx.worlds, cx.relaunch),
            }
        }
        _ => {}
    }
}

/// Opens a listed World, through Trust first when its Game or Mods need it.
pub(super) fn play(
    l: &mut Launcher,
    installed: &mut Installed,
    relaunch: &mut Switch,
    path: PathBuf,
) {
    let Some(entry) = l.entry(&path) else {
        return;
    };
    let needs_game = entry.details().is_none_or(|d| d.game.is_some());
    let ids = match entry.details() {
        Some(d) => {
            if !installed.unmet(d).is_empty() {
                return;
            }
            d.game
                .iter()
                .map(|g| g.id.clone())
                .chain(d.mods.iter().map(|m| m.id.clone()))
                .collect()
        }
        None => installed.default_ids(),
    };
    l.selected = Some(path.clone());
    if needs_game && let Err(e) = installed.prepare_trust(&ids) {
        l.problem = Some(format!("Could not check Trust: {e}"));
        l.version += 1;
        return;
    }
    if needs_game && !installed.untrusted.is_empty() {
        l.show(Screen::Trust(Next::Open(path)));
    } else {
        open(l, relaunch, path);
    }
}

pub(super) fn open(l: &mut Launcher, relaunch: &mut Switch, path: PathBuf) {
    let name = l.display_name(&path);
    relaunch.0 = Some(Target::World(path));
    l.show(Screen::Opening(name));
}

/// Lets the Opening screen draw, then hands the World to the loader.
pub(super) fn opening(
    mut commands: Commands,
    launcher: Option<ResMut<Launcher>>,
    mut switch: ResMut<Switch>,
    next: Option<ResMut<NextState<AppState>>>,
) {
    let Some(mut l) = launcher else {
        return;
    };
    let Screen::Opening(name) = &l.screen else {
        return;
    };
    let name = name.clone();
    l.opening_frames += 1;
    if l.opening_frames == OPENING_FRAMES
        && let Some(mut next) = next
        && let Some(Target::World(folder)) = switch.0.take()
    {
        commands.insert_resource(OpenRequest {
            what: What::Folder(folder),
            name: Some(name),
            from_list: true,
        });
        next.set(AppState::Loading);
    }
}

//! Create World: the steps, the Biomes draft and making the World folder.

use super::*;

/// The acts on Create World and its steps.
pub(super) fn on_create_world(l: &mut Launcher, cx: &mut Acting, act: Act) {
    match act {
        Act::New => {
            if cx.installed.game.is_none() {
                return;
            }
            l.form = Form {
                game: cx.installed.game.as_ref().map(|g| g.id.clone()),
                mods: cx
                    .installed
                    .game
                    .as_ref()
                    .map(|g| cx.installed.suggested(&g.id))
                    .unwrap_or_default(),
                ..Form::default()
            };
            // In the field, so the seed is seen and can be copied before the World is made.
            l.form.seed = l.form.random_seed.to_string();
            l.initial = l.form.clone();
            l.show(Screen::Create(Step::Game));
        }
        Act::ReCreate => {
            let Some(details) = l.selected().and_then(Entry::details).cloned() else {
                return;
            };
            // A different Game or Mod version would silently change the content.
            if details.game.is_none() || !cx.installed.unmet(&details).is_empty() {
                return;
            }
            let copy = format!("{} (copy)", details.name);
            l.form = Form {
                name: copy.chars().take(library::MAX_NAME_CHARS).collect(),
                seed: details.seed.to_string(),
                voxel_size_m: details.voxel_size_m,
                game_mode: details.game_mode,
                game: details.game.as_ref().map(|g| g.id.clone()),
                mods: details.mods.iter().map(|m| m.id.clone()).collect(),
                seeded: details.biomes.clone(),
                ..Form::default()
            };
            if let Some(game) = l.form.game.clone() {
                l.clashes = cx.installed.clashes(&game, &l.form.mods);
            }
            l.initial = l.form.clone();
            // Through the Game step's Trust and read, as Next from it would.
            l.show(Screen::Create(Step::Game));
            go(l, cx.installed, cx.previewer, Step::World);
        }
        Act::CopyResolved => {
            if let Ok(seed) = l.form.seed() {
                copy(cx.commands, cx.clipboard.as_deref_mut(), seed);
            }
        }
        Act::Randomize => {
            l.form.seed = tesserae_core::seed::random().to_string();
            l.version += 1;
        }
        Act::PickGame(index) => {
            let Some(game) = cx.installed.games.get(index) else {
                return;
            };
            if l.form.game.as_ref() == Some(&game.id) {
                if l.screen == Screen::Create(Step::Game) {
                    go(l, cx.installed, cx.previewer, Step::World);
                }
                return;
            }
            l.form.game = Some(game.id.clone());
            l.form.climate = None;
            l.terrain = None;
            // Each Game starts from the Mods it suggests; picks made for another do not carry over.
            l.form.mods = cx.installed.suggested(&game.id);
            l.version += 1;
        }
        Act::ToggleMod(index) => {
            let (Some(game), Some(m)) = (l.form.game.clone(), cx.installed.mods.get(index)) else {
                return;
            };
            if cx.installed.blocked(&game, &m.id).is_some() {
                return;
            }
            if let Some(clash) = l.clashes.get(&m.id) {
                // A Mod that clashes with a pick swaps it out; one that clashes with what the
                // Game requires cannot be added.
                let Some(out) = clash.replaces.clone() else {
                    return;
                };
                l.form.mods.remove(&out);
                l.form.mods.insert(m.id.clone());
            } else if !l.form.mods.remove(&m.id) {
                let needed = cx.installed.closure_for(&game, &l.form.mods);
                // A dependency of another pick is locked on; ticking it by hand adds nothing.
                if !needed.contains(&m.id) {
                    l.form.mods.insert(m.id.clone());
                }
            }
            l.clashes = cx.installed.clashes(&game, &l.form.mods);
            l.refocus = Some(Act::ToggleMod(index));
            l.version += 1;
        }
        Act::TrustStep => {
            let Screen::StepTrust { on, to } = l.screen else {
                return;
            };
            l.screen = Screen::Create(on);
            if let Err(e) = cx.installed.grant_all() {
                l.form.problem = Some(format!("Could not record Trust: {e}"));
                l.version += 1;
                return;
            }
            go(l, cx.installed, cx.previewer, to);
        }
        Act::Next => {
            if let Screen::Create(step) = l.screen
                && let Some(next) = step.next()
            {
                go(l, cx.installed, cx.previewer, next);
            }
        }
        Act::Step(target) => go(l, cx.installed, cx.previewer, target),
        Act::ToggleBiome(at) => edit_biomes(l, |d| {
            if let Some(choice) = d.choices.get_mut(at) {
                choice.enabled = !choice.enabled;
            }
        }),
        Act::SelectBiome(at) => edit_biomes(l, |d| d.select(at)),
        Act::CycleGround(layer) => edit_biomes(l, |d| cycle(d, layer)),
        Act::ResetBiomes => edit_biomes(l, |d| d.resetting = true),
        Act::ConfirmReset => {
            if l.screen == Screen::Create(Step::Biomes)
                && let Some(climate) = l.form.climate.as_mut()
            {
                climate.set(climate.declared);
            }
            edit_biomes(l, BiomesDraft::reset);
        }
        Act::KeepBiomes => edit_biomes(l, |d| d.resetting = false),
        Act::KeepEditing => {
            if let Screen::Discard(step) = l.screen {
                l.show(Screen::Create(step));
            }
        }
        Act::DiscardSetup => {
            if matches!(l.screen, Screen::Discard(_)) {
                l.show(Screen::Worlds);
            }
        }
        Act::Voxel(size) => {
            l.form.voxel_size_m = size;
            l.version += 1;
        }
        Act::Mode(mode) => {
            l.form.game_mode = mode;
            l.version += 1;
        }
        Act::Create => {
            if !matches!(l.screen, Screen::Create(_)) {
                return;
            }
            let ids = chosen_ids(&l.form, cx.installed);
            if let Err(e) = cx.installed.prepare_trust(&ids) {
                l.problem = Some(format!("Could not check Trust: {e}"));
                l.version += 1;
                return;
            }
            let needs_trust = !cx.installed.untrusted.is_empty();
            if needs_trust && validate(&l.form).is_ok() {
                l.show(Screen::Trust(Next::Create));
            } else {
                create(l, cx.installed, cx.worlds, cx.relaunch);
            }
        }
        _ => {}
    }
}

/// Moves Create World to `target`. Moving forward passes every step before it,
/// so an invalid one holds the player there with its message.
pub(super) fn go(l: &mut Launcher, installed: &mut Installed, previewer: Previewer, target: Step) {
    let Screen::Create(current) = l.screen else {
        return;
    };
    if target > current {
        for step in Step::ORDER.iter().copied().filter(|s| *s < target) {
            if let Err(problem) = check(step, &l.form, installed) {
                l.form.problem = Some(problem);
                l.show(Screen::Create(step));
                return;
            }
        }
    }
    l.form.problem = None;
    if let Some(game) = l.form.game.clone() {
        l.clashes = installed.clashes(&game, &l.form.mods);
    }
    if current == Step::Game && target > Step::Game && !game_trusted(l, installed, target) {
        return;
    }
    if target > Step::Game && !read_game(l, installed, previewer) {
        return;
    }
    // Biomes is gated by `read_biomes`, which also covers a jump there from Review.
    if current <= Step::Mods && target > Step::Biomes && !mods_trusted(l, installed, target) {
        return;
    }
    if target == Step::Biomes && !read_biomes(l, installed, previewer) {
        return;
    }
    l.show(Screen::Create(target));
}

/// Whether the chosen Game may run. Its previews on the later steps run its Generator, so leaving
/// the Game step asks for its Trust in a dialog when it is missing.
fn game_trusted(l: &mut Launcher, installed: &mut Installed, to: Step) -> bool {
    let ids = game_ids(&l.form, installed);
    if let Err(e) = installed.prepare_trust(&ids) {
        l.form.problem = Some(format!("Could not check Trust: {e}"));
        l.show(Screen::Create(Step::Game));
        return false;
    }
    if installed.untrusted.is_empty() {
        return true;
    }
    l.show(Screen::StepTrust { on: Step::Game, to });
    false
}

/// Reads what the chosen Game declares, for the World step's Climate sizes and terrain preview,
/// unless already read for it. `false` when the player must stay on the Game step to read why not.
pub(super) fn read_game(l: &mut Launcher, installed: &Installed, previewer: Previewer) -> bool {
    if l.form.game.is_none() {
        return true;
    }
    // Picking another Game clears its Climate sizes, so these are this Game's.
    if l.form.climate.is_some() {
        return true;
    }
    let ids = game_ids(&l.form, installed);
    match (previewer.0)(installed, &ids) {
        Ok(declared) => {
            let mut climate = ClimateDraft::new(declared.climate);
            if let Some(seeded) = l.form.seeded.as_ref().and_then(|e| e.climate) {
                climate.set(seeded);
            }
            l.form.climate = Some(climate);
            l.terrain = declared.source.map(|source| (ids, source));
            true
        }
        Err(e) => {
            l.form.problem = Some(format!("Could not read the Game: {e}"));
            l.show(Screen::Create(Step::Game));
            false
        }
    }
}

/// Whether the chosen Game and Mods may run. Leaving the Mods step reads their Biomes, which runs
/// them, so Trust comes first; when it is missing a dialog over the Mods step asks for it.
fn mods_trusted(l: &mut Launcher, installed: &mut Installed, to: Step) -> bool {
    let ids = chosen_ids(&l.form, installed);
    if let Err(e) = installed.prepare_trust(&ids) {
        l.form.problem = Some(format!("Could not check Trust: {e}"));
        l.show(Screen::Create(Step::Mods));
        return false;
    }
    if installed.untrusted.is_empty() {
        return true;
    }
    l.show(Screen::StepTrust { on: Step::Mods, to });
    false
}

/// Reads the Biomes the chosen Game and Mods declare, unless already read for
/// exactly them. `false` when the player must stay on Mods, to trust them or to read why not.
pub(super) fn read_biomes(
    l: &mut Launcher,
    installed: &mut Installed,
    previewer: Previewer,
) -> bool {
    let ids = chosen_ids(&l.form, installed);
    if l.form.biomes.as_ref().is_some_and(|d| d.ids == ids) {
        return true;
    }
    // Reached from Review or Biomes this skips `go`'s check, so Trust revoked since is caught here.
    if !mods_trusted(l, installed, Step::Biomes) {
        return false;
    }
    match (previewer.0)(installed, &ids) {
        Ok(mut declared) => {
            if let Some(source) = declared.source.take() {
                l.terrain = Some((ids.clone(), source));
            }
            let mut draft = BiomesDraft::new(ids, declared);
            if let Some(seeded) = l.form.seeded.take() {
                draft.seed(&seeded);
            }
            l.form.biomes = Some(draft);
            true
        }
        Err(e) => {
            l.form.problem = Some(format!("Could not read the Biomes: {e}"));
            l.show(Screen::Create(Step::Mods));
            false
        }
    }
}

pub(super) fn edit_biomes(l: &mut Launcher, change: impl FnOnce(&mut BiomesDraft)) {
    if l.screen != Screen::Create(Step::Biomes) {
        return;
    }
    if let Some(draft) = l.form.biomes.as_mut() {
        change(draft);
        l.version += 1;
    }
}

/// What a Biome's slider ranges over, and its step.
pub(super) fn knob_range(knob: Knob) -> std::ops::RangeInclusive<f32> {
    match knob {
        Knob::Temperature | Knob::Humidity => 0.0..=1.0,
        Knob::Caves => 0.0..=tesserae_core::climate::MAX_CAVE_SCALE,
    }
}

pub(super) fn knob_step(knob: Knob) -> f32 {
    match knob {
        Knob::Temperature | Knob::Humidity => 0.01,
        Knob::Caves => 0.1,
    }
}

/// Sets the selected Biome's value, on its slider's step.
pub(super) fn set(draft: &mut BiomesDraft, knob: Knob, value: f32) {
    let Some(choice) = draft.choices.get_mut(draft.selected) else {
        return;
    };
    let step = knob_step(knob);
    let range = knob_range(knob);
    let value = ((value / step).round() * step).clamp(*range.start(), *range.end());
    // Rounded again, so 0.35 is stored as 0.35 and not as 0.35000002.
    let value = (value * 100.0).round() / 100.0;
    match knob {
        Knob::Temperature => choice.temperature = value,
        Knob::Humidity => choice.humidity = value,
        Knob::Caves => choice.cave_scale = Some(value),
    }
}

/// The next of the selection's solid Materials for one ground layer; a Biome
/// still on the World's ground starts from it.
pub(super) fn cycle(draft: &mut BiomesDraft, layer: Layer) {
    let materials = &draft.declared.materials;
    let fallback = draft.declared.ground.clone();
    let Some(choice) = draft.choices.get_mut(draft.selected) else {
        return;
    };
    let Some(mut ground) = choice.ground.clone().or(fallback) else {
        return;
    };
    let slot = match layer {
        Layer::Rock => &mut ground.rock,
        Layer::Soil => &mut ground.soil,
        Layer::Surface => &mut ground.surface,
    };
    let at = materials.iter().position(|m| m == slot);
    if let Some(next) = materials.get(at.map_or(0, |i| (i + 1) % materials.len())) {
        *slot = next.clone();
        choice.ground = Some(ground);
    }
}

/// Why a step cannot be left forward, if it cannot.
pub(super) fn check(step: Step, form: &Form, installed: &Installed) -> Result<(), String> {
    match step {
        Step::Game => {
            let installed_game = form
                .game
                .as_ref()
                .is_some_and(|id| installed.games.iter().any(|g| &g.id == id));
            if installed_game {
                Ok(())
            } else {
                Err(NO_GAME.into())
            }
        }
        Step::World => validate(form).and_then(|_| {
            form.climate
                .as_ref()
                .and_then(ClimateDraft::problem)
                .map_or(Ok(()), Err)
        }),
        Step::Biomes => form
            .biomes
            .as_ref()
            .and_then(BiomesDraft::problem)
            .map_or(Ok(()), Err),
        Step::Mods | Step::Review => Ok(()),
    }
}

impl Form {
    /// Equal but for a problem shown on it.
    pub(super) fn unchanged_from(&self, initial: &Form) -> bool {
        // Reading the Game fills in its Climate sizes; only typing over them is a change.
        let climate = |f: &Form| {
            f.climate
                .as_ref()
                .filter(|c| c.sizes != ClimateDraft::new(c.declared).sizes)
                .cloned()
        };
        let bare = |f: &Form| Form {
            problem: None,
            climate: climate(f),
            ..f.clone()
        };
        bare(self) == bare(initial)
    }
}

/// Why a World name was refused; the limit lives here rather than in the hint.
pub(super) fn too_long() -> String {
    format!(
        "Use a World name of {} characters or fewer.",
        library::MAX_NAME_CHARS
    )
}

pub(super) fn validate(form: &Form) -> Result<(String, u64), String> {
    let name = library::valid_name(&form.name).map_err(|_| too_long())?;
    Ok((name, form.seed()?))
}

/// The chosen Game and the Mods it requires: what its Generator needs to run.
pub(super) fn game_ids(form: &Form, installed: &Installed) -> Vec<String> {
    let Some(game) = &form.game else {
        return Vec::new();
    };
    std::iter::once(game.clone())
        .chain(installed.closure_for(game, &Default::default()))
        .collect()
}

/// The Game and every Mod, dependencies included, a World made from this form would load.
pub(super) fn chosen_ids(form: &Form, installed: &Installed) -> Vec<String> {
    let Some(game) = &form.game else {
        return Vec::new();
    };
    std::iter::once(game.clone())
        .chain(installed.closure_for(game, &form.mods))
        .collect()
}

pub(super) fn create(
    l: &mut Launcher,
    installed: &Installed,
    worlds: &Path,
    relaunch: &mut Switch,
) {
    let Some(game) = l
        .form
        .game
        .as_ref()
        .and_then(|id| installed.games.iter().find(|g| &g.id == id))
        .cloned()
    else {
        l.form.problem = Some(NO_GAME.into());
        l.show(Screen::Create(Step::Game));
        return;
    };
    let missing = installed.missing(&game.id);
    if !missing.is_empty() {
        l.form.problem = Some(format!(
            "{} needs {}, which {} not installed.",
            ui::title(&game.id),
            missing.join(", "),
            if missing.len() == 1 { "is" } else { "are" }
        ));
        l.show(Screen::Create(Step::Game));
        return;
    }
    let mods: Vec<_> = installed
        .closure_for(&game.id, &l.form.mods)
        .iter()
        .filter_map(|id| installed.mods.iter().find(|m| &m.id == id))
        .map(|m| library::ModRef {
            id: m.id.clone(),
            version: m.version.clone(),
        })
        .collect();
    let (name, seed) = match validate(&l.form) {
        Ok(valid) => valid,
        Err(e) => {
            l.form.problem = Some(e);
            l.show(Screen::Create(Step::World));
            return;
        }
    };
    let now = library::now_unix_s();
    let sidecar = Sidecar {
        name: name.clone(),
        game: Some(game),
        mods,
        seed,
        voxel_size_m: l.form.voxel_size_m,
        created_unix_s: now,
        last_played_unix_s: now,
        world: tesserae_core::save::WorldId::new(),
        pending: true,
        game_mode: l.form.game_mode,
        biomes: l.form.edits(),
        player_mode: None,
        mode_changes: Default::default(),
    };
    match library::create(worlds, &sidecar) {
        Ok(path) => {
            if let Some((_, source)) = &l.terrain {
                saved::draw(path.clone(), std::sync::Arc::clone(source), &sidecar);
            }
            relaunch.0 = Some(Target::World(path));
            l.opening_game = sidecar.game.as_ref().map(|g| game_title(Some(g)));
            l.show(Screen::Opening(name));
        }
        Err(e) => {
            l.form.problem = Some(format!("Could not create the World folder: {e}"));
            l.show(Screen::Create(Step::World));
        }
    }
}

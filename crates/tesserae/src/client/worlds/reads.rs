//! Create World's reads of what the Game and Mods declare, and its clash check. Each loads and
//! runs Mod code for a third of a second or more, so each runs off the main thread: a step shows
//! the moment it is chosen and only the part waiting on a read says so.

use std::collections::{BTreeMap, BTreeSet};

use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, futures::check_ready};

use super::*;
use crate::launch::Clash;

type Declared = Result<BiomePreview, String>;

/// The Game and picked Mods a clash check is for.
pub(super) type ClashKey = (String, BTreeSet<String>);

struct Job<K, T> {
    key: K,
    task: Task<T>,
}

#[derive(Resource, Default)]
pub(super) struct Reads {
    game: Option<Job<Vec<String>, Declared>>,
    biomes: Option<Job<Vec<String>, Declared>>,
    clashes: Option<Job<ClashKey, BTreeMap<String, Clash>>>,
    /// Lands each read in the frame that starts it, so a test needs no waiting loop.
    pub(super) wait: bool,
}

impl Reads {
    #[cfg(test)]
    pub(super) fn busy(&self) -> bool {
        self.game.is_some() || self.biomes.is_some() || self.clashes.is_some()
    }
}

/// The Game and required Mods to read for the World step's Climate sizes and terrain preview.
fn wanted_game(l: &Launcher, installed: &Installed) -> Option<Vec<String>> {
    let Screen::Create(step) = l.screen else {
        return None;
    };
    (step > Step::Game && l.form.game.is_some() && l.form.climate.is_none())
        .then(|| create::game_ids(&l.form, installed))
}

/// The Game and Mods whose Biomes the Biomes step and Review list. `go` passed their Trust and
/// cleared a draft read for other ones.
fn wanted_biomes(l: &Launcher, installed: &Installed) -> Option<Vec<String>> {
    let Screen::Create(step) = l.screen else {
        return None;
    };
    (step >= Step::Biomes && l.form.biomes.is_none())
        .then(|| create::chosen_ids(&l.form, installed))
}

pub(super) fn clash_key(form: &Form) -> Option<ClashKey> {
    Some((form.game.clone()?, form.mods.clone()))
}

fn wanted_clashes(l: &Launcher) -> Option<ClashKey> {
    if !matches!(l.screen, Screen::Create(_)) {
        return None;
    }
    clash_key(&l.form).filter(|key| l.clashes_for.as_ref() != Some(key))
}

/// Whether Create must wait: a World made now would miss the Climate sizes or Biome edits.
pub(super) fn pending(l: &Launcher, installed: &Installed) -> bool {
    wanted_game(l, installed).is_some() || wanted_biomes(l, installed).is_some()
}

/// Starts what the screen wants read and not yet asked for, and lands what has been read.
pub(super) fn run(
    launcher: Option<ResMut<Launcher>>,
    installed: Res<Installed>,
    previewer: Res<Previewer>,
    mut reads: ResMut<Reads>,
    focus: Res<InputFocus>,
    tagged: Query<(Option<&Act>, Option<&Field>)>,
) {
    let Some(mut l) = launcher else {
        return;
    };
    let previewer = *previewer;
    let reads = &mut *reads;
    if let Some(ids) = wanted_game(&l, &installed) {
        start(&mut reads.game, ids, &installed, move |i, ids| {
            (previewer.0)(i, ids)
        });
    }
    if let Some(ids) = wanted_biomes(&l, &installed) {
        start(&mut reads.biomes, ids, &installed, move |i, ids| {
            (previewer.0)(i, ids)
        });
    }
    if let Some(key) = wanted_clashes(&l) {
        start(&mut reads.clashes, key, &installed, |i, (game, picked)| {
            i.clashes(game, picked)
        });
    }
    let wait = reads.wait;
    let mut landed = false;
    if let Some((ids, declared)) = poll(&mut reads.game, wait)
        && l.form.climate.is_none()
        && create::game_ids(&l.form, &installed) == ids
    {
        create::game_read(&mut l, ids, declared);
        landed = true;
    }
    if let Some((ids, declared)) = poll(&mut reads.biomes, wait)
        && l.form.biomes.is_none()
        && create::chosen_ids(&l.form, &installed) == ids
    {
        create::biomes_read(&mut l, ids, declared);
        landed = true;
    }
    if let Some((key, clashes)) = poll(&mut reads.clashes, wait)
        && clash_key(&l.form).as_ref() == Some(&key)
    {
        l.clashes = clashes;
        l.clashes_for = Some(key);
        landed |= matches!(l.screen, Screen::Create(Step::Mods));
    }
    if landed {
        // The page rebuilds around what was read; whatever the player had focused keeps it.
        if let Some((act, field)) = focus.get().and_then(|e| tagged.get(e).ok()) {
            l.refocus = act.cloned().or(l.refocus.take());
            l.refield = field.copied();
        }
        l.version += 1;
    }
}

/// Starts a job for `key` unless one for it is already running; one for another key is dropped,
/// as its answer would be thrown away.
fn start<K, T>(
    job: &mut Option<Job<K, T>>,
    key: K,
    installed: &Installed,
    work: impl FnOnce(&Installed, &K) -> T + Send + 'static,
) where
    K: Clone + PartialEq + Send + 'static,
    T: Send + 'static,
{
    if job.as_ref().is_some_and(|j| j.key == key) {
        return;
    }
    let installed = installed.clone();
    let for_task = key.clone();
    let task = AsyncComputeTaskPool::get().spawn(async move { work(&installed, &for_task) });
    *job = Some(Job { key, task });
}

fn poll<K, T>(job: &mut Option<Job<K, T>>, wait: bool) -> Option<(K, T)> {
    let running = job.as_mut()?;
    let done = if wait {
        Some(block_on(&mut running.task))
    } else {
        check_ready(&mut running.task)
    }?;
    job.take().map(|j| (j.key, done))
}

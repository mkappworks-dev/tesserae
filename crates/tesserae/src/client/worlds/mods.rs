//! The Mods step's list: which Mods it shows and in what order, and ticking them all at once.

use std::collections::BTreeSet;

use super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::client) enum ModSort {
    #[default]
    Name,
    NameDescending,
    Category,
    SelectedFirst,
}

impl ModSort {
    pub(super) const ALL: [ModSort; 4] = [
        ModSort::Name,
        ModSort::NameDescending,
        ModSort::Category,
        ModSort::SelectedFirst,
    ];

    pub(super) fn label(self) -> &'static str {
        match self {
            ModSort::Name => "Name A\u{2013}Z",
            ModSort::NameDescending => "Name Z\u{2013}A",
            ModSort::Category => "Category",
            ModSort::SelectedFirst => "Selected first",
        }
    }
}

/// A popup beside the Mods search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::client) enum Pop {
    Filter,
    Sort,
}

/// How the Mods step narrows and orders its list. The search lives in `Launcher::mod_search`,
/// since typing filters in place without a rebuild.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct ModView {
    /// Empty shows every category.
    pub(super) categories: BTreeSet<String>,
    pub(super) selected_only: bool,
    pub(super) sort: ModSort,
    pub(super) pop: Option<Pop>,
}

impl ModView {
    /// What the filter icon's badge counts.
    pub(super) fn filters(&self) -> usize {
        self.categories.len() + usize::from(self.selected_only)
    }
}

/// The select-all box: none, some or every Mod shown is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Bulk {
    None,
    Some,
    All,
}

pub(super) fn category<'a>(installed: &'a Installed, id: &str) -> &'a str {
    installed.about(id).0.unwrap_or(pages::OTHER_SECTION)
}

/// What the search matches a Mod on.
pub(super) fn words(installed: &Installed, id: &str) -> String {
    format!(
        "{} {}",
        ui::title(id),
        installed.about(id).1.unwrap_or_default()
    )
}

/// The Mods a World made now would carry: the picks, what they need and what the Game requires.
pub(super) fn carried(l: &Launcher, installed: &Installed) -> BTreeSet<String> {
    let game = l.form.game.clone().unwrap_or_default();
    installed
        .closure_for(&game, &l.form.mods)
        .into_iter()
        .collect()
}

/// The Mods the filter keeps, as indexes into `installed.mods`, in the chosen order. The search
/// is applied on top, in place.
pub(super) fn listed(l: &Launcher, installed: &Installed) -> Vec<usize> {
    let view = &l.mod_view;
    let on = carried(l, installed);
    let mut list: Vec<usize> = (0..installed.mods.len())
        .filter(|&i| {
            let id = &installed.mods[i].id;
            (view.categories.is_empty() || view.categories.contains(category(installed, id)))
                && (!view.selected_only || on.contains(id))
        })
        .collect();
    let name = |i: &usize| ui::title(&installed.mods[*i].id);
    let sections: Vec<String> = pages::sections(installed, &[])
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    let rank = |i: &usize| {
        let c = category(installed, &installed.mods[*i].id);
        sections.iter().position(|s| s == c).unwrap_or(usize::MAX)
    };
    match view.sort {
        ModSort::Name => list.sort_by_key(name),
        ModSort::NameDescending => list.sort_by_key(|i| std::cmp::Reverse(name(i))),
        ModSort::Category => list.sort_by_key(|i| (rank(i), name(i))),
        ModSort::SelectedFirst => {
            list.sort_by_key(|i| (!on.contains(&installed.mods[*i].id), name(i)))
        }
    }
    list
}

/// [`listed`] less what the search hides: what select-all acts on.
pub(super) fn shown(l: &Launcher, installed: &Installed) -> Vec<usize> {
    listed(l, installed)
        .into_iter()
        .filter(|&i| ui::matches(&l.mod_search, &words(installed, &installed.mods[i].id)))
        .collect()
}

/// Whether ticking `id` by itself would add it, rather than being refused or swapping a pick out.
fn addable(l: &Launcher, installed: &Installed, game: &str, id: &str) -> bool {
    installed.blocked(game, id).is_none() && !l.clashes.contains_key(id)
}

/// The select-all box over the Mods shown, and how many of them are on.
pub(super) fn bulk(l: &Launcher, installed: &Installed) -> (Bulk, usize, usize) {
    let game = l.form.game.clone().unwrap_or_default();
    let on = carried(l, installed);
    let shown = shown(l, installed);
    let ids = shown.iter().map(|&i| installed.mods[i].id.as_str());
    let chosen = ids.clone().filter(|id| on.contains(*id)).count();
    let full = ids
        .clone()
        .all(|id| on.contains(id) || !addable(l, installed, &game, id));
    let state = match chosen {
        0 => Bulk::None,
        _ if full => Bulk::All,
        _ => Bulk::Some,
    };
    (state, chosen, shown.len())
}

/// The acts on the Mods step's search line, its popups and its select-all box.
pub(super) fn on_mods(l: &mut Launcher, installed: &Installed, act: Act) {
    if l.screen != Screen::Create(Step::Mods) {
        return;
    }
    let mut refocus = act.clone();
    match act {
        Act::ModPop(pop) if l.mod_view.pop == Some(pop) => l.mod_view.pop = None,
        Act::ModPop(pop) => {
            l.mod_view.pop = Some(pop);
            refocus = first_in(pop, l, installed);
        }
        Act::ModCategory(name) => {
            if !l.mod_view.categories.remove(&name) {
                l.mod_view.categories.insert(name);
            }
        }
        Act::ModSelectedOnly => l.mod_view.selected_only = !l.mod_view.selected_only,
        Act::ModFilterReset => {
            l.mod_view.categories.clear();
            l.mod_view.selected_only = false;
            refocus = first_in(Pop::Filter, l, installed);
        }
        Act::ModSort(sort) => {
            l.mod_view.sort = sort;
            l.mod_view.pop = None;
            refocus = Act::ModPop(Pop::Sort);
        }
        Act::SelectShownMods => select_shown(l, installed),
        _ => return,
    }
    l.refocus = Some(refocus);
    l.version += 1;
}

/// The control a popup focuses when it opens.
fn first_in(pop: Pop, l: &Launcher, installed: &Installed) -> Act {
    match pop {
        Pop::Filter => pages::sections(installed, &[])
            .into_iter()
            .next()
            .map_or(Act::ModSelectedOnly, |(name, _)| Act::ModCategory(name)),
        Pop::Sort => Act::ModSort(l.mod_view.sort),
    }
}

/// Ticks every Mod shown, or clears them when all already are. A Mod that clashes or cannot be
/// added is left as it is, as ticking it alone would refuse or swap; required Mods stay on.
fn select_shown(l: &mut Launcher, installed: &Installed) {
    let Some(game) = l.form.game.clone() else {
        return;
    };
    if l.clashes_for != reads::clash_key(&l.form) {
        create::clashes_now(l, installed);
    }
    let ids: Vec<String> = shown(l, installed)
        .into_iter()
        .map(|i| installed.mods[i].id.clone())
        .collect();
    if bulk(l, installed).0 == Bulk::All {
        for id in &ids {
            l.form.mods.remove(id);
        }
    } else {
        for id in ids {
            let needed = installed.closure_for(&game, &l.form.mods);
            if !needed.contains(&id) && addable(l, installed, &game, &id) {
                l.form.mods.insert(id);
                create::clashes_now(l, installed);
            }
        }
    }
    create::clashes_now(l, installed);
}

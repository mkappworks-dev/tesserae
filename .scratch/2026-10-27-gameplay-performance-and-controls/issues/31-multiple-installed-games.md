# 31: G0: Multiple installed Games and per-World Mod sets

Status: done
Blocked by: none (27 and 30 are done)

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: G0, one PR of three commits that each build and pass tests;
keep it separate from other PR scopes. The implementer may split along the
commit lines only if the diff proves too large to review.

## What to build

Let several Games be installed at once and let each World choose its Game and
the Mods enabled for it, as Luanti does (`games/<id>/`, per-World mod
selection) and Minecraft does with per-World data packs. Today the Mods
directory permits a single root (`ModError::SecondRoot`), so W0's Create form
offers only that Game, and `load_world` loads every installed Mod for every
World, so installing an unrelated Mod changes the required content of every
existing World.

### Layout and migration (commit 1)

- Sibling `games/<id>/` and `mods/<id>/` under the user data directory, one
  `mod.toml` format and one loader. `root = true` is valid only under
  `games/`; a non-root manifest is valid only under `mods/`. A mismatch is a
  `ModError`. One id names one thing across both folders (`DuplicateId`).
- `discover` returns what it found plus per-folder failures: a malformed
  manifest, a duplicate id, a root under `mods/` or a symlinked directory is
  an error row for that folder and never hides the rest (today one bad
  `mod.toml` fails the whole scan and the list shows no Game). Cap the scan
  at 256 entries and report the overflow.
- The scan only reads `mod.toml` and stats the component. It never hashes,
  compiles or runs guest code. `component_path()` keeps rejecting a `wasm`
  path that resolves outside its folder. Hashing and the Trust prompt happen
  at open or Create, for the chosen set only.
- Migration: on launch, move a legacy `mods/<id>/` whose manifest has
  `root = true` into `games/<id>/`. Idempotent, logs one line, refuses on an
  existing `games/<id>`, and touches no other folder. `worlds/` and
  `trust.json` stay where they are; Trust is keyed by id and hash, so grants
  survive. After the move a root under `mods/` is the commit-1 error.
- Update the dev demo path (`games/demo/dist`), `default_mods_dir` users and
  the docs that name the single Mods folder.

### Selection and refusal (commit 2)

- A Game is identified by its manifest `id`; `version` and the build hash are
  recorded in the save as today. Two versions of one id cannot be installed.
- The World's Mod set is chosen at Create, includes the dependency closure
  (`plan()` order), and is frozen for that World. The sidecar gains
  `mods: [{id, version}]` beside `game`; an older `world.json` without it
  keeps its meaning. The commit's `RequiredMod` list stays authoritative.
- The chosen set travels in `Opening` and is the only input that selects what
  `load_world` loads. `loads_mods()` becomes "load exactly what the record
  names, from `games/` and `mods/`". A World saved without a Game still loads
  no Mods. A pre-G0 commit that lists the Mods it loaded is its own set.
- A World opens only if every required Game and Mod is installed with the
  exact version and build hash (`commit.compatible`). Otherwise it is refused
  before anything is written, never opened without the Mod and never treated
  as a new World. The refusal and the World list name each missing or
  changed item by id and version; Open is disabled. Trust is still asked per
  Mod at open, for the Mods the World names.
- No new cross-session state: no retained `Engine`, compiled components or
  manifest list beyond what `Installed` holds. `release_mods` is unchanged.
- Content ownership is unchanged and proved by tests: ground, World
  parameters, movement, character and tools come only from the root Game of
  the opened World; a non-root Mod may add Materials, appearance and
  textures within the existing per-Mod caps. The Engine ships only Air
  (ADR-0016).

### Create form (commit 3)

- One screen: a Game picker (a single choice from `games/`, with version and
  Trust state) and a Mod list with toggles. Enabling a Mod enables its
  `depends`, shown locked. A Mod whose `depends` cannot be satisfied is shown
  disabled with the reason. Mods start unticked. Changing the Game does not
  change the Mod pool. No Game installed still blocks Create. Error rows
  from the scan are shown, not hidden.
- Reuse the form's existing pad, keyboard and touch focus path: one focusable
  row per Game or Mod, no new input mechanism. Screenshots are for review
  only: one `/tmp` folder, never committed, deleted when review is done.

## Acceptance criteria

- [x] Two Games installed; each World is created with and reopens under its
      own Game and Mod set.
- [x] Worlds with different sets coexist in one process; opening World A then
      World B shows only B's Materials, textures and ground, and a Mod enabled
      in A but not B leaves no name in B's Registry.
- [x] Installing or removing an unrelated Mod does not change any existing
      World's required content or stop it opening.
- [x] A missing, reversioned or rebuilt Game or Mod refuses before any write
      and names each unmet item; no "open anyway".
- [x] One bad folder is an error row and does not hide the others; symlinked
      directories and escaping `wasm` paths are rejected; the scan runs no
      guest code and respects the cap.
- [x] Migration is idempotent, refuses on a clash and leaves worlds and trust
      intact.
- [x] The Create form lists Games and Mods and is fully operable with
      keyboard, controller and touch.
- [x] The soak harness (`TESSERAE_SOAK_CYCLES`) gains a mode that alternates
      two Games; counts do not grow, and RSS growth is no worse than ticket
      34's baseline.
- [x] The 0.3 WIT contract, the manifest fields and ADR-0001's batched
      boundary are unchanged. An ADR ("Games and Mods are installed side by
      side; a World freezes its set") and `docs/world-persistence.md` and
      `CONTEXT.md` updates land in this PR.

## Non-goals

No Mod download manager, store, auto-install or in-app install: the user
places folders by hand. Not in this ticket: editing a World's Mod set after
Create (needs a Registry migration story), Game-private Mods, side-by-side
versions of one id, a "works with Game X" manifest field, "open anyway" or
tombstones for missing names (ADR-0016 keeps those separate), and Windows or
Linux data-directory verification beyond path logic.

## Notes

Raised during ticket 27's 2026-10-29 grilling (Q24). Ticket 34 (session
memory growth) may change how a Mod instance or the wasmtime `Engine` is
built and dropped (`ModHost::new()` runs per load). G0 must not touch that;
if 34 shares an `Engine` across sessions, it must hold for any selected set.

## Comments

### Grilled 2026-10-31

Scope agreed one question at a time: layout, migration, World record,
refusal policy, Create form, untrusted folders, session plug-in, content
ownership, WIT, non-goals and the single PR with three commits.

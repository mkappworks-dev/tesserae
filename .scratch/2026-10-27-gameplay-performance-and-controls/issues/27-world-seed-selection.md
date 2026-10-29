# 27: W0: World list, creation and seed selection

Status: done
Blocked by: none (06, 22, 23 and 24 are merged on main 96ea33e)

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: W0, ticket 27 only. Title:
`feat(world): World list, creation and seed selection`.

## What to build

Launch opens a World list instead of a World. It lists, creates, re-creates,
opens, renames and deletes Worlds. Opening or creating re-execs the process on
the chosen World directory; the process boundary is what keeps old-World jobs
out of the new World. The next process inherits window size, position and
fullscreen, and shows a loading screen at once. In-process switching is the
long-term target, ticket 29 (W1). `TESSERAE_WORLD_DIR`, `TESSERAE_EPHEMERAL`,
profile fixtures and screenshots bypass the list exactly as today and keep
print-and-exit on failure.

### Ownership

- World list, Create form, Rename/Delete UI: Engine, client front-end. It
  exists before any Game runs; Mods only live inside a World through WIT.
- World library (list, folder naming, sidecar, rename, delete): Engine,
  std-only persistence module beside `core::save`; the client UI only calls
  it. Rename and Delete try-lock `writer.lock` and refuse with "open in
  another Tesserae" when held, so a running World is never changed under it.
- Seed parsing: pure `tesserae-core` function, reusable by a future dedicated
  server's config.
- First commit, `last_played` and the recovered-save notice: Server, the
  sole writer of World saves (ADR-0002).
- Trust store and checks: Engine host policy; the dialog is client UI.
- Out of scope: Game-declared World options (game mode, difficulty) through
  future declarations; showing the seed in-World (Server policy, as
  multiplayer servers may hide it); multiple installed Games, ticket 30.

### Storage

Each World is a folder under `worlds/` beside the Mods directory, named at
creation from a slug of the World name with a collision suffix (`My World`,
`My World (2)`). Sanitize for every OS: Windows reserved names, trailing dots
and spaces, case-insensitive collisions. The folder is never renamed. Identity
stays the commit's UUID; the list is keyed by path. A copied folder carries a
duplicate UUID; document that this is harmless because nothing global is keyed
by World UUID.

An atomically written sidecar `world.json` holds name, Game id/version,
resolved seed, Voxel size, created and last-played times, the UUID and a
`pending` flag. Commits stay authoritative: Open cross-checks the sidecar UUID
and takes seed, Voxel size and content from the commit. `Commit` and its
format version 1 do not change. A missing/corrupt sidecar never hides a World:
it lists as "Unnamed" with its UUID and Rename repairs it. The existing
`worlds/default` is adopted as "Default World" with a generated sidecar on
first listing.

### Create and Re-Create

Fields: name, Game, seed (Randomize button), and a collapsed "More World
options" section holding Voxel size. Re-Create opens the same form prefilled
from a World's seed, Game and Voxel size, named "<name> (copy)".

Create validates Game discovery and Trust before writing anything, then writes
the folder and sidecar with `pending: true` and the new UUID, then re-execs. A
folder without commits takes UUID and seed from its sidecar, never
`WorldId::new()`, writes its first commit immediately and then clears
`pending`. If creation still fails in the new process, the folder is kept and
lists as Incomplete with Retry and Delete; nothing is deleted automatically.

- Names: trimmed, 1–64 characters, any Unicode, duplicates allowed. Blank
  defaults to "New World".
- Game: the Mods directory's single root Game (manifest id and version) or
  Empty World. No root: only Empty World, with a note. Discovery error (e.g.
  `SecondRoot`): show it; only Empty World can be created.
- Empty World loads no Mods: built-in registry, no-Game free-flight fallback,
  empty required content. Non-root Mods are not loaded on their own.
- Trust: an untrusted Game is tagged "needs Trust". Create/Open shows an
  in-UI Trust dialog with the existing `trust_prompt` text and hash before any
  Mod code runs: Trust and continue, or Cancel. It writes the same trust file;
  `--trust-all` and `TESSERAE_TRUST_MODS` still work.
- Voxel size: 1.0, 0.5 or 0.25 m (the tested `DEV_VOXEL_SIZES`), default 1.0,
  each labelled with its cost, e.g. "0.25 m: 4× finer detail, ¼ view
  distance". Shown read-only in World details. Reopen always uses the saved
  size, never the environment (ADR-0003). Game-restricted sizes need a
  declaration/contract change and are separate work.

### Seed

A pure `tesserae-core` function, pinned by golden vectors:

- Trim whitespace. Blank: random `u64` from the OS random source.
- ASCII digits within `u64`: that value.
- `-` then ASCII digits within `i64`: its two's-complement bits as `u64`
  (`-1` is `18446744073709551615`).
- Digits, signed or not, outside both ranges: rejected with a message, never
  hashed.
- Anything else: a word. UTF-8, case-sensitive, not normalized; first 8 bytes
  of SHA-256, big-endian `u64`.

Create and World details always show the canonical decimal `u64` with Copy
(use Bevy 0.20's clipboard if it has one, else `arboard`). The seed is
read-only after creation. Benchmark/fixture seeds stay hard-coded;
`WorldSettings::default().seed` is used only by ephemeral/fixture runs, never
by Create.

### Generation

Use the existing generator. Change `fbm2`'s `seed + octave` to
`wrapping_add`, matching `fbm3`, in its own commit. `GENERATOR_VERSION` stays
1: non-overflowing seeds produce identical terrain and release already
wrapped. Document that the `u64`→`u32` fold lets distinct seeds share terrain
(e.g. `0x0000_0001_0000_0001` equals 0), which the spec allows.

### Open, failures and journeys

List rows read sidecars only; full validation happens on Open. Corrupt or
newer commits, incompatible versions, missing content, lost Trust and a
writer lock held by another Tesserae return to the list with an error card:
World name, reason and any recovered-older-commit diagnostics. Nothing is
written. A successful open updates `last_played`. When Open falls back to an
older commit, the World shows a one-time banner, e.g. "Opened the save from
14:02; a newer save was damaged".

The in-World menu keeps Save & Quit (to desktop) and adds Return to World
list beside it, through the same guarded save: Retry, Return and confirmed
unsafe exit. Window close keeps save-then-exit.

### List and Delete

Sorted by last played, newest first; rows show name, Game or "Empty World",
seed and last played. No Worlds: Create is first. Actions: Open, Create,
Re-Create, Rename, Delete, Show World folder (Finder or the platform file
manager), Quit. Duplicate, backup and import are out of scope.

Delete is only in the World's details/action row, never a bare keypress. The
confirmation names the World, its Game and last played; Cancel has default
focus, Delete is styled destructive. The folder moves to the OS Trash (`trash`
crate, pending cargo-deny), never a hard delete; a Trash failure shows an
error and leaves the files. No type-the-name step.

### Input and layout

Mouse and keyboard only: Tab/Shift-Tab focus, Enter activates, Escape is
Back/Cancel. Name and seed fields use `Owner::TextEntry`. Tab handling stays
local to the World screens so ticket 07 can generalize it. Sizes follow
`UiScale`. Controller and touch journeys for these screens belong to tickets
07 and 08–10.

No Mod download manager, multi-Game install layout, in-process World
switching or new terrain/biome algorithm.

## Acceptance criteria

- [x] Create → Open → Rename → reopen preserves UUID, seed, Voxel size, edits
      and Player state; Rename touches only the sidecar.
- [x] Seed conversion golden vectors: blank/random, unsigned and signed
      decimal, `u64::MAX`, `i64::MIN`, out-of-range rejection, words, Unicode.
      The canonical seed is shown and copyable.
- [x] `fbm2` boundary sweep (0, 1, `u32::MAX - n`, `u64::MAX`,
      fold-cancelling values) does not panic in debug; golden terrain digests
      for existing seeds are unchanged.
- [x] Two Worlds with the same name and seed get distinct UUIDs and folders;
      editing one never changes the other. Folder names are sanitized and
      suffixed on collision, including case-only collisions.
- [x] Corrupt, incompatible, missing-content, untrusted and locked Worlds land
      on the list with an explanation; their saves are byte-identical after.
      Recovery from an older commit shows the in-World banner.
- [x] Delete moves to the Trash after a Cancel-default confirmation; Rename
      and Delete are refused while the writer lock is held. An Incomplete
      World offers Retry and Delete.
- [x] Worlds created at 0.25 and 0.5 m reopen at that size regardless of
      environment; Re-Create copies seed, Game and Voxel size.
- [x] Re-exec keeps window size, and position once the window has moved; a
      World opened from the list shows a loading screen once its window
      exists. (Narrowed 2026-10-29: the rest moved to tickets 29 and 31.)
- [x] Mouse and keyboard controls lay out at 1024×640 and 1920×1080 under
      `UiScale`; Escape and activation are covered by tests; native captures
      in one /tmp folder for user review. (Narrowed 2026-10-29: native Tab
      and Enter checks moved to ticket 31.)
- [x] Return to World list reuses the guarded Save & Quit, including Retry,
      Return and confirmed unsafe exit.

## Validation and authorization

Use behavior/intent regressions and the spec's build/review requirements.
Preserve shared Bevy artifacts, Server/core/WIT boundaries and count bounds.
Native captures/measurements stay in one unique /tmp folder until review.
Scope confirmed by the user in the 2026-10-29 grilling session below.
Implementation, push, publication and merge need separate user instruction.

## Comments

### Grilling 2026-10-29 (Q1–Q24, all accepted)

Supersedes the earlier draft notes and the "accepted focus/touch/controller
operations" criterion. Q1–Q18 set the first design; a comparison with
Minecraft and Luanti then revised it. Re-exec stays for W0 with window
carry-over, and in-process sessions move to ticket 29 (Q19). Signed seeds are
accepted (Q20). Folders get readable names instead of UUIDs, plus Show World
folder (Q21). Voxel size is exposed under More World options (Q22).
Re-Create and the recovered-save banner are added (Q23). The ownership split
and the writer-lock rule for Rename/Delete are adopted, and multi-Game
installs move to ticket 30 (Q24).

### Seams with parallel worktrees

- Character/cameras (25–26): `client/menu.rs` header (Return to World list
  sits beside Save & Quit) and the Player `movement_v1` extension. This ticket
  does not change `Commit`; the menu header is the likely textual conflict.
- Controller (07): `client/input.rs`, `actions.rs` and menu focus. This
  ticket only uses `Owner::TextEntry` and keeps Tab handling local.
- Only this ticket should change `main.rs` startup order, window creation and
  `server/persistence.rs` `Opening`. Whichever branch merges second rebases;
  no pre-merging of the others' work.

### Implementation 2026-10-29

Implemented on `feat/world-seed-selection` (28a782f..48ba6d8); see
[validation](../world-selection-validation.md). Full cargo-ci passed. The
re-exec criterion stays open: size always carries, position only after the
window has moved, native macOS fullscreen not at all, and the loading
screen appears only once the World process has a window, after Mod
loading. The keyboard criterion stays open too: tests drive Escape and
activation, but Tab and Enter were not pressed natively, as no input
automation is available. Return to World list, Copy and Show folder are
covered by tests, not native clicks. User visual review and Windows/Linux
runs are pending. No push, PR or merge.

### Criteria narrowed 2026-10-29

The user chose to keep this ticket done and narrow the two open criteria.
A loading screen before Mod loading moves to ticket 29, which restructures
startup anyway. Position before the first move, native fullscreen and the
native Tab/Enter check move to ticket 31.

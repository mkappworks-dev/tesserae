# 06: Watch data and safely restart a development World

Status: done

Blocked by: 03

## What to build

One development command opens a clearly disposable World and watches the
selected local Game and Mods, including their referenced PNGs. Valid content
changes restart a fresh World with the same seed; invalid changes leave the
current World running and explain what to fix.

Use the existing content preparation and validation boundary. The loop can
operate with the capabilities available after ticket 03; full Declaration parity
from ticket 02 is not a prerequisite for the watcher lifecycle.

## Acceptance criteria

- [ ] The command clearly identifies the World as disposable and limits its
      development Trust grants to selected local content for the current run,
      without writing permanent Trust records.
- [ ] Watch the selected Game and selected local Mods, their Manifests,
      Declarations and referenced artwork. Unselected local projects do not
      cause development restarts.
- [ ] Installed dependencies can load normally without enabling automatic
      builds. This ticket does not execute source build commands.
- [ ] Prepare and validate a consistent candidate before closing the current
      session. Malformed data, missing or invalid artwork and content rejection
      keep the current World running and report useful diagnostics.
- [ ] A subsequent valid edit recovers without manually restarting the
      development command; bursts of file updates do not install a stale
      candidate after a newer accepted version.
- [ ] A successful replacement uses the same seed, resets the Player to the
      Game's initial spawn, and discards development Edits and simulation
      progress rather than migrating them.
- [ ] Each replacement uses normal session teardown so old jobs, Mod instances
      and session content do not survive into the new World.
- [ ] A failure during replacement startup returns to a useful error state.
      Saved Worlds and their exact-build compatibility rules remain untouched.
- [ ] Behaviour tests verify successful restarts, invalid edits, recovery,
      selected dependencies, artwork changes, seed and spawn, Trust persistence,
      session cleanup and saved-World isolation.

## Verification

Exercise observable lifecycle outcomes through existing Server and session
boundaries. Manually edit a small data Game and PNG in the running app and
record the restart and failure-recovery results when completing this ticket.

## Completion notes

`tesserae dev <project>... [--mods <dir>] [--seed <n>]` (`crates/tesserae/src/dev.rs`)
reuses `author::check` for validation and `server::mods::start_all` for
starting. Lifecycle behaviour is covered by the `dev::tests` suite, which drives
the real Server and session plugins headlessly. The windowed manual edit of a
data Game and PNG was not run by the implementing agent (no display); it is
still to be recorded by a human.

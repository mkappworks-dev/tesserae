# Show it, decide `lightbox`, write it down

Status: ready-for-agent
Blocked by: 02, 05

## What to build

- Find a mouth the `dev` camera reaches (the camera starts about 20 m above
  spawn tipped down 17 degrees; the seed is fixed, so this is a search, not
  luck), take the screenshot with `TESSERAE_SCREENSHOT`, and show it.
- Decide `server/lightbox.rs` deliberately. It exists to make light visible.
  Caves now make sky-light propagation visible, but not an Edit's
  de-propagation (the roof) or block light (the lantern). Keep it, narrow it or
  delete it, and say which and why in the PR.
- CONTEXT.md: **Cave**, if the term is used in code or docs. DESIGN.md: the open
  item on the chunk budget (ticket 05) and a note under step 3 or 4 that caves
  exist and are off the build order. `docs/architecture.md` if the worldgen line
  needs it. `cargo fmt --all` and `npm run fmt`.
- The PR description in full in the session, in a fenced block of four
  backticks, no "Generated with Claude Code" line.

## Acceptance criteria

- The screenshot shows sky light falling off into a cave and Dirt and Stone in
  its walls.
- `./scripts/cargo-ci` passes, including `cargo fmt --all --check` and
  `npm run fmt:check`.
- Every ticket's `Status` is `done`, set in the commit that finished it.

## Comments

# Docs and the PR

Status: done
Blocked by: 03, 04

## What to build

- `CONTEXT.md`: **Distance fog**, in the Space section, as the spec words it. A
  glossary entry: no decision, no implementation.
- `DESIGN.md`: the LOD section gains a short "Fog" bullet (what it is, that it
  is Bevy's `DistanceFog` on all six cameras from one `FogSettings`, derived
  from the reach, the curve that was kept and why in a sentence, that the clear
  colour is the fog colour). The Look packs section's "fog parameters" says they
  are `FogSettings`. Build order step 6's paragraph or Open items carries the
  frame cost figure from ticket 04. Nothing else.
- `docs/architecture.md`: the layout tree gains `render/fog.rs`, and the
  `render/` line says what it holds now.
- The spec's Outcome is complete and its text above is corrected where the build
  disagreed with it. The LOD spec's loose end "No distance fog or atmosphere" is
  left alone: it is a record, and the new spec points back to it.
- No ADR: nothing here is hard to reverse (the spec says so). If the build found
  otherwise, write one.
- Prettier on the `.md` outside `.scratch/` (`npm run fmt`); `.scratch/` is
  ignored by it, so those files are hand-wrapped at 80 columns and never run
  through it.
- Rebase on `main` (or `sync_with_base_branch` if the app offers it), and say in
  the PR what was coordinated around: the four branches in the spec.
- Push the branch. `gh` is not installed: give the compare URL and the PR body
  inline.

## Acceptance criteria

- `./scripts/cargo-ci` passes on the rebased branch.
- `git diff main --stat` has no image and no `target/` path, and the screenshot
  folder in `/tmp/tesserae-fog-review/` is deleted.
- The PR body is filled from `.github/pull_request_template.md` with
  inapplicable items struck through, prose wrapped at 80 columns, and no
  "Generated with Claude Code" line at the end. Title is a Conventional Commit,
  for example `feat(render): distance fog on every LOD camera`.
- The PR says what was measured, what was seen in the captures, and what was
  coordinated around.
- `Status: done` is set in the commit that finishes this ticket.

## Comments

### Done

The docs half is done: `CONTEXT.md` (**Distance fog**), `DESIGN.md` (a Fog
bullet under LOD, `FogSettings` in the Look packs list, the frame cost in step
6), `docs/architecture.md` (`render/fog.rs`), and the spec corrected and its
Outcome completed (ticket 05 section: no ADR and why, the rebase onto `9ec5571`,
and the captures after it, with the one thing not re-measured). The push and the
PR body are the caller's, as are deleting `/tmp/tesserae-fog-review/` and the
acceptance criteria about the PR text.

# Show it, and say what it is

Status: done
Blocked by: 01, 02, 03

## What to build

- `TESSERAE_SCREENSHOT=/tmp/x.png TESSERAE_SCREENSHOT_FLY_S=0 cargo dev` (retry
  once on a black capture; `TESSERAE_SCREENSHOT_OFFSCREEN=1` if the screen is
  locked). Look at the capture: ramps on real hills, no dark seams along stair
  risers, no missing faces, lit sides consistent. Show it.
- DESIGN.md "Shaped voxels": terrain now places stairs and slabs; the cap and
  the budget as numbers.
- `CONTEXT.md`: a term only if the rule needs a name in code (e.g. the step it
  eases); no more than that.
- Spec 5's decision 11 says worldgen places none: leave the historical spec, add
  a pointer to this one in this spec's Outcome.
- Fill in this spec's `## Outcome` with what the measurements corrected, as the
  caves spec did.

## Acceptance criteria

- The screenshot is in the PR description or shown in the session.
- The PR body fills in `.github/pull_request_template.md`, with the before/after
  tables from tickets 04 and 05.
- `npm run fmt` and `cargo fmt --all` have been run.

## Comments

Captured with `TESSERAE_SCREENSHOT_OFFSCREEN=1` (the screen was in use). Same
camera, smoothing on against `TESSERAE_SMOOTHING=0`: every terrace on the hills
gains a half-height tread, the cave mouth is unchanged, the dev gallery is
unchanged, and there are no dark seams along the risers or missing faces. The
two images are in the session, not in the repo. `CONTEXT.md` gains nothing:
the code calls the step "a rise" and the glossary does not need a term for it.

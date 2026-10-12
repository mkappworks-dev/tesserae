# Loaded and meshed counts, this branch against main

Status: ready-for-agent
Blocked by: 06, 07

Closes the one item of ticket 03 that was not done: streaming now follows the
Player, so the same Chunks should fill.

## What to build

Nothing to build. Run `scripts/frame-runs` (the capture harness with
`TESSERAE_SCREENSHOT_OFFSCREEN=1`) on `main` and on this branch, same machine,
same settings, and put `loaded`, `meshed` and `drawn` side by side in the PR and
in this ticket's Comments. A difference is a finding, not a failure to hide.

## Then

Mark ticket 03's "Not done" resolved.

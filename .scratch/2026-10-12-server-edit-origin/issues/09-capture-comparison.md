# Loaded and meshed counts, this branch against main

Status: done
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

## Comments

`TESSERAE_SCREENSHOT_OFFSCREEN=1 scripts/frame-runs 3` on `main` (`f232c4e`) and on
this branch (`e25f8a7`), same machine, back to back, with other builds running on
it, so the timings are noise-limited and the counts are the evidence.

| figure      | main (median, min-max) | branch (median, min-max) |
| ----------- | ---------------------- | ------------------------ |
| loaded      | 2442 (2442-2442)       | 2442 (2442-2442)         |
| meshed      | 910 (910-910)          | 910 (910-910)            |
| drawn       | 335 in every run       | 335 in every run         |
| anchor cell | `0 2 -3` in every run  | `0 2 -3` in every run    |
| fill_s      | 2.97 (2.88-3.28)       | 3.47 (2.77-3.76)         |
| p99_flight  | 117.1 (75.7-133.3) ms  | 54.2 (37.1-123.8) ms     |

The same Chunks load, mesh and draw, around the same cell, so streaming around
the `Player` fills what streaming around the camera did. The fill and frame
times overlap across three runs each and are not a finding in either direction.

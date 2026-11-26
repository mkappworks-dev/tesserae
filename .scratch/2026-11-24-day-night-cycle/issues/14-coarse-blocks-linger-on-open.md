# 14: Coarse blocks linger after a saved World opens

Status: ready-for-agent

Spec: [Day and night cycle](../spec.md).

## What to build

Opening the user's saved World, the view past level 1 shows big coarse-level
blocks for 10 to 15 s before the finer levels replace them. On a fresh World
from the same seed both `main` and this branch had filled in at 4 s, so the
branch does not slow streaming there; the saved World's load path is the
suspect. Find why and fix it, or record that it is expected and why.

Reproduce with ticket 13's World copy and spawn, at
`TESSERAE_SCREENSHOT_SETTLE_S=4` against `30`. `scripts/profile-frames`
fixtures and their fill metrics may measure it better than captures.

## Acceptance

- [ ] The cause found and written here.
- [ ] Fixed, with the time to fill measured before and after, or closed as
      expected with the reason.

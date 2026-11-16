# 30: The Use prompt and the place hint no longer overlap

Status: ready-for-agent

Spec: [Phase 20](../spec.md).

## What to build

Play-testing found, aiming at a Door with a block in hand, "G Open Door" and
"Right mouse to place" drawn over each other under the reticle. The place hint
stands 24 px below the reticle and the Use prompt 37 px, so they overlap; the
mockup has the same overlap (`.use-hint` at 26 px, the place hint at 24 px).

- Under the reticle, the Use prompt and the place hint stack in one column
  with a gap, the Use prompt first, each centred. Either shows alone where it
  stood before.
- The mockup changes first, then the Client.

## Acceptance criteria

- [ ] The mockup shows both lines stacked when both apply.
- [ ] The Client lays both out in one column, tested: their boxes do not
      overlap when both are shown.

## Comments

# 07: A right click removes whatever is aimed at

Status: done
Blocked by: 02

Spec: [decisions 13 and 14](../spec.md).

## What to build

The edit intent's break, remove-source and put-out actions become one remove
action. The Server casts the ray once, stopping at the first Voxel holding a
solid or a fluid source and going through flowing fluid, and removes one
layer there: a burning solid's fire, else the solid (leaving any fluid in its
Voxel), else the source. The refusals are today's, and a 0.3 Mod sees the
same Edit kinds as today. A right click always sends it. A click places the
held item, and does nothing with an empty slot. The HUD says "remove" for the
right click.

## Acceptance criteria

- [x] On a burning solid, the first remove puts the fire out and the second
      breaks the solid.
- [x] On a solid with a source in its Voxel, remove breaks the solid and keeps
      the source.
- [x] On a source alone, remove takes the source.
- [x] Aiming through flowing fluid reaches the solid behind it.
- [x] A right click sends remove whatever the slot holds; a click with an
      empty slot sends nothing.
- [x] 0.3 Mods see put-outs, solid Edits and fluid Edits as before.
- [x] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.

## Comments

### 2026-10-26: implementation

EditAction has one Remove. Its ray stops at solids and packed sources,
passes flowing fluid, and removes fire before solid before source. Breaking
a solid retains its fluid through the existing solid-layer rule. History
still records put-outs, solid Edits and source removals by the layer changed.
Every right click sends Remove, including an empty hand; empty left clicks
send nothing. Existing action-path tests use the unified intent. Changed
layer-order expectations are part of the deferred final review and CI pass.

### 2026-10-26: final verification

Checked on the completed phase tree in ticket 13. Formatting and full CI
pass; the final captures were reviewed and accepted by the user. See
13-final-verification.md for benchmark results and validation exceptions.

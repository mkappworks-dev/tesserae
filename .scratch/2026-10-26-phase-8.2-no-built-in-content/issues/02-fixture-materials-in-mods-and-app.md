# 02: The Mods and app suites on fixture Materials

Status: done
Blocked by: 01

Spec: [decision 21](../spec.md).

## What to build

The second migration batch. Every test and bench in `tesserae-mods` and
`tesserae` that names a built-in solid constant moves to ticket 01's register
calls and the ids they return. Where a test loads a Mod, the fixtures register
after the Mod's Materials or the test uses the Mod's own, so a fixture id
never aliases a Mod id. Tests of production paths still tied to a built-in id
(the hotbar defaults, the gallery, the render palette, the catalogue's Shape
check, the Registry's built-in names) stay on it until the ticket that moves
the path, listed in the Comments.

## Acceptance criteria

- [ ] No test or bench in `tesserae-mods` or `tesserae` names a built-in solid
      constant, except the listed sites, each with its ticket.
- [ ] No test asserts a built-in id as a literal where a fixture or Mod id is
      meant.
- [ ] A test that loads a Mod never uses a fixture id that equals one of the
      Mod's ids.
- [ ] The host-calls bench is unchanged in workload.
- [ ] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.

## Comments

### 2026-10-26: implementation

Native validation, edit, streaming, LOD, World-state and mesh-view fixtures
now use registered solid ids and the corresponding tables. The host-calls
bench keeps its native validation workload on fixture ground. The shared
Server test tables also register the example fence for shaped Edit tests.
A fluid harness loading Mod content appends fixture ground after that content.

Remaining production-coupled sites: the runtime queue guest and tests of its
literal placements (ticket 06); demo reaction/stage expectations and fluid
tests asking for core:stone by name (ticket 04); hotbar defaults, gallery and
catalogue Shape checks (ticket 05); built-in Registry and palette tests
(ticket 06). The frozen echo/builder guests keep their literal return values
and gain refusal assertions in ticket 06. Formatting, review and full CI are
deferred to the phase's final pass per the user's 2026-10-26 instruction.

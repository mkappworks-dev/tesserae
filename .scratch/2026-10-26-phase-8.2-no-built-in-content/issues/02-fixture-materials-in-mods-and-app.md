# 02: The Mods and app suites on fixture Materials

Status: ready-for-agent
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

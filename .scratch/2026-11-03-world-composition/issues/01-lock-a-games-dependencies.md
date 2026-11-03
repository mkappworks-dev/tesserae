# 01: Lock a Game's dependencies on in Create World

Status: ready-for-agent

## What to build

Picking a Game adds the Mods it depends on, transitively, and shows them locked. The recorded World holds them.

## Acceptance criteria

- [ ] Creating `ember` with nothing ticked records strata, waters, flora and hearth.
- [ ] A required Mod cannot be unticked; an optional one can.
- [ ] A test covers the closure including the Game's own `depends`.

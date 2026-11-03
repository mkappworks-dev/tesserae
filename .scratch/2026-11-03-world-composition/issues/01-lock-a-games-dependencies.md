# 01: Lock a Game's dependencies on in Create World

Status: done

## What to build

Picking a Game adds the Mods it depends on, transitively, and shows them locked. The recorded World holds them.

## Acceptance criteria

- [x] Creating `ember` with nothing ticked records strata, waters, flora and hearth.
- [x] A required Mod cannot be unticked; an optional one can.
- [x] A test covers the closure including the Game's own `depends`.

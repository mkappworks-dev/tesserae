# 02: Rebuild meadow on shared Mods

Status: done

Blocked by: 01

## What to build

Rebuild `games/meadow` as a small Rust Game depending on strata, flora and glassworks. It declares gentle rolling land and, on start, places a few trees with one batched Command.

## Acceptance criteria

- [ ] No Material is declared in Rust; all come from the Mods.
- [ ] The Game loads, opens a World and places its trees; a test checks both.
- [ ] README covers what it shows, its dependencies and how to run it.

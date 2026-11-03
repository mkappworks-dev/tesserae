# 02: Rebuild meadow on shared Mods

Status: done

Blocked by: 01

## What to build

Rebuild `games/meadow` as a small Rust Game depending on strata, flora and glassworks. It declares gentle rolling land and, on start, places a few trees with one batched Command.

## Acceptance criteria

- [x] No Material is declared in Rust; all come from the Mods.
- [x] The Game loads, opens a World and places its trees; a test checks both.
- [x] README covers what it shows, its dependencies and how to run it.

## Comments

The test loads meadow with its Mods and checks the trees and gazebo in the Commands its `tick` returns, validated as the Server would; it does not apply them to a running World.

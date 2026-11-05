# 17: Create World in steps

Status: done

Blocked by: 16, 06

Spec: [Phase 13.1](../spec.md).

## What to build

Create World becomes steps with a rail: World (name, Game, game mode, seed, Voxel size under More World options), Mods (required Mods kept on), and Review (a summary with Edit links). Ticket 18 inserts Biomes before Review. Invalid values hold the player on their step with a message. Leaving an unfinished setup asks before discarding. Trust still runs before opening when a Mod needs it. It replaces ticket 06's temporary mode choice.

## Acceptance criteria

- [x] Creating through the steps produces the same World as today for the same inputs, plus its default mode.
- [x] A name over 64 characters or a missing Game is held on the World step.
- [x] Cancel on a changed setup asks before discarding.
- [x] Client App tests walk the steps by keyboard.

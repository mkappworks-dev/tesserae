# 11: Per-Biome Soundscapes

Status: ready-for-agent

Blocked by: 08

## What to build

A Biome may name a Soundscape, so a desert and a tundra sound different.

- `biome-def` gains an optional `soundscape`, a Soundscape's full name, appended
  after `surface-features` in the unreleased 0.3, with a sentence in the
  contract-compatibility paragraph ticket 02 adds. The data TOML takes the same
  field.
- The Server's priority becomes underwater, underground, the Soundscape the
  Eye's Biome names, then default.

## Acceptance criteria

- [ ] Both Declaration formats accept an optional Soundscape on a Biome; an
      unknown one is refused by name; a Biome without one generates and loads
      exactly as before.
- [ ] A Mod that adds a Biome may name its own Soundscape, which carries no
      context.
- [ ] Server tests: walking between Biomes sends the new Soundscape once;
      underground and underwater still win; a Biome without one falls through
      to default.

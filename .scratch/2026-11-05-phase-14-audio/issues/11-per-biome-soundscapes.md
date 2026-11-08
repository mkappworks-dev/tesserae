# 11: Per-Biome Soundscapes

Status: done

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

- [x] Both Declaration formats accept an optional Soundscape on a Biome; an
      unknown one is refused by name; a Biome without one generates and loads
      exactly as before.
- [x] A Mod that adds a Biome may name its own Soundscape, which carries no
      context.
- [x] Server tests: walking between Biomes sends the new Soundscape once;
      underground and underwater still win; a Biome without one falls through
      to default.

## Comments

- From ticket 08: the priority is `Soundscapes::here` in
  `server/soundscape.rs`; insert the Eye's Biome between underground and the
  default there. `tesserae_mods::Sounds::soundscapes` holds every Soundscape by
  full name in load order, so a `SoundscapeId` is its index, and a Mod's
  Soundscape already resolves with `context: None`. Ticket 08's Server tests
  build a `Soundscapes` by hand and drive the Eye through `Player::set_eye`.

- Soundscape bindings resolve by full Biome name, so disabled/reordered Biomes cannot shift their audio. Tests cover cross-Biome publish-once, context priority, unknown references, data parsing and a Mod naming its own context-free Soundscape.

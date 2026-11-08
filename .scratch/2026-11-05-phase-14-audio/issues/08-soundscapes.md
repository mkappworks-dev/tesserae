# 08: Soundscapes

Status: done

Blocked by: 02, 04

## What to build

A Game declares a Soundscape for the open world, underground and underwater, and
a Player hears its ambient loops and music change as the Eye moves between them.

- Ambient and Music Sounds: not positional, may be stereo, and refuse
  distances.
- The `soundscapes` list, from any Mod: a name, an optional context
  (`default`, `underground`, `underwater`), ambient beds (an Ambient Sound and a
  volume), Music Sounds, and the silence between tracks as a range in seconds,
  defaulting to 60 to 180. Only the Game's Soundscapes may carry a context, at
  most one per context; a Mod's exists to be named by a Biome (ticket 11).
- The Server works out the Eye's context each tick, in priority underwater (the
  Eye is inside a fluid), underground (the Eye's Voxel receives no sky light),
  default, falling through contexts with no Soundscape, and sends the
  Soundscape only when it changes.
- The Client loops the beds and crossfades them over 2 s on a change. The core
  music scheduler picks a random track that is not the last one after a silence
  from the range, and fades the current track out over 4 s on a change.

## Acceptance criteria

- [x] Both formats load `soundscapes`; refusals by name for a context from a
      Mod, two for one context, an Effects Sound as a bed or track, distances
      on Ambient or Music, more than 8 tracks.
- [x] Server tests: the context follows the priority; a missing context falls
      through; nothing is sent while the context is unchanged.
- [x] Core tests: the scheduler waits within the gap range, never repeats the
      last track (with two or more), fades out over 4 s on a change, and is
      deterministic under a seed.
- [x] Ambient and Music follow their category volumes from ticket 04.
- [x] The data-only Mod guide and the authoring guide describe Soundscapes.

## Comments

- Limits the spec left open: at most 16 Soundscapes per Mod and 8 beds per
  Soundscape, like the other per-Mod lists; `silence-s` lies in 0 to 3600 s; a
  bed's `volume` in 0 to 1, multiplied by its Sound's own drawn volume.
- The Server sends `HereSoundscape(Option<SoundscapeId>)` from
  `server/soundscape.rs`, and the Soundscape table reaches the Client as
  `SoundscapeLibrary`. It sends nothing while the Eye's Chunk is not held or
  its light is not yet solved, since a Chunk arrives dark. What was sent is a
  session resource, not a `Local`, so a second World in the same run is told
  afresh.
- The first track of a Soundscape also waits a silence from its range.
- The Client tells the scheduler a track ended when its sink despawns itself.

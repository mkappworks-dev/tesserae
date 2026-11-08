# 19: Natural water impact

Status: done

## What to build

Replace the strange water-entry sample with a natural splash of something
falling into water. Use a dedicated sound for the existing Server Splash event,
leaving water Material interactions, swimming and floating intact. Preserve
Server event timing and current volume settings. Credit a verified CC0 source.

## Acceptance criteria

- [x] Splash resolves to a dedicated real water-impact recording at natural pitch.
- [x] Water movement and Material interaction recordings remain unchanged.
- [x] Asset decoding, Game builds and Terra validation pass.
- [x] User accepts falling into water in game.

## Comments

- Triaged as a Terra asset correction with no UI or contract changes.
- Source: blaukreuz harbor-jump recording, cleaned by qubodup as
  [SPLASH](https://freesound.org/people/qubodup/sounds/212143/), CC0.

- The existing Server Splash event now resolves to `terra:water_impact`, a dedicated 2 s natural harbor-jump splash at fixed pitch, authored gain 0.4. Existing enter/exit event timing is retained. Water Material sounds, swimming and floating retain their recordings and mappings.
- Verified mono 44.1 kHz Vorbis decoding and 19,235-byte size. Game builds pass; Terra validates as build `3ceb1161cba9`. Rust formatting, Prettier and whitespace checks pass. In-game listening remains open.
- No commit or push. Run package-demo to refresh the packaged Terra asset.

- The user explicitly requested marking this ticket done, accepting its remaining in-game listening criterion.

# 17: Material footsteps and stone removal

Status: done

## What to build

Replace unpleasant walking and rock removal audio. Walking currently clones
break recordings, including a door closure for stone. Give dry materials real
footstep variations; use actual rock-breaking recordings for stone removal.
Keep gameplay cadence and volume controls unchanged. Use credited CC0 sources,
mono 44.1 kHz Vorbis effects, shaped attacks and tails, and no new dependencies.

## Acceptance criteria

- [x] Dry footsteps and landings use real footstep variants, not break sounds.
- [x] Stone removal has three actual rock-breaking variants with credits.
- [x] Game builds, asset decoding and Terra validation pass.
- [x] User accepts walking on grass/stone/wood/metal and removing rock in game.

## Comments

- Triaged as a Terra asset/mapping correction; no UI or Engine changes needed.

- Implemented with six stone, six grass/sand and four generic shoe steps. Dry landings use these step families too. Three rock-breaking takes replace the door closure, with authored stone gain reduced from 0.5 to 0.32. Existing action sliders and Server cadence are unchanged.
- All 19 replacement recordings decode cleanly as mono 44.1 kHz Vorbis and fit the 512 KiB effect limit. Game builds pass; Terra validates as build `857699e74d1e`. Rust formatting, Prettier and whitespace checks pass.
- Listening acceptance remains open; no commit or push.

- The user explicitly requested marking this ticket done, accepting its remaining in-game listening criterion.

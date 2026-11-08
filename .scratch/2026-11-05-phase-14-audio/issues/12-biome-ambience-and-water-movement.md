# 12: Distinct Biome ambience and water movement

Status: done

## What to build

User follow-up: replace Terra's shared outdoor recording with distinct temperate,
dry and cold ambience. Add Game-bound `swimming` and `floating` Causes to the
unreleased 0.3 contract. The Server emits swimming strokes by distance travelled
in metres, and gentle floating sounds periodically while stationary at the water
surface. Flying, dry land, grounded wading, death and paused play stay silent.
Keep the existing splash on entry/exit and underwater Soundscape priority.

## Acceptance criteria

- [x] Terra's three Biomes reference different Ambient recordings with CC0 credits.
- [x] Both declarations accept swimming/floating Cause bindings; unbound is silent.
- [x] Server tests cover stroke distance, surface floating cadence and silence.
- [x] Contract docs and author guides describe the new Causes.
- [x] Build, tests, lint and Terra validation pass; no commit or push.

## Comments

- Temperate uses birds/breeze; dry and cold use separately processed CC0 wind
  recordings. Credits document both variants. Music remains shared.
- Validated with 1,155 Engine tests and 214 Mod tests passing, Clippy with
  warnings denied, formatting checks, rebuilt WASM Games, and headless Terra
  validation (build c9310ca4b6b6). Four pre-existing ignored tests remain ignored.
- No commit or push. Listening acceptance remains ticket 09.

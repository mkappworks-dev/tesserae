# 01: A Recipe needs an Obtainable Station, and every Template is checked

Status: ready-for-agent

Blocked by: None (can start immediately)

Spec: [Shared building Materials](../spec.md).

## What to build

The Obtainable check, run when a World's Game and Mods are composed, counts a
Recipe whose Station is not `hand` only once some Material that a voxel-use
declares a Station with that tag is itself Obtainable. It checks every Template
any loaded Mod declares, not only those a Biome or Settlement names, and every
declared Settlement's path Material, used or not. A breach refuses the World at
Create World and on open, as today; when a Material fails only because a
Recipe's Station is out of reach, the message names the Station tag.

CI's validate step, which today composes each shared Mod with its dependencies,
also runs over every Game under `games/`, so a breach in a demo Game that has no
tests of its own fails CI. Validate already runs the Obtainable check; a Game is
a WASM component, so the step builds the guests for the WASM target first and
validates each Game with `--mods mods --trust-for-run`. The local CI script does
the same.

## Acceptance criteria

- [ ] A Recipe at a Station whose Material is not Obtainable does not make its
      output Obtainable, and the refusal names the Station tag.
- [ ] A Station Material made by a hand Recipe from laid Materials opens its
      Recipes; a chain of Stations (one made at another) resolves.
- [ ] A hand Recipe needs no Station.
- [ ] A Template that no Biome or Settlement names, using a Material no Player
      can get, refuses the World; so does an unused Settlement's path.
- [ ] The existing exemptions hold: `ground` cells, fluid cells, a crop's later
      Stage; a Grown Material and a Door's open Material are still refused.
- [ ] Terra still composes and passes.
- [ ] CI's validate step and the local CI script cover `games/*/`, and all five
      demo Games pass.

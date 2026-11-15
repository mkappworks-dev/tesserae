# 07: Marker kinds are Declaration data

Status: ready-for-agent

Blocked by: 01

Spec: [Phase 19](../spec.md).

## What to build

The unreleased 0.3 Declaration gains Marker kinds: a name, an icon and a colour.
The icon is one of a closed set of names the Engine draws as shapes (home, camp,
mine, farm, landmark, death and a few more), not a glyph, so no font has to carry
it. The Declaration may name which Marker kind marks a death. It is data only: no
Mod is called and no host call is added (ADR-0001, ADR-0012). Validation follows
the existing table pattern: a row cap and a name rule per Mod, an error naming
the Mod, the row and the reason, and a final resolve after every Mod has
declared; a death kind must name a declared Marker kind. Older contract
adapters supply none and a World with no Game has none (ADR-0016). The Server
sends the Marker kinds to the Client once at session start, as sounds are.
Terra declares Home, Camp, Mine, Farm, Landmark and a death kind; the starter and
the other games compile unchanged.

## Acceptance criteria

- [ ] The contract (WIT, core table, Mods crate, host conversion) accepts valid Marker kind rows and rejects each invalid one with an error naming the Mod, the row and the reason.
- [ ] A death kind that names no declared Marker kind is rejected; older adapters supply none.
- [ ] Terra declares its Marker kinds; the SDK starter and the other games build.
- [ ] The Client receives the Marker kinds at session start (a message test).
- [ ] The contract-compatibility, data-mods and authoring docs describe the table; fmt, clippy, game manifests and tests pass.

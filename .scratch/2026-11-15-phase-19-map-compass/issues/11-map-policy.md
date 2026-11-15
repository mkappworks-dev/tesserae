# 11: A Game can forbid the Map and Compass

Status: ready-for-agent

Blocked by: 07, 09

Spec: [Phase 19](../spec.md).

## What to build

The World parameters gain an optional Map policy, in the manner of the movement
policy: absent means permitted, the 0.1 and 0.2 adapters supply none, and a
Game-only field is rejected from a Mod that is not the Game. The Server inserts
the policy and tells the Client whether Map and Compass are permitted. When
forbidden, the Client draws no Compass or minimap, the Map key and the Menu
button do nothing (the Menu button is absent), the settings and key bindings
stay but are marked unavailable, and the Server stops scanning and refuses
Waypoint intents. An existing saved Map is kept and returns if the policy later
permits it. A World with no Game is permitted. This ticket and ticket 08 both edit the
0.3 contract file and its starter copy and the guest crates: whichever lands
second rebases on the other.

## Acceptance criteria

- [ ] A Game declaring the policy forbids Map and Compass in its Worlds; absent leaves them permitted.
- [ ] While forbidden: no Compass, minimap or Map window; the Server does not scan and refuses Waypoint intents with a reason.
- [ ] A forbidden World's saved Map is kept and returns when permitted.
- [ ] The contract, adapters, docs and a Mods-crate declaration test cover the field; fmt, clippy, game manifests and tests pass.

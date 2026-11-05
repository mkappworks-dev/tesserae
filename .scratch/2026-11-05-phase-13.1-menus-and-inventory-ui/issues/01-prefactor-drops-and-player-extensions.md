# 01: Prefactor: drops at a position in metres, and Player save extensions by registry

Status: ready-for-agent

Blocked by: None (can start immediately)

Spec: [Phase 13.1](../spec.md).

## What to build

Make the next tickets easy. A Dropped item can be spawned at any position in metres, spreading several stacks around that point, not only at the centre of a Voxel with the fixed pickup delay and pop used after a removal. Saved Player data is registered per extension key, so adding a key (Inventory, Vitals, death, game mode) is one registration that also extends the allow-list older builds check, instead of edits to the movement reader, the dirty check and the quit check. No visible change.

## Acceptance criteria

- [ ] Several stacks dropped at one point in metres land spread out, each a Dropped item that despawns after the usual 5 minutes.
- [ ] Existing removal drops behave exactly as before.
- [ ] A registered Player extension round-trips through save and reopen; an unregistered key is still refused as incompatible.
- [ ] The existing movement and Carried extensions use the registry, and their save round-trip tests still pass.
- [ ] `tesserae-core` still depends on neither Bevy nor wgpu.

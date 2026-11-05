# 09: Vitals and damage

Status: ready-for-agent

Blocked by: 06, 08

Spec: [Phase 13.1](../spec.md).

## What to build

Survival Players have health and air on the Server, ticked at 64 Hz in metres and seconds. Fall damage comes from the landing impact speed as a fall height (v² / 2g) above the safe height. Air drains while the Eye is under a fluid surface and refills on surfacing; at zero the Player takes drowning damage per second. Contact damage applies per second while the body overlaps a Material that declares it. Creative takes none. The Server sends the Vitals and the last damage cause when they change. The HUD draws health as mosaic tiles left of centre above the hotbar (half points split on the diagonal), the air row above the hunger side with its height always reserved, a low-health vignette and a hit flash, both stilled by reduced motion.

## Acceptance criteria

- [ ] The same fall in metres deals the same damage at two Voxel sizes.
- [ ] Air drains only with the Eye under a fluid; drowning starts at zero air.
- [ ] Contact damage applies per second of overlap and stops on leaving.
- [ ] Creative Players take no damage.
- [ ] Vitals survive save and reopen; the HUD follows Server messages only.

# 04: Screen-space ambient occlusion

Status: ready-for-agent

Spec: [Colour, art and light](../spec.md).

## What to build

Solution item 4: Bevy's screen-space ambient occlusion on the game camera,
measured before it lands. The chunk material already has a prepass with a
normal output (`chunk_prepass.wesl`), which screen-space AO reads. It missed
the frame budget on every tier, so the user chose (Q60) a player setting, off
by default, with the prepass cut to the levels AO can reach.

## Acceptance

- [ ] GPU frame time on `flight` against `main` recorded.
- [ ] Canopies and building corners darken where blocks meet, and the jungle
      floor gets no darker than 02 leaves it; approved on captures.
- [ ] A player setting, off by default; its cost with the cut prepass recorded.

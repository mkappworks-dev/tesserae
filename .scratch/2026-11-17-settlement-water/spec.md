Status: ready-for-agent

# Water by Settlements, and caves kept from Structures

Ponds, brooks and waterfalls by hamlets, villages and towns, and no cave
opening beside or under a Structure. Builds on
[Settlement farms](../2026-11-17-settlement-farms/spec.md) and step 17's
underground.

## Problem Statement

A Settlement has fields round it but no water of its own: the nearest is
whatever river or lake the land put there, often kilometres off. And version 3's
tunnels, caverns and shafts open anywhere, so a cave mouth gapes beside a
cottage or under a ruin.

## Solution

- **Outskirt groups.** A Settlement's outskirts become groups, each drawn by its
  own chance and count from its own Templates: farms one group, water another.
  Each later group takes the spots the earlier left.
- **Water Templates.** Terra declares a small pond, a lily pond, a pond fed by a
  spring along a stone brook, and a waterfall pouring off a rock ledge into a
  pool. Every Settlement kind lists them in a water group by chance.
- **Caves keep off Structures.** No tunnel, cavern or shaft carves within 16 m
  of a Structure's footprint, or of a Settlement's disc, approach paths or
  outskirts, at any depth. Version 3 only.

## User Stories

1. As a Player near a village, I often find a pond by its fields, sometimes with
   lilies or a brook running into it.
2. As a Player, some Settlements have a little waterfall off a rock ledge.
3. As a Player, no cave opens beside a house, a bridge or a ruin.
4. As a Player with a World made under version 2, its caves are as they were.

## Implementation Decisions

- **Groups.** `Settlement.outskirts` is a list; group `i` salts its draws with
  `i`, so adding a group never moves an earlier one's Plots. The contract's
  `outskirts` becomes `option<list<settlement-outskirts>>` in the unreleased 0.3
  Declaration; a saved World's single `outskirts` object restores as one group.
- **Water in Templates** is the World's water as source cells, which Templates
  already carry (a well). A pond and a brook are level sources, so nothing flows
  when a Chunk is made; a waterfall's fall flows once and settles in its pool.
- **Keep-out** is a per Chunk column mask from the claims and Settlements near
  it, asked before any carve.

## Testing Decisions

- A second outskirt group lays its own Templates and leaves the first group's
  Plots where they were.
- A saved World with a single `outskirts` object reopens unchanged.
- No version 3 cave Voxel stands within 16 m of a Structure's footprint, at
  1 m and 0.5 m; version 2 is unchanged.
- Captures: a village's pond and a waterfall.

## Out of Scope

- Streams that run across the land between Settlements (the drainage's rivers).
- Fish and water animals.

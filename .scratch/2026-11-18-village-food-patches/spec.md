Status: done

# Food patches by every Settlement

A small patch of each demo Game's first food beside every hamlet, village and
town, so a new Player who enters a village finds a meal within a short walk.
Builds on [Water by Settlements](../2026-11-17-settlement-water/spec.md).

## Problem Statement

Under version 3 a new Player enters a village. The wild's scatter keeps its food
off a Settlement's disc, paths and outskirts, so the nearest food lies beyond
them: Meadow's pumpkins up to 164 m from the spawn, against the 96 m short walk
version 2 keeps.

## Solution

- **Food patch Templates.** Each demo Game declares a `food_patch`: a small bed
  of its own soil with its first food lying ripe on it (Terra's melons, Meadow's
  pumpkins, Ember's peppers, Tidewater's coconuts, Lumen's glowberries).
- **A food group.** Every Settlement kind lists the patch in an outskirt group
  of its own, always drawn, one or two patches, after its farms and water.

## User Stories

1. As a new Player, I find my Game's food beside the village I enter.
2. As a Player passing any Settlement, a patch of food lies by it.

## Implementation Decisions

- The patch is a Game content change only: no engine or contract change.
- The group comes last, so its salt leaves every earlier group's Plots where
  they were.
- A saved World keeps its own Settlement table, so it lays no patches.

## Testing Decisions

- Version 3's spawn test is back to the 96 m short walk for every demo Game at
  its seeds.
- The exported Templates match the mockup (`export-templates.mjs --check`).
- A capture of a village's patch.

## Out of Scope

- Food for Settlements of Games other than the five demo Games.

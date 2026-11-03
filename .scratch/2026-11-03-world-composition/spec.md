# Composing a World from a Game and Mods

Status: done

## Problem Statement

Creating a World with a Game that depends on Mods (`ember` needs four) is
broken and cramped. Picking a Game does not add its dependencies, so a World
made with no ticks records no Mods and cannot load. Game and Mods share one
long page that does not scroll to the Create button. Trust is asked one item at
a time, and its text says "it runs its own code" for data Mods that run none.
Nothing shows which Mods a saved World holds, and a Game cannot take a
character from a shared Mod.

## Solution

Create World becomes two steps (Game, then Mods) with a Trust step for the
whole set. A Game's dependencies are locked on. The chosen set is checked for
rule clashes before the World is made. The World list shows a World's Mods, and
Re-create reopens the Mods step. More shared data Mods add solids and fluids,
and a Mod may supply the World's character.

The design is the Tesserae-themed mockup agreed in the session of 2026-11-03.

## Decisions

- A Game's `depends`, and theirs, are locked on in step 2 and recorded in the
  World; the user can add optional Mods but cannot remove a required one.
- A World freezes its set (ADR-0018). There is no editing of a saved World's
  Mods; "Re-create" opens step 2 with the World's Mods ticked and makes a new
  World.
- Trust is one screen for the set. Data Mods are marked "data only"; only
  components are marked "runs code". Per-item hashes are expandable.
- Conflicts are the rule clashes the loader already reports (a fluid or fuel
  defined twice, two reactions on one contact). Step 2 runs the data-Mod part of
  that check and greys out a Mod that would clash, naming the other.
- A character comes from exactly one source: the Game's own, or one character
  Mod. Two sources are a load error naming both. This relaxes "only a Game
  declares `world`" for `character` alone, so it needs an ADR and a note in the
  contract compatibility policy.

## Out of scope

- Editing a saved World's Mods in place.
- Network or marketplace installation of Mods.

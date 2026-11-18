# 17: The Games are tuned for version 3

Status: done

Spec: [Generator determinism](../spec.md).

Blocked by: None (can start immediately).

## What to build

Seven Game tests ticket 04 left on version 2 (see its Comments) fail under
version 3 because the Games' data was tuned for version 2's land. Retune Terra,
Tidewater, Meadow and Ember (Biome climates and shares, tree spacing and fit,
bare Biomes, Settlement sizes and spacing, crop weights) so each test passes
under version 3 unchanged:

- Terra's temperate and Tidewater's lagoon Biomes cover the most ground.
- Terra's jungle trees stand 2.5 to 6 m apart and fit at least 95% of jungle
  columns.
- Meadow's dense Biomes are clearly denser than its sparse ones.
- Terra's alpine and beach Biomes are bare (no mushrooms, no neighbour's
  coconuts).
- Ember has a large Settlement within 768 m.
- Terra's wheat leads its fields.

## Acceptance criteria

- [ ] Every listed test runs under version 3 and passes with its thresholds
      unchanged.
- [x] Only Game data changes, not Engine code; the Engine digests do not move.
- [x] Captures of each retuned Game's spawn surroundings are taken for review
      (screenshots stay out of the repo) and listed in a comment here.

## Comments

- The first criterion is not met: five of the seven tests run under version 3
  and pass unchanged; two stay pinned to version 2, for 05 to decide:
  - `each_biome_scatters_its_trees_at_its_density_class`: version 3 drops a
    tree whose crown radius × 1.5 holds a stronger tree's trunk, which caps a
    one-species stand at 1/(π(1.5r)²): 57 a hectare for `flora:jungle` (r 5 m),
    115 for `flora:swamp` (r 3.5 m), against the dense class's 278 (6 m apart).
    Terra's swamp, meadow's thicket and fen, ember's cinder bog and
    tidewater's tidal marsh grow only one of those trees, so no Game data
    reaches the class; every dense Biome of every Game, Lumen's too, measures
    9 to 13 m. The fix is the crowding rule, the crowns in `flora`, or the
    class.
  - `every_biome_of_every_demo_game_grows_its_declared_species_and_none_is_bare_by_accident`:
    the alpine Biome holds mushrooms from Terra's `mushroom_grotto` cave Biome,
    whose caves run under the mountains; capping its altitude at sea level still
    left 3 Voxels, and a deeper cap would move every grotto for one census. The
    beach, narrowed to a metre, holds one coconut from a neighbour's palm; the
    test lets a neighbour's limbs and vines over a border but not its fruit.
- Effective Climate under version 3 is about 0.1 wetter (coast) and a little
  colder (altitude) than the field Climate the Games were tuned on, and seed 7's
  8 km square is 66% sea. The Climate points were found offline: a dump of each
  column's site and shifted Climate reproduced `biome_at` on every column, and a
  search over the points and the strand's width met the share and fit tests
  with the least margin largest.
- Terra: Climate points temperate (0.38, 0.59), dry (0.54, 0.42), cold (0.29,
  0.55), jungle (0.65, 0.71), swamp (0.36, 0.75), savanna (0.63, 0.56), so all
  six cover over 4% at seeds 7, 1 and 2024 with temperate ahead of dry and cold
  (seed 7: 5.6, 4.2, 4.6, 4.2, 5.0, 4.2%), and each native tree fits at least
  95.8% of its Biome; beach 0 to 1 m (was 3), freeing 6% of seed 7's coast;
  village radius 40 to 72 m (was 64), 12 to 30 Plots (was 8 to 20), lattice
  240 m (was 224), since the moved map left none of 8 buildings near spawn
  (now 2 to 10); the stone bridge listed in temperate, cold, jungle and
  savanna too, as the river nearest spawn now runs through them; savanna tuft
  8.4 m and dry grass 4.6 m (cover 11.9% to under 10%); temperate's scattered
  wheat 7.5 m (was 9), so wheat leads carrot and potato; cold's pines 9 m (was
  10), as the moved points left version 2's cold at 11.7 m.
- Tidewater: Climate points lagoon (0.44, 0.62), salt flats (0.52, 0.42), cold
  shallows (0.34, 0.56), rainforest (0.69, 0.64), tidal marsh (0.33, 0.69),
  lagoon largest at every seed (seed 7: 6.6, 5.2, 5.4, 5.2, 5.2%); shore 0 to
  1 m (demo-kit's `SHORE_M` is 3); lagoon fern 3.6 m (cover at the 10% cap);
  lagoon fields every 88 m within 88 m of water (were 80 and 48), the first
  spacing that stands all three crops near the moved spawn.
- Meadow: meadow Biome's cherry and apple 40 m, oak 28 m (were 30 and 22), so
  its 20 trees a hectare fall under half the thicket's 57, which the crown rule
  already caps; meadow fields every 48 m (was 96) so a carrot plot stands within
  256 m.
- Ember: village chance 1.0 (was 0.8), radius 44 to 72 m, 12 to 30 Plots,
  lattice 176 m (the floor is twice radius and margin), so a copy of 10
  buildings stands within 768 m; ditched ash fields every 64 m within 64 m of
  water (were 72 and 40), since the larger village took the old ditch's place.
- Lumen, not named here but in the same tests: garden fields every 40 m (was
  64); its two field plots near spawn were both potato, and its wheat came from
  a Settlement's farm Templates.
- The field census's World "without" fields rebuilds the Biome table from bare
  `Biome`s, which drops their selectors, so under version 3 it differs
  everywhere (30 M Voxels of Terra's basalt) and the census counts scattered
  crops as well as field plots. The test still passes; its measure is looser
  than its name.
- Captures in `/tmp/t17-captures/` (not committed): `<game>-spawn.png` at each
  Game's spawn and `<game>-above-spawn.png` 60.21 m over it, pitched 35° down,
  for terra, tidewater, meadow and ember, taken with
  `TESSERAE_SCREENSHOT_OFFSCREEN=1` and `TESSERAE_TRUST_MODS=1` from an install
  holding one Game and its dependencies.

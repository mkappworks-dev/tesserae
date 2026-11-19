# 19: Terra's field plots grow all its crops

Status: done

Spec: [Generator determinism](../spec.md).

Blocked by: 05.

## What to build

The fields test compares against a World that drops only the field plots, so it
measures field plots alone (ticket 18's Comments). Meadow, Tidewater, Lumen and
Ember pass that way; Terra does not: near spawn at seed 7 its fields hold one
wheat and one potato plot, no carrot and no ditch, because it plants fields only
in its temperate Biome, every 72 m or 112 m, with carrot 2 in 10. Retune Terra's
fields (spacing, Biomes, crop weights) so its field plots alone carry the test,
and switch the test to the fields-only comparison.

## Acceptance criteria

- [x] The fields test compares against a World without field plots only, and
      passes for every Game with its thresholds unchanged.
- [x] Only Terra's content and that test change; no Engine code, and no other
      Game test breaks.
- [x] What changed in Terra's fields, and the counts before and after, are noted
      in a comment here.

## Comments

- The fields census's World "without" now drops only the crop fields: each
  Biome keeps its surface features, Structures and Settlements, and the table
  keeps its selectors and strata. Under it Terra failed at seed 7 within 512 m
  of spawn with wheat 54, carrot 0, potato 50 Voxels and no ditch, as ticket 18
  found; with Terra retuned, the other four Games pass it unchanged.
- Within 512 m of seed 7's spawn Terra's land is temperate 5.0%, dry 9.1%,
  jungle 8.5%, savanna 6.0%, swamp 12.2%, cold 2.7%, beach 0.3%, and the rest
  water (56.3%). Closer temperate spacing alone stood all three crops (fields
  every 48 m within 40 m of water and every 72 m: wheat 184, carrot 21, potato
  35) but never a ditch, since no temperate ground near spawn lies within 48 m
  of water.
- Terra's fields now:
  - temperate every 64 m within 48 m of water (was 72 m within 40 m) and every
    96 m anywhere (was 112 m); carrot 3 in 11 (was 2 in 10), wheat 6, potato 2;
  - savanna, which grew only scattered wheat, plants the same two fields, wheat
    7, carrot 2, potato 2.
- After, within 512 m of spawn: seed 7 wheat 390, carrot 171, potato 72, ditch
  22 Voxels of water, sand paths 607; seed 1 wheat 498, carrot 114, potato 156,
  ditch 28; seed 2024 wheat 2068, carrot 564, potato 634, ditch 76 (before:
  seed 1 had wheat and carrot tied at 111 and no ditch; seed 2024 wheat 259,
  carrot 70, potato 183, ditch 21). The test asserts seed 7 only.
- Variants were searched in-test by rewriting the loaded params' crop fields,
  so no Terra rebuild per try. Dry fields also passed but were left out: dry
  land is cactus and palm, not a farm.
- `cargo test -p tesserae -p tesserae-mods --features tesserae/dev` passes
  after merging `tech/generator-determinism` at b0171d5b (1676 and 310 tests);
  before it, only ticket 20's two failures.

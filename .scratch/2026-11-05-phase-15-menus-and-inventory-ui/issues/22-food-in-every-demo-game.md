# 22: Food in every demo Game

Status: done

Blocked by: 12, 21

Spec: [Phase 15](../spec.md).

## What to build

Every demo Game has a food that lies in its World. Terra's melon becomes a surface feature on grass. Each of the other four demo Games declares one food of its own, themed to the Game: a placeable solid Material whose Item has a food value from 4 to 8, with its own texture and appearance colour, scattered as a surface feature in the Biomes whose surface suits it, at a mean spacing comparable to Terra's melons (findable while exploring, not a carpet). Where a Game's ground comes from a shared data Mod, the food lives where it fits best, recorded in this ticket. New Materials append after existing ones so no id moves.

## Acceptance criteria

- [x] Every demo Game declares at least one food Item with a food value.
- [x] A new Survival World of every demo Game has its food on the surface within a short walk of spawn.
- [x] Each food can be placed, broken, picked up and eaten with G.

## Comments

- Maintainer, 2026-11-05: every demo Game gets food, not only Terra. Variety (crops, cooking, meat) waits for steps 16, 18 and 22.
- Each food is the Game's own solid Material, declared after every Material it
  had (the four other Games had none, so each food is the last Material of its
  World and no id moves), with an Item that places it, its own procedural
  texture and appearance colour, and a surface feature on the Biomes below.
  Every Biome's surface but Terra's grass comes from a shared data Mod; the food
  stays the Game's, on the Game's own Biomes, so no shared Mod changes:

  | Game      | Food                | Hunger | Biomes (surface)                                       | Spacing |
  | --------- | ------------------- | ------ | ------------------------------------------------------ | ------- |
  | terra     | `terra:melon`       | 6      | `temperate` (`terra:grass`)                            | 16 m    |
  | meadow    | `meadow:pumpkin`    | 8      | `meadow` (`strata:turf`), `heath` (`strata:soil`)      | 20 m    |
  | ember     | `ember:pepper`      | 4      | `ash_flats` (`hearth:ash`), `highland` (`flora:grass`) | 16 m    |
  | tidewater | `tidewater:coconut` | 6      | `lagoon` (`strata:turf`), `salt_flats` (`strata:sand`) | 16 m    |
  | lumen     | `lumen:glowberry`   | 5      | `garden` (`strata:turf`), `frost_court` (`frost:snow`) | 20 m    |

- Left bare, as no food fits them: Terra's `dry` sand and `cold` snow (the
  ticket puts the melon on grass), Meadow's `dunes`, Ember's `scorched` clay
  and Tidewater's `cold_shallows` stone. The glowberry emits light 6, like a
  dim lantern, which is Lumen's theme; it lies on the frost court's snow too.
- 16 m is one melon in every 256 m² of grass, "a melon every few hundred square
  metres". The pumpkin and the glowberry are a little sparser at 20 m.
- "A short walk" is 96 m, checked at six seeds. Measured at the default seed,
  1 to 9 and 2024, the nearest food is at most 18 m away when the spawn is in a
  Biome with food and 27 m when it is in one without, except a spawn deep in
  Terra's `cold` or Tidewater's `cold_shallows`, at about 80 m. Every Game uses
  the same draw for the same seed, so at one seed the foods lie in the same
  columns wherever they may.
- A Terra World saved before this keeps its bare grass: its snapshot pins the
  Biome table without the melon (ticket 21). A new Terra World has melons; the
  "Terra generates none" note on ticket 12 is superseded.
- The end-to-end test runs every Game's food through the Server's own edit,
  drop, pickup and eat systems in Survival; `step_entities`, `eat` and the edit
  tests' `app_with_tables` became visible inside `server` for it.

# 04: The demo Game declares its own water and lakes

Status: done
Blocked by: 03

Spec: [decision 8](../spec.md).

## What to build

The demo moves to 0.3. It declares `demo:water` as a Material: translucent,
dimming 2, hiding the same Material, drawn blended and not solid. It gives it a
fluid row with today's numbers: reach 7 m, 4 m/s sideways, 8 m/s down, 4 m/s
recession, 0.5 m reset drop, waterlogging and extinguishing. It also gives it
today's placeholder colour.

The lava-and-water reaction names `demo:water`, and every scenario that pours
water uses it. The demo's World parameters name `demo:water` for its lakes, at
a level that leaves the scenario stage dry. It still builds only against the
WIT contract.

## Acceptance criteria

- [x] The demo names no `core:water` anywhere. The demo boundary check passes.
- [x] The demo World has `demo:water` lakes, and none on or beside the
      scenario stage, at 1.0, 0.5 and 0.25 m.
- [x] Every scenario plays out as it does today at all three Voxel sizes:
      waterfall and recession, lava cooled to stone, mud fired to brick, and the
      post burning and put out by poured water. The existing demo runtime tests
      prove it.
- [x] The 0.2 `fluids` guest fixture names a fluid it declares itself in place
      of `core:water`.
- [x] README and CONTRIBUTING describe the demo's water as the demo's.
- [x] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.

## Comments

### 2026-10-26: implementation

- The demo is on 0.3 (`wit/0.3`, `api = "0.3"`). `demo:water` is its last
  Material, id 14, so lamp to brick keep 8 to 13. Its fluid row is the first
  the demo declares, ahead of lava and mud: registration order decides an
  equal claim, and the built-in water ranked before both. The colour is
  `srgba(0.10, 0.32, 0.62, 0.65)` rounded to bytes, (26, 82, 158, 166).
- Lake level: 25.5 m of `demo:water`, a decision the spec leaves open. At
  the engine's default seed the demo's ground (24 to 28 m) is nowhere below
  26.0 m within 20 m of the stage, and the nearest shore is about 60 m from
  it; about a fifth of the ground within 160 m is under the lake. The level
  only holds for that seed, which is the only one the engine opens today.
- The lava-and-water reaction names `demo:water`; resolved canonically it is
  now (lava, water) consuming the first, the same rule as before.
- The `fluids` guest declares `fluids:brine`, its Material 11 with water's
  numbers, and its slime reaction names it. `lines.rs`'s registered rows and
  `behaviour.rs`'s guest test follow.
- Server tests on `demo:water`: `fluids/tests.rs` (every `declared()` and
  `demo_and_fluids` test; `a_mods_own_pair_reacts_through_the_server` uses
  the brine) and `measure.rs`'s `Run`.
- `demo_contract_content_build_and_scenarios_reach_real_voxels_and_light`
  now runs at 0.25 m too, holds enough Chunks for the stage at that size,
  and steps the fluid simulation beside the Mod's ticks (never ahead of
  them), then settles it. It asserts what was only implied before: water
  falls off the pillar and has receded once its source goes, mud fired to
  brick and lava cooled to stone on the beam, and the post is still wood and
  not burning. It also asserts `demo:water`'s Material and fluid row equal
  the fixture water's, that no reaction or lake names `core:water` and no
  held Chunk holds it, and, generating the World at each size, a valley
  within 160 m holds a `demo:water` lake while no lake Voxel lies on the
  stage or within 20 m of it.
- README and CONTRIBUTING say the demo's water is its own and where its
  lakes stand. The hotbar's water slot is still the built-in one (ticket 06).
- `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.

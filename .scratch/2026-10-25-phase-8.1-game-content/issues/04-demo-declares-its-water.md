# 04: The demo Game declares its own water and lakes

Status: ready-for-agent
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

- [ ] The demo names no `core:water` anywhere. The demo boundary check passes.
- [ ] The demo World has `demo:water` lakes, and none on or beside the
      scenario stage, at 1.0, 0.5 and 0.25 m.
- [ ] Every scenario plays out as it does today at all three Voxel sizes:
      waterfall and recession, lava cooled to stone, mud fired to brick, and the
      post burning and put out by poured water. The existing demo runtime tests
      prove it.
- [ ] The 0.2 `fluids` guest fixture names a fluid it declares itself in place
      of `core:water`.
- [ ] README and CONTRIBUTING describe the demo's water as the demo's.
- [ ] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.

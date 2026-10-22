# 07: Ignite, burn and extinguish declared fuel

Status: ready-for-agent
Blocked by: 04, 05

Spec: [decisions 3, 6, 10 and 11](../spec.md).

## What to build

Hot lava and burning Material expose nearby reachable fuel to heat. Fuel
ignites after its declared delay, spreads fire, burns to its declared residue,
and can be extinguished and protected by waterlogging.

## Acceptance criteria

- [ ] Native evaluation uses declared heat strength/reach, ignition threshold
  and delay, finite burn duration, emission, and residue. Spatial units are
  metres and elapsed time is logical simulation seconds.
- [ ] Heat follows reachable open space and attenuates monotonically over
  physical path length. Solid walls, Shape partitions, joint seals, and missing
  Chunks block it; an open route within reach can transmit exposure.
- [ ] The strongest source determines exposure. Dropping below the threshold
  resets incomplete ignition progress. No temperature field or solid heat
  conduction is introduced.
- [ ] Fuel is whole-Voxel solid state. Waterlogging by an extinguishing fluid
  anywhere in accessible voids protects the whole fuel, extinguishes burning,
  and resets ignition; it does not leak suppression through neighboring walls.
- [ ] Dry fuel ignites, spreads exposure, consumes finite fuel, and replaces
  its solid Material with residue while preserving Shape/Orientation, or Air.
  Replacing/removing fuel ends obsolete burning.
- [ ] Residue only burns under its own declaration. Ordinary fire eventually
  settles without fuel; permanent sources require explicit declaration.
- [ ] Removing water allows reignition after a fresh normal delay. Different
  declared fluids can have different extinguishing behavior.
- [ ] A basic flame indicator and block light reveal burning without replacing
  collision geometry or depending on a Look pack.
- [ ] Shared quotas/fairness also cover thermal work. Stale Jobs, step replay,
  Chunk load/eviction, and saturated frontiers cannot bypass the contracts
  established for fluids.
- [ ] Tests cover partial wet Shapes, protected fuel, finite fuel chains,
  source removal during Jobs, light decrease on extinction, and controlled
  native/Server/real-Mod behavior.

## Verification and completion

Run cargo fmt --all, npm run fmt, and full scripts/cargo-ci before committing.
Hand-wrap this file; never run Prettier on .scratch. In the finishing commit,
check the criteria, set Status: done, and append implementation, verification,
and evidence under Comments. Use the required Conventional Commit and trailer.

## Comments

- This delivers the approved local heat/fire model. Regional fuel destruction,
  conductive temperatures, gases, weather, and entity damage are outside it.

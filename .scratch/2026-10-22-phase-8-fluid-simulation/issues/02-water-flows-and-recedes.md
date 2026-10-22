# 02: Make placed water flow and recede

Status: ready-for-agent
Blocked by: 01

Spec: [decisions 2, 4, 5, 6 and 11](../spec.md).

## What to build

Deliver a full-detail waterfall and shallow stream from an explicit water
source, plus source controls that make the flow reversible. Run the native
kernel as bounded worker work and land only on the Server.

## Acceptance criteria

- [ ] Generated and explicitly placed water is source water; simulated flow
  never becomes a source or fills a Cave upward through pressure.
- [ ] Flow prefers downward travel and spreads sideways over metre-valued
  reach with diminishing surface height. Successive physical drops can renew
  reach; a horizontal surface slope cannot renew it.
- [ ] Reset-drop distance and propagation/recession rates use metres and
  simulation seconds. Equal physical fixtures at 1.0, 0.5, and 0.25 m Voxel
  sizes respect the documented quantization tolerance.
- [ ] Cutting supply and removing sources removes unsupported flow and cycles.
  Blocking a stream does not delete its upstream source.
- [ ] Explicit fluid-tool intents place/remove sources using Server-validated
  rays, origin/reach checks, and the existing queued-intent lifecycle.
  Removal preserves solid state; refusal changes neither layer.
- [ ] The ordinary solid Edit ray retains its behavior. Placeholder controls
  and feedback make source placement/removal exercisable without inventory.
- [ ] Full-detail geometry and lighting follow changed water state. Addition
  can darken and removal can brighten; an older Job cannot resurrect a source.
- [ ] The native kernel's quota/continuation and controlled Server tests
  demonstrate a complete single-region flow/recession cycle.

## Verification and completion

Run cargo fmt --all, npm run fmt, and full scripts/cargo-ci before committing.
Hand-wrap this file; never run Prettier on .scratch. In the finishing commit,
check the criteria, set Status: done, and append implementation, verification,
and evidence under Comments. Use the required Conventional Commit and trailer.

## Comments

- Draft initial tuning and the physical reset-drop rule are proposed in the
  spec; they are not fixed counts of Voxel steps.

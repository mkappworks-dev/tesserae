# 09: The default curve, fog and Terra's ceiling

Status: ready-for-agent
Blocked by: 03, 04, 05, 06, 07, 11, 12

Spec: [LOD features](../spec.md).

## What to build

The default reaches well past today's: Medium, `u` 256 m (8 Chunks) to 6 km, with Terra's ceiling
raised from 3000 m and the engine's default from 4096 m to 8192 m so High (`u` 384 m to 8 km) can be
chosen, and fog lightened
so the far field reads. It becomes the default only when, on the M5 with Terra
at `stationary` and `sprint-flight`, with Impostors and Canopy cover in:

- coarse vertex memory on the GPU is at most 192 MB;
- the whole field is drawn within 10 s of spawn;
- GPU frame is at most 1 ms over `main`;
- level 0's fill time does not regress.

Otherwise the default is the widest curve that passes, recorded in the spec. High is measured too and
may exceed them, as a Player's choice; its numbers go in Comments.
No shadow series: 07's test keeps coarse meshes off the shadow casters.
Terra's level 0 is measured at 192 m (6 Chunks, the engine default every other Game uses) beside
its 160 m: Terra moves to 192 m only if level 0's fill time and GPU frame stay within the budgets
above; either way the reason is recorded in the spec, since 160 m has none on record.
DESIGN.md's LOD section and
step 22 note are updated.

## Acceptance criteria

- [ ] Budget series against 01's, in Comments: the only full series
      after 01's.
- [ ] Captures from fixed Eyes at 0.5, 1, 2, 4 and 8 km beside your reference
      shots, shown to you; the fog's value chosen on them.
- [ ] The chosen default, and why, in the spec.
- [ ] Terra's level 0 at 160 m and 192 m in the budget series; the kept value, and why, in the spec.

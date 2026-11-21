# 09: The default curve, fog and Terra's ceiling

Status: ready-for-agent
Blocked by: 03, 04, 05, 06, 07

Spec: [LOD features](../spec.md).

## What to build

The default looks like other engines with distant terrain: Detailed, about
`384 * 2^k` m to 8192 m, with Terra's ceiling raised from 3000 m and the
engine's default from 4096 m to 8192 m, and fog lightened
so the far field reads. It becomes the default only when, on the M5 with Terra
at `stationary` and `sprint-flight`, with Impostors and Canopy cover in:

- coarse vertex memory on the GPU is at most 192 MB;
- the whole field is drawn within 10 s of spawn;
- GPU frame is at most 1 ms over `main`;
- level 0's fill time does not regress.

Otherwise the default is the widest curve that passes, recorded in the spec.
No shadow series: 07's test keeps coarse meshes off the shadow casters.
DESIGN.md's LOD section and
step 22 note are updated.

## Acceptance criteria

- [ ] Budget series against 01's, in Comments: the only full series
      after 01's.
- [ ] Captures from fixed Eyes at 0.5, 1, 2, 4 and 8 km beside your reference
      shots, shown to you; the fog's value chosen on them.
- [ ] The chosen default, and why, in the spec.

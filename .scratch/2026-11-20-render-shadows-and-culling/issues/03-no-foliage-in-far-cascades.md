# 03: No cutout casters in the far cascades (1a)

Status: ready-for-agent
Blocked by: 01

Spec: [Shadows and culling](../spec.md).

## What to build

Cutout layers (foliage) are the casters that run a fragment shader in the
shadow pass, once per cascade that lists them.

- A system after Bevy's directional caster visibility removes every cutout
  Chunk layer from the far cascades' visible lists, cascades 2 and 3 (from
  25 m at the defaults). Near cascades are untouched.
- Which cascades are far is one named constant, revisited by ticket 06.

## Acceptance criteria

- [ ] A Bevy app test: a cutout layer in a far cascade is not in its list, and
      is in a near cascade's; opaque layers are in both.
- [ ] Gated by 01's method; numbers in Comments.
- [ ] Before and after captures of `flight`, `stationary` and `hidden-hill`
      shown to you; lands only on your approval.

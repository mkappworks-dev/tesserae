# 01: The Engine lays outskirt Plots

Status: ready-for-agent

Spec: [Settlement farms](../spec.md).

## What to build

`Settlement` gains outskirt Plot rules and a count range. `lay_out` tries them
after the disc's Plots at slots along the approach paths past the disc, keeping
each footprint and apron outside the disc and inside the margin. Their
footprints join `LaidSettlement::footprints`.

## Acceptance criteria

- [ ] An outskirt Building stands whole outside the disc and inside the margin,
      in any Chunk order and the same in metres at 1 and 0.5 m.
- [ ] No field, tree or scatter Plant stands on an outskirt Plot or its apron.
- [ ] A Settlement without outskirts lays out exactly as before; recorded
      digests pass.

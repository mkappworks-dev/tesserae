# 02: Mipmapped Material texture array

Status: ready-for-agent
Blocked by: 01

Spec: [Texture shimmer](../spec.md).

## What to build

Solution item 2: the atlas becomes a 2D array, one 16x16 layer per Material
face, with a CPU-built mip chain, an all-linear 16x anisotropic repeat sampler,
a shader texel snap where the mip level is at or below 0, and
`textureSampleGrad` on continuous UVs. Item models and tile icons follow.

## Acceptance

- [ ] Mip builder unit tests from the spec's Testing section.
- [ ] The shimmer pair from 01 shows no texel crawl on level 0 or coarse levels.
- [ ] Near view unchanged: texels still crisp, no seams on merged quads.
- [ ] Leaves keep their coverage at distance.
- [ ] `flight` GPU frame no worse than `main` beyond noise.

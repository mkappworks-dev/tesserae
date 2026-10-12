# Merging collinear arm faces across Voxels in pass 2

Spun out of ticket 09 of `2026-10-12-connected-shapes-and-tall-collision`. That
spec's decision 6 says pass 2 emits per Voxel and "nothing merges across
Voxels"; ticket 09 asked whether lifting that, for the rails of a fence line, is
worth it. This spec records the answer rather than leaving the question open.

**Expect this to come out marginal.** The first ticket measures before anything
is built, and a no-go from it is a successful outcome.

## What it would buy, and what it cannot

A fence line is posts joined by arms, and an arm of one fence meets the arm of
the next at the Voxel boundary. Today each emits its own top, bottom and side
quads there; the end caps already cull each other. Merged, two collinear quads
become one. By hand, after ticket 09 (one bar per arm), a fence in a long line
is about 20 quads, of which the arms are 8 and would become 4: about 20% fewer
quads in a line and none for an isolated fence. The estimate is the thing ticket
01 replaces with a measurement.

It **cannot lower `ShapeTable::cap_units`**. The weight is the worst case of one
Voxel (all its arms out), and merging shrinks the pair, not the Voxel. Merging
is therefore a mesh-time and vertex-count win for lines, not a way to fit more
fences under the cap. Redefining the cap to count measured quads is a different
decision, with an ADR, and is not taken here.

## Decisions (defaults; change them here and the tickets follow)

### 1. What merges

Two pass-2 quads merge when they are on the same face and plane, abut exactly
along an edge at a Voxel boundary, share the full length of that edge, and agree
on Material, render layer and flat `Light`. Chunks mesh independently, so
nothing merges across a Chunk boundary.

Light equality is the limit that matters: an arm's top and sides are interior
quads, lit by the Voxel's own light, and two Voxels beside a torch or a shadow
edge differ. Ticket 01 reports how often neighbours agree in the dev gallery and
in terrain, because a merge that rarely fires is not worth its cost.

### 2. Scope: every shaped quad

A generic pass over pass-2's output would also merge a floor of slabs, whose
tops are the same problem. The default is **every shaped quad**, because the
test for "same face, same plane, same light, abutting" does not care which Shape
a quad came from and a special case for arms would be more code, not less.
Ticket 01 measures both so the scope can narrow to arms if the generic pass
costs terrain with stairs anything.

### 3. Mechanism

Pass 2 stops pushing quads straight to the layer. It collects, per layer, the
quads as absolute rectangles (face, plane, extent in sub-cell units over the
Chunk, Material, light), merges runs along the quad's `u` and then `v`, and
pushes the result. UVs continue across the run, as pass 1's do, so the tiling
sampler repeats per Voxel as before. The cost is the buffering, and it is paid by
every shaped Voxel, which is why `terrain_stairs_at_cap` and `all_stairs` are
gated.

### 4. The hole-free test needs less than the ticket feared

`tests/meshing.rs`'s oracle counts the sub-cell squares each quad covers, from
the quad's own extent, not "one quad per Voxel face". A merged quad covers the
same squares, so "every exposed sub-cell face is drawn, none twice" holds
unchanged and needs no oracle rewrite. What does change:

- tests that count a fence line's quads (the hidden-end-caps test among them)
  expect fewer;
- new: a line of fences merges its arms, and does not across a light step, a
  Material change, a gap, or a Chunk boundary;
- the oracle stays independent of the merge code, as it is of `connections`.

### 5. Pass 1, the light solvers and the cap are untouched

The cap and `cap_units` do not change (see above). Pass 1 and the light solvers
are not edited; they are in the benchmark gate only because pass 2 shares the
file and the step-5 regression came from an innocent-looking call.

## Performance gate

Interleaved A/B against the commit this builds on, on a quiet machine (load under
5, medians within ~2% of minimums), Apple M5, release:

- **Must not move:** `greedy_mesh/terrain`, `/solid`, `/checkerboard`,
  `light_chunk/*`, `mesh_chunk/straddling`.
- **May cost, but reported and justified:** `terrain_stairs_at_cap`,
  `all_stairs` (every shaped Voxel pays the buffering).
- **Must improve:** `terrain_fences_at_cap` and `flat_fences_at_cap`, by an amount
  that justifies the cost above. The go/no-go threshold is ticket 01's to
  propose from its measurement, and the maintainer's to accept.

## Questions for review

1. ~~**Every shaped quad, or connected arms only** (decision 2).~~ Decided by the
   maintainer: every shaped quad.
2. **Whether a ~20% line saving is worth a buffered pass 2** at all. The default
   is that ticket 01 decides, on measurement, and that "no" is fine.
3. **Redefining the cap on measured quads** would let merging buy capacity; it is
   deliberately out of scope here.

## Done when

Either ticket 01 records a no-go with its numbers, DESIGN.md's step 5 loose end
says merging was weighed and not done, and ticket 02 is `wontfix`; or ticket 02
lands with `./scripts/cargo-ci` green, the hole-free test passing over all seeds,
and the gate above reported in the PR.

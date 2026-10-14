# Blended quads in an exact order, re-sorted when it can change

Status: ready-for-agent
Blocked by: 04

Ticket 04's sort draws a Chunk's blended quads farthest centre first and
re-sorts after the Eye moves a Voxel or a sixteenth of its distance. Neither is
exact: two long quads can overlap with the nearer one's centre farther away,
and an order can be stale within the threshold.

## What to build

- Order quads by separating planes. Every quad is axis-aligned, so for two of
  them one's plane usually has the other wholly on one side; the one on the
  Eye's side of that plane is in front. Parallel quads: the farther plane is
  behind. A topological order of that relation, farthest centre first among
  quads it does not order and when it has a cycle (two greedy quads that cross,
  which only a checkerboard of two Materials makes).
- Re-sort exactly when the order can change: the relation depends only on which
  side of each quad's plane the Eye is, so a mesh re-sorts when the Eye crosses
  one of its planes and never otherwise. Replaces both thresholds.
- Keep the flat-layer skip.

## Acceptance criteria

- A ray-cast test: for the gallery's glass house and pool, and for a pair of
  long quads that centre order gets wrong, from views all around, every ray
  meets its blended layers in back-to-front draw order.
- Moving inside the same cell of the planes never re-indexes; crossing a plane
  that changes the order does.
- Sort count and worst frame on a flooded flight no worse than ticket 06's
  figures.
- `./scripts/cargo-ci` passes.

## Comments

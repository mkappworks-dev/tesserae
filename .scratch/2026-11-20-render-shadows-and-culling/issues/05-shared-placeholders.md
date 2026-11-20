# 05: Shared placeholders in size buckets (item 4)

Status: ready-for-agent
Blocked by: 01

Spec: [Shadows and culling](../spec.md).

## What to build

On Metal each mesh asset is its own indirect draw in every pass and cascade,
culled or not, and every Chunk layer has its own placeholder. Layers that
share a placeholder share a draw, with one instance each.

- Bucket sizes grow by 2^(1/4) quads. Before any GPU run, take the fixtures'
  layer quad counts and report the average padding and the bucket count;
  adjust the ratio there if they surprise.
- One render-world placeholder per bucket, built once: two-byte indices up to
  16,384 quads, four past them. A layer's entity takes its bucket's handle.
- The vertex shader reads the layer's quad count from its header and
  degenerates every quad past it.
- ADR-0034's placeholder section and its "each layer has its own placeholder"
  consequence are amended, with the numbers.
- Launch the game after the change.

## Acceptance criteria

- [ ] Tests: the bucket for a quad count; the expansion test draws a layer
      through a larger bucket and finds the padding degenerate.
- [ ] Gated by 01's method: draws per view, GPU frame and the shadow span,
      held mesh bytes and upload per frame; numbers in Comments.
- [ ] Pixel-identical captures against the commit before it.

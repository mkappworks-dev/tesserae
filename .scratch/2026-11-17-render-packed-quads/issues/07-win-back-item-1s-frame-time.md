# 07: Win back item 1's frame time

Status: ready-for-agent
Blocked by: 06

Spec: [Packed quads](../spec.md).

## What to build

Ticket 06 failed 02 on frame time: p50 +8-13% and p95 +13-26%, with the GPU's
depth prepass and shadow passes slower and main CPU p50 +11-42%. Find and fix
both costs while keeping the record contract (ADR-0030), then gate again.

- **GPU.** The placeholder is drawn non-indexed, six vertices a quad, so the
  vertex shader and its quad buffer reads run six times a quad in every pass,
  against four with the old index buffer's vertex reuse. Try drawing it
  indexed: four placeholder vertices a quad and the shared `0 1 2 0 2 3`
  pattern, so the corner is the index modulo 4. Measure the extra index bytes
  against the 84% saved.
- **Main CPU.** Not yet attributed. Profile the item 1 binary's main schedule
  against the base's (quad buffer writes and extraction, `Aabb`s, the blended
  sort, specialization) and fix what shows.
- **Coarse remeshing.** Level 1's mesh worker time rose, so all levels are
  1-10% higher; look at `level_the_water` and `split_quads` on records.

## Acceptance criteria

- [ ] What costs main CPU, with the profile that shows it.
- [ ] 02 plus this ticket against the base under ticket 06's method: no worse
      frame p50 and p95, held mesh bytes still well below the base. Numbers
      here.
- [ ] If it still fails, the numbers here and item 1 dropped, with item 3
      ported back to the old vertex output.

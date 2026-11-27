# 12: Falls drawn

Status: done
Blocked by: 11

Spec: [Water that looks natural](../spec.md), Falls drawn.

## What to build

Water pours over every generated lip that has a lower pool past it. The mesher
emits a thin blended sheet of the upper pool's water on the lip's far face, a
sixteenth off the rock, from the upper Water surface to the lower, and the lower
pool's columns beside it are flagged for foam. Vertical faces of a water
Material draw falling Flow, so a Settlement's simulated waterfall (Terra's
`waterfall` Outskirts Template) looks the same. Nothing is added to the World.

## Acceptance criteria

- [x] Meshing tests: a lip with a lower pool past it emits one sheet spanning
      the two surfaces; a lip beside dry ground emits none; the pool below is
      flagged for foam.
- [x] Captures at the river fixture and a sea-cliff Fall, shown to the user
      (amended by the user).
- [x] At the river fixture, how many frames a blended mesh waits for its sort,
      against 11, in Comments.

## Comments

- 2026-11-27, on 06215c9c plus this ticket:
  - `water_columns::fall_sheets` gives the Server's mesh job one blended quad
    per lip column with a Fall: on the lip's far face a sixteenth off the rock,
    from the lower Water surface to the upper, cut to the Chunk, in the World's
    water Material, lit as the air past the lip. The landing pool's columns
    were flagged for foam in ticket 06.
  - Every vertical water face draws falling water: streaks of the foam mask
    stretched down the wall and dragged down it at 4 m/s through the same two
    flow taps, so a Fall's sheet, a Settlement's waterfall and a poured block's
    walls look alike. Seen near, so the mask's second mip stands in for
    derivatives.
  - A profiling column, `sort_waiting`, counts the stale blended meshes a
    frame's sort budget leaves. River fixture, three rounds each (load 4.6):
    ticket 11's binary sorted 889 meshes over 8 028 frames, sort time p99
    0.118 ms; this ticket's 891 over 7 887 frames, p99 0.161 ms. Meshes waited
    only during startup's first fill, in 11, 25 and 22 frames of the three
    runs, the longest wait 23 frames in a row; none in the measured phase.
    Ticket 11 has no counter, so its waits are not known.
- Captures in `/tmp/tesserae-water-look-captures/12-after/`: river (and
  `river-yaw45`), slope, and the difference against 11 at the slope, which is
  thin bright curves at every step of the brook, the sheets and their streaks
  seen almost edge-on from above. A Settlement waterfall is not captured: no
  fixture stands at one. They wait on the user's review.
- 2026-11-27: the user accepted the sea-cliff Fall, now running down its
  ledges (ticket 31), in place of a Settlement waterfall.

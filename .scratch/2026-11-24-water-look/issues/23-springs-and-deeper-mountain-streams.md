# 23: Springs and deeper mountain streams

Status: done
Blocked by: 22

Spec: [Water that looks natural](../spec.md), Rivers. From the user's review:
streams start from nowhere, and mountain streams are too shallow.

## What to build

A stream starts where its catchment and slope together pass a threshold
(area times slope squared), only in a hollow, and its head tapers back to a
small spring pool, so it rises from a spring high on a slope. Hillslope area
is spread to every downhill neighbour (D-infinity), so planar slopes stop
drawing parallel streams. Mountain streams are 1.5 to 2 m deep in their pools.

## Acceptance criteria

- [x] Every stream begins at a spring pool or a lake's outlet, never mid-slope
      at full width.
- [x] Streams side by side a node apart are rare (amended by the user).
- [x] A mountain stream's pools are at least 1.5 m deep.

## Comments

- 2026-11-27: a stream's head node is flagged in the solve, and its channel
  widens to a 5 m spring pool there; a test checks every head stands in one.
  The narrowest brook lies 1.6 m deep, so its band is up to 8 m across.
  Parallel streams: a node carries a stream only once two or more nodes drain
  into it, or once its catchment is four times a stream's, so short brooks
  no longer start side by side down an even slope. Long parallel pairs a
  node apart remain on the 1 km map (`25-map-before-after.png`, bottom
  left); a D-infinity spread is still open for them.
- 2026-11-27, parallel pairs. A stream beside another a node away that stands
  no higher joins it (the larger of two), once per line per round, and the
  catchment is counted again between up to four rounds; joining at every
  touch drew zigzags. Only a node the flood reached first can be joined, so
  water never runs uphill and no loop forms. On 8 worlds of the default
  landforms, stream nodes beside another: 9.2% to 1.4%. Test
  `few_streams_run_side_by_side`; map
  `/tmp/tesserae-water-look-captures/23-map/before-after.png`. D-infinity was
  not used: on an even slope it gathers every line alike, which is what drew
  the pairs.
- 2026-11-27: the user amended the criterion to "rare": 1.4% of stream nodes,
  test `few_streams_run_side_by_side`.

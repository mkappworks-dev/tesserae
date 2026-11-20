# 08: A lone Chunk decides its neighbours' Plants only on the strip it needs, if it is worth it

Status: wontfix

Blocked by: 05

Spec: [Generation cost](../spec.md).

## What to build

First the gate: in the flight, the share of level-0 generation spent building column caches and Plants for Cells no level-0 Chunk stands over. At 10% or less, closed `wontfix` with the figure: in a flight, a spawn and a teleport, the next Chunks of the same ring ask for those Cells within milliseconds. Otherwise a neighbour Cell that is not held has its Plants decided only on the strip within a tree's reach and crowding of the Chunk, from Biomes asked point by point, as `surface_height_m` already answers an unheld Cell; a held Cell keeps the cached path.

## Acceptance criteria

- [x] The gate's figure in Comments.
- [ ] ~~If built: a tree reaching across a Chunk border is the same from both Chunks whether the neighbour Cell is held or not, in any order.~~
- [ ] ~~If built: the fingerprint and the committed digests match; bench figures in Comments.~~
- [x] The end flight figures and sampled symbols, against 01's, in Comments, whichever way the gate goes.

## Comments

### The gate, 2026-11-20

Every column cache level 0 builds is 1.5% of its generation in the sampled flight of fa6fd574 (`Worldgen::build_columns` under a level-0 Chunk), so the caches built for Cells no level-0 Chunk stands over are less than that, under the 10% gate: closed. A lone Chunk, as at spawn or after a teleport, still builds its neighbours' caches, which the next Chunks of the ring then read.

### The end figures, 2026-11-20

This spec's tickets are all done or closed; against its starting commit:

| Fixture | Level | Stage | 4e78a50f | Mid-point (fded14c8) | After 05 (fa6fd574) | After / start |
| --- | --- | --- | --- | --- | --- | --- |
| `flight` | 0 | generate | 184.6 | 58.1 | 46.6 | 25% |
| `flight` | 0 | relight | 91.3 | 102.4 | 102.0 | 112% |
| `flight` | 0 | mesh | 27.5 | 27.6 | 30.7 | 112% |
| `flight` | 1 | generate | 251.4 | 220.9 | 199.8 | 79% |
| `flight` | 1 | mesh | 274.9 | 277.6 | 302.6 | 110% |
| `flight` | 2 | generate | 49.1 | 34.1 | 29.5 | 60% |
| `flight` | 3 | generate | 8.5 | 4.6 | 4.5 | 52% |
| `flight` | 0–3 | generate | 495.0 | 317.3 | 281.7 | 57% |
| `sprint-flight` | 0 | generate | 568.4 | 247.0 | 202.2 | 36% |
| `sprint-flight` | 0 | relight | 405.6 | 402.4 | 449.0 | 111% |
| `sprint-flight` | 0 | mesh | 38.9 | 39.7 | 44.4 | 114% |
| `sprint-flight` | 1 | generate | 797.4 | 612.6 | 537.2 | 67% |
| `sprint-flight` | 1 | mesh | 151.9 | 184.4 | 199.2 | 131% |
| `sprint-flight` | 2 | generate | 258.6 | 165.2 | 151.1 | 58% |
| `sprint-flight` | 3 | generate | 18.9 | 9.5 | 9.4 | 50% |
| `sprint-flight` | 0–3 | generate | 1640.2 | 1028.3 | 899.6 | 55% |
| `teleport` | 0 | generate | 469.5 | 371.3 | 337.5 | 72% |
| `teleport` | 0 | relight | 89.6 | 89.9 | 98.0 | 109% |
| `teleport` | 0 | mesh | 34.8 | 33.2 | 37.5 | 108% |
| `teleport` | 1 | generate | 493.6 | 370.5 | 345.2 | 70% |
| `teleport` | 1 | mesh | 132.7 | 137.6 | 146.2 | 110% |
| `teleport` | 2 | generate | 428.2 | 266.3 | 255.7 | 60% |
| `teleport` | 3 | generate | 63.8 | 30.0 | 33.6 | 53% |
| `teleport` | 0–3 | generate | 1453.4 | 1045.6 | 970.7 | 67% |

The teleport's level 0 settles in 2.05 s (1.89 to 2.14) after 05, 2.11 s at the mid-point and 2.53 s (2.49 to 2.65) at the start.

How they were taken: builds of 4e78a50f, fded14c8 (the mid-point) and fa6fd574 (this spec's tickets 01 to 05 and the sister spec's 01, 02 and 04), each `RUSTFLAGS="-D warnings" cargo build --locked -p tesserae --features profiling --release`, copied out; one `scripts/profile-frames --fixtures flight sprint-flight teleport --repeats 3` series per build per round, nothing else building or running a `tesserae` process. Three rounds of 4e78a50f and fded14c8 (7:55 to 10:10) and two of fa6fd574 (10:10 and 10:41), interleaved with them; medians over the logging-on runs, worker CPU in ms per second of the fixture. `uptime` load from 3.86 to 7.03 throughout.

Level 0's generation is 25% of the start's at 12 m/s and 36% at 64 m/s, the coarser levels' 70% to 80%. Top symbols inside generation, sampled, at fa6fd574:

| Symbol | Level 0 | Levels 1–5 | All |
| --- | --- | --- | --- |
| `Worldgen::cavern_open` | 9.5% | 8.1% | 8.5% |
| `Worldgen::claims_near` | 21.9% | 0.0% | 6.6% |
| `Worldgen::column_cache` | 1.8% | 8.5% | 6.5% |
| `Worldgen::build_columns` | 1.5% | 8.0% | 6.0% |
| `CaveBiomeCells::get` | 2.3% | 7.2% | 5.7% |
| `Drainage::wet` | 12.8% | 2.3% | 5.5% |
| `Terrain::columns_x4` | 1.2% | 6.3% | 4.8% |
| `CaveField::carves_near` | 2.3% | 4.4% | 3.8% |
| `Worldgen::crossing` | 11.6% | 0.0% | 3.5% |
| `Worldgen::surface_height_m` | 8.8% | 0.6% | 3.0% |
| `Worldgen::carved` | 8.8% | 0.1% | 2.7% |
| `Worldgen::plants_on` | 8.9% | 0.0% | 2.7% |

# 04: Map tiles reach the Client

Status: done

Blocked by: 03

Spec: [Phase 19](../spec.md).

## What to build

The Server tracks which Map tiles changed since they were last sent and sends
them as a message under a per-step budget; the Client keeps a cache of them and
never reads Chunk storage. A tile carries Material ids, not colours, so a later
Look pack cannot make a saved Map stale: the Client colours a sample from the
`MaterialPalette` it already receives, which needs a Client-side palette
resource because today the palette only reaches a shader uniform. At session
start, and after a reopen, every tile of the Map is marked changed and trickles
out under the same budget, so a long journey never stalls the first frame.

The Client's cache holds the whole Map: the Map has its own size cap in tiles
(generous, about a 4 km square), past which the Server records no new tile, so
the cache needs no eviction and the full-screen Map never loses explored ground.

Nothing is drawn yet; this is verified at the message seam.

## Acceptance criteria

- [x] A Player who explores causes only the changed tiles to be sent, under a per-step budget; a test with many changed tiles shows the budget holds and all arrive over several steps.
- [x] At session start and after reopen the whole Map is sent under the budget and arrives within a bounded number of steps.
- [x] Tiles carry Material ids; the Client resolves colour from the palette message and the Server holds no colour.
- [x] The Map's size cap holds (the Server records no new tile past it) and the Client cache never evicts.
- [x] The Client never reads Chunk storage.
- [ ] Client and Server tests at the message boundary; fmt, clippy and tests pass. (clippy and the full suite deferred to phase end)

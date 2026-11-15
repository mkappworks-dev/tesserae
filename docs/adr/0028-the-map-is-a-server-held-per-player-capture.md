# The Map is a Server-held, per-Player capture, never regenerated or read by the Client

Status: accepted.

Step 19 needs a map. The terrain is a pure function of the seed and a position
([ADR-0022](0022-biomes-are-column-content-and-a-world-pins-its-generator.md)),
so the obvious design is to regenerate any region on demand and draw it. We
capture instead.

**The Server records what a Player has seen.** Near the Eye, for columns whose
Chunks are held and lit, it stores per column sample the surface height, the top
Material and the Biome. The sample edge is a fixed number of metres, so the Map
is the same at every Voxel size ([ADR-0003](0003-voxel-size-is-per-world.md)).
Unknown terrain is never marked
([ADR-0008](0008-the-server-owns-the-players-pose.md)), so the Map has no
guesses in it.

**Regeneration was rejected for three reasons.** It would hide every Edit: a
house, a dug lake or a felled forest would not appear. It cannot show a region
the Player has not visited without making the whole World knowable, which
removes exploring. And it would tie the Map's colours to the generator revision,
so a later one would redraw a Player's Map.

**The Client never reads World storage.** It receives Map tiles as Server
messages under a per-step budget and keeps a cache. Waypoints and the Explored
record save per Player with the World. A save without a Map loads with an empty
one, so no Generator version moves.

**Marker kinds are Declaration data.** A Game declares name, icon and colour; no
Mod is called and no host call is added ([ADR-0001](0001-wasm-mod-boundary.md),
[ADR-0012](0012-mods-declare-once-and-command-in-batches.md)). A sample's colour
comes from its top Material's existing declared colour.

**Cost accepted.** About four bytes per sample, grown only by exploring, and a
per-step scan budget. Stale tiles stay stale until the Player returns.

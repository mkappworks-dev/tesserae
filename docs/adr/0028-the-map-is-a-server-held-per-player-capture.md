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

## Amendment 2026-11-15: what the build settled

Decisions made while building step 19, all on the unreleased 0.3 contract.

- **The heading is published in the pose.** `PoseCorrection.yaw` carries the
  Player's heading, so the Compass and the minimap's arrow read what the Server
  holds and agree in first and third person
  ([ADR-0008](0008-the-server-owns-the-players-pose.md)). North is the World's
  -Z axis at every Voxel size.
- **A tile carries ids, not colours.** A sample is a height, a Material id and
  an optional Biome id, over 64 by 64 samples of 2 m (a 128 m tile), reaching 96
  m from the Eye. The Client colours it from the palette of declared Material
  colours, so a Mod reload recolours the Map and the Server never sends a
  colour.
- **The Map saves as its own section,** like Containers, not a Player extension.
  A tile is six deflated byte planes (heights as the change from the sample
  before) in base64, and a Map revision counter gates re-encoding so a quiet Map
  costs a comparison. A save without the section opens with an empty Map; no
  format or Generator version moves. Measured on a synthetic 2 km square of
  rolling ground (256 tiles, five Biomes): 4,194,304 bytes raw, 246,451 as JSON,
  about 5.9 percent. A plain run-length encoding was tried first and saved only
  about half. A reopened Map queues every tile, so the budgeted sender trickles
  it to the Client.
- **The Client cache holds the whole Map.** The Server's cap of 1024 tiles (8 km
  of ground in a square, about 16 MB) bounds it, so the cache never evicts.
- **A Marker kind's icon is a closed enum** (`home`, `camp`, `mine`, `farm`,
  `landmark`, `death`, `flag`, `pin`, `star`) the Engine draws as shapes, so the
  Declaration holds no glyph or texture. A Mod declares up to 16 kinds. The
  death kind is optional: a Game names one with `death-marker-kind`, and without
  it a death leaves no Waypoint.
- **The Map policy** is `map-policy` in the contract and `[world.map]` in a
  Game's manifest. `enabled = false` forbids the Map and the Compass; absent
  permits them. While forbidden the Server neither scans nor accepts Waypoint
  intents, the Client draws and opens nothing, and the saved Map is kept.
- **Waypoint intents are Add and Remove,** judged on the Player's own Map: the
  point must be Explored, the kind declared, the name valid and the list under
  64 Waypoints. A refusal carries a reason. The death Waypoint is the Server's
  and cannot be removed by an intent.
- **Known simplifications.** The Map window's Biome tint is derived from the
  Biome's index, not declared by the Game. The Map view draws Waypoint icons
  with no name labels; names show in the Waypoint list.
- **Cost.** The scan budget per step, the tile budget per step to the Client, up
  to 16 MB of samples in memory and, saved, roughly a twentieth of that.

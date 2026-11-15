Status: ready-for-agent

# Phase 19: Map, compass

DESIGN.md roadmap step 19: Server-provided summaries (the Client never reads
World storage) and a map view. Depends on step 13 (Biomes, done). The Engine
adds the mechanisms; a Game or Mod supplies Marker kinds.

Vocabulary: **Map**, **Map tile**, **Explored**, **Waypoint**, **Marker kind**,
**Compass** and **North** are defined in [GLOSSARY.md](../../GLOSSARY.md), and
the decision is
[ADR-0028](../../docs/adr/0028-the-map-is-a-server-held-per-player-capture.md).
Related decisions:
[ADR-0001](../../docs/adr/0001-wasm-mod-boundary.md),
[ADR-0003](../../docs/adr/0003-voxel-size-is-per-world.md),
[ADR-0008](../../docs/adr/0008-the-server-owns-the-players-pose.md),
[ADR-0012](../../docs/adr/0012-mods-declare-once-and-command-in-batches.md),
[ADR-0016](../../docs/adr/0016-only-air-is-built-in.md),
[ADR-0022](../../docs/adr/0022-biomes-are-column-content-and-a-world-pins-its-generator.md)
and
[ADR-0025](../../docs/adr/0025-sounds-are-declared-data-the-server-names-and-the-client-plays.md).
The approved UI is the step 19 mock in [`mockup/`](../../mockup/README.md),
reachable at `/map` and in the Gameplay HUD.

## Problem Statement

A Player has no way to know where they are. The World has Biomes, lakes and
caves sized in metres, but the only view of it is what the camera sees. There
is no heading, so "which way is home" cannot be answered; there is no record of
what has been explored, so a trip cannot be retraced; and nothing can be
marked, so a base, a mine or a place of death is remembered only by the Player.
The Client must not read World storage to draw any of this (ADR-0002), so a map
cannot be a view the Client builds for itself.

## Solution

The Server keeps a **Map** for each Player: as the Player explores, it records
the surface of the ground near the Eye that is loaded and lit, one sample every
2 m, as the surface height, the top Material and the Biome as they were when
seen. It sends changed **Map tiles** to the Client, which caches them, and
saves the Map with the World. Unloaded terrain is never marked, so the Map
holds no guesses, and it shows a Player's builds and lakes as of their last
visit (ADR-0028).

The Client presents it three ways, each a user setting and none World state:

- A **Compass**, a strip across the top of the HUD, off by default, showing the
  Player's heading, the cardinal directions and the bearing of each Waypoint.
- A full-screen **Map**, opened with a key, with fog of war, Biome tint and
  height shading, pan and zoom from 2 to 16 m per pixel, a hover readout, and
  the Waypoint list.
- A **minimap**, off by default: a small north-up Map centred on the Player.

The Compass and the minimap each have a key that toggles them in play, and a
switch in Settings; the full-screen Map has its own key.

A **Waypoint** is a named point on a Player's Map with a **Marker kind** a Game
declares as data (a name, an icon and a colour). The Server owns Waypoints and
adds one where a Player died. The Engine ships no Marker kind: a World with no
Game has a Map and a Compass but no Waypoints. A Game or World parameter can
forbid Map and Compass; they are permitted by default and need no Item.

## User Stories

The Compass

1. As a player, I want a strip along the top of the screen showing my heading in
   degrees, so that I know which way I am facing.
2. As a player, I want the four cardinal letters, with North picked out, so
   that I can navigate without reading numbers.
3. As a player, I want the compass to turn smoothly as I look around, so that
   it feels attached to my view.
4. As a player, I want North to be the same direction in every World, so that
   what I learn in one carries to the next.
5. As a player, I want the compass to show my heading in third person as well,
   so that the view mode changes nothing about it.
6. As a player, I want a Waypoint to appear on the strip at its bearing, so
   that I can walk towards it.
7. As a player, I want a Waypoint that is off to the side to rest at the edge of
   the strip, dimmed, so that I know which way to turn.
8. As a player, I want the name and distance of the Waypoint I am facing, so
   that I know what I am walking towards and how far.
9. As a player, I want the compass off until I turn it on, with a key and a
   setting, so that my screen stays clear and I can show it when I need it.
10. As a player, I want the compass hidden while a menu or window covers the
    World, so that it never floats over a panel.

The Map

11. As a player, I want to open a full-screen Map with one key, so that I can
    see where I have been.
12. As a player, I want Escape or the same key to close it, so that I return
    to play at once.
13. As a player, I want ground I have not explored to be fogged, so that
    finding a place means going there.
14. As a player, I want explored ground tinted by its Biome and shaded by its
    height, so that I can read forests, plains and hills at a glance.
15. As a player, I want lakes drawn as water, so that I can see where I cannot
    walk.
16. As a player, I want a house or a lake I made to show on the Map as it was
    when I last stood there, so that the Map reflects my own work.
17. As a player, I want my own marker, drawn as an arrow pointing the way I
    face, so that I can orient the Map to the World.
18. As a player, I want to drag to pan, so that I can look at any explored
    ground.
19. As a player, I want to zoom between 2, 4, 8 and 16 metres per pixel, so
    that I can see a stretch of road or a whole region.
20. As a player, I want a scale bar in metres, so that distances are readable.
21. As a player, I want a "Centre on me" control, so that I can find myself
    after panning.
22. As a player, I want a readout of the ground under the pointer, with its
    position, height, Biome and distance from me, so that I can plan a route.
23. As a player, I want the readout to say "Not explored" over fog and name no
    Biome there, so that the Map never gives away what I have not seen.
24. As a player, I want a legend of the Biomes I have seen, so that colours
    mean something.
25. As a player, I want to see how much I have explored, so that exploring
    feels like progress.
26. As a player, I want a Map that works the same at every Voxel size, so that a
    fine World does not change what the Map shows.

The minimap

27. As a player, I want an optional small map in the corner, centred on me and
    north up, so that I can navigate without opening the Map.
28. As a player, I want to choose its size, so that it fits my screen.
28b. As a player, I want a key that toggles the minimap, so that I can show it
    only while I navigate.
29. As a player, I want it to name the Biome I am in and show my position, so
    that I know where I am at a glance.
30. As a player, I want a Waypoint beyond its edge to rest on the edge, so that
    I know which way it lies.

Waypoints

31. As a player, I want to place a Waypoint at my own position, so that I can
    mark where I am standing.
32. As a player, I want to place a Waypoint by clicking explored ground on the
    Map, so that I can mark a place I can see.
33. As a player, I want a refusal when I click fog, so that a Waypoint is never
    set on ground the Map knows nothing about.
34. As a player, I want to give a Waypoint a name and a Marker kind, so that I
    can tell my base from my mine.
35. As a player, I want a default name if I give none, so that adding one takes
    a single click.
36. As a player, I want to select a Waypoint in the list or on the Map and
    centre the Map on it, so that I can find it.
37. As a player, I want to remove a Waypoint I made, so that the Map stays
    tidy.
38. As a player, I want a Waypoint to be mine alone and to survive saving and
    reopening, so that my markers are there next session.
39. As a player, I want a marker where I died, so that I can go back for what I
    dropped.
40. As a player, I want the death marker to stay until I choose to remove it
    or die again elsewhere, so that I do not lose it by accident.

Saving and games

41. As a player, I want my Map to save with the World, so that exploring is not
    lost when I quit.
42. As a player, I want a World saved before Maps existed to open with an
    empty Map, so that no old save is lost or changed.
43. As a player, I want opening the Map to be quick even after a long journey,
    so that a big Map does not stall the game.
44. As a Game author, I want to declare Marker kinds as data, so that Terra can
    offer Home, Camp, Mine, Farm and Landmark without code.
45. As a Game author, I want to name which Marker kind marks a death, so that
    my Game decides what a death looks like.
46. As a Game author, I want to turn Map and Compass off for a World, so that a
    Game about being lost can be lost.
47. As a Mod author, I want a Marker kind to validate like any other row, with a
    clear error naming the Mod and the row, so that I can fix a mistake.
48. As a Mod author, I want Marker kinds to need no code and no host call, so
    that the Mod boundary is unchanged.
49. As an Engine maintainer, I want the Client never to read World storage and
    to receive only Server messages, so that the Server stays the authority.
50. As an Engine maintainer, I want the Map's scan bounded in cost per Logical
    step, so that exploring never stalls the tick.

## Implementation Decisions

**Data and the Server**

- A new Map module in the Bevy-free core crate holds the types and rules: the
  sample, the Map tile, the Explored test, the per-Player Map and its compact
  encoding. Keeping it in core keeps it headless and benchmarkable (invariant
  3).
- A sample is 2 m on a side and stores the surface height, the top Material id
  and the Biome id as of when it was seen. A Map tile is a fixed square of
  samples, in metres, independent of Voxel size (ADR-0003). It is not a Chunk
  and not a Cell.
- A new Server system scans, each fixed step after the Player's body has been
  advanced, columns within a fixed reach (96 m) of the Eye whose Chunks are
  held and lit, using the same "held and not unlit" test the Soundscape and
  Plants systems use. It never marks a column whose Chunk is unloaded. It scans
  at most a bounded number of samples a step, with a cursor, in the manner of a
  Rescan, so exploring never stalls the tick.
- A sample is read from the held level-0 Chunks, so a placed Voxel or a dug lake
  is what the Map records. The Biome comes from the World generator's column
  query, which is pure in the seed and a position in metres.
- The Map tile is the unit of change. The Server tracks which tiles changed
  since they were last sent and sends them as a message under a per-step
  budget, in the manner of the Server's other changing datasets. The Client
  keeps a cache and never reads Chunk storage.
- The Server has no Material colour today: appearance reaches the Client once
  as a palette. The Server keeps the declared colour of each Material, from the
  appearance it already loads, so a sample's tile can be sent with a colour. The
  Map stores the Material id, not a colour, so a later Look pack cannot make a
  saved Map stale.
- The Server publishes the Player's heading. Today yaw lives in the Client's
  camera and goes up as intent, and the Server keeps it privately. The
  published pose gains the Player's look heading, so the Compass reads a
  Server-published value as ADR-0008 describes. North is −Z, the direction the
  engine already treats as forward at heading zero. A bearing is measured
  clockwise from North.
- Waypoints are Server-owned, per Player. The Client sends an add intent
  (position, name, Marker kind) and a remove intent; the Server answers with a
  refusal and a reason, as it does for other intents, and a refused add names
  its reason (fog, no such Marker kind, too many, a bad name). Waypoints are
  validated against the Player's own Map: a point must be Explored.
- The Server adds a Waypoint where a Player dies, of the Marker kind the Game
  names for death. A Game that names none gets no death marker. A new death
  replaces the previous death marker.

**Saving**

- A Player's Map is large and grows only by exploring; it does not ride on the
  per-frame JSON capture that the small per-Player records use. It saves as its
  own section of the World's save, in the manner of Containers: a compact,
  tile-granular encoding, a revision counter compared to decide whether the
  section is dirty, and a load that starts empty when the section is absent.
  No Generator version moves and no save format version changes: a save without
  a Map opens with an empty one.
- Waypoints are small and save as a per-Player record in the usual way.

**Declaration**

- The unreleased 0.3 Declaration gains Marker kinds: a name, an icon and a
  colour. The icon is one of a closed set of names the Engine draws as shapes,
  not a glyph, so no font has to carry it. The Declaration may name which Marker
  kind marks a death. This is data only: no Mod is called and no host call is
  added (ADR-0001, ADR-0012). Older contract adapters supply none, and a World
  with no Game has no Marker kinds (ADR-0016).
- Validation follows the existing table pattern: a row cap and a name rule per
  Mod, an error naming the Mod, the row and the reason, and a final resolve
  after every Mod has declared. A death kind must name a declared Marker kind.
- The World parameters gain an optional Map policy, in the manner of the
  movement policy: absent means on, adapters for older lines supply none, and
  the Server tells the Client whether Map and Compass are enabled.
- Terra declares Home, Camp, Mine, Farm, Landmark and a death kind. The Marker
  kinds travel to the Client once at session start, as sounds do.

**The Client**

- The Compass is a HUD element in the manner of the hotbar: a strip rebuilt from
  the published heading and the Waypoints, hidden while a menu or window covers
  the World or the Player is dead. The ticks, the cardinal letters and the
  Waypoint marks follow the approved mock.
- The Map is a window in the manner of the Inventory window, opened by a new
  action. It must be added to the set of things that cover the World. It has the
  pointer readout, the zoom levels 2, 4, 8 and 16 m per pixel (downsampling the
  cached tiles on the Client), the scale bar, the legend of Biomes seen, the
  Waypoint list and the new-Waypoint form.
- The Map key is `N`. `M` already cycles Tools in the engine, and a default
  binding on an existing key would silently lose to it. The Map is also a
  button in the in-game Menu and appears in Key bindings, the help text and the
  controls documentation.
- Two more actions toggle the Compass (`B`) and the minimap (`O`), both free in
  the engine's defaults. Each flips its preference at once, persists it, and is
  listed in Key bindings, the help text and the controls documentation.
- The minimap is a small north-up view centred on the Player, at 2 m per pixel,
  of three sizes.
- The Compass, the minimap and its size are user preferences, saved with the
  others and defaulting to off, off and medium. A profile saved before them
  reads as the defaults. They live in Settings under Display, in a "Map and
  compass" group, as in the mock.
- Biome names come from the list the Server already sends at session start;
  a sample's Biome id indexes it.
- The Client keeps a cache of Map tiles and draws from it. It never reads Chunk
  storage and sends only intents (invariant 1).

**Documentation**

- ADR-0028 is amended with what the build settled: the published heading, the
  separate save section, and the closed icon set. GLOSSARY.md already defines
  the terms. The DESIGN.md roadmap row, the architecture notes, the contract
  compatibility notes, the data-mod and authoring docs and the controls
  documentation are updated.

## Testing Decisions

A good test checks external behaviour at a seam the code already has, not the
shape of a helper. The seams, highest first:

- **The Server and Client message boundary.** Server systems are tested by
  building a small World in the Server's chunk store, a Player at a known
  position, running the system and asserting the messages and resources that
  come out, as the Soundscape and Plants tests do. Client windows and HUD
  elements are tested by injecting those same Server messages into a minimal
  app and querying the UI, as the hotbar and vitals tests do. This one seam
  covers capture, sending, intents, refusals and the HUD without a window.
- **The core Map module,** as plain unit tests: Explored rules, the compact
  encoding round-trip, tile arithmetic in metres, and that the same World at
  two Voxel sizes yields the same Map. No Bevy, as the crate's CI check
  requires.
- **A save round trip,** through the persistence rig: explore, save, reopen and
  read the Map and Waypoints back; a save without a Map section opens empty.
- **The Declaration,** through the Mods crate's data and declare tests: a valid
  Marker kind row, each rejection, and that older adapters supply none.

What to test, not how it is built: that unloaded terrain is never marked; that
the scan is bounded in a step; that a placed Voxel appears on the Map at the
next visit; that Voxel size does not change a Map; that a refused Waypoint
names its reason; that a death adds one marker and replaces the last; that the
Compass is hidden under a covering window; that the Map key opens and closes
the window.

Prior art: the Soundscape, footfall, Plants, hotbar, vitals and settings tests,
the persistence rig's reopen tests, and the Mods crate's data tests. The final
review, at the end of the build, is a code review of the whole branch and a run
of the test suite, formatting and lint checks.

## Out of Scope

- A cave, depth or vertical-slice view. The Map is the surface only.
- Live Entities on the Map: Dropped items, other Players, and mobs.
- Sharing a Map or Waypoints between Players.
- A map Item that must be held or crafted: gating by Item is Game content for
  later.
- Mod-placed markers or a Mod callback of any kind.
- Regenerating terrain on the Map from the seed, or revealing unexplored
  terrain by any means.
- Rotating the minimap with the heading, a circular minimap, and a Waypoint
  limit.
- The placement ghost toggle and swimming animation requested alongside this
  phase: they are unrelated Client changes made on the same branch.

## Further Notes

- At 4 bytes a sample and 2 m spacing, a 2 km square is about 4 MB raw. The
  save section must use a compact, tile-granular encoding so a long journey
  does not slow every save, and the Client cache must be bounded.
- The mock draws Marker kind icons as glyphs (⌂ ▲ ◆ ✿ ★ ✖). They stand for the
  closed icon names the Engine draws as shapes.
- The mock's terrain is noise. The Client draws the Server's tiles.
- The heading sign convention in the Server is that a positive yaw turns left,
  with yaw 0 facing North; a bearing clockwise from North is the negative.
- Do not share a Cargo target directory across worktrees, and build with the
  workspace form and `RUSTFLAGS="-D warnings"` so the Bevy cache is reused.

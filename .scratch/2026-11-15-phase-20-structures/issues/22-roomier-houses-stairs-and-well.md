# 22: Roomier houses, stairs, doors and a working well

Status: done

Blocked by: 17, 21

Spec: [Phase 20](../spec.md).

## What to build

The user reviewed the Games' Settlement buildings in play and found these
problems. The shapes come from the mockup's builders (`mockup/structures.js`),
exported by `mockup/export-templates.mjs`, so every fix lands in the mockup
first and is then exported.

- **Small houses are cramped.** Measure each kind against 2.5 m clear inside
  each storey and 2 m clear floor beyond each piece of furniture, and enlarge
  only the kinds that fail: the cottage and other one-room or small houses.
  Leave the inn, warehouse, barn, farmhouse and the other kinds that pass at
  their size.
  - Keep each kind's own shape and Materials.
  - Keep every Template within the Settlement radius and within the undo
    budget at 0.25 m, which is 131,072 changed Voxels.
- **Upper levels cannot be reached.** Add a stair only to a Structure with an
  upper level or a cellar a Player is meant to stand in, in Settlements and in
  the wild: built from the Game's stair Shape in a wall Material, with a
  matching opening in the floor above. No stair to a roof void, an attic with
  no floor, a single-storey building or an open-fronted shed.
- **Entrances.** A building meant to be entered with no way in gets a door, on
  the side the Settlement turns to the road. A big enclosed building with an
  open way gets a double door, two leaves side by side; a smaller one a single
  door. Only what is meant to be open stays open. Every door has a full-height
  opening, a lintel and a floor level with the ground outside.
- **Doors are too small.** Make every Game's Door wider and taller, on whole
  Template cells so the opening fits it at 1.0, 0.5 and 0.25 m, with room round
  a Player's 0.6 by 1.8 m body. Player-placed Doors change the same way.
- **Hinges.** The mockup says each Door cell's hinge, `left` or `right` seen
  from outside, and the export turns it into the cell's Orientation: a single
  Door hinges away from the nearer corner, a double Door's leaves on their outer
  edges. The authoring guide explains choosing a hinge.
- **The well.** Its posts stand on the well's base, with no gap under them. It
  holds water from its floor to about 1 m below its rim, in every Game.
  - The water is a Template cell of the Game's water Material, placed as a
    source.
  - If the loader or the Obtainable check refuses a fluid cell, fix the rule,
    not the content, and record the fix.
- Settlement radii and spacings grow only as far as the larger buildings need.

## Acceptance criteria

- [x] Every kind that failed the headroom and clearance rule is larger, in the
      mockup and in every Game, and every house storey keeps 2.5 m clear. The
      mockup export check passes.
- [x] A test walks from each building's door to every level it is meant to
      reach, over Voxels a Player can stand on and step up, with no gap over 1
      step.
- [x] Every building meant to be entered has a door that opens a way out to
      the edge the Settlement turns to its road, and doors face the path in a
      generated World.
- [x] A Player's body walks through each leaf of an open single and double
      Door at 1.0, 0.5 and 0.25 m, through the Server's collision.
- [x] Each well's posts rest on its base, and its shaft holds water, tested
      through the Game's Templates; a Template's water lands as sources that
      the flow keeps.
- [x] Every shipped Template still fits the undo budget at 0.25 m, tested.
- [x] The Settlement tests, the benchmark margin and the Obtainable check
      still pass.

## Comments

Sizes of the walls, without the roof's overhang (cells of 1 m):

- Cottage: 6 x 5 m to 8 x 7 m. It was 4 x 3 m inside, with no room round its
  table, chest and bench.
- Family house: 7 x 6 m to 9 x 8 m. It was 5 x 4 m inside, with no room for a
  stair.
- Bakery: 7 x 6 m to 9 x 8 m. Its counter stood 1 m from the door wall.

The inn, warehouse, barn, farmhouse, smithy, stall and well keep their size.
Every storey already had 3 m clear (a storey is 4 cells: a floor and three), so
STOREY stays 4. The cellars grew from 3 to 4 cells deep, for 3 m clear: the
farmhouse, inn, watchtower, observatory and both mound crypts.

Levels each kind is meant to reach, in metres above the bottom of the Ground
layer (`server/mods/demo_games/rooms.rs` lists them; a Template with a door
that is not listed fails the test):

- Cottage, bakery, warehouse, barn: ground floor (1).
- Family house: ground (1) and upper storey (5). Inn: 1, 5 and cellar (-3).
  Farmhouse: 1 and cellar (-3).
- Watchtower: 1, roof deck (12), cellar (-3). Lighthouse: 1, deck (18).
  Observatory: 1, deck (10), cellar (-3). Glass dome: 1.
- Stilt house: its floor (4), up an outside stair. Treehouse: 5, likewise.
- Barrow and burrow: the portal (1) and the crypt (-3).
- Smithy and stall: open, no door, not walked. The well has no inside.
- Not given a stair: every roof void, the barn's hay loft (no floor; its hay
  door stays a high opening), the warehouse and barn halls.

Doors, per kind:

- Double door, two 2 m leaves: the inn, warehouse and barn of every Game (the
  harbour inn, ember inn, starlight inn and tavern; net loft, ore store,
  crystal vault and granary; boathouse, basalt hall, glass hall and mill barn).
- Single door: cottage, family house, farmhouse, bakery, the towers, the glass
  dome, the stilt houses, and the mounds' new stone portals. The farmhouse's
  door moved to the wing's end wall: on the main front, its nearest edge was
  the side, so the Settlement turned the side to the road.
- Kept open on purpose: the smithy's lean-to working front and the market
  stall's counter. Not entered: the ruin, rings, arches, spires, walls, altars,
  pit, dock, bridges and Lumen's geode hollow.

Door size: every Game's Door is now 2 m wide and 3 m tall (Terra's in
`games/terra/src/lib.rs`, the others through `demo_kit::building`'s
`DOOR_WIDTH_M` and `DOOR_HEIGHT_M`). The example 1.5 by 2.5 m does not sit on
1 m Template cells; 2 by 3 is the nearest whole-cell size above 1 by 2. A Door
wider than one Voxel already lets bodies through when open
(`VoxelUses::passes_bodies`), so the engine is unchanged; the test sweeps a body
through each leaf. A Player-placed Door takes the same size, so placing one now
needs a 2 by 3 m opening. Terra's Gate stays 1 by 1.

Choosing a Door's hinge (also in the authoring guide):

- A Player hinges a placed Door on the half of the cell they aim at.
- A Template hinges each Door cell by its orientation byte. Upright, the hinge
  is on the side its own +X turns to; upside down (4 to 7) the front stays and
  the hinge swaps. With the panel on +Z, 0 hinges on +X and 6 on -X. The
  Structure's turn turns both. The mockup's `doorway()` gives each cell
  `hinge: "left" | "right"`, seen from outside, and the exporter writes the
  byte.

The well: its shaft's floor is a cell lower, water fills three cells as
sources, and the surface stands 1 m under the ring's top (1.5 m under the slab
lip). The corner posts start on the ring's top cubes, not on slabs.

Rule fixes:

- The loader refused every fluid Material in a Template. A fluid cell is now a
  whole cube with no Shape or orientation, and reads as a source wherever it
  lands, at every Voxel size; a shaped fluid is still refused, and foundations
  and paths still refuse fluids.
- The Obtainable check skips fluid cells. Lumen and Meadow had no water, so
  they now depend on `waters`. Their wells are their only water, and a source in
  a well can be taken as a water Item, by design.

Other changes: Terra's town needs one cottage, not two: two of the larger
cottages at the head of the queue left no room for the other minimums, so no
town stood at the test's spot. No radius or spacing grew for this ticket. Meadow's and Lumen's
texture fingerprints are re-pinned because `waters` brings its obsidian texture
into their Worlds; nothing a Game draws changed. The tests' load orders and
dependency lists for Lumen and Meadow gained `waters`.

The largest Template at 0.25 m is the inn, 92,736 Voxels of 131,072.

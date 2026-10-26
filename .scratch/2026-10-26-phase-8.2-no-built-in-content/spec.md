# Step 8.2: the engine ships no named content but air

Status: done

Drafted 2026-10-26 on `feat/phase-8.1-game-content` at 3f41695 as a second
part of step 8.1, and moved to `feat/phase-8.2-no-built-in-content` at 212d900
once step 8.1 merged (PR #20). Approved by the user on 2026-10-26. A push or
PR is separate authorization.

Decided with the user on 2026-10-26:

- This is a step of its own, on its own branch, after step 8.1 merged.
- A World with no Game is empty.
- The ground's Materials are named on contract line 0.3, which is on `main` but
  in no release, so no 0.4 line is added.
- A 0.2 or 0.1 Game's World has no ground and logs a warning saying why.
- The hotbar opens with the ground rock in four Shapes, then the other declared
  solids.
- The gallery picks its Materials by property.
- The demo declares stone, dirt, grass, glass and leaves, and its lamp stands
  in for the lantern.
- Test fixtures return their ids from a register call, as the fixture water
  does.
- A right click removes whatever is aimed at, whatever the hotbar holds.
- The player sees an outline on the target, a ghost of the held Voxel where it
  would land, and the held item in a corner of the view, all on this branch.
- The menu does two jobs. A click on an item holds it at once, outside the
  nine slots, with a solid as a cube. Dragging an item onto a slot, or
  pressing 1 to 9 over it, sets that slot.
- F cycles the held solid's Shape in play (R is already return-to-spawn). The
  menu also shows the Shapes of the solid under the pointer as tiles.
- The menu's slots and items are square tiles, and the same nine squares form
  a hotbar at the bottom of the screen during play.
- The on-screen controls are listed one per line.

Amended by the user during capture review on 2026-10-26: the menu and
hotbar tiles show miniature block previews instead of flat colours. This
supersedes decision 19's original flat-colour rendering and the matching
out-of-scope item. The corner held-item model is smaller (scale 0.06).

Further user amendment on 2026-10-26, after PR #21 opened: menu tiles
highlight on hover. The quick slots stay fixed at the bottom centre of the
menu while the item selection scrolls vertically. Mouse-wheel/trackpad
input in the open menu browses items without selecting a hotbar slot.

## Problem Statement

Step 8.1 made water Game content, but the engine still ships six named solids:
stone, dirt, grass, lantern, glass and leaves, under permanent ids 1 to 6. The
generator fills every World's ground with the first three, so a Game cannot
choose what its ground is made of. A sand desert or a World of basalt is out of
reach without the stone underneath. The other three exist so that the `dev`
gallery and the tests have an emitter, a glass and a cutout Material, so they
are test fixtures shipped as content. Every Game also gets `core:stone` in its
menu and Registry whether it wants it or not, and the demo's lava cools to
`core:stone` rather than to a stone of its own.

The player's controls also make them think about the hotbar before removing
anything. A right click means break, remove a source or put out a fire depending
on what the selected slot holds. Nothing shows what a click will strike or where
a held Voxel will land, so a stair turned with Q and E is only seen once placed.

The menu does one thing: choosing an entry overwrites the selected slot. Trying
one block for a moment costs a slot, and arranging the bar means selecting each
slot in turn. A solid is chosen in two steps, a Material and then a Shape, even
when a cube is all that is wanted. The menu is a column of text buttons, and
during play the hotbar is only a line of text. The controls are run together
on one line that is hard to scan.

No saves exist yet, so retiring ids now costs little, and a migration later.

## Solution

The engine keeps the kinds of content (solid, fluid and fire, the render
classes, Shapes and light) and ships no named Material but air. Ids 1 to 7 are
retired like 7, so a Mod's Materials still start at 8.

A Game names its ground's three Materials in its World parameters, as it names
its lake fluid: the rock underneath, the soil above it and the surface on top.
With no Game there is no ground, so the World is empty air and the player flies
in it. `cargo dev` still loads the demo by default, and the demo declares its
own stone, dirt, grass, glass and leaves and names the first three as its
ground. The engine's tests and benches register fixture Materials the way they
already register a fixture water.

The hotbar's defaults, the gallery and the render palette read what the Game
declared rather than built-in ids.

A click places what the selected slot holds. A right click removes what the ray
strikes, whatever the slot holds: fire first, then a solid, then a fluid
source. While the player aims, an outline marks what a right click would
remove. A see-through ghost of the held item shows where a click would put it,
in its Shape and orientation. A small model of the held item sits in the
corner of the view.

The menu is a grid of square tiles: solids, fluids and fire, then the Shapes
of the solid under the pointer, then the nine slots. A click on a tile holds
that item straight away, outside the slots, and closes the menu. A solid is
held as a cube, and F cycles its Shape in play. Dragging a tile onto a slot, or
pressing 1 to 9 with the pointer over it, puts it in that slot. During play the
same nine squares sit at the bottom of the screen, with the selected one
marked, and the controls are listed one per line.

## User Stories

1. As a Game author, I want to name the Materials my ground is made of, so
   that my World can be sand over sandstone, or basalt, rather than the
   engine's stone.
2. As a Game author, I want my ground's rock, soil and surface to be my own
   declared Materials, so that their light, occlusion and colour are mine.
3. As a Game author, I want a ground name that is unknown, a fluid or not solid
   to fail the load with a message naming the field, so that a typo does not
   give me an empty World.
4. As a Game author, I want lakes without ground to fail the load, so that I do
   not get a fluid with no bottom.
5. As a Game author, I want the default World parameters the engine hands me to
   have no ground, so that a World's content is always mine.
6. As a Game author, I want `core:stone`, `core:dirt`, `core:grass`,
   `core:lantern`, `core:glass` and `core:leaves` to be unknown names, so that
   a lookup for one fails plainly instead of finding content I did not
   declare.
7. As a Game author, I want my reactions to produce my own stone, so that lava
   cooling is my rule in my namespace.
8. As a Game author, I want the generator's smoothing stairs and slabs made of
   my surface Material, so that eased slopes match the ground they ease.
9. As a Game author on the 0.2 or 0.1 line, I want my Game to keep loading, so
   that I am not forced to rebuild for this release.
10. As a Game author on the 0.2 or 0.1 line, I want a warning that my World has
    no ground and why, so that an empty World is explained.
11. As a Mod author on the 0.1 line, I want my Mod's ids to keep starting at 8,
    so that the adapter's promise holds.
12. As a Mod author, I want a Command placing a retired id to be refused as an
    unknown Material, so that the refusal is the same as for any other id the
    table does not hold.
13. As a player in a World with no Game, I want to fly in empty air rather than
    see an error, so that the engine on its own still runs.
14. As a developer, I want the log to say that the World has no ground because
    no Game named one, and how to build the demo, so that an empty screen is
    not a mystery.
15. As a developer, I want `cargo dev` to open the demo with the same terrain,
    glass, leaves and lamp light as before, so that this change is invisible on
    screen.
16. As a player, I want the hotbar to start with the Game's ground rock as a
    cube, slab, stair and post, then the Game's other solids, then its lake
    fluid and fire, so that a new World is playable without opening the menu.
17. As a player in a World with no Game, I want an empty hotbar and an empty
    menu, so that nothing is offered that the Server would refuse.
18. As a player, I want a click to place what the selected slot holds, so that
    building is one click.
19. As a player, I want a click with an empty slot to do nothing, so that a
    stray click does not change the World.
20. As a player, I want a right click to remove what I am aiming at whatever
    the slot holds, so that I never switch slots to clear something.
21. As a player, I want a right click on a burning solid to put the fire out
    first and break the solid on the next one, so that I can save what is
    burning.
22. As a player, I want a right click on a solid with fluid in it to break the
    solid and leave the fluid, so that breaking a soaked stair does not also
    drain the pool.
23. As a player, I want a right click on a fluid source to remove it, so that I
    can stop a stream I started.
24. As a player, I want an outline around what a right click would remove, so
    that I know what I am aiming at before I click.
25. As a player, I want the outline to fit the Voxel's Shape, so that a slab is
    outlined as a slab.
26. As a player, I want a see-through ghost of the held solid where a click
    would place it, in its Shape and orientation, so that I can turn a stair
    with Q and E and see the result before placing it.
27. As a player holding a fluid, I want a ghost of a source where a click would
    put it, so that I can see where the fluid will start.
28. As a player holding fire, I want only the outline on the solid a click would
    ignite, so that the preview never shows a Voxel that will not appear.
29. As a player, I want no ghost where a click would be refused because the
    place is taken, so that the preview does not promise a placement the Server
    will refuse.
30. As a player, I want a small model of the held item in the corner of the
    view, in its colour, Shape and orientation, so that I know what I hold
    without reading the HUD.
31. As a player, I want the outline, ghost and held item hidden while the menu
    is open, so that the menu is not drawn over the World's preview.
32. As a player, I want the HUD to say "click: place <item>" and
    "right click: remove", so that the controls are stated where I look.
33. As a developer, I want the gallery to draw its Shape rows in the Game's
    ground rock and soil, so that every built-in Shape is still on show with
    the demo loaded.
34. As a developer, I want the gallery's glass house, canopy and lamp to use the
    Game's first Blend, Cutout and emitting solid, so that the blended pass,
    the cutout pass and block light are still checked on screen.
35. As a developer, I want each part of the gallery left out when the Game has
    no Material of that kind, so that the gallery never places content the Game
    did not declare.
36. As a developer, I want the gallery's fence line to use the Game's first
    connecting Shape and be left out when there is none, so that the engine no
    longer registers a fence of its own.
37. As an engine developer, I want tests and benches to register fixture stone,
    dirt, grass, lantern, glass and leaves and use the ids the calls return, so
    that a fixture id can never alias a Mod's id.
38. As an engine developer, I want benches to measure the same terrain and the
    same Materials as on main, so that their numbers stay comparable.
39. As a reviewer, I want an ADR that amends ADR-0009 and ADR-0015's list of
    built-in Materials to air alone, so that the save-data promise is stated
    where it is enforced.
40. As a reviewer, I want CONTEXT.md, architecture.md, DESIGN.md, README.md and
    CONTRIBUTING.md to stop naming built-in solids and to describe the new
    clicks and preview, so that the docs match the code.
41. As a reviewer, I want the outline, ghost and held item to be Client drawing
    from Server answers, so that no Client system reads a Chunk or mutates the
    World.
42. As a reviewer, I want the phase 7 drain cap and the Wasmtime, WASI and
    Cranelift pins unchanged, so that this part moves no measured budget or
    security fix.
43. As a player, I want a click on an item in the menu to put it in my hand at
    once and close the menu, so that I can try a block without giving up a
    slot.
44. As a player, I want a solid I pick that way to come as a cube, so that the
    common case is one click.
45. As a player, I want 1 to 9 or the scroll wheel to take me back to the bar
    from an item held outside it, so that my slots are where I left them.
46. As a player, I want to drag an item onto one of the nine slots to put it
    there, so that arranging the bar is direct.
47. As a player, I want to press 1 to 9 with the pointer over an item to put it
    in that slot, so that I can fill the bar without dragging.
48. As a player, I want to drag one slot onto another to swap them, so that I
    can reorder the bar.
49. As a player, I want the Shapes of the solid under the pointer shown as
    tiles, so that I can pick or slot a slab or stair from the menu.
50. As a player, I want F to cycle the held solid through every registered
    Shape in play, so that I can change Shape without opening the menu.
51. As a player, I want the menu's items and slots drawn as square tiles in
    their colours, with names under them, so that the menu reads as a grid of
    things and not a list of words.
52. As a player, I want the nine slots as squares at the bottom of the screen
    during play, with the selected one marked, so that I see my bar without
    opening the menu.
53. As a player, I want an item held outside the bar shown beside it, so that
    I know I am not holding a slot.
54. As a player, I want the controls listed one per line, so that I can find a
    key at a glance.
55. As a player in a World with no Game, I want an empty grid and nine empty
    squares, so that the menu still opens and explains itself.
56. As a reviewer, I want the menu and the bar to stay engine Client UI that the
    Game has no say in, so that 8.1's menu decision holds.

## Implementation Decisions

1. **Built-in Materials.** The only built-in Material is air, id 0. Ids 1 to 7
   are retired. The Material table keeps a slot for each, answering as the
   opaque stand-in. The Registry keeps a slot for each with no name. A Mod's
   Materials still start at 8. The stone, dirt, grass, lantern, glass and
   leaves constants and rows are deleted, not kept as aliases, and so are their
   `core:` names. The single retired constant becomes the first Mod id and a
   retired range.
2. **ADR-0016** records decision 1 and the ground parameters. It amends
   ADR-0009's and ADR-0015's lists of permanent ids. It notes what changes in
   ADR-0012 (the 0.3 record gains ground, and old lines lose it) and in the
   gallery's reading of ADR-0006 and ADR-0010 (fixtures by property).
3. **The ground is a World parameter.** The engine's World parameters gain an
   optional ground of three Materials: rock, soil and surface. Generation fills
   the ground with them exactly as it fills it with stone, dirt and grass today.
   Smoothing places stairs and slabs of the surface Material. With no ground,
   generation yields only air, caves and smoothing have nothing to act on, and
   the spawn point stays where the surface would be. Validation runs after
   every Declaration's rows, beside the lake check. It fails the load, naming
   the field, when a ground name is unknown, not solid or has a fluid row, and
   when lakes are set with no ground.
4. **The engine default has no ground.** The World parameters the engine hands
   a Game have no ground and no lakes. During the expand step the default
   still names the built-in stone, dirt and grass, so the suite and the
   running app stay green. The contract step removes them.
5. **Contract line 0.3 is amended, not superseded.** 0.3 merged with step 8.1
   but is in no tagged release, and the only 0.3 Game is the in-repo demo,
   which this step rebuilds. Its World-parameter record gains an optional
   ground record of three full names. No new line is added.
6. **0.2 and 0.1 keep loading.** Their World parameters name no ground, so they
   come up with none and the load logs a warning that the World has no ground
   before 0.3, as the warning for their standing water does. A 0.2 or 0.1 row
   or Command naming a retired `core:` name fails the way any unknown name
   does. The 0.1 `builder` and `echo` guests are not rebuilt. Their placements
   of id 1 now fail validation as an unknown Material. Their tests still assert
   what each guest returned before validation, and also assert the refusal. A
   Mod's ids still start at 8 in every 0.1 fixture.
7. **The demo declares its own.** `demo:stone`, `demo:dirt` and `demo:grass`
   are opaque solids. `demo:glass` is a solid with dimming 0, hiding the same
   Material and drawn blended. `demo:leaves` is a solid with dimming 1, hiding
   nothing and drawn as cutout. Each carries today's placeholder colour, and
   the demo's lamp takes the lantern's near-white colour. The demo names
   stone, dirt and grass as its ground, and its lava cools to `demo:stone`. Its
   scenarios and stage are built from its own stone. Every scenario plays out
   as it does today at 1.0, 0.5 and 0.25 m.
8. **A World with no Game is empty.** With no ground and no lakes, every Chunk
   is air. The Server logs once at startup that no Game named a ground. In a
   `dev` build without the built demo, the log line says to run the guest build
   script, as it does for the dry World today.
9. **The render palette has no built-in colours.** Every solid's placeholder
   colour comes from its Game's appearance, as lava's does. Air's magenta
   sentinel and the flame's colour stay.
10. **The catalogue names the ground.** Beside the lake fluid, the content
    catalogue carries the ground's rock id, so the Client's defaults can read
    it without reading a Chunk.
11. **Hotbar defaults.** The defaults are:
    - the ground rock as a cube, slab, stair and post;
    - then the other declared solids, as cubes, in registration order, until
      seven slots are used;
    - then the lake fluid, or else the first declared fluid;
    - then fire, when anything burns.

    Any slot left over is empty. With no ground, the solids start at the first
    declared solid as a cube. With no Game, every slot is empty.
12. **The gallery picks by property.** It is still `dev` only and still built
    through the edit path. Its parts are drawn from:

    | Part                            | Material or Shape                    |
    | ------------------------------- | ------------------------------------ |
    | plinths, Shape rows, basin rim  | the ground rock                      |
    | Shape rows' second Material     | the ground soil                      |
    | trunk                           | the ground soil                      |
    | slab floor                      | the ground surface                   |
    | lamp                            | the first registered emitting solid  |
    | glass house                     | the first registered Blend solid     |
    | canopy                          | the first registered Cutout solid    |
    | pool                            | the lake fluid                       |
    | fence line                      | the first registered connected Shape |

    A part whose Material or Shape does not exist is left out. With no ground
    there is no gallery. The `dev` Server no longer registers a fence Shape of
    its own, so its resource and its `dev:fence` catalogue name go. The fence
    stays an example Shape in `tesserae-core` for tests.
13. **One remove action.** The edit intent's break, remove-source and put-out
    actions become a single remove action. The Server casts the ray once,
    stopping at the first Voxel holding a solid or a fluid source and going
    through flowing fluid. In that Voxel it removes one layer:
    - fire, if the solid is burning, by the put-out rule a Mod's extinguish
      meets;
    - otherwise the solid, leaving any fluid it shared the Voxel with;
    - otherwise the source.

    The refusals are today's. A 0.3 Mod sees the result as the same Edit kinds
    as today: a put-out, a solid Edit or a fluid Edit. Aiming through a lake's
    surface removes the source at the surface rather than the lake bed. That
    follows from the ray stopping at sources, which placing a source already
    does.
14. **Click mapping.** A click places the held item (decision 17): a solid, a
    fluid source or fire (ignite). With nothing held it does nothing. A right
    click sends the remove action whatever is held.
15. **The Server answers where the player is aiming.** The Client sends an aim
    report each frame the ray has moved: a sequence number, an origin and a
    direction in metres, and the held item. It is intent only and is dropped
    when not finite, not newer than the last, or when its origin is farther
    from the Eye than the edit tolerance. Once a tick, for the latest report,
    the Server answers with an aim target:
    - the outline: the metre bounds of the Shape of what a remove would take,
      or nothing out of reach;
    - the ghost cell: the metre position where a click would put the held
      solid or source, present only when that cell would accept it;
    - for fire, the outline of the solid a click would ignite, and no ghost.

    The same ray and stopping rules as the edit path decide all of this, so the
    preview and the click cannot disagree. The answer is content about a
    position and holds no Voxel or Chunk, so the Client still never reads a
    Chunk (ADR-0002). It trails the aim by one tick.
16. **The Client draws the preview.**
    - The outline is a thin box of lines.
    - The ghost is the held Voxel's mesh, see-through, in its placeholder
      colour.
    - The held item is the same mesh, small, in a corner of the view.

    The Client meshes the single held Voxel with the core mesher from Shape
    geometry. The geometry reaches it as read-only content in the catalogue,
    beside the Shape names. A connected Shape is shown unconnected. A fluid is
    shown as a cube of its colour, and fire as the flame colour. All three
    previews are hidden while the menu is open.
17. **The held item.** The hotbar keeps its nine slots and gains an optional
    item held outside them. When that item is set it is what a click places,
    and otherwise the selected slot is. Selecting a slot by 1 to 9, the scroll
    wheel or a click on a slot clears it. Q, E and F act on whichever solid is
    held.
18. **F cycles the held solid's Shape** through the catalogue's Shapes in id
    order, wrapping, and keeps its orientation. It does nothing to a fluid or
    fire.
19. **The menu is a grid of square tiles**, still engine Client UI opened with
    Tab. Its sections, in order:
    - solids, each tile showing the Material as a cube;
    - the Shapes of the last solid the pointer was over, or at first of the
      held solid, or else the first solid;
    - fluids;
    - fire, when anything burns;
    - the nine slots, numbered in a corner.

    Each tile is square and shows an item model in its placeholder colour,
    with its name under it. The selected slot and the held item are marked by a border.
    Tiles show rendered item models, following the capture-review amendment.
    The menu acts on its tiles as
    follows:
    - **A click on an item tile** holds that item outside the slots, closes the
      menu and grabs the cursor again.
    - **A click on a slot** selects it. The menu stays open.
    - **Dragging an item tile onto a slot** puts the item in that slot, and
      dragging a slot onto another swaps them. A drop anywhere else does
      nothing.
    - **1 to 9 with the pointer over an item tile** puts that item in that slot.
      The menu stays open and the selection does not move. With the pointer
      elsewhere, 1 to 9 select a slot as before.
20. **The bar and the controls on screen.** During play the nine slots are
    squares at the bottom centre of the screen, drawn as in the menu, with the
    selected one marked. An item held outside them is a tenth square set apart
    beside them, marked in place of a slot. The held item's name sits above the
    bar. The controls are listed one per line in the bottom-right corner:
    - click: the held item's place verb;
    - right click: remove;
    - 1 to 9 or scroll: choose a slot;
    - Q: turn, E: flip, F: Shape;
    - Tab: menu;
    - movement, sprint, speed, return to spawn and Esc.

    Notices go on their own line under the list. The camera's
    look-around prompt and the menu's header are split one control per line in
    the same way. Text stays ASCII, because the embedded font draws nothing
    else.
21. **Test fixtures.** `tesserae-core`'s test support gains register calls
    beside the fixture water:
    - a ground register call that adds stone, dirt and grass with today's rows
      and returns the ground parameters;
    - one call each for lantern, glass and leaves, each returning its id.

    The deterministic test terrain takes the ground's ids as an argument. Tests
    and benches in all three crates use these calls and the ids they return.
    No fixture has a fixed id. Nothing in the running engine calls them.
22. **Unchanged:** the phase 7 drain cap of two Voxels a tick, Wasmtime and
    WASI at 49.0.4 and Cranelift at 0.136.4, every simulation bound in
    ADR-0014, the Overlay and Occupants encoding in ADR-0013, and the built-in
    Shapes (cube, slab, stair and post), which are kinds and stay.

## Testing Decisions

A good test drives a seam from outside and asserts what a player, a Mod or the
World would see. It never asserts on private structure. Wall-clock limits in a
test are hang detectors with wide margins, never budgets.

These are the existing seams, highest first. No new seam is introduced.

1. **The Mod load path:** components and Manifests in; the tables, the
   Registry, the World parameters and load errors out. It covers:
   - 0.3 ground names;
   - unknown, fluid and non-solid ground, and lakes without ground, failing
     the load;
   - 0.2 and 0.1 World parameters coming up with no ground and a warning;
   - the six retired `core:` names unknown on every line;
   - the Registry listing only `core:air` below 8;
   - Mod ids from 8;
   - `builder` and `echo` refused as an unknown Material.
2. **World generation:** parameters and tables in, Chunk Voxels out. It covers
   all air with no ground, ground of named Materials, smoothing in the surface
   Material, and lakes over a named ground.
3. **The Server's edit path:** an edit intent in, Occupants and rejections
   out. It covers:
   - the single remove action's layer order: fire, then solid, then source;
   - a waterlogged solid broken with its fluid kept;
   - a retired id refused as unknown.
4. **The Server's aim answer:** an aim report in, an aim target out, beside the
   edit path tests. It covers outline bounds per Shape, the ghost cell present
   and absent, fire with no ghost, stale and far reports dropped, and the
   answer agreeing with what the matching edit intent does.
5. **The Client's input and HUD:** key and button input with a catalogue in;
   sent intents, aim reports, HUD text and preview entities out. It covers:
   - defaults from the ground rock;
   - an empty bar with no Game;
   - a right click sending remove whatever the slot holds;
   - an empty slot's click sending nothing;
   - previews hidden while the menu is open;
   - a click on an item tile holding it outside the slots, and 1 to 9
     returning to them;
   - a drop on a slot setting it, and a slot dropped on a slot swapping them;
   - 1 to 9 over an item tile setting that slot without moving the selection;
   - F cycling the held solid's Shape;
   - the bar's squares and the one-per-line control list.
6. **The gallery's plan:** the existing plan tests, with parts chosen by
   property and left out when nothing fits.
7. **Coarse derivation and light:** the existing level-1 tests, on fixture
   ground.
8. **The demo end to end:** the existing demo runtime tests, with the demo's
   own ground, at the three Voxel sizes.

Benches register the same fixture Materials on the same terrain, so their
numbers stay comparable with main. Ids 8 and up in place of 1 to 6 change no
Chunk palette's size. The comparison uses the interleaved thread-CPU method of
step 8.1's ticket 10.

## Out of Scope

- **Data-only Mods**, still their own session.
- **A separate in-repo test Game** declaring one fixture per render class and
  building the gallery with Commands. It is better done after data-only Mods,
  and under the current drain cap a Command-built gallery would take tens of
  seconds to appear.
- **Ground made of more than three layers, or biomes.** One rock, soil and
  surface, as generation has today.
- **Client-side raycasting or any Client copy of the World.** The preview is
  the Server's answer.
- **A hand animation or swing**, and an inventory or item counts.
- Dragging a slot off the bar to empty it.
- **Save data, tombstones beyond ids 1 to 7, and migration.**
- **Network transport.** Messages stay in-process.

## Further Notes

- **Build hygiene** as in step 8.1: `RUSTFLAGS="-D warnings"`, the
  `--workspace --features tesserae/dev` forms, `caffeinate -i`, and
  `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` before each commit.
  Never run Prettier on `.scratch`.
- **Rebuild guests and the demo** after any WIT or `games/` change.
- **The ADR, tickets and commits** name no other open-source voxel engine.
- **Screenshots** for review go in one `/tmp` folder. They are never committed
  and are deleted after review.

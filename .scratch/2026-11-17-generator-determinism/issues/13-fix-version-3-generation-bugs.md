# 13: Fix version 3 generation bugs found while moving tests

Status: done

Spec: [Generator determinism](../spec.md).

Blocked by: 09.

## What to build

Three version 3 defects found by tickets 02 and 03 (see their Comments) are
fixed inside the one pre-release exception ADR-0032 allows, before 12 re-records
the digests:

1. A Surface feature on a column's top Voxel at a Chunk's top can go missing
   from the Chunk above, because that Chunk asks the raw tunnel field, which
   does not know version 3 keeps tunnels shut beside the sea.
2. In ticket 03's scan area, 1 m Voxels place crop fields near water and 0.5 m
   Voxels place none. Find out why; if placement depends on Voxel size rather
   than metres (ADR-0003), fix it.
3. At seed 3907 with overhangs on, smoothing places stairs that climb two steps
   and slabs that ease fewer than two.
4. A landform height (`landforms.height_m`) of 0 divides by zero: heights turn
   NaN and drainage loops forever, so a Game declaring it hangs the Server
   (found by ticket 16). Validation refuses it with a clear error; no valid
   World's output changes.
5. Version 3 drops a tree within 1.5 crown radii of a stronger one, which caps a
   stand of one tree far below the dense density class (ticket 17's Comments:
   jungle 57 per hectare against 278). Each Plant declares its own crowding
   distance, defaulting to today's rule, so a dense Biome's trees can stand
   closer; the Games' dense trees declare one that reaches the class.

## Acceptance criteria

- [x] A test per defect fails before the fix and passes after it, at 1 m and 0.5
      m.
- [x] Ticket 03's raised-sea case in the Surface feature test is wet again, and
      the smoothing tests also run at seed 3907 with overhangs.
- [x] The digest and spawn positions each fix moves are listed in a comment
      here; they are re-recorded in this ticket, and 12's commit message lists
      them alongside its own.
- [x] If item 2 turns out not to be a defect, the reason is written here and the
      test asserts the intended behaviour.
- [x] Parameters with a landform height of 0 (or any value that makes a height
      non-finite) are refused at validation, shown by a test; no digest moves
      for this item.
- [x] `each_biome_scatters_its_trees_at_its_density_class` runs under version 3,
      unignored, with its thresholds unchanged.
- [x] A Plant that declares no crowding distance crowds exactly as before; the
      new field is optional in the Declaration and documented where Plant fields
      are (contract compatibility, data mods).

## Comments

- Item 1, a defect. `still_solid` outside its Chunk, and `carved` (which the Plant pass, its mirror of smoothing and crop fields ask), read the raw tunnel field. The cube pass also keeps tunnels shut beside water (`sealed_below_m`), clear of Structures (`CAVE_CLEAR_M`) and off a flooded cavern's shell, and opens shafts. `carved` now asks all of it, and `still_solid` uses it for a cell it does not hold. The Surface feature test's lifted World is wet again, with wider tunnels so they reach the shore, at 4 m round the origin and at 1 and 0.5 m on the shore at (144, -112); it failed at each size before. Nothing recorded moved.
- Item 2, not a defect. The test's window was ground 0.6 m under the sea everywhere. A 1 m Voxel's top fills to the sea's level, so no water stood over it and fields did; a 0.5 m one leaves half a Voxel of water over the ground, which no field stands under, so none stood anywhere in the window, near water or not. Placement asks in metres, and the Voxel lattice decides what is wet. The test now has small continents (800 m), so its window holds a shore, and it asserts a field near water at every Voxel size. Left as it is: `water_surface_m` reports the sea over such ground at 1 m, where no water Voxel stands, so a near-water field there may have none in sight.
- Item 3, a defect. Smoothing counted a neighbour as one step up from its surface height, but the overhang density pushes cells past that surface: at seed 3907, (-3, -20) at 1 m had one neighbour one higher by its surface and a second pushed solid at the stair's level, so it got a stair that climbs two. A neighbour now rises where its cell at the stair's level is ground and the cell above is not, read from the Chunk or asked as the Chunk holding it decides (`filled`, then `carved`); the Plant pass's `eased` asks the same. With overhangs off but caves on, 3907 also failed (a tunnel opening the cell over a higher neighbour); that is fixed by the same rule. The two smoothing tests run at `SEED` and `COAST` (3907), at 1 and 0.5 m.
- Item 4. `landforms.height_m` of 0 is refused, naming the field ("must be above 0 m, since a range's rise is a share of it"); the least positive height validates and generates finite heights at 1 and 0.5 m. Nothing moved.
- Item 5. `crowding-m` is an optional last field of `tree-def` on the unreleased 0.3 text (both WIT copies), above 0 and at most 12 (1.5 radii of the widest crown, so the scan a tree makes stays bounded), frozen in the Biome snapshot as an optional member. A tree without it crowds at 1.5 crown radii, as before, and saves without the field. flora's jungle and swamp trees declare 2 m: at 1 m that is the same as 1.5 m (only the adjacent and diagonal columns are refused), and 2.5 m left the swamp tree's Biomes at 6.1 to 6.5 m. Spacing at seed 7, 1 m, before and after: terra jungle 9.7 to 4.6, swamp 10.5 to 5.5; meadow thicket 13.2 to 4.9, fen 10.3 to 5.1, heath (moderate) 10.3 to 9.5; ember cinder bog 10.1 to 4.8; tidewater rainforest 9.4 to 4.7, tidal marsh 9.9 to 5.2, lagoon (sparse) 21.6 to 20.5; lumen conservatory 10.6 to 4.4, lily pond 9.8 to 5.1. `each_biome_scatters_its_trees_at_its_density_class` takes about 220 s in the dev profile.
- Moved, all re-recorded on this Mac (macOS arm64), all by item 3: `coarse_generation` seed 1 level 1 (`04e4d0f6…` to `93ccb8b4…`) and seed `0xdead_beef_cafe_f00d` level 1 (`b4922c3c…` to `02ad388e…`); `generator_version`'s high-ground box at seed 1, 1 m (`a294e853…` to `1c036f51…`) and at seed `0xdead_beef_cafe_f00d`, 1 m (`95d2a641…` to `f0270d67…`), which `content_registered_in_another_order_makes_the_recorded_chunks` reads too. No other digest moved, and no spawn position: the content, structure, plant, forest and settlement boxes and every recorded spawn are as ticket 09 recorded them. Items 1, 2, 4 and 5 moved nothing recorded.
- Cost not measured: smoothing now asks the cube pass's question of a neighbour outside the Chunk (its surface, the overhang push and the caves) for every candidate column on a Chunk's edge, where it asked only the caves when the heights matched. `filled` asks the column's surface and its neighbours', not their Chunk's whole Stack.

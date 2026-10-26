# 06: The hotbar holds declared solids, fluids and fire

Status: done
Blocked by: 05

Spec: [decisions 11, 12, 13 and 16](../spec.md).

## What to build

The Server publishes a read-only content catalogue to the Client, beside the
existing palette message:
- each registered Material's full name and id, whether it is solid, whether it
  has a fluid row and whether it has a combustion row;
- each registered Shape's full name and id, and whether turning it changes
  anything.

The Client's hotbar becomes data: nine slots on keys 1 to 9 in every build,
each holding a solid, a fluid source, fire or nothing.
- **Defaults:** stone cube, slab, stair and post, lantern, glass and leaves.
  Then the Game's lake fluid, or else its first declared fluid. Then fire when
  anything burns.
- **Scrolling** the wheel, or a two-finger swipe on a trackpad, moves the
  selection one slot at a time and wraps. A trackpad's pixel deltas accumulate
  to a notch, so one swipe does not skip several slots.
- **A click places and a right click removes** (spec decision 13, amended on
  2026-10-25):
  - solid: click places, right click breaks;
  - fluid: click places a source, right click removes one;
  - fire: click ignites, right click puts out;
  - empty: click does nothing, right click breaks.

  The right click is the secondary button macOS reports, a two-finger tap by
  default. Nothing waits to tell a double click from a single one.
- **The Alt modifier goes.** The dev fence is no longer a fixed slot.
- **The HUD** names the held slot and what each button does with it.

The Client still never reads a Chunk.

## Acceptance criteria

- [x] With the demo loaded, the default hotbar holds the seven built-in solids,
      `demo:water` and fire. Given a catalogue with no fluid and no fuel, it
      holds the seven solids and two empty slots. Until ticket 09, a World with
      no Game still has the built-in water.
- [x] A click sends the held slot's place intent and a right click its remove
      intent, each on the press, for every slot kind.
- [x] Scrolling one notch either way selects the next or previous slot,
      wrapping at both ends.
- [x] Holding a fluid sends place-source and remove-source intents for that
      fluid. With the demo, water, lava and mud each reach the World as sources
      when placed this way, which is verified at the Server seam.
- [x] Holding fire sends ignite and put-out intents, which ticket 05's rules
      decide.
- [x] Holding a solid sends the same place and break intents as today, with Q
      and E turning and flipping only Shapes the catalogue marks as turnable.
- [x] The HUD text names the slot and its two actions for each slot kind.
- [x] The catalogue is content only: it carries no Voxel, Chunk or Occupants.
- [x] CONTRIBUTING's controls table matches the new bindings.
- [x] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.

## Comments

### 2026-10-26: implementation

- **Catalogue.** `content::ContentCatalogue`, a message written once beside
  `MaterialPalette` by `server::mods::plugin`. Per Material: id, full name,
  `solid`, `fluid` (the row's rank, so "first declared" survives the demo
  registering its water last) and `burns`. Per Shape: id, full name and
  `turnable` (not the cube, not connected). Plus `lake`, the Material of the
  World parameters' lake after the dev overrides. Built by
  `server::mods::catalogue` from the Registry and `ContentTables`; the dev
  fence, the one Shape no Registry names, is listed as `dev:fence`. No Voxel,
  Chunk or Occupants.
- **Hotbar.** `Hotbar { slots: [Holding; 9], selected }`, where `Holding` is
  `Solid { material, shape, orientation }`, `Fluid(material)`, `Fire` or
  `Empty`; each solid slot keeps its own orientation. The Client keeps the
  last catalogue and rebuilds the defaults from it when it arrives. Keys 1 to
  9 in every build; `DevFence` is now Server-only.
- **Buttons.** Left: `Place`, `PlaceSource`, `Ignite` or nothing. Right:
  `Break`, `RemoveSource`, `PutOut` or `Break`. Both on the press; the
  grabbing click still edits nothing. Alt is gone, and so are the
  `expect(dead_code)` attributes on `Ignite` and `PutOut`.
- **Scroll.** Down is the next slot, wrapping. A wheel line is a notch; a
  trackpad's pixels add up to one notch per 100 px (`PIXELS_PER_NOTCH`), and
  the remainder is dropped when a gesture starts (`TouchPhase::Started`) or
  the direction reverses. 100 px is a guess not yet tried on hardware.
- **Decision the spec leaves open:** the fly camera's speed was on the
  scroll wheel. It moves to `-` and `=`, a factor of 1.5 a press.
- **HUD:** `8: demo:water source`, then
  `[click] place source  [right click] remove source`, then the key hint and
  any rejection. A shaped solid reads `core:stone core:stair  up +Y, turn 0`;
  orientation only when turnable.
- **Tests.** `client/edit.rs` runs on catalogues shaped like the demo's and
  a dry one (no built-in water slot left): defaults for the demo, dry, no
  lake, lava lakes and fuel only; both buttons for each slot kind; every key;
  scroll by line and by pixel; Q/E on stair, cube and fence; HUD text per
  kind. `server/mods.rs`: the demo's published catalogue and a no-Game one
  (built-in water as lake until ticket 09, `dev:fence` under `dev`).
  `fluids/tests.rs`: every fluid the demo's catalogue lists, `demo:water`,
  `demo:lava` and `demo:mud` among them, lands as a source and is removed, at
  1.0 and 0.25 m. Fire intents are ticket 05's Server tests.
- Docs: CONTRIBUTING's controls table, and DESIGN loses "places water only".
  README has no controls.
- `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass: tesserae
  359 (2 ignored), core 324, meshing 68, tesserae-mods 92 (3 ignored).

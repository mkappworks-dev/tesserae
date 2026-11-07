# 02: Breaking a Voxel is heard

Status: ready-for-agent

Blocked by: 01

## What to build

The tracer bullet. A Mod declares an Effects Sound and a Sound set with a
`break` Sound, gives the set to a Material, and a Player removing a Voxel of
that Material hears it, attenuated by distance in metres and panned by
direction.

- Line 0.3 is amended in place: the `declaration` record gains `sounds`, `sound-sets` and `material-sounds`, appended after `survival`, with the full shape the spec gives
  (`sound-sets` carries every Voxel Cause, though only `break` is emitted
  here). The data TOML takes the same fields; the starter's WIT copy matches.
  Recordings are bytes from a component and relative paths from a data
  Declaration, read with the PNG artwork path rules.
- Names resolve after every Mod has declared. A Material with two Sound sets
  fails the load naming both Mods. The Ogg page and Vorbis identification
  header are checked by hand; Effects must be mono.
- A new `tesserae-core` audio module, no Bevy: inverse-clamped attenuation in
  metres, pan, and seeded choice of recording, volume and pitch.
- The Server emits a Sound event (Sound id, position in metres, Cause) from the
  same point a deliberate Remove drops an Item, and drops it beyond the Sound's
  maximum distance from the Eye. Undo, redo, Layout, fluid, fire and Commands
  make none.
- The Client plays each Sound event through the variant ticket 01 chose, at
  fixed default volumes, from in-memory sources made from the checked bytes.
- `contract-compatibility.md` gains a paragraph under "Amended before release", in the form of the Biome and Survival paragraphs; the data-only Mod guide and the authoring guide describe the three
  lists.

## Acceptance criteria

- [ ] Both Declaration formats load the three lists; 0.1 and 0.2 Declarations
      adapt to none.
- [ ] Refusals name the Mod and field: unknown Sound or set, wrong category in
      a set slot, stereo Effects, malformed Ogg, missing distances on Effects,
      escaping path, two sets for one Material.
- [ ] Core tests: attenuation 1 within the reference, 0 at and beyond the
      maximum, the same at Voxel sizes 0.5 and 2.0; pan left and right; seeded
      variation stays in range and honours weights.
- [ ] Server tests: Remove makes one `break` at the Voxel's centre in metres;
      undo, Layout and Commands make none; an event beyond range is dropped; a
      Material with no set is silent.
- [ ] The Client never decides a Sound event happens; it holds no mutable World
      state.
- [ ] Ogg fixtures are tiny, generated once and committed, like the PNG
      fixtures.
- [ ] No new dependency in any crate.

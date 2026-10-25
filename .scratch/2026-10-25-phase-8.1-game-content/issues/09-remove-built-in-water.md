# 09: Built-in water goes and id 7 is retired

Status: ready-for-agent
Blocked by: 01, 02, 03, 04, 06, 08

Spec: [decisions 1, 2, 3 and 9](../spec.md).

## What to build

The contract step of the phase. Delete the built-in water Material constant,
its Material row, the built-in water fluid row and `core:water` from the
Registry's built-in names. The built-ins are ids 0 to 6.
- **Id 7 is retired:** the Material table reports it unknown, the Registry
  never lists it, and a Mod's Materials still start at 8.
- **The built-in fluid table is empty.**
- **The engine's placeholder palette** has no water entry.
- **The translucent gallery** draws its pool with the Game's lake fluid, and
  leaves the pool out when there is none.
- **The temporary lake bridge from ticket 02 goes:** a World with no Game is
  dry.

Write ADR-0015 amending ADR-0009's permanent ids. It notes what this changes in
ADR-0012 and ADR-0013, describes the approach in Tesserae's own terms and names
no other engine. Update CONTEXT.md (Fluid, Source, Water level),
architecture.md's "Mechanics are the engine's, content is the Game's" section
and its ADR list, and DESIGN.md.

## Acceptance criteria

- [ ] Nothing in the workspace refers to a built-in water Material, a built-in
      fluid row or `core:water`, except where the ADR and docs say they were
      removed.
- [ ] Id 7 is unknown to the Material table and absent from the Registry. A Mod
      Material registered first gets 8, and the 0.1 world-loader fixture passes
      unchanged.
- [ ] A Chunk holding id 7 draws the opaque stand-in and is never treated as a
      fluid.
- [ ] A World with no Game has no lakes and no fluid rows. Igniting anything is
      refused as not flammable, and the default hotbar offers no fluid and no
      fire.
- [ ] With the demo loaded, lakes, the gallery pool, scenarios and LOD water
      look as they do on main, confirmed by a screenshot pass reviewed with the
      user and then deleted.
- [ ] ADR-0015 exists and is linked from ADR-0009 and the architecture ADR
      list. The glossary and architecture table no longer say water is built in.
- [ ] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.

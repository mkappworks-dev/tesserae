# 03: The ground is named by the World parameters

Status: ready-for-agent
Blocked by: 01

Spec: [decisions 3, 4 and 5](../spec.md).

## What to build

A Game names its ground's rock, soil and surface in its World parameters, as
it names its lake fluid, and generation uses them.

- The engine's World parameters gain an optional ground of three Materials.
  Generation fills the ground with them where it uses stone, dirt and grass
  today, and smoothing places stairs and slabs of the surface Material. With
  no ground every Chunk is air; caves and smoothing have nothing to act on;
  the spawn point stays where the surface would be.
- Validation runs beside the lake check, after every Declaration's rows. It
  fails the load, naming the field, when a ground name is unknown, not solid
  or has a fluid row, and when lakes are set with no ground.
- Contract line 0.3's World-parameter record gains an optional ground record
  of three full names. 0.3 is amended in place; no new line.
- For now the engine's default World parameters, and those an old-line Game
  comes up with, still name the built-in stone, dirt and grass, so the running
  app and every suite are unchanged. Ticket 06 removes that.

## Acceptance criteria

- [ ] Generation with a named ground places those Materials where stone, dirt
      and grass went, at 1.0, 0.5 and 0.25 m, and smoothing uses the surface
      Material.
- [ ] Generation with no ground yields air in every Chunk, and the spawn point
      is unchanged.
- [ ] A 0.3 Declaration naming its ground loads, and the generated World uses
      it.
- [ ] Unknown, fluid and non-solid ground names fail the load with a message
      naming the field. Lakes with no ground fail the load.
- [ ] Ticket 01's deferred World generation tests move to fixture ground.
- [ ] The guests and the demo are rebuilt against the amended 0.3 WIT, and
      every 0.1, 0.2 and 0.3 fixture passes.
- [ ] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.

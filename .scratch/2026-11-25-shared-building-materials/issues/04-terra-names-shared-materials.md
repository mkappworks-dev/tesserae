# 04: Terra builds from the shared Mods

Status: done

Blocked by: 01, 03

Spec: [Shared building Materials](../spec.md).

## What to build

Terra depends on `timber`, `brickwork`, `glassworks` and `workshop`, and every
Template, Settlement path, loot table, reaction and sound row that named one of
Terra's building Materials names the shared one from the spec's mapping table.
Terra still declares its own building Materials in this ticket, now unused;
ticket 05 deletes them. Terra adds hand Recipes from `terra:rock` to
`strata:stone` and from `terra:sand` to `strata:sand`, one for one.

The mockup maps each of Terra's building roles to its shared Material. For
Terra, the furnishing table names the shared workbench, Stations, chest, door,
glass and lantern, and the exporter's Shape prefixing gives door cells
`timber:leaf` and the workbench `workshop:table`. Every Game's
Templates are regenerated; only Terra's change.

Sound rows give the shared Materials a set each, as the spec lists: most were
silent in Terra before, so this is an addition. The guest build's list of Mods
shipped with Terra follows its new `depends`.

## Acceptance criteria

- [ ] Terra composes and passes the Obtainable check with the Station rule.
- [ ] No Terra Template, Settlement path, loot table or reaction names a Terra
      building Material, Station, chest, Door, glass or lamp.
- [ ] The Templates-match-the-mockup test passes, the generator's check mode
      reports no drift for any Game, and the other four Games' generated
      Templates are unchanged.
- [ ] The Terra building test walks the shared Materials Terra's Templates use
      and asserts each is Obtainable through a Station that is.
- [ ] The Terra `depends` test lists the four new Mods.
- [ ] Breaking planks, cobble, glass and thatch in Terra plays the wood, stone,
      metal and soft sets.
- [ ] A village at a fixed seed is compared by eye in a dev capture before and
      after: same layout and roles; colours shift only where the spec says
      (review-only, not committed).
- [ ] Terra's boundary check passes.

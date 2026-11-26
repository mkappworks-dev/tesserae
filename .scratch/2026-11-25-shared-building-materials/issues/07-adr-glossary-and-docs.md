# 07: ADR-0038, the glossary and the docs

Status: done

Blocked by: 05, 06

Spec: [Shared building Materials](../spec.md).

## What to build

A new ADR-0038, "Building Materials come from shared Mods, and a Recipe needs an
Obtainable Station", recording: building Materials, Stations and furnishing are
declared by shared Data-only Mods and made from shared world Materials; a Recipe
counts towards Obtainable only through an Obtainable Station; every declared
Template and Settlement path is checked; per-Game retinting and Recipe inputs by
tag are deferred. ADR-0029 gains "Amended by ADR-0038" at its end. The
glossary's Obtainable entry gains the Station clause. The authoring guide's
Obtainable steps and its Door example are updated. The Mod table in the Mods
README and the demo Games doc name `workshop` and the new users, and each shared
Mod README's "Used by" is checked against them.

## Acceptance criteria

- [ ] ADR-0038 exists and ADR-0029 links it.
- [ ] The glossary's Obtainable entry says a Recipe counts only when its Station
      is Obtainable.
- [ ] The authoring guide no longer says Stations or unlisted Templates are not
      checked, and names no deleted Terra Shape.
- [ ] Every shared Mod README's "Used by", the Mods README and the demo Games
      doc agree.
- [ ] No planning doc names the block game the engine is compared with.
- [ ] `npm run fmt:check` passes.

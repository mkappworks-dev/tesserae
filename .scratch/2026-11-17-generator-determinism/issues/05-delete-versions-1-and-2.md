# 05: Delete Generator versions 1 and 2

Status: ready-for-agent

Spec: [Generator determinism](../spec.md).

Blocked by: 02, 03, 04, 15, 16, 17, 18.

## What to build

Version 3 is the only Generator version (ADR-0033). Versions 1 and 2's
generation, every version gate, their digests and tests, and the version 2
fixture parameters are deleted; the fixtures' default is version 3. The save
supports generator version 3 only: a World stamped 1 or 2 is refused with a
message naming its version and saying it predates version 3. The reference docs
that describe versions 1 and 2 (glossary, world persistence, data mods, contract
compatibility, the design document's step notes) are corrected in the same
change.

## Acceptance criteria

- [ ] No code path asks which Generator version a World uses, except the save's
      check.
- [ ] A save test shows a World stamped 1 or 2 is refused with that message.
- [ ] The version 3 digests are unchanged.
- [ ] No doc outside ADRs and `.scratch/` describes version 1 or 2 as
      generating.
- [ ] No test is pinned to version 1 or 2. The one exception,
      `each_biome_scatters_its_trees_at_its_density_class`, runs under version 3
      ignored with a reason naming ticket 13.

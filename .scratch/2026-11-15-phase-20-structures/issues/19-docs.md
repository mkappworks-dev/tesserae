# 19: Docs and the roadmap

Status: done

Blocked by: 11, 17, 18

Spec: [Phase 20](../spec.md).

## What to build

The authoring guide and data-mods documents describe Templates, the `ground`
entry, the Biome `structures` and `settlements` rows and Settlements, the
contract-compatibility document lists them, and DESIGN.md marks step 20 done
with its measured numbers and adds step 20b, Blueprints and Sites.

## Acceptance criteria

- [x] The authoring guide has Templates and Settlements sections with worked
      examples, and explains Obtainable.
- [x] The data-mods and contract-compatibility documents list the new fields.
- [x] DESIGN.md's step 20 row says Done, its note records what shipped and what
      did not, and a step 20b row exists.
- [x] ADR-0029 is checked against what was built and amended where it differs.

## Comments

- Authoring guide: a new `## Templates and Settlements` section (Templates,
  Obtainable, Biomes stand Structures, Settlements, Where they stand, The place
  Tool), the schema table rows for `[[templates]]`, `[[settlements]]` and the
  Biome `structures` and `settlements` rows, and `place` in the tool kinds.
- The worked examples are tested: the hut and the hamlet are also in "A
  Declaration using every table", which `dev::sdk::the_guides_full_declaration_
  validates_with_its_artwork` extracts and validates, Obtainable check
  included (checked by swapping in an un-Obtainable Material: the refusal
  printed is the one the guide quotes).
- Data-mods lists every new field; contract-compatibility already listed the
  WIT fields from tickets 02, 03, 10, 13 and 14 and gained the Obtainable rule
  (a loader rule, not a field).
- DESIGN.md: step 20 Done with what shipped and what did not (20b, the undo
  budget and `MAX_EDIT_EVENTS` gap, R shared with Respawn, ticket 11's mockup
  differences, no foundation or Sound from the Tool); ticket 18's benchmark
  text kept whole, including the bare 2 to 8.5% open item; a 20b row; step
  18's "tree templates (step 20)" now says step 20 kept trees parametric.
- ADR-0029 amended ("as built", 2026-11-16): Plot rule vs Template tolerance,
  door rule, road grading, the layout cache, Obtainable over Items and checking
  every Template, `ground` never rock, snapshot keeps only used Templates, the
  Tool's turn from R not facing, the undo refusal and gap.
- GLOSSARY: added Apron and Road.
- For ticket 20: its spread and roads are described in one place, the guide's
  `### Where they stand`; DESIGN.md links there instead of repeating it. The
  Settlement row's `chance` is described in `<!-- ticket-20 -->` ...
  `<!-- /ticket-20 -->` blocks in the authoring guide, data-mods and
  contract-compatibility. The guide's tested example has a Biome
  `settlements` row without `chance`: if ticket 20 makes it required, that
  test fails until the row gains one.

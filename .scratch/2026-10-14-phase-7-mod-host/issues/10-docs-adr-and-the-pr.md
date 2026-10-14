# Docs, ADR-0012 and the PR

Status: ready-for-agent
Blocked by: 09

Closes the step. Nothing here changes behaviour.

## What to build

- `docs/adr/0012-mods-declare-once-and-command-in-batches.md`: the Mod host's
  hard-to-reverse decisions, in the house style: the contract is two imports and
  no read; commands are metres and go through `set_voxel`; Mod code is a Job
  driven asynchronously, and why (the sleep); fuel and not epochs; the Game is
  the only source of World parameters; the Registry's order is load order; Trust
  is per exact build. Link ADR-0001, 0002, 0003, 0004, 0007.
- `DESIGN.md`: step 7 marked Done with the numbers (fuel per ms, bench results,
  `MOD_VOXELS_PER_TICK`), the Open items entry for the step-6 loose end "the
  reach is not configurable at run time" resolved, and the loose ends of the
  spec's list added.
- `docs/architecture.md`: the Layout tree and the `tesserae-mods` box updated;
  ADR list gains 0012.
- `CONTEXT.md`: Manifest, Declaration, Registry, Command, Command batch, Trust.
  Glossary only, no decisions.
- ADR-0009: a one-sentence note that appearance for a Mod's Material is the
  Declaration's stopgap until step 9.
- The spec's Outcome section, one subsection per ticket with what changed from
  the text and why, and the claims decision 9 and decision 2 made that the
  tickets proved or disproved.
- `CONTRIBUTING.md` or the README: how to build a guest (`rustup target add
  wasm32-wasip2`, `scripts/build-guests`) in one short paragraph.
- Run `npm run fmt` on everything **except** `.scratch/`; `.scratch/` is ignored
  by Prettier on purpose and hand-wrapped at 80.
- The PR: push the branch, print the compare URL (`gh` is not installed), and
  print the title and the whole body inline in a four-backtick fenced block for
  copying. Title `feat(mods): ...` in Conventional Commits. Body from
  `.github/pull_request_template.md`: concise bullets, one per point throughout,
  the checklist ticked where it applies and struck through where it does not,
  hard-wrapped at 80 columns, **no** "Generated with Claude Code" line. Commit
  messages keep the `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`
  trailer.

## Acceptance criteria

- `./scripts/cargo-ci` passes in full, benches included as compile checks.
- Every ticket 01 to 09 has `Status: done` and Comments; the spec's Done when is
  checked line by line in Comments, with what is not met stated.

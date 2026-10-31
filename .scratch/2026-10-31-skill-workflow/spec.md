Status: ready-for-agent

# Skill workflow, glossary rename and PR body

## Problem

The `mattpocock-skills` plugin is at 1.3.1 on the dev machine and the repo's
docs have drifted from it:

- 1.3.0 renamed the domain doc `CONTEXT.md` to `GLOSSARY.md`; the skills only
  look for the new name, so `domain-modeling`, `triage` and `tdd` cannot find
  our glossary.
- `CONTRIBUTING.md` does not pin a plugin version, grill, `implement-spec`,
  `code-review` or `retro`.
- The PR template and the plugin's `pr` skill disagree on the body's shape.

## Decisions

- Plugin install stays; no skills are vendored. `CONTRIBUTING.md` pins 1.3.1.
- `git mv CONTEXT.md GLOSSARY.md`; update every live reference (docs, template,
  source comments). Historical `.scratch/` files keep the old name.
- Chain: `/grill-with-docs` (optional, for non-trivial work) → `/to-spec` →
  `/to-tickets` → `/triage` → `/implement` per ticket, or `/implement-spec`
  when several tickets are unblocked at once → `/code-review` (already run by
  `/implement-spec`) → `/retro` (optional) → PR body via `pr`.
- `/implement-spec` does not replace tickets; it parallelises them.
- PR body order: Summary (a visual), Evidence (before/after), Merge Danger
  (door and blast radius), then the existing checklist. The `Changes` bullets
  are dropped.

## Out of scope

- Vendoring or forking skills.
- Rewriting past `.scratch/` specs and tickets.

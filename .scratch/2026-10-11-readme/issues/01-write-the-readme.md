# Write README.md

Status: done
Blocked by: none

## Task

Write `README.md` to the spec. Lead with what it is and what state it is in,
not with ambition.

Gather the commands from CONTRIBUTING.md, `.github/pull_request_template.md`
and `.github/workflows/ci.yml` into a single block, and run each one before
claiming it works.

## Done when

The file exists, `npm run fmt:check` passes, and every command in it has been
executed.

## Notes

Keep CONTRIBUTING.md as the detailed source — the README's command block is a
quick reference, not a second copy of the dev loop. Link across rather than
duplicating prose.

The licence section matters more than usual here. GitHub's sidebar shows only
"View license" for BUSL, so the README is the only place a visitor learns the
terms before reading the LICENSE file itself.

## Comments

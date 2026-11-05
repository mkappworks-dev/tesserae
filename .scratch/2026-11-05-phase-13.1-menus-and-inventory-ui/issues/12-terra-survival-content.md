# 12: Terra Survival content

Status: ready-for-agent

Blocked by: 08

Spec: [Phase 13.1](../spec.md).

## What to build

Terra declares at least one food Item with its food value, so hunger can be survived in the reference Game, and declares contact damage for lava in every Game that declares lava. Any other Game's Survival numbers stay at the engine defaults unless this ticket finds a reason to change them, recorded in the ticket.

## Acceptance criteria

- [ ] Terra has a food Item that can be gathered or taken from the Catalogue.
- [ ] Lava hurts on contact in every Game that declares it.
- [ ] Existing World saves of every Game still open.

# 02: Support the complete current Declaration as data

Status: ready-for-agent

Blocked by: 01

## What to build

An author can express all current contract 0.3 Declaration capabilities as TOML
and observe the same registered content and behaviour as a component
Declaration. This includes Shapes and connections, appearance, World and view
settings, movement, character definitions, fluids, reactions, combustion and the
existing Tool kinds.

Use the existing pixel representation for artwork at this stage; ticket 03 adds
PNG references. Extend the shared data representation rather than adding a
second content registry or simulation path.

## Acceptance criteria

- [ ] Every current Declaration field has a documented TOML representation,
      including nested records, optional values and closed variants. Absent
      fields follow documented contract semantics rather than inventing
      additional gameplay defaults.
- [ ] Equivalent data and component Declarations produce equivalent registered
      Shapes, appearance, behaviour and Game settings.
- [ ] Game-only restrictions, supported Tool kinds, character bounds, geometry
      bounds, numeric ranges and per-Mod limits use shared validation.
- [ ] Cross-Mod Material references resolve after all Declarations, in the
      existing dependency order, including mixed data and component content.
- [ ] A fixture demonstrates declared fluids, reactions and combustion reaching
      native simulation, and a Game fixture exposes a declared Tool.
- [ ] Unknown fields or variants, invalid optional values and ambiguous
      behaviour rows fail with the Mod and offending field identified.
- [ ] Data-only Mods remain contract 0.3 content with no guest callbacks or
      runtime Command producer. Older component adapters retain their existing
      behaviour and restrictions.
- [ ] Tests cover representative success and refusal cases at registration and
      Server boundaries, including character data, connected Shapes, Game-only
      fields and mixed-content references.

## Verification

Extend existing Declaration and behaviour fixtures; test observable content and
simulation outcomes rather than duplicating decoder implementation details.
Record focused test and formatting results when completing this ticket.

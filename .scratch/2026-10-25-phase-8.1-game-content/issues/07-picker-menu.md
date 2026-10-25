# 07: A menu picks anything declared into the hotbar

Status: ready-for-agent
Blocked by: 06

Spec: [decision 14](../spec.md).

## What to build

An engine Client menu, opened and closed with Tab and also closed by Escape.
While it is open, the cursor is free and mouse look and edit clicks are off;
closing it grabs the cursor again. It lists, from the content catalogue:
- solid Materials, then the Shapes for the chosen one;
- fluids;
- fire, when anything burns.

Each entry shows its full name and its placeholder colour. Choosing one puts it
in the selected hotbar slot, which then behaves as ticket 06 describes. The
Game has no say over the menu. It is engine UI drawn with Bevy's UI.

## Acceptance criteria

- [ ] With the demo loaded, the menu offers every registered solid Material,
      including `demo:wood`, `demo:brick` and `demo:lamp`, in every registered
      Shape, the dev fence included in a `dev` build. It also offers
      `demo:water`, `demo:lava`, `demo:mud` and fire.
- [ ] Given a catalogue with no fluid and no fuel, it offers the built-in solids
      and Shapes only, with no fluid section and no fire.
- [ ] Choosing an entry fills the selected slot. The next click sends that
      slot's intent once the menu is closed.
- [ ] No edit intent is sent and the camera does not turn while the menu is
      open.
- [ ] Placing a declared solid chosen from the menu is accepted by the Server
      and reported to Mods as a solid Edit.
- [ ] A screenshot pass of the open menu with the demo loaded is reviewed with
      the user and then deleted. Screenshots are never committed.
- [ ] CONTRIBUTING's controls table lists Tab.
- [ ] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.

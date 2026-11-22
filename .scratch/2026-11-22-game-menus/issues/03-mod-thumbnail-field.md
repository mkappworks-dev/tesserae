# 03: Optional thumbnail in mod.toml

Status: ready-for-agent

Blocked by: [01: Approve the mockup](./01-mockup-approval.md)

Spec: [Menus that feel like a game](../spec.md).

## What to build

A Mod or Game may declare a thumbnail in mod.toml: a PNG inside its folder,
512×288. Validation rejects a path outside the folder, a non-PNG, a wrong size
or an oversized file and reports each by path and reason. Thumbnails are display
data and may be shown before trust. The demo Games and the Timber, Crystals,
Hearth and Frost Mods ship one, and the Mods library on the title screen shows
them; a Mod without one shows one swatch per declared Material.

## Acceptance criteria

- [ ] An ADR records the contract change; the data-Mods documentation and
      authoring guide describe the field
- [ ] Validation reports each bad thumbnail case by path and reason
- [ ] A Mod without a thumbnail loads and shows its Material swatches
- [ ] The Mods library shows each Mod's thumbnail and a summary counted from its
      declaration

## Comments

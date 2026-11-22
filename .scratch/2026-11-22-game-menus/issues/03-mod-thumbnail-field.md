# 03: Optional thumbnail in mod.toml

Status: done

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

- [x] An ADR records the contract change; the data-Mods documentation and
      authoring guide describe the field
- [x] Validation reports each bad thumbnail case by path and reason
- [x] A Mod without a thumbnail loads and shows its Material swatches
- [x] The Mods library shows each Mod's thumbnail and a summary counted from its
      declaration

## Comments

- ADR-0037. The thumbnail file stays out of build identity, but adding the
  `thumbnail` line changes the Manifest bytes: Worlds saved with the old demo
  builds no longer open, and Trust is asked again.
- Thumbnails come from `scripts/generate-thumbnails` (Mods: their own textures
  as Voxels; Games: a seeded island in the Game's colours), not copies of the
  mockup's illustrations.
- A component Mod has no summary or swatches before it runs; the library shows
  its description only. The Mods step (08) and Game step (05) reuse
  `pages::picture` and `Installed::adds`.
- The summary line leaves out kinds a Mod adds none of, where the mockup always
  printed a Material count.

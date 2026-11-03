# 03: Load PNG textures and character skins

Status: ready-for-agent

Blocked by: 01

## What to build

An author references local PNG artwork from a data Declaration and sees it used
for Material surfaces and item previews, or the Game's player character. The
artwork is part of the Mod's exact content build, not an unchecked file read
after Trust or saved-World compatibility checks.

This ticket may add the appearance and character data support needed for its own
complete path without waiting for ticket 02. Keep that representation compatible
with the full Declaration model; ticket 02 is not a blocker.

## Acceptance criteria

- [ ] Material appearance accepts a local PNG reference and produces the
      existing 48 by 16 RGBA atlas. Game character artwork accepts a local PNG
      reference and produces the existing 64 by 64 RGBA skin.
- [ ] Existing colour, alpha, geometry and artwork limits remain enforced;
      decoding is bounded and does not relax dimensions or content limits.
- [ ] Missing files, invalid PNGs and incorrect dimensions fail before World
      opening with the Mod and artwork field identified.
- [ ] Artwork references remain inside their Mod directory after symlink
      resolution; escaping paths are refused.
- [ ] Manifest, Declaration and referenced artwork bytes contribute to build
      identity. Changing referenced artwork changes identity; unrelated PNGs and
      editor files do not.
- [ ] The artwork consumed is the artwork whose identity was checked. Changed
      files cannot silently replace a previously checked payload.
- [ ] An unchanged packaged data Game with artwork can save and reopen; changed
      artwork is rejected under exact-build compatibility before save writes.
- [ ] Tests verify decoded artwork reaches registered appearance or character
      data and the Client-facing content path, and cover invalid artwork,
      directory escapes, identity changes and unchanged reopening.

## Verification

Use small known PNG fixtures with distinguishable pixels and alpha. Verify
content data and existing rendering integration rather than private decoder
helpers. Record focused test and formatting results when completing this ticket.

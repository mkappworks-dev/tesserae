# Only air is built in; the Game names the ground

Status: accepted. Amends ADR-0009 and ADR-0015's permanent Material lists.

## Context

The engine owns mechanics and Games declare content. Six built-in solids still
chose every World's ground and supplied the gallery's demonstration Materials.
There are no saves yet, so retiring their ids now avoids a later migration.

## Decision

Air, id 0, is the only built-in Material. Ids 1 to 7 remain retired and are
never reused. The Material table keeps opaque stand-in rows but reports them
unknown; the Registry keeps unnamed slots. Mod Materials still start at 8.
Commands and player Edits naming retired ids fail as unknown Materials.

A Game's World parameters name optional ground: rock, soil and surface.
Generation uses these layers, including the surface Material for smoothing.
Names resolve after every Declaration and behaviour row. Unknown, non-solid or
fluid ground fails the load, as do lakes without ground. Without ground,
generation yields air while the computed spawn height remains unchanged.

The unreleased 0.3 contract is amended in place. No 0.4 line is introduced. This
amends ADR-0012: 0.2 and 0.1 World parameters convert without ground and log a
warning. Retired core names are unknown on every contract line; no adapter maps
them to Game content. Frozen builder and echo components still return their
original ids, and validation refuses their retired placements.

The demo declares its own solids and ground. The engine's default World has no
ground, lakes or playable Materials. Its palette contains no Material colours
beyond air's sentinel; the flame colour remains engine drawing. Tests and
benches register fixture Materials and use the returned ids.

The dev gallery remains an engine diagnostic under ADR-0006 and ADR-0010, but
chooses Materials and connected Shapes from declared properties. It registers no
content, omits parts without matching content, and is absent without ground.
Built-in Shapes remain engine kinds.

The user extended appearance during review to match previews and placed Voxels.
The unreleased 0.3 appearance record carries one optional texture: a 48 by 16
RGBA atlas with top, side and bottom tiles, exactly 3072 bytes. The host accepts
at most 64 appearance rows per Mod and validates byte lengths before publishing
them to the Client. Adapters for 0.1 and 0.2 supply no texture, preserving
colour fallback. The demo owns its pixel patterns; the Client uploads one shared
atlas for World surfaces, tiles, drag feedback, held models and ghosts. Greedy
faces repeat per Voxel, and depth/shadow cutouts sample the same alpha as the
visible face. Fire's artwork remains engine drawing. Basic texture appearance
amends ADR-0009's temporary flat palette; the Look-pack API remains separate.

## Consequences

A Game controls every named Material in its World. A World without a Game opens
as empty air and explains this in the startup log. Save support must preserve
the retired range and name-to-id Registry. Broader tombstones and missing-Mod
migration remain separate decisions.

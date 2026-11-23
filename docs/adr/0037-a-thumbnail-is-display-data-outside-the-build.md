# A thumbnail is display data outside the build

Status: accepted.

## Context

Menus list Games and Mods by name, category and a line of description, so a
Player cannot tell what a Mod adds before enabling it. A picture is wanted, and
it must show before Trust: a Player decides whether to trust a Mod partly from
how it is presented. Nothing that runs may be needed to draw it
([ADR-0001](0001-wasm-mod-boundary.md)).

## Decision

**A Manifest may name a `thumbnail`**: a PNG inside the Mod's folder, exactly
512 by 288 pixels and at most 512 KiB. It is confined like every other path
([ADR-0020](0020-data-only-mods-and-development-restarts.md)), and the scan
decodes it, so a folder whose thumbnail is outside it, not a PNG, the wrong size
or too large is refused as a row naming the file and the reason, not loaded with
a broken picture.

**The thumbnail file is not part of the build.** Build identity covers what
runs: the Manifest, the Declaration or component, and the files a Declaration
references ([ADR-0012](0012-mods-declare-once-and-command-in-batches.md)). A
thumbnail is shown before Trust, so Trust never vouches for it; hashing it would
make a new picture ask for Trust again and break every saved World made with
that build ([ADR-0018](0018-games-and-mods-install-side-by-side.md)) without
changing anything the World runs. The `thumbnail` line itself is Manifest bytes,
so adding or renaming it is a new build like any Manifest edit. Packaging copies
the file beside the Manifest.

**A Mod's summary is counted from its Declaration**: Materials, Shapes and
fluids, and one swatch colour per Material from its appearance. A Mod without a
thumbnail shows those swatches. Parsing a data Declaration runs no Mod code, so
it happens before Trust; a component must run to declare anything, so it has no
summary and no swatches until it is trusted and loaded.

## Consequences

- Adding `thumbnail` to an existing Mod's Manifest changes its build: Worlds
  saved with the old build no longer open with it.
- A picture and a summary cannot disagree with what loads only for the counts;
  the picture is the author's and is not checked against content.
- A Game without a thumbnail shows a neutral tile until it is trusted.

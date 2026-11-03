# Data-only Mods share validation; development reload restarts the World

Status: accepted. Implemented through the development loop and its build phase;
the manual running-window check is outstanding.

Step 10 designs the whole Modding SDK, implemented in increments starting with
data-only Mods. A basic Game can be authored using text files without Rust or a
WebAssembly toolchain. Its Declaration is TOML beside its Manifest, with a
documented schema and errors that identify the offending field.

Each Game or Mod initially has exactly one Declaration source: a data file or a
WebAssembly component. A World may combine both kinds of Mod. Both sources feed
the existing Declaration validation and registration path; data-only Mods do not
introduce an alternative content authority or scripted callbacks. This extends
ADR-0001 and ADR-0004's component-only loading model without changing the
batched guest boundary for executable Mods. Combining data and scripts within
one Mod is deferred, avoiding competing Declaration sources and merge rules.

The first development reload loop restarts a disposable development World when
content changes. It does not replace content inside a running World: rebuilding
the Registry may change ids, and Game settings may also change. Saved Worlds
retain ADR-0018's exact-build compatibility rule; development reload does not
provide save migration. Live reload remains a separate capability.

Data-only Mods initially follow the same explicit Trust policy as WebAssembly
Mods outside development mode. Development mode may trust selected local content
for that run without recording a permanent grant.

One development command watches data files and rebuilds a local WebAssembly Game
before restarting its disposable World. Automatic builds are explicitly enabled
for the local project. Parsing, compilation and content validation complete
before the current session closes. A failure at those stages reports the error,
keeps the current World running and waits for another edit. If opening a
validated replacement subsequently fails, the tool returns to a useful error
state without touching saved Worlds.

The development command watches the selected Game and its selected local Mods.
Only local projects explicitly enabled for automatic builds are rebuilt;
installed dependencies can load normally without enabling their build commands.
A successful restart regenerates the disposable World with the same seed, resets
the Player to the Game's initial spawn, and discards development Edits and
simulation progress. The command clearly identifies this World as disposable.

Data-only Mods can express every Declaration capability in the current contract,
including Shapes, fluids, reactions and Game settings, under the same validation
rules and Game-only restrictions as component Declarations. Material textures
and character skins may reference local PNG files instead of inline pixel
arrays. Images retain the existing dimension and size limits, participate in
build identity, and are watched for development restarts.

The SDK supplies a data-only starter and a Rust/WebAssembly starter, each usable
outside the engine checkout. The Rust starter uses a versioned WIT bundle, not
engine crate dependencies or paths into the checkout. Both starters are verified
from an external directory.

Data-only Mods initially target contract line 0.3 using the Manifest's existing
`api` field. Older component contract lines retain their existing support.
Breaking contract changes require a new line; introducing data-only content does
not itself require a new WIT line.

Declaration and PNG references stay inside their Mod's directory, including
after resolving symlinks. Build identity includes the Manifest and every
referenced content file; unrelated files such as editor settings are excluded.
This extends ADR-0012's Manifest-and-component build identity to data-only Mods
and their referenced artwork, preserving exact-content compatibility.

The SDK can validate a project without opening a World and produce a
self-contained installable folder. Output contains the Manifest and required
content, excluding development configuration and source files, in the existing
`games/<id>` and `mods/<id>` layout. The Rust starter's source and WIT bundle
remain in the authoring project, separate from its installed output.

The SDK establishes an authoring workflow and a WIT compatibility policy now;
the final 1.0 contract freeze remains in step 27 after later engine systems add
their APIs.

Acceptance tests exercise the existing Mod loader and Server startup boundaries,
both starters outside the checkout, failed reloads, and saved-World
compatibility. A manual edit-and-restart check complements the automated tests;
tests assert observable behaviour rather than private implementation details.

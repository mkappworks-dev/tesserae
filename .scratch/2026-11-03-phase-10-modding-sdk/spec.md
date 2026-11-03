# Step 10: Modding SDK

Status: ready-for-agent

## Problem Statement

A Game author currently needs a WebAssembly toolchain even when all their
content is a Declaration. The Rust examples depend on WIT paths inside the
Engine checkout, and editing guest code requires a manual build and World
reopen. There is no complete, independent path from a small authoring project to
validated, installable content with an automatic development loop.

Authors need that workflow before later gameplay systems extend the contract. It
must preserve the existing Server authority, shared content validation, Trust
policy and exact-build compatibility of saved Worlds.

## Solution

Provide a Modding SDK for both data-only and WebAssembly projects. An author can
start a basic Game using TOML and PNG files without Rust, or use an independent
Rust starter with a versioned WIT bundle for scripted behaviour. Both paths
produce content accepted by the same Engine loader.

Provide headless project validation, self-contained folder packaging, and one
development command that watches selected local projects. It prepares and
validates replacements before restarting a clearly disposable development World.
Successful restarts retain the seed but discard development Edits and simulation
progress and reset the Player to the Game's initial spawn.

Implement the whole step in increments, beginning with data-only loading. Define
compatibility policy now; leave the final 1.0 contract freeze to step 27.

## User Stories

1. As a Game author, I want to create a basic Game using text files, so that I
   can begin without installing Rust or a WebAssembly toolchain.
2. As a Mod author, I want to add content using the same data format as a Game,
   so that I do not need a separate extension mechanism.
3. As an author, I want TOML Declarations beside my Manifest, so that project
   metadata and content are easy to find and edit.
4. As an author, I want each Mod to have exactly one Declaration source, so that
   I can understand which content will load.
5. As a World creator, I want to combine data-only and WebAssembly Mods, so that
   choosing one authoring format does not restrict my other content.
6. As an author, I want to declare Materials and Shapes, so that I can build
   distinct terrain and geometry without guest code.
7. As an author, I want to declare appearance and reference PNG textures, so
   that my content uses ordinary artwork files.
8. As a Game author, I want to declare ground, generation, lakes and view
   settings, so that a text-only Game can produce its own World.
9. As a Game author, I want to declare movement, a character and its PNG skin,
   so that a data-only Game can configure the existing Player presentation.
10. As an author, I want to declare fluids, reactions and combustion, so that
    native Engine simulation can run my content without scripted callbacks.
11. As a Game author, I want to declare the existing Tool kinds, so that my
    data-only Game can expose supported interactions.
12. As an author, I want data-only Declarations to obey the component rules, so
    that switching format does not change content semantics or permissions.
13. As an author, I want errors to identify my Mod and offending field, so that
    I can fix invalid content without guessing.
14. As an author, I want missing or invalid artwork reported before opening a
    World, so that I can correct the project early.
15. As a user, I want content references confined to the installed Mod, so that
    its required files are self-contained.
16. As a user, I want the normal explicit Trust policy for either format, so
    that installing data content preserves the existing authorization model.
17. As a developer, I want selected local content trusted only for the run, so
    that iterative development does not create permanent Trust grants.
18. As a user, I want build identity to include referenced artwork, so that
    changing a texture cannot silently change the content of a saved World.
19. As a developer, I want unrelated editor files excluded from build identity,
    so that they do not invalidate unchanged installed content.
20. As a World owner, I want unchanged mixed content to reopen correctly, so
    that data-only support preserves my Registry and saved content selection.
21. As a World owner, I want incompatible builds refused before any write, so
    that development tooling does not silently migrate my saved World.
22. As an author, I want to validate a project without opening a World, so that
    I can check content from a terminal or automated build.
23. As an author, I want a self-contained installable folder, so that users do
    not need my source checkout or development configuration.
24. As an author, I want data-only and Rust starters usable outside the Engine
    checkout, so that independent Game development is supported in practice.
25. As a Rust author, I want a versioned WIT bundle without Engine crate
    dependencies, so that I build against the public contract.
26. As a developer, I want one command to watch data and PNG edits, so that I
    can test content changes without manually reopening the World.
27. As a Rust developer, I want that command to rebuild explicitly enabled local
    projects, so that code changes use the same development loop.
28. As a developer, I want selected local Mods watched with the Game, so that
    dependency edits trigger the appropriate replacement.
29. As a user, I want installed dependencies to remain loadable without
    automatic source builds, so that authoring configuration stays local.
30. As a developer, I want a failed build or validation to leave the current
    World running, so that I can correct an error without losing the session.
31. As a developer, I want a successful restart to use the same seed and initial
    spawn, so that comparisons between content versions are repeatable.
32. As a developer, I want the command to identify the World as disposable, so
    that I understand that Edits and simulation progress will be discarded.
33. As a developer, I want a useful error state if replacement startup fails, so
    that failure does not affect saved Worlds or require a silent recovery.
34. As a Mod author, I want documented contract compatibility rules, so that I
    understand what changes require a new contract line.

## Implementation Decisions

- ADR-0020 records the agreed design and its extensions to the component-only
  loading and build-identity decisions. Existing Server authority, native
  simulation and batched WebAssembly boundaries remain in force.
- The Manifest selects exactly one source: a TOML Declaration or a WebAssembly
  component. Reject ambiguous or missing source selection; retain compatibility
  with existing component Manifests.
- Data-only Mods initially use contract line 0.3 through the existing Manifest
  API field. Existing older component lines and their adapters remain supported
  under their current restrictions.
- Normalize data into the current Declaration model and use the existing
  validation, dependency-first registration and name resolution. Data-only Mods
  have no guest callbacks or runtime Command producer.
- Support all existing Declaration capabilities by completion of this step:
  Materials, Shapes and connections, appearance, World and view settings,
  movement, characters, fluids, reactions, combustion and Tools. Preserve
  Game-only restrictions and all existing bounds.
- TOML field diagnostics identify the Mod and offending field. Missing,
  malformed or invalid referenced files are load errors, not ignored content.
- PNG references supply Material textures and character skins. Convert to the
  existing pixel representation before shared validation; preserve the existing
  atlas dimensions, skin dimensions, alpha semantics and limits.
- Declaration and PNG paths must remain inside their Mod directory after symlink
  resolution. Do not add network asset references or external files.
- Extend exact build identity to include Manifest, Declaration and referenced
  artwork bytes. Unrelated source and editor files are excluded. Consume the
  content bytes whose identity was checked rather than silently rereading a
  changed payload. Preserve existing component identity semantics.
- Outside development mode, apply the existing explicit Trust policy to both
  source formats. Development Trust is limited to selected local content for
  that run and creates no permanent grant.
- Keep the existing World selection, dependency closure, Registry replay and
  exact-build reopening rules. Data-only support is not save migration.
- Provide headless validation using the same selected content and validation
  path as loading. It does not open or mutate a World; component Declarations
  still require the applicable Trust and sandbox policy.
- Packaging produces a self-contained installed folder in the existing Game and
  Mod layout, containing the Manifest and required runtime content. Exclude
  source and development configuration from the installed result.
- Supply a text-only starter and a Rust/WebAssembly starter, documented and
  verified from outside the Engine checkout. The Rust starter carries a
  versioned WIT bundle and does not depend on Engine crates or internal paths.
- One development command watches the selected Game and selected local Mods,
  including referenced artwork. Automatic rebuilding is explicitly enabled for
  local projects; installed dependencies can load without enabling builds.
- Prepare and validate candidate content before closing the active development
  World. Parsing, compilation or content validation failure leaves that World
  running, reports the failure and waits for a later edit.
- A successful reload starts a fresh disposable development World with the same
  seed and the Game's initial Player spawn. Development Edits, Player progress
  and simulation state are not transferred.
- Clearly identify the development World as disposable. Keep saved Worlds out of
  the development restart lifecycle. A later replacement-startup failure returns
  to a useful error state without modifying saved Worlds.
- Contract patches remain compatible; breaking closed records or variants
  require a new contract line. Publishing the SDK does not freeze all later
  gameplay APIs. The final 1.0 contract freeze stays in release hardening.
- CLI spelling, internal module boundaries and exact TOML serialization details
  are implementation choices within these constraints, not further product
  decisions requiring another interview.

## Testing Decisions

- Test externally observable behaviour at the existing Mod loader and Server
  startup boundaries. Reuse folder fixtures and content-registration tests;
  avoid asserting private helper layout or mirroring the decoder.
- Load equivalent data and component fixtures and compare registered content,
  behaviour and Game settings. Cover mixed dependency order and Game-only
  restrictions, not only successful TOML parsing.
- Exercise normal Trust, changed content, directory escapes and malformed assets
  through loading or validation. Verify identity changes for referenced content
  and stability for unrelated editor files.
- Verify saved-World reopen with unchanged data and mixed content, including
  Registry identity. Changed or missing required content is refused before
  writes. Retain existing older-component compatibility coverage.
- Run headless validation and packaging on representative projects; load the
  packaged output independently of source and development configuration.
- Copy both starters to a directory outside the checkout. Validate and package
  the data starter without a Rust build; build the Rust starter from its bundled
  WIT and exercise the same installed loading path.
- Exercise the development lifecycle with selected local dependencies, data and
  PNG changes, successful builds, failed builds, invalid Declarations, recovery
  after the next edit and failure after replacement startup begins. Assert
  session outcomes, seed, spawn, Trust persistence and saved-World isolation.
- Supplement automated lifecycle tests with a manual edit-and-restart check in
  the running application. Record the result during implementation; the design
  interview is not evidence that the loop has been tested.
- Run formatting, boundary checks and tests appropriate to each increment, then
  the repository's required checks for the integrated SDK.

## Out of Scope

- Live mutation of registered content inside a running World.
- Preserving development Edits or simulation state across a restart.
- Saved-World migration or bypassing exact-build compatibility.
- Combining a data Declaration and guest script within one Mod.
- Scripted callbacks or arbitrary guest behaviour in data-only Mods.
- Additional gameplay systems from later roadmap steps.
- Replacing existing component adapters, or data schemas for older lines.
- Automatic builds of arbitrary installed content.
- A Mod browser, remote asset loading, or automatic public publishing.
- A new Engine crate dependency for independent Games.
- The final 1.0 contract freeze, which belongs to step 27.

## Further Notes

The design and proposed testing boundaries were accepted through the
grill-with-docs interview. The whole SDK is specified here so that early
data-only increments support the later authoring loop. Individual tickets may
add capabilities incrementally; step 10 is complete only when the full scope and
acceptance checks above are satisfied.

The approved implementation breakdown is published as eight numbered tickets
under this effort's issues directory. Ticket 01 is the initial unblocked
frontier; later tickets can start only after their listed blockers are done.

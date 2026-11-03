# 01: Load a minimal data-only Game

Status: ready-for-agent

Blocked by: None (can start immediately)

## What to build

An author can install a small TOML Game, create a World with its declared ground
and Materials, and save and reopen it without a WebAssembly component or Rust
toolchain. A data-only content Mod can participate in the same selection and
dependency ordering as component Mods.

This is the first vertical slice of the parent SDK spec. Establish one source
per Manifest and normalize data into the existing Declaration validation path.
The initial schema can be limited to Materials, colour appearance and basic
World ground settings; unsupported fields must fail explicitly until ticket 02
adds the remaining capabilities. Do not silently ignore them.

## Acceptance criteria

- [ ] Existing component Manifests remain valid. A Manifest selects exactly one
      Declaration source; missing or ambiguous selection is refused.
- [ ] A contract 0.3 data Game and data content Mod are discovered in their
      existing installation locations, selected normally and loaded in
      dependency-first order alongside WebAssembly Mods.
- [ ] Declared Materials, colours and Game ground reach the Server and Client
      catalogue through existing registration. No guest instance or callbacks
      are required for a data-only Mod.
- [ ] Invalid TOML, unsupported fields, invalid values, unknown ground names,
      missing files and disallowed Game settings name the Mod and relevant field
      in the error. Declaration paths cannot escape the Mod directory, including
      through symlinks.
- [ ] Normal explicit Trust applies to data-only content. Manifest and
      Declaration bytes determine its exact build identity; loading consumes the
      bytes checked. Unrelated files do not change identity.
- [ ] An unchanged data or mixed-content World saves and reopens with the same
      Registry. Changed or missing required content is refused before writes.
- [ ] Behaviour tests exercise loader and Server startup boundaries, including
      shared validation, Trust refusal, mixed dependencies and saved reopening.
      Existing component compatibility remains covered.

## Verification

Use a minimal data Game with declared ground and a small data content Mod as
folder fixtures. Compare their registered content with equivalent component
Declarations. Record focused test and formatting results when completing this
ticket.

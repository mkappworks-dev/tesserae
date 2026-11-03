# 01: Load a minimal data-only Game

Status: done

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

- [x] Existing component Manifests remain valid. A Manifest selects exactly one
      Declaration source; missing or ambiguous selection is refused.
- [x] A contract 0.3 data Game and data content Mod are discovered in their
      existing installation locations, selected normally and loaded in
      dependency-first order alongside WebAssembly Mods.
- [x] Declared Materials, colours and Game ground reach the Server and Client
      catalogue through existing registration. No guest instance or callbacks
      are required for a data-only Mod.
- [x] Invalid TOML, unsupported fields, invalid values, unknown ground names,
      missing files and disallowed Game settings name the Mod and relevant field
      in the error. Declaration paths cannot escape the Mod directory, including
      through symlinks.
- [x] Normal explicit Trust applies to data-only content. Manifest and
      Declaration bytes determine its exact build identity; loading consumes the
      bytes checked. Unrelated files do not change identity.
- [x] An unchanged data or mixed-content World saves and reopens with the same
      Registry. Changed or missing required content is refused before writes.
- [x] Behaviour tests exercise loader and Server startup boundaries, including
      shared validation, Trust refusal, mixed dependencies and saved reopening.
      Existing component compatibility remains covered.

## Verification

Use a minimal data Game with declared ground and a small data content Mod as
folder fixtures. Compare their registered content with equivalent component
Declarations. Record focused test and formatting results when completing this
ticket.


## Comments

Implemented on `t3/load-data-only-game`, starting at the verified integration
and plan tip `b10490cf29fb317092bc1fb15af61f7456dadf36`.

The Manifest accepts exactly one `wasm` or `declaration` path. Data content uses
contract 0.3, normal selection, dependency ordering and explicit Trust. The
strict initial TOML schema normalizes Materials, colours and basic World/ground
settings into the existing Declaration registration and resolution path. Data
Mods create no guest instances and receive no callbacks. The schema and example
are documented in `docs/data-mods.md`; unsupported capabilities remain errors.

Trust retains an immutable ordered file snapshot. Data identity includes named
Declaration bytes and Manifest bytes; component hashing is unchanged, verified
against a fixed legacy digest and Trust record. The snapshot can collect PNG
files in ticket 03 without rereading checked artwork. Saved reopening compares
exact required builds before guest startup or save writes. Tests compare the
Registry on reopening and every saved-folder file after refused reopening.

TDD evidence at the approved loader and Server startup boundaries:

- Red: data discovery refused the unknown `declaration` Manifest field; green:
  discovered data requires explicit Trust and loads without a component.
- Red: startup refused data appearance; green: declared ground, colour and
  catalogue Material register without callbacks.
- Red: missing Declaration errors omitted the field; green: diagnostics name
  the Mod and `declaration`.
- Additional regression coverage checks source selection, unsupported fields,
  shared numeric/Game-only/name validation, symlink escapes after discovery,
  checked-byte identity, unrelated files, mixed dependency order, equivalent
  component Material properties, and unchanged/changed/missing saved content.

Verification (shared build cache via the authorized `CARGO_TARGET_DIR`):

- `./scripts/build-guests`: passed. Cached 0.3 fixtures were stale (eight rather
  than nine Declaration fields); rebuilt all guests and demo/meadow locally.
- `cargo test -p tesserae-mods --lib`: 124 passed, 3 existing measurement tests
  ignored, no failures.
- `cargo test -p tesserae server::mods::tests`: 32 passed, no failures.
- `cargo test -p tesserae --features dev server::mods`: 54 passed, 1 existing
  ignored test, no failures.
- `cargo test -p tesserae --features dev server::persistence`: 17 passed, no
  failures.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: passed.
- `cargo fmt --all`, `cargo fmt --all --check`, `npm run fmt`, and
  `npm run fmt:check`: passed.
- `./scripts/check-demo-boundary`: passed. Normal dependency trees for
  `tesserae-core` and `tesserae-mods` contain neither Bevy nor wgpu.
- `git diff --check`: passed.

A broader Mod runtime run without `dev` failed the existing demo scenario's
empty-log assertion: its spawn fixture is gated on `dev`. The same test passes
with CI's required `dev` configuration. No runtime test or simulation code was
changed to accommodate it. Full repository CI was not run.

Integration notes: ticket 02 should extend `data.rs`'s strict records and native
Declaration conversion, preserving shared `apply`/`resolve` validation. Ticket
03 should collect referenced, confined file bytes into `CheckedPayload.files`
before Trust comparison and decode from that snapshot. Data hashing already
frames the sorted file count, names and bytes; component identity keeps its
legacy algorithm. Rust Manifest constructors now set `wasm: Option<String>` and
`declaration: Option<String>`. Neither later ticket is implemented or closed.

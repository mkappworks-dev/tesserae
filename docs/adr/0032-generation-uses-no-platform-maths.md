# Generation uses no platform maths

Status: accepted.

Everything a Generator version derives from the seed (Chunks, their Overlay,
Settlements, Structures and spawn) calls `libm` for transcendental functions,
never `std`'s `sin`, `cos`, `atan2`, `exp`, `powf`, `powi` and kin, which Rust
documents as varying by platform and version. Clippy refuses them outside code
that explicitly allows them, and name-based golden digests run on macOS, Linux
and Windows on every PR. Basic float operations and `sqrt` are correctly rounded
everywhere and stay as they are.

**`libm` is pinned to an exact version.** Its answers are part of every
Generator version, so updating it is a generator change and must pass the
digests unchanged.

**Version 3 was corrected in place, once.** It had merged with `std`
trigonometry in Settlements, crop fields and the spawn search, and `powi` in
Surface features and landforms, but nothing had been released: every World made
under it was a development World, and versions 1 and 2 were deleted outright
([ADR-0033](0033-generator-version-3-is-the-only-version.md)). This overrides,
once, the rule that a World's pinned version never changes its terrain
([ADR-0022](0022-biomes-are-column-content-and-a-world-pins-its-generator.md))
and version 3's freeze
([ADR-0030](0030-generator-version-3-is-a-layered-field-stack-in-a-declared-band.md)).
Freezing it on `std` and adding a version 4 that differed only in its maths
would have left version 3 unportable for good and added a Generator version
nobody could see. After the first release, a fix to a Generator version, however
small, is a new version.

**The toolchain is not pinned.** With generation off `std`'s platform maths, a
new `rustc` has nothing platform-dependent left to change there; the digest job
is the canary when `stable` moves.

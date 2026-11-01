# Drop the RUSTSEC-2026-0344 ignore

Status: done

`deny.toml` ignores RUSTSEC-2026-0344, a decompression bomb in `ruzstd`: a
crafted `.zst` input under 128 KB can make it allocate gigabytes. It is ignored
and not fixed because Bevy 0.20 requires `ruzstd ^0.8` while the fix is only in
0.9.1, so `cargo update -p ruzstd` is refused. Revisit it; it should not live in
the policy file indefinitely.

## Why ignoring is safe today

`ruzstd` is enabled by Bevy's `zstd_rust` feature, which `bevy/3d_api` turns on,
and it decodes KTX2 supercompression only. The only KTX2 in this build is
Bevy's own embedded tonemapping and SMAA lookup tables (`include_bytes!` and
`embedded_asset!`). Tesserae loads no KTX2 of its own and its `assets/` holds only
a README, so no untrusted input reaches the decoder.

## Revisit when either happens

- **Bevy releases a version that depends on `ruzstd >= 0.9.1`.** Then
  `cargo update -p bevy` (or the Bevy bump itself) clears it and the ignore goes.
- **A Mod, a save or the network can supply a texture or any KTX2 data.** That
  makes the decoder reachable by untrusted input, so the ignore is no longer
  honest. At that point either decode such data with something else, turn
  `zstd_rust` off by moving off `bevy`'s default features, or carry a
  `[patch.crates-io]` backport of the 0.9.1 fix onto 0.8.3.

## What to do

1. `cargo tree -i ruzstd` shows who still pulls it and at which version.
2. If it is `>= 0.9.1`, delete the `ignore` entry (and its comment) from the
   `[advisories]` table in `deny.toml`.
3. Run `./scripts/cargo-ci`; `cargo deny check advisories` must pass without the
   entry.

## Done when

`deny.toml` has no `RUSTSEC-2026-0344` entry and `./scripts/cargo-ci` passes,
or the ignore is replaced by a fix that makes the decoder unreachable and the
reason is recorded here.

## Comments

Resolved by the Bevy 0.20.1 bump (`.scratch/2026-11-01-bevy-0.20.1/`): `bevy_image`
0.20.1 depends on `ruzstd` 0.9.1, and the ignore is removed from `deny.toml`.

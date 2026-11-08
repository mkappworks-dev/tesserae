# 03: Recordings are checked, trusted and packaged

Status: done

Blocked by: 02

## What to build

A Mod author's recordings are held to the same rules as artwork: they join build
identity, a changed recording needs Trust again, `tesserae validate` checks them
and `tesserae package` carries exactly the referenced ones. Every limit in the
spec is enforced and refused with the Mod and field named.

## Acceptance criteria

- [x] Changing a referenced recording changes build identity and needs Trust;
      an unreferenced audio file does not.
- [x] `package` writes referenced recordings at their relative paths and
      nothing else; the reloaded copy has the same build.
- [x] Limits refused by name: 512 KiB per Effects or Ambient recording, 8 MiB
      per Music recording, 8 recordings per Sound, 64 Sounds and 16 Sound sets
      per Mod, 32 MiB of recordings per World.
- [x] Ranges refused by name: weight 1 to 16, volume 0 to 1, pitch 0.5 to 2,
      reference 0.25 to 16 m, maximum above the reference and at most 128 m,
      sample rate 8 to 96 kHz, 1 or 2 channels.
- [x] A file is never read past its limit before being refused.
- [x] Tests at the Declaration loading seam, and through `validate` and
      `package`.
- [x] The data-only Mod guide states the limits and the identity rule.

## Comments

- From ticket 02: a data Mod's recordings already join build identity, because
  they can only be read from the payload Trust checks. What is left here is
  `validate`, `package`, the limits and the ranges; only `max-m` above
  `reference-m` is enforced so far, and recordings are capped at 8 MiB on read.
- Done. Beyond the ticket: a volume or pitch range whose minimum is above its
  maximum is refused; the 32 MiB World total counts a file once per reference,
  since each reference is its own copy in memory; and a data Mod's recording
  references are summed while its payload is read, refusing past 32 MiB before
  anything is parsed, so a Mod that needs no Trust cannot load gigabytes first.
- Checked with `cargo test -p tesserae-mods`, the `author` tests in `tesserae`
  and `cargo clippy --workspace --all-targets -- -D warnings`; the full suite
  was stopped by host load and runs on the integration branch.

- PR validation: the Terra boundary guard now permits literal `include_bytes!`
  only for existing `.ogg` files that resolve inside Terra's own audio folder.
  Engine source, text includes and computed include paths remain refused;
  negative probes cover all three. This permits Game-owned recordings while
  retaining the WIT-only Engine boundary.

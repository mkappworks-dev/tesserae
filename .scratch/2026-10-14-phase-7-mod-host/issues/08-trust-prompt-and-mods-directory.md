# The trust prompt and the mods directory

Status: ready-for-agent
Blocked by: 06

Spec decision 11 and the directory half of decision 3.

## What to build

- The default mods directory as `tesserae_mods::default_mods_dir()` (`dirs` for
  the platform data directory, plus `tesserae/mods`), `--mods <dir>` and
  `TESSERAE_MODS_DIR` in that order, and `trust.json` beside it. Create neither
  unasked: a missing directory is "no Mods".
- In `main`, before the `App`: for each Mod `Trust::check` returns `Untrusted`
  for, print the prompt of spec decision 11 (id, version, hash, root or not, the
  plain list of what a Mod can and cannot do, the limits) and read `y/N` from
  stdin. `y` grants and writes the record; anything else, including end of
  input, skips that Mod and prints that it was skipped. A skipped dependency of
  a trusted Mod is an error naming both.
- `--trust-all` and `TESSERAE_TRUST_MODS=1` grant without asking and print that
  they did; a build with neither and no terminal (stdin not a TTY) skips
  untrusted Mods and says why, never silently granting.
- The prompt text lives in one function that takes the Manifest and limits, so a
  window prompt later reuses it.

## Tests

- The prompt function: a snapshot of the text for a fixture Manifest, which
  names what a Mod cannot do (files, network, environment).
- Answer handling over an injected reader: `y`, `Y`, `n`, empty, end of input.
- Flag and environment precedence for the directory; `--trust-all` prints.
- No TTY and no flag grants nothing.
- A changed Mod is asked again; an unchanged one is not.

## Acceptance criteria

- `./scripts/cargo-ci` passes.
- Run once by hand against the `hello` fixture: the prompt appears, `n` skips
  it, `y` loads it and a second run does not ask. Paste the three outputs into
  Comments.

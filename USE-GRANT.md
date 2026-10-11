# What you may do with Tesserae

This is a plain-English summary. **[`LICENSE`](LICENSE) is the governing
document**; where the two differ, `LICENSE` wins.

Tesserae is licensed under the Business Source License 1.1. BSL's only
prohibition is production use, and Tesserae's Additional Use Grant gives that
back for everything except reselling the engine itself.

## Yes

- **Build a game on Tesserae and sell it.** Commercially, at any price, with no
  royalty, no revenue threshold and no reporting.
- **Ship your game's binaries** to players, on any store or platform.
- **Run servers** for your game, public or private, free or paid.
- **Write and sell mods** and look packs.
- **Modify the engine** for your game, including shipping your modified engine
  inside your game.
- **Fork the repository, study it, and redistribute your fork** — BSL permits
  copying, modification, derivative works and redistribution from day one.
- **Use it in a company**, for commercial game development, with no licence
  purchase.

## No

- **Offer Tesserae itself as a competing product.** You may not take the engine,
  or a modified version of it, and provide it to third parties as a game engine,
  a development tool, or a hosted or embedded service that competes with
  Tesserae.

That is the whole restriction. If you are building a game, it does not apply to
you.

## Things worth knowing

- **BSL is source-available, not open source.** It is not OSI-approved, and the
  licence text says so. Please don't describe Tesserae as open source.
- **Each release has its own four-year clock.** Four years after a version's
  first public distribution, that version becomes available under the Apache
  License 2.0, automatically and irrevocably. Older releases keep flipping as
  their dates arrive.
- **You cannot relicense a derivative work.** Redistributing your fork is fine;
  publishing it under MIT is not.
- **BSL code cannot be mixed into a GPL project** before its Change Date,
  because the pre-change grant is limited to non-production use.
- **`games/` and `assets/` are not BSL.** The demo game and the default assets
  are licensed permissively on purpose — they are templates to copy. See
  [CONTRIBUTING.md](CONTRIBUTING.md).
- **"Tesserae" is a trademark.** The licence grants no rights in it. You can
  ship a game built with Tesserae; you cannot ship something called Tesserae.

## Why BSL and not MIT

The engine is the thing worth protecting; games built on it are not competitors.
A permissive licence would also allow a well-resourced third party to take the
engine, rename it, and out-distribute it. The four-year clock means that
protection is temporary by construction — every line eventually becomes
Apache-2.0.

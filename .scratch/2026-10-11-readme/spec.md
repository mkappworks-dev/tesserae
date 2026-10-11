# A README

Status: done

## Problem

The repository has no README, and the commands needed to work in it are spread
across three files: CONTRIBUTING.md has the dev loop,
`.github/pull_request_template.md` has the pre-merge checks, and
`.github/workflows/ci.yml` has what actually gates a merge. The two commands
most likely to fail a PR — `cargo clippy --workspace --all-targets
--all-features` and `cargo test --workspace` — appear only in the template and
in CI, never in the guide someone reads to set the project up.

GitHub also shows a bare "View license" link rather than the licence name,
because `licensee` does not carry BUSL. With no README, a visitor lands on a
file listing and cannot tell what the project is, whether it runs, or what they
are allowed to do with it.

## Why now

Step 2 of the build order put something on screen. Before it there was nothing
to quick-start into; now `cargo run -p tesserae` opens a window, so a front
door has something to point at.

## What a good outcome looks like

Someone who has never seen the repository can tell within fifteen seconds what
Tesserae is, that it is not yet usable for anything, how to run it, and what the
licence permits. Someone who works in it daily can find every command in one
place.

## Scope

In scope:

- What the project is, in a sentence, using [CONTEXT.md](../../CONTEXT.md)'s
  vocabulary.
- **Status, stated plainly and early.** Build-order step 2 of 10. There is no
  gameplay, no mod API, no streaming, no LOD, no network play.
- Requirements and a quick start.
- Every command, in one block: run, format, test, lint, bench.
- Licence: BUSL-1.1, the four-year Change Date, Apache-2.0 as the Change
  License, and a pointer to [USE-GRANT.md](../../USE-GRANT.md).
- Pointers to CONTEXT.md, DESIGN.md, docs/architecture.md and CONTRIBUTING.md.

Out of scope:

- **Any comparison with Minecraft or Luanti.**
  [docs/positioning.md](../../docs/positioning.md) says in its second line
  "Internal. Not for the README or any public page", marks several rows
  unverified and probably wrong, and says to promote a row to public copy only
  once it is verified. None has been.
- Screenshots. There is nowhere to host one yet.
- Any claim about 4km view distance, LOD, WASM mods or look packs as things
  that work. They are steps 6, 7 and 9. They may appear as stated goals,
  labelled as goals.
- A contributor guide. CONTRIBUTING.md exists and is good.

## Constraints

The README is the one file in the repository that makes public claims, so
everything in it has to be true today or explicitly marked as intent. An
engine that renders nine chunks of test terrain must not read as an engine that
renders a 4km world.

## Done when

`README.md` exists, Prettier passes, every command in it has been run from a
clean checkout, and nothing in it contradicts DESIGN.md or claims a
positioning.md row.

## Comments

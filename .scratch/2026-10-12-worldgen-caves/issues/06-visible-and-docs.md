# Show it, decide `lightbox`, write it down

Status: done
Blocked by: 02, 05

## What to build

- Find a mouth the `dev` camera reaches (the camera starts about 20 m above
  spawn tipped down 17 degrees; the seed is fixed, so this is a search, not
  luck), take the screenshot with `TESSERAE_SCREENSHOT`, and show it.
- Decide `server/lightbox.rs` deliberately. It exists to make light visible.
  Caves now make sky-light propagation visible, but not an Edit's
  de-propagation (the roof) or block light (the lantern). Keep it, narrow it or
  delete it, and say which and why in the PR.
- CONTEXT.md: **Cave**, if the term is used in code or docs. DESIGN.md: the open
  item on the chunk budget (ticket 05) and a note under step 3 or 4 that caves
  exist and are off the build order. `docs/architecture.md` if the worldgen line
  needs it. `cargo fmt --all` and `npm run fmt`.
- The PR description in full in the session, in a fenced block of four
  backticks, no "Generated with Claude Code" line.

## Acceptance criteria

- The screenshot shows sky light falling off into a cave and Dirt and Stone in
  its walls.
- `./scripts/cargo-ci` passes, including `cargo fmt --all --check` and
  `npm run fmt:check`.
- Every ticket's `Status` is `done`, set in the commit that finished it.

## Comments

**The screenshot.** `target/screenshots/caves-mouth.png`, taken with

```
TESSERAE_SCREENSHOT_OFFSCREEN=1 TESSERAE_SCREENSHOT=out.png \
  TESSERAE_SCREENSHOT_FLY_S=3.4 TESSERAE_SCREENSHOT_SETTLE_S=5 cargo dev
```

The camera ends 40.6 m ahead of spawn (the flight is horizontal along -Z at
12 m/s), where a trench mouth 20-30 m to the right and a smaller one in the near
left are in frame. Sky light falls from lit grey down the trench wall to black;
Dirt and Stone are both in the walls of both mouths.

**Why `TESSERAE_SCREENSHOT_OFFSCREEN` exists.** Every window capture came out
pure black, on this branch and on `main` alike (checked in a clean checkout of
`main`), with the window on screen and the frame loop running at 60 Hz. The
cause was the session: `CGSSessionScreenIsLocked` was 1, so the window server
presented nothing and the camera never rendered into it. The note in step 5's
gallery ticket ("the first capture was black, a few seconds later it was fine")
was probably the same thing at a smaller scale. Rendering the camera into an
`Image` target and screenshotting that does not touch the display, so the same
command gives the same picture whether or not anyone is at the machine. It is
opt-in: the frame figures in ticket 05 want the real presentation path.

**Finding a mouth.** The seed is fixed, so it is a search and not luck: a
throwaway example listed the columns whose Grass cap a tunnel removes and
clustered them. Along the camera's path there is a 363-column trench at about
x = +26, z = -142 and a 220-column pit at x = -27, z = -96 (39 m deep). Mouths
are 4% of surface columns at the default.

**Decision: `server/lightbox.rs` is deleted.** It existed because light had
nothing real to act on. What it showed, and where that is now covered:

- *Sky light entering at a mouth and dimming along a floor.* The terrain does
  this now, in the screenshot above.
- *Block light.* `gallery.rs` places a lantern, and
  `the_two_solvers_agree_on_an_enclosed_tunnel_with_lanterns_on_its_seams`
  covers it on real geometry.
- *The de-propagation pass (the roof taking sky from a lit column).* This is the
  one thing the shelter showed live that nothing else does. It is now tested
  on cave geometry, digging and refilling a tunnel, and a mutation of
  `relight` that forgets one neighbour fails that test and no older one. What
  is lost is watching it happen in the running app; the finished picture is
  identical to a bulk solve, and the tests say so.

It also stood 46 m ahead of spawn, dead centre in the default view, which is
where the caves are. `builder.rs` stays, for the gallery.

**Docs.** CONTEXT.md gains **Cave** (and **mouth**). DESIGN.md's build order
notes caves under step 3 and the budget open item says what they did and did not
move. `docs/architecture.md`'s `noise.rs` line says it is no longer 2D only.
`capture.rs`'s module doc names the new variable.

# 17: Quiet controls HUD and stable, readable placement preview

Status: done
Blocked by: 16

User follow-up on 2026-10-27 for PR #21.

## What to build

Hide the controls panel by default. A visible information button and I
key toggle it. Keep diagnostics independent: F3 toggles the dev-only
chunk/FPS readout, also hidden by default. Rejections remain visible even
when help is closed.

Raise the placement ghost opacity from 40 to 75 percent while retaining
its shared texture. Stabilise Server-answered targeting during movement:
accept monotonic replies to recent known aim requests instead of requiring
an exact match to a sequence that changes every render frame. Keep the
last compatible reply briefly while waiting for the next, snap directly
to accepted cells, and hide replies that expire or belong to a different
held item, a distant Eye, a large turn or the open menu. No Client World
read or speculative placement resolver is introduced.

## Acceptance criteria

- [x] I/button toggles controls independently of F3 diagnostics.
- [x] Controls and dev diagnostics start hidden; rejection feedback remains.
- [x] Shared ghost artwork is readable at 75 percent opacity.
- [x] Moving aim remains visible across delayed replies and snaps to cells.
- [x] Old/out-of-order, incompatible or expired replies cannot remain drawn.
- [x] Relevant tests, native captures, formatting and full CI pass.

## Comments

The user also requested exploration of empty hands, erasing and click
bindings. That interaction model is being clarified separately; this
ticket fixes the independent presentation and targeting issues.

### 2026-10-27: implementation and validation

I/the information button toggle controls; F3 independently toggles the
dev readout. Both panels start hidden. Rejection text has a separate
always-available node. The button does not grab the cursor when clicked.
The ghost shares the Material artwork at 75 percent opacity.

The Client now keeps 64 known aim requests, accepts monotonic replies and
retains compatible answers for up to 200 ms. A changed item, Eye movement
beyond 2 m, turn beyond roughly 20 degrees, expiry or open menu hides the
answer. The ghost snaps directly to Server cells. This fixes the exact-
sequence gate that hid every delayed reply during continuous movement.
The Client still reads no World data and invents no placement position.

Tests cover independent key/button visibility and delayed/out-of-order,
expired and mismatched-item replies, including exact bounds replacement.
The final combined tree with ticket 18 passes formatting and full CI:
890 tests, zero failures, five existing ignored manual measurements.
Clippy, rustdoc, default build, rebuilt guests/demo, bench compilation,
boundaries and cargo-deny pass. As in the user's implement-first workflow,
individual commits are not independently validated; the combined tree is.

Native Metal captures are retained for review in
/tmp/tesserae-p82-controls-review-9x02y9zh: quiet-play.png,
controls-help.png, diagnostics.png, empty-hand.png and stronger-ghost.png.
There are no shader/atlas warnings. The existing Bevy/winit skipped
Destroyed warning occurs at screenshot-app shutdown. Captures show static
states; delayed-reply movement is covered by the headless regression test.
User review is pending, so the folder remains. Core/bench inputs did not
change and the existing interleaved benchmark evidence remains applicable.

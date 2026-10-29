# PR 3 after reverting all menu UI changes

The user requested reverting all menu UI changes. Menu, menu chrome,
settings, profile preferences and controls documentation are restored exactly
to merged main `4b86e43`. The new readability module was removed. Earlier
keyboard/organization/search/session changes were already reverted. Gameplay
input, opening identity, movement, physics, WIT and World saves match main.

Only the non-UI hotbar failed-write retry correction remains: the observed
snapshot advances after a successful atomic write, so clearing a temporary
write failure can retry without requiring another slot edit. Its regression
now tests recovery without such an edit. Existing per-Game arrangements,
active slot, names, Shape/Orientation and transient hand semantics are intact.

Superseded source changes are archived in:
`/tmp/tesserae-catalogue-review-UH4hl1/before-menu-revert/`.
Prior full-feature and partial-revert executables/captures are historical.
No settings/profile/World data was deleted. The user subsequently authorized
one consolidated commit, branch push and PR publication for the remaining
fix. Merge is not authorized.

Post-revert validation: all 151 Client regressions pass, including actual
menu/input behavior and failed-write recovery. Rustfmt, Prettier and diff
checks pass. Byte-for-byte comparison confirms the menu, menu chrome,
settings, preferences, gameplay input, opening identity and controls docs
match main. No menu code or sizing module remains in the diff.

The rebuilt executable and logs are retained under:
`/tmp/tesserae-catalogue-review-UH4hl1/after-menu-revert/`.
The earlier full/partial-revert executables remain superseded. Restart the
game to load the restored UI; an already-running process is not changed.
The earlier UI implementation CI results are historical. The final
non-UI fix was subsequently validated with the complete CI script below.


## Publication validation

The remaining source diff is confined to `client/hotbar_store.rs`: defer
updating the observed layout until the atomic write succeeds, and require
its existing write-failure regression to recover without another layout edit.
Complete `scripts/cargo-ci` passed for the final diff: 975 tests passed,
zero failed, five existing tests ignored, with no exclusions. All 151 Client
regressions passed. Formatting, all-target/all-feature Clippy, rustdoc,
default-feature checks, guest builds, benchmark compilation, architectural
boundaries and dependency checks passed. Cargo used caffeinate, warnings
denied and the requested shared cache. The full log is retained in
`/tmp/tesserae-catalogue-review-UH4hl1/after-menu-revert/publication-ci.log`.

The branch had no commits ahead of main before this publication step; the
remaining fix and notes are consolidated into one commit. Reverted UI work
is excluded from that commit. The user authorized push and PR publication;
merge remains unauthorized.


## Accepted ticket closure

The user subsequently requested marking tickets 05, 06 and 17 done against
the accepted existing main behavior, removing additions they do not want.
The tickets now describe only that completed behavior, with checked criteria.
The catalogue and creative sections of the spec and the execution plan are
aligned with this narrower scope, so removed requirements do not become
implicit follow-up tasks. This is a scope decision, not a claim that reverted
features were implemented. PR #25 still contains only the hotbar retry fix
as source code. These final closure edits are documentation-only.

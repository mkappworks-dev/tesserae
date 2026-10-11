# Make the `Dependencies` job pass with Bevy in the graph

Status: done
Blocked by: 01

## Task

`cargo deny check licenses bans sources` fails on this branch for two reasons,
only one of which this branch caused.

**Caused here.** Bevy pulls in `encase`, `encase_derive` and
`encase_derive_impl` under `MIT-0`, which is not in `deny.toml`'s allow list.
MIT-0 is MIT without the attribution clause, so it is strictly more permissive
than a licence already allowed, and the list exists to keep copyleft out of a
BSL-licensed binary.

Also caused here: `tesserae-core = { path = "../tesserae-core" }` has no
version, which `[bans] wildcards = "deny"` rejects. A path dependency without a
version is also unpublishable to crates.io, which DESIGN.md's distribution
table says is a supported channel.

**Pre-existing.** `tesserae-core`'s own `license = "BUSL-1.1"` is rejected, so
the job has been failing on `main` since the crate was added — the same shape
of problem as the `Boundaries` job in ticket 05, and not noticed for the same
reason.

## Done when

`cargo deny check advisories licenses bans sources` passes, a third-party
BUSL-licensed dependency would still be rejected, and `cargo publish --dry-run`
is not blocked by a versionless path dependency.

## Notes

BUSL-1.1 is allowed through a per-crate `[[licenses.exceptions]]` rather than
added to `allow`. The allow list describes what may be linked into our binary;
our own crates are the binary. Allowing BUSL-1.1 globally would also wave
through someone else's BSL crate, which is exactly what the policy is for.

`[bans] multiple-versions = "warn"` leaves roughly thirty duplicate-version
warnings from Bevy's dependency graph. Left as warnings deliberately: they are
Bevy's to resolve, not ours.

## Comments

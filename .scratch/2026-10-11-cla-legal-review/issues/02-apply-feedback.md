# Apply the reviewer's feedback to the CLA and LICENSE

Status: needs-info
Blocked by: 01

Cannot be specified until ticket 01 produces written feedback. Deliberately left
vague: writing a plan for changes nobody has asked for yet would be invention,
not planning.

## Task

Apply whatever the reviewer requires. Likely shapes, not commitments:

- Wording changes to section 9 of `.github/CLA.md`.
- A separate corporate CLA, if section 1's entity wording is judged
  insufficient.
- A governing-law clause.
- Changes to the Additional Use Grant in `LICENSE`.

## Constraints

- `LICENSE`'s **Terms and Covenants must stay byte-identical** to SPDX's
  `BUSL-1.1.txt`. Covenant 4 forbids modifying the License, and MariaDB's
  permission to use the text and the "Business Source License" name is
  conditional on the Covenants. Only the Parameters and Notice blocks are ours
  to change. Verify with:

  ```bash
  curl -sSfL https://raw.githubusercontent.com/spdx/license-list-data/main/text/BUSL-1.1.txt \
    -o /tmp/busl.txt
  diff <(sed -n '6,71p' /tmp/busl.txt) <(sed -n '/^Terms$/,$p' LICENSE)
  ```

- Bump the CLA's version number in its header. Anyone who signed v1.0 signed
  v1.0; a changed document is a different agreement.
- Keep `USE-GRANT.md` in step with `LICENSE`. A summary that contradicts the
  instrument is worse than no summary.

## Done when

Changes are applied, the CLA version is bumped, the `diff` above is empty, and
`USE-GRANT.md` still describes the grant accurately.

## Comments

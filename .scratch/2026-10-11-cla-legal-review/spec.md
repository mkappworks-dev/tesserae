# CLA and licensing parameters: legal review

Status: ready-for-human

## Problem

Under the Business Source License, [`.github/CLA.md`](../../.github/CLA.md) is
the only instrument that permits relicensing contributed code at the Change
Date. If it does not work, the project cannot do the one thing BSL promises:
convert to Apache-2.0 four years after each release. That makes it the most
consequential document in the repository, and it has not been reviewed by anyone
qualified.

The document was adapted from the Apache Software Foundation's Individual
Contributor License Agreement v2.2. Sections 1–8 are the ICLA's, with "the
Foundation" replaced by a named individual. Section 9 is ours. Neither the
substitution nor section 9 has been tested by a lawyer.

## Why now

Nothing breaks while the project is solo — there are no contributors, so there
is no CLA to rely on. The deadline is **the first outside contribution**. After
that, every unreviewed clause is a clause someone has already signed, and fixing
it means asking contributors to re-sign.

## What a good outcome looks like

- A lawyer familiar with open-source and source-available licensing has read
  `.github/CLA.md`, `LICENSE` and `USE-GRANT.md` as a set, not individually.
- Written feedback exists, in the repo or linked from it.
- Any required changes are applied, and the CLA's version number is bumped.
- CLA Assistant is wired up so signatures are actually collected and recorded.

## Scope

In scope:

- `.github/CLA.md` — especially section 9 (relicensing and the Change Date
  conveyance) and the foundation-to-individual substitution.
- `LICENSE` — the Parameters block: whether the Additional Use Grant says what
  we think it says, and whether naming an individual as Licensor creates
  problems that naming a company would not.
- `USE-GRANT.md` — whether the plain-English summary conflicts with the licence
  it summarises. A summary that contradicts the instrument is worse than no
  summary.
- Whether a separate corporate CLA is needed, as the ASF has, or whether the "or
  legal entity … authority to bind" wording in section 1 suffices.

Out of scope:

- Trademark registration for "Tesserae". Related, and worth the same
  engagement's time, but tracked separately.
- Choosing a different licence. BSL 1.1 with Apache-2.0 at four years is
  settled; see `DESIGN.md`.

## Known questions to put to the reviewer

1. Does section 9 actually let the Licensor convey the Apache-2.0 patent grant
   at the Change Date, given the patent licence in section 3 runs to "the
   Licensor and to recipients of software distributed by the Licensor"?
2. Is an individual Licensor assignable to a company later without re-papering
   contributions already received? Section 1 asserts it is.
3. Does the absence of a governing-law clause matter for an agreement signed by
   contributors worldwide? The Apache ICLA also omits one.
4. Moral rights are not addressed. In jurisdictions where they cannot be
   assigned, does that affect the relicensing right?
5. Is the retention statement under "Personal information" an adequate basis
   under GDPR for keeping a contributor's name and email indefinitely?

## Comments

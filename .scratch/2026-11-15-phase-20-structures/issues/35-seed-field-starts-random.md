# 35: The seed field starts with a random seed

Status: needs-info

Spec: [Phase 20](../spec.md).

## What to build

Play-testing asked: "by default in the randomize text field add a random seed
whenever a user goes to create a world".

Today the Create World form opens with the seed field blank: a seed is drawn
from the OS random source when the form opens (`Form::random_seed`), shown
only in the label under the field ("Random seed: N"), and used if the field is
left blank. Randomize puts a fresh random number in the field. Text is
resolved by `tesserae_core::seed::resolve`: a whole number is the seed, a
negative one its two's complement, and any other text the first 8 bytes of its
SHA-256. Nothing comes from time or place; worldgen later hashes the seed with
a fixed salt per feature and positions in metres.

- The field opens filled with the random seed, as the number it is, so it is
  seen and copied before the World is made. Randomize replaces it; typed words
  or numbers still resolve as before; a field emptied by hand still picks a
  random seed at creation.
- "Create from these settings" keeps the source World's seed.
- Mockup first (`mockup/app.js`: `randomSeed()`, `startCreate`, Randomize,
  the hint and the label), then the Client, once the mockup is approved.

## Acceptance criteria

- [x] The mockup opens the form with a random numeric seed and Randomize
      draws another.
- [ ] The mockup is approved.
- [ ] The Client's form opens with `random_seed` in the field, tested.

## Comments

Waiting for the user to approve the mockup before the Client changes.

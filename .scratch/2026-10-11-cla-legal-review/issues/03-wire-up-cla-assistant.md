# Install and configure CLA Assistant on the repository

Status: ready-for-human
Blocked by: 02

Requires repository admin and a GitHub App installation, so an agent cannot do
it. Blocked by 02 because collecting signatures against a document that is about
to change means collecting them twice.

## Task

- Install the [CLA Assistant](https://github.com/cla-assistant/cla-assistant)
  GitHub App on `mkappworks-dev/tesserae`.
- Point it at `.github/CLA.md`.
- Configure it to require a signature before a pull request can be merged, and
  to record signatures somewhere durable — a repository or a gist, not only the
  app's own state.
- Confirm the `Personal information` section of the CLA accurately describes
  what the app actually stores.

## Done when

Opening a pull request from an account that has not signed prompts for a
signature and blocks merge until it is given, and the signature record is
visible outside the app.

## Comments

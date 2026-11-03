# 04: Validate and package authoring projects

Status: ready-for-agent

Blocked by: 02, 03

## What to build

An author can validate a selected Game and its Mods without opening a World,
then produce self-contained installable folders. Validation and loading agree on
accepted content; packaging does not require the user's source tree when the
result is installed.

## Acceptance criteria

- [ ] A documented headless command validates the selected Game and dependency
      set using the same source loading, Trust, Declaration validation and
      reference resolution as normal startup.
- [ ] Validation does not open or mutate a World. Component Declaration calls
      retain the existing sandbox, limits and applicable Trust requirements.
- [ ] Validation returns success or a failing exit status with actionable
      diagnostics identifying the Mod and relevant field or file.
- [ ] Packaging handles data-only and already-built component content and
      produces the existing Game and Mod installation layout.
- [ ] Output contains the Manifest and every required Declaration, component or
      referenced image while preserving usable local references. Source,
      development configuration and unrelated files are excluded.
- [ ] Packaged content passes normal installed loading independently of the
      authoring directory and can be selected with its required dependencies.
- [ ] Packaging uses the checked content consistently; a changing or invalid
      input cannot be reported as a successfully validated package.
- [ ] Command-level tests cover valid data, valid components, mixed dependency
      sets, failures and loading the output after source is unavailable.

## Verification

Run the commands on representative projects from outside their output folder.
Load the results through the normal installed path and record focused test and
formatting results when completing this ticket.

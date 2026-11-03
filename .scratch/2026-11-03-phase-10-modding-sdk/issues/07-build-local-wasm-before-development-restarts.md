# 07: Rebuild enabled local WebAssembly projects before restart

Status: ready-for-agent

Blocked by: 06

## What to build

A Rust Game author uses the same development command as a data-only author.
Source edits rebuild explicitly enabled local projects and only validated,
successful builds replace the current disposable World.

This extends the development lifecycle with a build phase. It does not require
the new starter from ticket 05: existing independent component fixtures can
verify the feature.

## Acceptance criteria

- [ ] Local authoring configuration explicitly enables automatic building for
      each selected local project; content installation alone does not enable
      execution of a build command.
- [ ] Watch source changes for enabled local Game and Mod projects and rebuild
      affected outputs before candidate validation and World replacement.
- [ ] Selected data-only content and installed component dependencies continue
      to load without an unnecessary source build.
- [ ] Compilation failures report the project and useful build diagnostics,
      leave the current World running and wait for another source edit.
- [ ] Successful compilation followed by invalid content likewise preserves the
      current World. Only a fully prepared and validated candidate can initiate
      replacement.
- [ ] Further edits during a build do not cause an older output to replace a
      newer accepted candidate; generated output does not cause an endless
      rebuild loop.
- [ ] The replacement retains ticket 06's seed, initial spawn, disposable state,
      teardown, Trust and saved-World isolation behaviour.
- [ ] Lifecycle tests cover build failure and recovery, validation failure after
      compilation, selected local Mods, disabled builds and edits during an
      in-progress build.

## Verification

Use a local Rust component fixture for a real build-and-load check and
controlled build failures for lifecycle tests. Manually exercise code edit,
failure and correction in the running development loop; record the results.

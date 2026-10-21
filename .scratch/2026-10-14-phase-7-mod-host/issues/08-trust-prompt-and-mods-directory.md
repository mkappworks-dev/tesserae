# The trust prompt and the mods directory

Status: done
Blocked by: 06

Spec decision 11 and the directory half of decision 3.

## What to build

- The default mods directory as `tesserae_mods::default_mods_dir()` (`dirs` for
  the platform data directory, plus `tesserae/mods`), `--mods <dir>` and
  `TESSERAE_MODS_DIR` in that order, and `trust.json` beside it. Create neither
  unasked: a missing directory is "no Mods".
- In `main`, before the `App`: for each Mod `Trust::check` returns `Untrusted`
  for, print the prompt of spec decision 11 (id, version, hash, root or not, the
  plain list of what a Mod can and cannot do, the limits) and read `y/N` from
  stdin. `y` grants and writes the record; anything else, including end of
  input, skips that Mod and prints that it was skipped. A skipped dependency of
  a trusted Mod is an error naming both.
- `--trust-all` and `TESSERAE_TRUST_MODS=1` grant without asking and print that
  they did; a build with neither and no terminal (stdin not a TTY) skips
  untrusted Mods and says why, never silently granting.
- The prompt text lives in one function that takes the Manifest and limits, so a
  window prompt later reuses it.

## Tests

- The prompt function: a snapshot of the text for a fixture Manifest, which
  names what a Mod cannot do (files, network, environment).
- Answer handling over an injected reader: `y`, `Y`, `n`, empty, end of input.
- Flag and environment precedence for the directory; `--trust-all` prints.
- No TTY and no flag grants nothing.
- A changed Mod is asked again; an unchanged one is not.

## Acceptance criteria

- `./scripts/cargo-ci` passes.
- Run once by hand against the `hello` fixture: the prompt appears, `n` skips
  it, `y` loads it and a second run does not ask. Paste the three outputs into
  Comments.

## Comments

Implemented the existing pre-App `server/mods.rs::load_with` authorization hook,
with accepted Mods filtered before compilation and dependencies checked against
that accepted set. Skipped roots cannot contribute tables, world/view parameters,
or the Game flag. The accepted subset retains dependency-first/id order.

`tesserae_mods::default_mods_dir() -> Option<PathBuf>` uses `dirs::data_dir()`
plus `tesserae/mods`. Directory precedence remains CLI > environment > platform;
`trust.json` is a sibling. Discovery, refusal and empty loads create nothing.
`--trust-all` / `TESSERAE_TRUST_MODS=1` log each new grant. No terminal means an
explicit skip unless either override is set. Existing exact-build Trust works
without a terminal, and changed component or Manifest bytes ask again.

Public reusable interfaces for ticket09:

- `trust_prompt(&Manifest, ModLimits, &str) -> String`: the final argument is
  the build SHA-256; pass `ModLimits::HOST.lowered_by(&manifest.limits)`.
- `confirm_trust(&mut impl BufRead) -> io::Result<bool>`: only `y`/`Y` grants;
  `n`, empty input and EOF decline. Output is flushed before reading.
- `Trust::grant_exact(&Found, &str)` records only the displayed hash and rejects
  a component changed during the prompt; the loader rechecks before compiling.
- `LoadedMods` remains the pre-App result; `ModJobs` consumes its instances.
  No Client dependency, WIT, WASI, runtime queue, Edit, or two-Voxel cap changes.

Validation: `cargo fmt --all`, `npm run fmt`, full `./scripts/cargo-ci`
(including cargo-deny), and `cargo test -p tesserae --features dev --locked`.
Final logs: `target/phase7-runs/t08-ci-final.log` and
`t08-dev-tests-final.log`. Coverage includes prompt snapshot (files, network,
environment, effective limits), all answer cases, directory precedence, no-TTY,
logged override grants, missing/empty directories, changed/unchanged Trust,
change while prompting, skipped root tables, dependency errors for accepted and
recorded Mods, and deterministic registration after skipping.

Manual acceptance used a unique fixture directory:
`target/phase7-runs/t08-hello-v0j8wnzp/mods/hello`, copied from
`target/guests/hello.wasm`. All three runs used the actual binary under `script`
(a PTY), with `TESSERAE_TRUST_MODS` unset and `BEVY_ASSET_ROOT` set to this
checkout. No shared `/tmp` path was used. The first sandbox-only exploratory
run reached refusal but had no GPU; the accepted runs below used GPU access.
After the window/startup was observed each was stopped with terminal Ctrl-C;
the PTY command returned status 0. Bevy logged its window-destroy notification
warning during shutdown. The final run used the dev build, with no screenshot or
capture overrides; the first two used default features. A window prompt and
bulk relight remain outside this ticket.

### Run 1: n skips

```text
Mod: hello
Version: 0.1.0
SHA-256: 0508233991fa95bf56947315ef934b05205da7b2a10994d71a0d3b16d34ad25e
Game (root): no

Can add Materials and Shapes.
A Game can change World generation and view distance.
Can place and remove Voxels in loaded Chunks in batches.
Cannot access files, network or environment.

Limits: 200000000 fuel per call; yield every 10000000 fuel; 250 ms per call.
Memory: 67108864 bytes per memory; 10000 table elements; 4 instances; 4 memories; 4 tables.
Commands: at most 1024 per batch; 65536 Voxels per call; 32768 Voxels per fill.
Content: 56 Materials and 252 Shapes total; 64 of either per Mod; 32 bytes per name.

Trust this exact build? [y/N] n
Skipped Mod `hello`
2026-10-21T20:51:13.317885Z  INFO bevy_diagnostic::system_information_diagnostics_plugin::internal: SystemInfo { os: "macOS 27.0.1", kernel: "27.0.0", cpu: "Apple M5", core_count: "10", memory: "24.0 GiB" }
2026-10-21T20:51:13.504361Z  INFO bevy_render::renderer: AdapterInfo { name: "Apple M5", vendor: 0, device: 0, device_type: IntegratedGpu, device_pci_bus_id: "", driver: "", driver_info: "", backend: Metal, subgroup_min_size: 4, subgroup_max_size: 64, transient_saves_memory: Some(true), limit_bucket: None }
2026-10-21T20:51:13.690642Z  INFO bevy_pbr::cluster: GPU clustering is supported on this device.
2026-10-21T20:51:13.690685Z  INFO bevy_render::batching::gpu_preprocessing: GPU preprocessing is fully supported on this device.
2026-10-21T20:51:13.808268Z  INFO bevy_winit::system: Creating new window Tesserae (68v0)
^C2026-10-21T20:51:40.199555Z  WARN bevy_winit::state: Skipped event Destroyed for unknown winit Window Id WindowId(524310349056)
```

### Run 2: y grants and loads

```text
Mod: hello
Version: 0.1.0
SHA-256: 0508233991fa95bf56947315ef934b05205da7b2a10994d71a0d3b16d34ad25e
Game (root): no

Can add Materials and Shapes.
A Game can change World generation and view distance.
Can place and remove Voxels in loaded Chunks in batches.
Cannot access files, network or environment.

Limits: 200000000 fuel per call; yield every 10000000 fuel; 250 ms per call.
Memory: 67108864 bytes per memory; 10000 table elements; 4 instances; 4 memories; 4 tables.
Commands: at most 1024 per batch; 65536 Voxels per call; 32768 Voxels per fill.
Content: 56 Materials and 252 Shapes total; 64 of either per Mod; 32 bytes per name.

Trust this exact build? [y/N] y
Trusted Mod `hello`
Loaded Mod `hello`
2026-10-21T20:52:02.781358Z  INFO bevy_diagnostic::system_information_diagnostics_plugin::internal: SystemInfo { os: "macOS 27.0.1", kernel: "27.0.0", cpu: "Apple M5", core_count: "10", memory: "24.0 GiB" }
2026-10-21T20:52:02.855825Z  INFO bevy_render::renderer: AdapterInfo { name: "Apple M5", vendor: 0, device: 0, device_type: IntegratedGpu, device_pci_bus_id: "", driver: "", driver_info: "", backend: Metal, subgroup_min_size: 4, subgroup_max_size: 64, transient_saves_memory: Some(true), limit_bucket: None }
2026-10-21T20:52:03.049950Z  INFO bevy_pbr::cluster: GPU clustering is supported on this device.
2026-10-21T20:52:03.050001Z  INFO bevy_render::batching::gpu_preprocessing: GPU preprocessing is fully supported on this device.
2026-10-21T20:52:03.166207Z  INFO bevy_winit::system: Creating new window Tesserae (68v0)
^C2026-10-21T20:52:14.477811Z  WARN bevy_winit::state: Skipped event Destroyed for unknown winit Window Id WindowId(522780263680)
```

### Run 3: recorded Trust loads without asking

```text
Loaded Mod `hello`
2026-10-21T20:52:26.137737Z  INFO bevy_diagnostic::system_information_diagnostics_plugin::internal: SystemInfo { os: "macOS 27.0.1", kernel: "27.0.0", cpu: "Apple M5", core_count: "10", memory: "24.0 GiB" }
2026-10-21T20:52:26.877361Z  INFO bevy_render::renderer: AdapterInfo { name: "Apple M5", vendor: 0, device: 0, device_type: IntegratedGpu, device_pci_bus_id: "", driver: "", driver_info: "", backend: Metal, subgroup_min_size: 4, subgroup_max_size: 64, transient_saves_memory: Some(true), limit_bucket: None }
2026-10-21T20:52:27.033223Z  INFO bevy_pbr::cluster: GPU clustering is supported on this device.
2026-10-21T20:52:27.033429Z  INFO bevy_render::batching::gpu_preprocessing: GPU preprocessing is fully supported on this device.
2026-10-21T20:52:27.059127Z  INFO bevy_winit::system: Creating new window Tesserae (68v0)
2026-10-21T20:52:30.105294Z  INFO tesserae::server::gallery: gallery: 1178 Voxels at Vec3(11.0, 57.846817, -44.0)
^C2026-10-21T20:52:54.482015Z  WARN bevy_winit::state: Skipped event Destroyed for unknown winit Window Id WindowId(516427810048)
```

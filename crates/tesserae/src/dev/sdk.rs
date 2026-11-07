//! The integrated SDK check: each starter is copied outside the checkout and taken through the
//! steps the authoring guide lists, using the commands and files the starter's README gives.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::tests::Rig;
use crate::author::tests::{Scratch, failed, files, path, png};
use crate::server::mods::{TrustMode, load, reopen_world, save_world};
use crate::session::AppState;
use crate::world::WorldSettings;

fn starter(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../starters")
        .join(name)
}

/// A copy of a starter without build output, as an author would make with `cp -R`.
fn copy_to(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let (src, dst) = (entry.path(), to.join(entry.file_name()));
        if src.is_dir() {
            if entry.file_name() != "target" {
                copy_to(&src, &dst);
            }
        } else {
            fs::copy(&src, &dst).unwrap();
        }
    }
}

fn tesserae(s: &Scratch, args: &[&str]) -> Result<(), String> {
    match s.run(args) {
        crate::author::Outcome::Done => Ok(()),
        other => Err(failed(other)),
    }
}

/// The first fenced `toml` block of a README after `marker`, so the check runs the snippet a
/// reader copies.
fn readme_snippet(readme: &Path, marker: &str) -> String {
    let text = fs::read_to_string(readme).unwrap();
    let after = &text[text.find(marker).unwrap()..];
    let body = &after[after.find("```toml\n").unwrap() + "```toml\n".len()..];
    body[..body.find("```").unwrap()].to_owned()
}

fn installed(out: &Path, trust: &Path) -> crate::server::mods::LoadedMods {
    load(
        &out.join("mods"),
        trust,
        TrustMode::All,
        &WorldSettings::default(),
    )
    .unwrap()
}

/// Saves a World from the installed content, then checks it reopens unchanged, ignores a stray
/// file and is refused before any write once `change` alters the installed build.
fn saved_world_is_bound_to_the_build(
    s: &Scratch,
    out: &Path,
    installed_dir: &Path,
    change: impl Fn(&Path),
    undo: impl Fn(&Path),
) {
    let (trust, world) = (s.0.join("trust.json"), s.0.join("world"));
    save_world(&installed(out, &trust), &world);
    let reopen = || reopen_world(&world, &out.join("mods"), &trust);
    let before = files(&world);
    assert!(reopen().is_ok());

    fs::write(installed_dir.join("notes.txt"), "stray").unwrap();
    assert!(
        reopen().is_ok(),
        "an unrelated file is not part of the build"
    );
    fs::remove_file(installed_dir.join("notes.txt")).unwrap();

    change(installed_dir);
    let error = reopen().err().expect("a changed build is refused");
    assert!(error.contains("changed"), "{error}");
    assert_eq!(files(&world), before, "a refused reopen writes nothing");

    undo(installed_dir);
    assert!(reopen().is_ok());
}

#[test]
fn the_data_starter_runs_the_whole_authoring_path_from_outside_the_checkout() {
    let s = Scratch::outside();
    let project = s.dir("work/my_game");
    copy_to(&starter("data-only"), &project);
    let project_arg = path(&project);

    tesserae(&s, &["validate", project_arg]).unwrap();
    assert!(
        !s.0.join("trust.json").exists(),
        "validation records no Trust"
    );

    // The development loop, on the copy.
    let mut rig = Rig::with(|_, _| vec![project.clone()]);
    rig.open();
    let first = rig.session();
    let text = fs::read_to_string(project.join("content.toml")).unwrap();

    fs::write(
        project.join("content.toml"),
        text.replace("emission = 0", "emission = 6"),
    )
    .unwrap();
    rig.restarted(first);
    rig.settle();
    assert_eq!(rig.emission(), 6);

    fs::write(project.join("art/turf.png"), png(48, 16, 77)).unwrap();
    rig.restarted(first + 1);
    rig.settle();
    let after_art = rig.session();

    fs::write(project.join("notes.txt"), "editor scratch").unwrap();
    rig.settle();
    assert_eq!(
        rig.session(),
        after_art,
        "an unreferenced file restarts nothing"
    );

    // A wrongly sized texture is explained and the World runs on; a correct one recovers.
    fs::write(project.join("art/turf.png"), png(16, 16, 0)).unwrap();
    rig.settle();
    assert_eq!((rig.session(), rig.state()), (after_art, AppState::InWorld));
    let message = rig.diagnostic().unwrap();
    assert!(
        message.contains("starter_data") && message.contains("texture"),
        "{message}"
    );
    fs::write(project.join("art/turf.png"), png(48, 16, 78)).unwrap();
    rig.restarted(after_art);
    rig.settle();
    assert_eq!(rig.diagnostic(), None);
    assert!(
        !rig.s.0.join("installed/trust.json").exists(),
        "a run records no Trust"
    );

    // Package, install, load, save a World and hold it to the exact build.
    let out = s.dir("dist");
    tesserae(
        &s,
        &[
            "package",
            project_arg,
            "--out",
            path(&out),
            "--mods",
            path(&out.join("mods")),
        ],
    )
    .unwrap();
    let game = out.join("games/starter_data");
    assert_eq!(
        files(&game),
        ["art/stone.png", "art/turf.png", "content.toml", "mod.toml"]
    );
    let loaded = installed(&out, &s.0.join("trust.json"));
    assert!(loaded.game);
    assert_eq!(loaded.textures.len(), 2);
    saved_world_is_bound_to_the_build(
        &s,
        &out,
        &game,
        |dir| fs::write(dir.join("art/stone.png"), png(48, 16, 5)).unwrap(),
        |dir| {
            fs::copy(project.join("art/stone.png"), dir.join("art/stone.png"))
                .map(drop)
                .unwrap()
        },
    );
}

#[test]
fn the_rust_starter_builds_validates_rebuilds_and_mixes_with_data_content() {
    let s = Scratch::outside();
    let project = s.dir("work/my_rust_game");
    copy_to(&starter("rust"), &project);
    let readme = starter("rust").join("README.md");

    // The README's build and copy, run as written.
    let built = Command::new("cargo")
        .args(["build", "--target", "wasm32-wasip2", "--release"])
        .current_dir(&project)
        .env_remove("CARGO_TARGET_DIR")
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    fs::copy(
        project.join("target/wasm32-wasip2/release/starter_rust.wasm"),
        project.join("starter_rust.wasm"),
    )
    .unwrap();

    // A component needs a Trust grant to be checked, or the run-only flag.
    let message = tesserae(&s, &["validate", path(&project)]).unwrap_err();
    assert!(
        message.contains("starter_rust") && message.contains("--trust-for-run"),
        "{message}"
    );
    tesserae(&s, &["validate", path(&project), "--trust-for-run"]).unwrap();
    assert!(!s.0.join("trust.json").exists());

    // The development loop with the README's `dev.toml`, rebuilding through real Cargo.
    fs::write(
        project.join("dev.toml"),
        readme_snippet(&readme, "opt in with a `dev.toml`"),
    )
    .unwrap();
    let mut rig = Rig::with(|_, _| vec![project.clone()]);
    rig.open();
    let first = rig.session();
    let lib = project.join("src/lib.rs");
    let source = fs::read_to_string(&lib).unwrap();

    fs::write(&lib, source.replace("emission: 0", "emission: 4")).unwrap();
    rig.restarted(first);
    rig.settle();
    assert_eq!(rig.emission(), 4);
    let restarted = rig.session();

    // A compile error names the project and leaves the World running and the last build in place.
    let good = fs::read(project.join("starter_rust.wasm")).unwrap();
    fs::write(&lib, format!("{source}\ncompile_error!(\"boom\");\n")).unwrap();
    rig.settle();
    assert_eq!((rig.session(), rig.state()), (restarted, AppState::InWorld));
    let message = rig.diagnostic().unwrap();
    assert!(
        message.contains("starter_rust") && message.contains("boom"),
        "{message}"
    );
    assert_eq!(fs::read(project.join("starter_rust.wasm")).unwrap(), good);

    fs::write(&lib, source.replace("emission: 0", "emission: 9")).unwrap();
    rig.restarted(restarted);
    rig.settle();
    assert_eq!((rig.emission(), rig.diagnostic()), (9, None));
    assert!(
        !rig.s.0.join("installed/trust.json").exists(),
        "a run records no Trust"
    );
    drop(rig);

    // A Rust Game with a data Mod that depends on it, packaged together and installed.
    let extra = s.dir("work/extra");
    fs::write(
        extra.join("mod.toml"),
        "id = \"extra\"\nversion = \"0.1.0\"\napi = \"0.3\"\ndepends = [\"starter_rust\"]\n\
         declaration = \"content.toml\"\n",
    )
    .unwrap();
    fs::write(
        extra.join("content.toml"),
        "[[materials]]\nname = \"glow\"\nemission = 9\ndimming = 15\nocclusion = \"opaque\"\n\
         render = \"opaque\"\nsolid = true\n\n[[appearance]]\nmaterial = \"glow\"\n\
         color = [240, 200, 60, 255]\n",
    )
    .unwrap();
    let out = s.dir("dist");
    let mods = out.join("mods");
    tesserae(
        &s,
        &[
            "package",
            path(&project),
            path(&extra),
            "--out",
            path(&out),
            "--mods",
            path(&mods),
            "--trust-for-run",
        ],
    )
    .unwrap();
    assert_eq!(
        files(&out.join("games/starter_rust")),
        ["mod.toml", "starter_rust.wasm"]
    );
    assert_eq!(files(&out.join("mods/extra")), ["content.toml", "mod.toml"]);

    let trust = s.0.join("trust.json");
    let loaded = installed(&out, &trust);
    let ids: Vec<_> = loaded.required.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids, ["starter_rust", "extra"]);
    assert!(
        trust.exists(),
        "installing grants Trust to the exact builds"
    );

    saved_world_is_bound_to_the_build(
        &s,
        &out,
        &out.join("mods/extra"),
        |dir| {
            let text = fs::read_to_string(dir.join("content.toml")).unwrap();
            fs::write(
                dir.join("content.toml"),
                text.replace("emission = 9", "emission = 8"),
            )
            .unwrap();
        },
        |dir| {
            let text = fs::read_to_string(dir.join("content.toml")).unwrap();
            fs::write(
                dir.join("content.toml"),
                text.replace("emission = 8", "emission = 9"),
            )
            .unwrap();
        },
    );
}

/// The block under `heading` of the guide, so its examples are the ones that run.
fn guide_block(heading: &str) -> String {
    let guide = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/authoring-guide.md");
    readme_snippet(&guide, heading)
}

#[test]
fn the_guides_full_declaration_validates_with_its_artwork() {
    let s = Scratch::outside();
    let project = s.dir("showcase");
    fs::create_dir_all(project.join("art")).unwrap();
    fs::write(
        project.join("mod.toml"),
        "id = \"showcase\"\nversion = \"0.1.0\"\napi = \"0.3\"\nroot = true\n\
         declaration = \"content.toml\"\n",
    )
    .unwrap();
    fs::write(
        project.join("content.toml"),
        guide_block("### A Declaration using every table"),
    )
    .unwrap();
    fs::write(project.join("art/stone.png"), png(48, 16, 1)).unwrap();
    fs::write(project.join("skin.png"), png(64, 64, 2)).unwrap();
    fs::create_dir_all(project.join("sounds")).unwrap();
    fs::write(
        project.join("sounds/crack.ogg"),
        include_bytes!("../../../tesserae-mods/tests/fixtures/sounds/mono.ogg"),
    )
    .unwrap();
    tesserae(&s, &["validate", path(&project)]).unwrap();
}

#[test]
fn the_guides_diagnostics_are_what_validation_prints() {
    let s = Scratch::outside();
    let project = s.dir("my_game");
    copy_to(&starter("data-only"), &project);
    let (content, manifest) = (project.join("content.toml"), project.join("mod.toml"));
    let (good_content, good_manifest) = (
        fs::read_to_string(&content).unwrap(),
        fs::read_to_string(&manifest).unwrap(),
    );
    let cases: [(&str, &str, &str, &str); 7] = [
        (
            "emission = 0",
            "emision = 0",
            "unknown field `emision`",
            "content",
        ),
        (
            "surface = \"starter_data:turf\"",
            "surface = \"starter_data:grass\"",
            "ground.surface names no registered Material",
            "content",
        ),
        (
            "dimming = 15",
            "dimming = 99",
            "emission and dimming are at most 15",
            "content",
        ),
        (
            "art/stone.png",
            "../outside.png",
            "must be a relative path inside the mod's directory",
            "content",
        ),
        (
            "art/stone.png",
            "art/missing.png",
            "art/missing.png`: No such file or directory",
            "content",
        ),
        (
            "declaration = \"content.toml\"",
            "declaration = \"content.toml\"\nwasm = \"x.wasm\"",
            "select exactly one source: wasm or declaration",
            "manifest",
        ),
        (
            "api = \"0.3\"",
            "api = \"0.2\"",
            "declaration requires api 0.3",
            "manifest",
        ),
    ];
    for (from, to, expected, file) in cases {
        let (file, text) = match file {
            "content" => (&content, &good_content),
            _ => (&manifest, &good_manifest),
        };
        fs::write(file, text.replacen(from, to, 1)).unwrap();
        let message = tesserae(&s, &["validate", path(&project)]).unwrap_err();
        assert!(
            message.contains("starter_data") && message.contains(expected),
            "{expected}: {message}"
        );
        fs::write(file, text).unwrap();
    }

    fs::write(project.join("art/turf.png"), png(16, 16, 0)).unwrap();
    let message = tesserae(&s, &["validate", path(&project)]).unwrap_err();
    assert!(
        message.contains("appearance[1].texture")
            && message.contains("is 16 by 16 pixels, expected 48 by 16"),
        "{message}"
    );
}

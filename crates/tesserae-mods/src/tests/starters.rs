//! The starters work from a directory outside the checkout, so a stray path
//! into the Engine fails here instead of on an author's machine.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures_lite::future::block_on;
use tesserae_core::Tables;

use crate::{Found, ModHost, Registry, Trust, apply, plan, resolve, scan};

fn checkout() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Under the OS temp dir, which no relative path from the checkout reaches.
struct Outside(PathBuf);

impl Outside {
    fn new() -> Outside {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "tesserae-starters-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        Outside(dir)
    }
}

impl Drop for Outside {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let (src, dst) = (entry.path(), to.join(entry.file_name()));
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&src, &dst);
        } else {
            fs::copy(&src, &dst).unwrap();
        }
    }
}

fn copy_starter(name: &str, outside: &Outside) -> PathBuf {
    let dir = outside.0.join("projects").join(name);
    copy_tree(&checkout().join("starters").join(name), &dir);
    dir
}

/// Installs the files a starter's README lists into `games/<id>/`.
fn install(outside: &Outside, project: &Path, id: &str, files: &[&str]) {
    let dir = outside.0.join("games").join(id);
    for file in files {
        let to = dir.join(file);
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::copy(project.join(file), to).unwrap();
    }
}

fn load_installed(outside: &Outside, id: &str) -> (ModHost, Vec<crate::Compiled>, Found) {
    let mods = outside.0.join("mods");
    let scanned = scan(&mods).unwrap();
    assert!(scanned.rejected.is_empty(), "{:?}", scanned.rejected);
    let found = scanned
        .find(id)
        .expect("installed Game is discovered")
        .clone();
    let mut trust = Trust::load(outside.0.join("trust.json"));
    trust.grant(&found).unwrap();
    let host = ModHost::new().unwrap();
    let compiled = host
        .load(plan(vec![found.clone()]).unwrap(), &trust)
        .unwrap();
    (host, compiled, found)
}

fn registers_ground(declaration: crate::Declaration, id: &str) {
    let mut tables = Tables::builtin();
    let mut registry = Registry::builtin();
    let applied = apply(declaration, id, &mut tables, &mut registry, true).unwrap();
    let (ground, _) = resolve(vec![applied.behaviour], &mut tables, &registry).unwrap();
    assert!(ground.is_some(), "{id} declares its ground");
}

#[test]
fn the_data_starter_installs_and_loads_without_a_rust_build() {
    let outside = Outside::new();
    let project = copy_starter("data-only", &outside);
    assert!(!project.join("Cargo.toml").exists());
    install(
        &outside,
        &project,
        "starter_data",
        &["mod.toml", "content.toml", "art/stone.png", "art/turf.png"],
    );
    let (_, compiled, found) = load_installed(&outside, "starter_data");
    assert_eq!(found.manifest.api, "0.3");
    let declaration = compiled[0].declaration().unwrap().clone();
    assert!(declaration.appearance.iter().all(|a| a.texture.is_some()));
    registers_ground(declaration, "starter_data");
}

#[test]
fn the_rust_starter_builds_from_its_bundle_and_loads_installed() {
    let outside = Outside::new();
    let project = copy_starter("rust", &outside);

    let bundled = project.join("wit");
    let published = checkout().join("crates/tesserae-mods/wit/0.3");
    for entry in fs::read_dir(&published).unwrap() {
        let name = entry.unwrap().file_name();
        assert_eq!(
            fs::read(bundled.join(&name)).unwrap(),
            fs::read(published.join(&name)).unwrap(),
            "{name:?} drifted from the published 0.3 contract"
        );
    }

    let manifest = fs::read_to_string(project.join("Cargo.toml")).unwrap();
    let source = fs::read_to_string(project.join("src/lib.rs")).unwrap();
    assert!(
        !manifest.contains("path =") && !manifest.contains("tesserae"),
        "{manifest}"
    );
    assert!(
        !source.contains("..") && !source.contains("tesserae_core"),
        "{source}"
    );

    // The only path from here back to the checkout would be a bug in the starter.
    let build = Command::new(env!("CARGO"))
        .args([
            "build",
            "--target",
            "wasm32-wasip2",
            "--release",
            "--locked",
        ])
        .arg("--manifest-path")
        .arg(project.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", outside.0.join("target"))
        .env("RUSTFLAGS", "")
        .current_dir(&project)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    fs::copy(
        outside
            .0
            .join("target/wasm32-wasip2/release/starter_rust.wasm"),
        project.join("starter_rust.wasm"),
    )
    .unwrap();

    install(
        &outside,
        &project,
        "starter_rust",
        &["mod.toml", "starter_rust.wasm"],
    );
    let (host, compiled, found) = load_installed(&outside, "starter_rust");
    assert_eq!(found.manifest.api, "0.3");
    let (_, declaration) = block_on(async {
        host.instantiate(&compiled[0], 1)
            .await
            .unwrap()
            .declare()
            .await
            .unwrap()
    });
    registers_ground(declaration, "starter_rust");
}

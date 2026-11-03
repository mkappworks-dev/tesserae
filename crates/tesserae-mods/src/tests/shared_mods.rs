//! The repository's shared data Mods load together in dependency order, and every name one
//! Mod uses from another resolves.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use tesserae_core::{Burn, Tables};

use crate::{Behaviour, Kind, ModHost, Registry, Trust, apply, plan, resolve, scan};

const SHARED: [&str; 15] = [
    "strata",
    "waters",
    "flora",
    "hearth",
    "glassworks",
    "timber",
    "brickwork",
    "lamps",
    "wool",
    "frost",
    "crystals",
    "bog",
    "wanderer",
    "golem",
    "sprite",
];

fn checkout() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A copy of `mods/` alone, so the `games/` folder beside the source tree is not scanned.
struct Installed(PathBuf);

impl Installed {
    fn new() -> Installed {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/mods-test")
            .join(format!(
                "shared-mods-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        copy_tree(&checkout().join("mods"), &dir.join("mods"));
        Installed(dir)
    }

    fn mods(&self) -> PathBuf {
        self.0.join("mods")
    }
}

impl Drop for Installed {
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

/// Every shared Mod declared in dependency order, then resolved as one set, and the Mods that
/// supplied a character.
fn load_all() -> (Vec<String>, Tables, Registry, Vec<String>) {
    let installed = Installed::new();
    let scanned = scan(&installed.mods()).unwrap();
    assert!(scanned.rejected.is_empty(), "{:?}", scanned.rejected);
    let order = plan(scanned.mods.clone()).unwrap();
    let mut trust = Trust::load(installed.0.join("trust.json"));
    for found in &order {
        trust.grant(found).unwrap();
    }
    let compiled = ModHost::new().unwrap().load(order, &trust).unwrap();
    let mut tables = Tables::builtin();
    let mut registry = Registry::builtin();
    let mut behaviours: Vec<Behaviour> = Vec::new();
    let mut ids = Vec::new();
    let mut characters = Vec::new();
    for component in &compiled {
        let id = component.found.manifest.id.clone();
        let declaration = component.declaration().unwrap().clone();
        let applied = apply(declaration, &id, &mut tables, &mut registry, false).unwrap();
        if let Some(character) = &applied.character {
            assert_eq!(character.bones.len(), 6, "{id}");
            characters.push(id.clone());
        }
        behaviours.push(applied.behaviour);
        ids.push(id);
    }
    resolve(behaviours, &mut tables, &registry).unwrap();
    (ids, tables, registry, characters)
}

fn material(registry: &Registry, name: &str) -> u16 {
    registry
        .id_of(Kind::Material, name)
        .unwrap_or_else(|| panic!("{name} is registered")) as u16
}

#[test]
fn the_shared_mods_are_all_installed_and_data_only() {
    let installed = Installed::new();
    let scanned = scan(&installed.mods()).unwrap();
    let mut ids: Vec<_> = scanned
        .mods
        .iter()
        .map(|f| f.manifest.id.as_str())
        .collect();
    ids.sort_unstable();
    let mut expected = SHARED;
    expected.sort_unstable();
    assert_eq!(ids, expected);
    for found in &scanned.mods {
        assert!(
            !found.manifest.root,
            "{} is a Mod, not a Game",
            found.manifest.id
        );
        assert!(
            found.manifest.declaration.is_some(),
            "{} has no code",
            found.manifest.id
        );
        assert_eq!(found.manifest.api, "0.3");
    }
}

#[test]
fn dependencies_load_before_the_mods_that_use_them() {
    let (ids, ..) = load_all();
    let position = |id: &str| ids.iter().position(|loaded| loaded == id).unwrap();
    for needed in ["hearth", "brickwork"] {
        assert!(
            position(needed) < position("bog"),
            "{needed} before bog: {ids:?}"
        );
    }
    for needed in ["strata", "waters", "flora"] {
        assert!(
            position(needed) < position("hearth"),
            "{needed} before hearth: {ids:?}"
        );
    }
}

#[test]
fn every_cross_mod_reference_resolves() {
    let (_, tables, registry, _) = load_all();

    let water = material(&registry, "waters:water");
    let lava = material(&registry, "hearth:lava");
    assert!(tables.fluids.contains(water) && tables.fluids.contains(lava));

    let products: Vec<_> = tables
        .reactions
        .rules()
        .iter()
        .map(|rule| rule.product)
        .collect();
    assert_eq!(products.len(), 4);
    for product in [
        "waters:obsidian",
        "strata:basalt",
        "brickwork:brick",
        "hearth:ash",
    ] {
        assert!(
            products.contains(&Some(material(&registry, product))),
            "{product}"
        );
    }
    for fluid in ["bog:mud", "bog:tar", "frost:slush"] {
        assert!(
            tables.fluids.contains(material(&registry, fluid)),
            "{fluid}"
        );
    }

    let ash = material(&registry, "hearth:ash");
    for fuel in ["flora:wood", "bog:peat"] {
        let burn = tables.combustion.get(material(&registry, fuel)).unwrap();
        assert!(
            matches!(burn.burn, Burn::For { residue, .. } if residue == ash),
            "{fuel}"
        );
    }
    assert!(
        tables
            .combustion
            .contains(material(&registry, "flora:leaves"))
    );
    assert!(!tables.combustion.contains(ash), "ash does not burn");

    for name in [
        "glassworks:slab",
        "glassworks:stair",
        "glassworks:fence",
        "brickwork:wall",
        "timber:beam",
        "wool:carpet",
    ] {
        assert!(registry.id_of(Kind::Shape, name).is_some(), "{name}");
    }
}

#[test]
fn the_character_mods_each_supply_a_six_bone_figure() {
    let (_, _, _, mut characters) = load_all();
    characters.sort_unstable();
    assert_eq!(characters, ["golem", "sprite", "wanderer"]);
}

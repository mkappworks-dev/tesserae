//! Writes the texture faces of every demo Game's own Materials as PNG under `mockup/blocks/<game>/`,
//! and an index of them and Terra's Shapes, for the mockup's `/blocks` gallery. Run
//! `scripts/build-guests` first.

use std::fs::{self, File};
use std::io::BufWriter;
use std::path::Path;

use futures_lite::future::block_on;
use serde_json::{Value, json};
use tesserae_core::mesh::{POS_X, POS_Z};
use tesserae_core::{Connections, MaterialId, Tables, Voxel};
use tesserae_mods::{Found, Kind, MATERIAL_TEXTURE_EDGE, ModHost, Registry, Trust, apply, resolve};

/// The order a texture lays its faces out in, left to right.
const FACES: [&str; 3] = ["top", "side", "bottom"];
const GAMES: [&str; 5] = ["terra", "tidewater", "ember", "lumen", "meadow"];

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = root.join("mockup/blocks");
    let _ = fs::remove_dir_all(&out);
    let mut materials = Vec::new();
    let mut shapes = Vec::new();
    for game in GAMES {
        let (game_materials, game_shapes) = export(&root, game, &out);
        materials.extend(game_materials);
        shapes.extend(game_shapes);
    }
    let index = json!({ "games": GAMES, "edge": MATERIAL_TEXTURE_EDGE, "faces": FACES, "materials": materials, "shapes": shapes });
    fs::write(
        out.join("index.json"),
        serde_json::to_string_pretty(&index).unwrap() + "\n",
    )
    .unwrap();
    println!("wrote {} Materials to mockup/blocks", materials.len());
}

/// The Materials `game` declares itself, their faces written to `out/<game>/`, and its Shapes.
fn export(root: &Path, game: &str, out: &Path) -> (Vec<Value>, Vec<Value>) {
    let found = Found::read(&root.join("games").join(game)).expect("games/<game>/mod.toml");
    let scratch = root.join("target/export-blocks");
    fs::create_dir_all(&scratch).unwrap();
    let mut trust = Trust::load(scratch.join("trust.json"));
    trust.grant_for_run(&found).unwrap();
    let host = ModHost::new().unwrap();
    let compiled = host
        .load(vec![found], &trust)
        .unwrap_or_else(|e| panic!("build {game} with scripts/build-guests: {e}"));
    let instance = block_on(host.instantiate(&compiled[0], 0)).unwrap();
    let (_, mut declaration) = block_on(instance.declare()).unwrap();
    // Only the Game is loaded, and its World and Settlements name the shared Mods' Materials.
    declaration.world = None;
    declaration.biomes.clear();
    declaration.ores.clear();
    declaration.settlements.clear();
    let mut tables = Tables::builtin();
    let mut registry = Registry::builtin();
    let applied = apply(declaration, game, &mut tables, &mut registry, true).unwrap();
    resolve(vec![applied.behaviour], &mut tables, &registry).unwrap();

    fs::create_dir_all(out.join(game)).unwrap();
    let prefix = format!("{game}:");
    let mut materials = Vec::new();
    for (id, name) in registry.names(Kind::Material) {
        let Some(short) = name.strip_prefix(&prefix) else {
            continue;
        };
        let id = id as MaterialId;
        let appearance = applied
            .appearance
            .iter()
            .find(|a| a.material == short)
            .unwrap_or_else(|| panic!("{name} has no appearance"));
        let c = appearance.color;
        let mut faces = serde_json::Map::new();
        if let Some(pixels) = &appearance.texture {
            for (face, label) in FACES.iter().enumerate() {
                let file = format!("{game}/{short}.{label}.png");
                write_face(&out.join(&file), pixels, face);
                faces.insert((*label).into(), file.into());
            }
        }
        let cube = Voxel::cube(id);
        let def = tables.materials.def(id);
        let boxes = |voxel| {
            tables
                .shapes
                .selection(voxel, Connections::NONE)
                .iter()
                .map(|b| json!([b.min, b.max]))
                .collect::<Vec<_>>()
        };
        // As placed by a Player looking along -Z who aimed at the +X half.
        let placed = tables.uses.placed(cube, POS_Z, POS_X);
        let usable = if let Some(door) = tables.uses.door(id) {
            let shut = tables.uses.placed(Voxel::cube(door.closed), POS_Z, POS_X);
            json!({ "door": {
                "closed": registry_name(&registry, door.closed),
                "open": registry_name(&registry, door.open),
                "width-m": door.width_m,
                "height-m": door.height_m,
                "boxes": { "closed": boxes(shut), "open": boxes(door.toggle(shut)) },
            } })
        } else if let Some(slots) = tables.uses.container(id) {
            json!({ "container": slots })
        } else if let Some(tag) = tables.uses.station(id) {
            json!({ "station": tag })
        } else {
            Value::Null
        };
        materials.push(json!({
            "name": name,
            "game": game,
            "color": [c.r, c.g, c.b, c.a],
            "render": format!("{:?}", tables.materials.render(cube)).to_lowercase(),
            "solid": def.solid,
            "emission": def.emission,
            "use": usable,
            "boxes": boxes(placed),
            "faces": faces,
        }));
    }

    // Every Shape is drawn in Terra's stone, so only Terra's Shapes are listed.
    let Some(stone) = registry.id_of(Kind::Material, "terra:stone") else {
        return (materials, Vec::new());
    };
    let shapes = registry
        .names(Kind::Shape)
        .map(|(id, name)| {
            let voxel = Voxel::new(stone as MaterialId, id as u8, 0);
            let boxes = |connections| {
                tables
                    .shapes
                    .selection(voxel, connections)
                    .iter()
                    .map(|b| json!([b.min, b.max]))
                    .collect::<Vec<_>>()
            };
            let mut shape = json!({ "name": name, "boxes": boxes(Connections::NONE) });
            if tables.shapes.is_connected_shape(id as u8) {
                // Joined on every side, as a run of them stands.
                shape["joined"] = boxes(tables.shapes.connections(voxel, [voxel; 6])).into();
            }
            shape
        })
        .collect();
    (materials, shapes)
}

fn registry_name(registry: &Registry, id: MaterialId) -> &str {
    registry
        .names(Kind::Material)
        .find(|&(at, _)| at == usize::from(id))
        .map(|(_, name)| name)
        .unwrap()
}

/// One face of a texture whose rows run top to bottom, each the three faces side by side.
fn write_face(path: &Path, pixels: &[u8], face: usize) {
    let edge = MATERIAL_TEXTURE_EDGE;
    let row = edge * FACES.len() * 4;
    let mut data = Vec::with_capacity(edge * edge * 4);
    for y in 0..edge {
        let start = y * row + face * edge * 4;
        data.extend_from_slice(&pixels[start..start + edge * 4]);
    }
    let mut encoder = png::Encoder::new(
        BufWriter::new(File::create(path).unwrap()),
        edge as u32,
        edge as u32,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&data)
        .unwrap();
}

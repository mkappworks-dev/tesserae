//! Every Game's Templates and Settlements: declared as the approved mockup exports them, Obtainable,
//! and laid out by worldgen whole, level and furnished.

use std::collections::{HashMap, HashSet};
use std::fs;

use serde_json::Value;
use tesserae_core::template::{Cell, Template, Turn};
use tesserae_core::usable::VoxelUse;
use tesserae_core::{CHUNK_SIZE, Chunk, MaterialId, Voxel, Worldgen};
use tesserae_mods::{Kind, ToolKind};

use super::{checkout, loaded, material};
use crate::server::mods::LoadedMods;

pub(super) const GAMES: [&str; 5] = ["terra", "tidewater", "ember", "lumen", "meadow"];

/// The mockup's export of `game`'s Templates, `mockup/templates/<game>.json`.
pub(super) fn export(game: &str) -> Vec<Value> {
    let path = checkout().join(format!("mockup/templates/{game}.json"));
    let json: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    json["templates"].as_array().unwrap().clone()
}

fn name(loaded: &LoadedMods, kind: Kind, id: usize) -> String {
    loaded
        .registry
        .names(kind)
        .find(|&(at, _)| at == id)
        .map(|(_, name)| name.to_owned())
        .unwrap()
}

/// A cell as the export writes it: `keep`, `air`, or a Material, Shape and orientation by name.
fn described(loaded: &LoadedMods, cell: Cell) -> String {
    match cell {
        Cell::Keep => "keep".into(),
        Cell::Air => "air".into(),
        Cell::Ground => "ground".into(),
        Cell::Voxel(v) => format!(
            "{} {} {}",
            name(loaded, Kind::Material, usize::from(v.material())),
            name(loaded, Kind::Shape, usize::from(v.shape())),
            v.orientation()
        ),
    }
}

fn exported_cell(palette: &HashMap<char, String>, layers: &Value, [x, y, z]: [usize; 3]) -> String {
    let row = layers[y][z].as_str().unwrap();
    palette[&row.chars().nth(x).unwrap()].clone()
}

#[test]
fn every_games_templates_are_the_cells_the_approved_mockup_exports() {
    for game in GAMES {
        let (_install, loaded) = loaded(game, 1.0);
        let exported = export(game);
        let names: Vec<&str> = loaded.tables.templates.iter().map(|t| &*t.name).collect();
        let expected: Vec<&str> = exported
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, expected, "{game}");
        let kinds: Vec<&str> = exported
            .iter()
            .map(|t| t["kind"].as_str().unwrap())
            .collect();
        assert_eq!(kinds.len(), 15, "{game}");
        assert_eq!(kinds.iter().filter(|&&k| k == "wild").count(), 5, "{game}");

        for (template, t) in loaded.tables.templates.iter().zip(&exported) {
            let label = &template.name;
            let size: Vec<usize> = t["size"]
                .as_array()
                .unwrap()
                .iter()
                .map(|n| n.as_u64().unwrap() as usize)
                .collect();
            assert_eq!(template.size().to_vec(), size, "{label}");
            assert_eq!(
                template.ground_layer() as u64,
                t["ground_layer"].as_u64().unwrap(),
                "{label}"
            );
            let foundation = template
                .foundation()
                .map(|m| name(&loaded, Kind::Material, m.into()));
            assert_eq!(foundation.as_deref(), t["foundation"].as_str(), "{label}");
            let tolerance = t["level_tolerance_m"].as_f64().unwrap_or(1.0) as f32;
            assert_eq!(template.level_tolerance_m(), tolerance, "{label}");
            assert_eq!(template.spans(), t["spans"].as_bool().unwrap(), "{label}");

            let palette: HashMap<char, String> = t["palette"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| {
                    let symbol = p["symbol"].as_str().unwrap().chars().next().unwrap();
                    let cell = match p["cell"].as_str() {
                        Some(cell) => cell.to_owned(),
                        None => format!(
                            "{} {} {}",
                            p["material"].as_str().unwrap(),
                            p["shape"].as_str().unwrap(),
                            p["orientation"].as_u64().unwrap()
                        ),
                    };
                    (symbol, cell)
                })
                .collect();
            let [w, h, d] = template.size();
            for y in 0..h {
                for z in 0..d {
                    for x in 0..w {
                        assert_eq!(
                            described(&loaded, template.cell([x, y, z])),
                            exported_cell(&palette, &t["layers"], [x, y, z]),
                            "{label} at {:?}",
                            [x, y, z]
                        );
                    }
                }
            }
        }
    }
}

/// A closed Door: the Material a Door row of the Game's names as its own.
pub(super) fn is_closed_door(loaded: &LoadedMods, material: MaterialId) -> bool {
    matches!(
        loaded.tables.uses.of(material),
        Some((row, VoxelUse::Door { .. })) if row == material
    )
}

#[test]
fn every_game_lays_out_a_settlement_of_its_buildings_each_door_on_a_closed_door_and_terra_a_town() {
    for game in GAMES {
        let (_install, loaded) = loaded(game, 1.0);
        let settlements = &loaded.tables.settlements;
        let names: Vec<&str> = settlements.iter().map(|s| &*s.name).collect();
        let expected = match game {
            "terra" => vec![
                "terra:village".into(),
                "terra:town".into(),
                "terra:hamlet".into(),
            ],
            "tidewater" => vec!["tidewater:hamlet".into(), "tidewater:harbour".into()],
            _ => vec![format!("{game}:hamlet"), format!("{game}:village")],
        };
        assert_eq!(names, expected, "{game}");
        let mut used = HashSet::new();
        let exported = export(game);
        let door = material(&loaded, &format!("{game}:door"));
        assert!(is_closed_door(&loaded, door), "{game}:door is a Door");
        for settlement in settlements {
            let buildings: Vec<_> = settlement
                .centre
                .iter()
                .chain(settlement.rules.iter().map(|r| &r.building))
                .collect();
            used.extend(buildings.iter().map(|b| b.template));
            for building in buildings {
                let template = &loaded.tables.templates[usize::from(building.template)];
                let kind = exported[usize::from(building.template)]["kind"]
                    .as_str()
                    .unwrap();
                assert_ne!(kind, "wild", "{}", template.name);
                let [w, _, d] = template.size_m();
                assert!(w.max(d) <= settlement.radius_min_m, "{}", template.name);
                // The smithy, warehouse and barn are open-fronted, and the well and stall have no
                // front: with no door, their +Z side faces the road, and that is where the mockup
                // opens them.
                let has_door = exported[usize::from(building.template)]["palette"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|p| p["material"] == format!("{game}:door"));
                assert_eq!(
                    building.door.is_some(),
                    has_door,
                    "{}: its door faces the road",
                    template.name
                );
                if let Some(at) = building.door {
                    assert_eq!(
                        building.door,
                        template.door_cell(|m| m == door),
                        "{}",
                        template.name
                    );
                    assert!(
                        (template.ground_layer()..template.size()[1]).any(|y| matches!(
                            template.cell([usize::from(at[0]), y, usize::from(at[1])]),
                            Cell::Voxel(v) if is_closed_door(&loaded, v.material())
                        )),
                        "{}",
                        template.name
                    );
                }
            }
        }
        assert_eq!(used.len(), 10, "{game}: every building kind");
    }
}

#[test]
fn every_game_holds_the_place_tool() {
    for game in GAMES {
        let (_install, loaded) = loaded(game, 1.0);
        assert!(
            loaded.tools.iter().any(|t| t.kind == ToolKind::Place),
            "{game}"
        );
    }
}

#[test]
fn a_world_with_no_game_has_no_template() {
    let loaded = LoadedMods::default();
    assert!(loaded.tables.templates.is_empty());
    assert!(loaded.tables.settlements.is_empty());
}

/// The Chunks of a square `half_m` either side of the origin, every row a Structure reaches.
pub(in crate::server) struct Generated {
    pub(in crate::server) chunks: HashMap<[i32; 3], Chunk>,
}

impl Generated {
    fn new(generator: &Worldgen, half_m: f32) -> Self {
        Self::around(generator, [0.0, 0.0], half_m)
    }

    /// The Chunks within `half_m` of `centre_m`.
    pub(in crate::server) fn around(generator: &Worldgen, centre_m: [f32; 2], half_m: f32) -> Self {
        let chunk_m = CHUNK_SIZE as f32 * generator.voxel_size();
        let (low, high) = generator.surface_bounds_m();
        let [low_x, low_z] = centre_m.map(|c| ((c - half_m) / chunk_m).floor() as i32);
        let [high_x, high_z] = centre_m.map(|c| ((c + half_m) / chunk_m).ceil() as i32);
        let mut chunks = HashMap::new();
        for cy in (low / chunk_m).floor() as i32 - 1..=(high / chunk_m).ceil() as i32 + 1 {
            for cz in low_z..high_z {
                for cx in low_x..high_x {
                    chunks.insert([cx, cy, cz], generator.chunk(cx, cy, cz));
                }
            }
        }
        Generated { chunks }
    }

    /// The highest non-air Voxel of the column `[x, z]`.
    pub(in crate::server) fn top(&self, [x, z]: [i32; 2]) -> Option<Voxel> {
        let size = CHUNK_SIZE as i32;
        let [cx, cz] = [x, z].map(|g| g.div_euclid(size));
        let [lx, lz] = [x, z].map(|g| g.rem_euclid(size) as usize);
        let mut rows: Vec<i32> = self
            .chunks
            .keys()
            .filter(|c| c[0] == cx && c[2] == cz)
            .map(|c| c[1])
            .collect();
        rows.sort_unstable_by(|a, b| b.cmp(a));
        rows.iter().find_map(|&cy| {
            let chunk = &self.chunks[&[cx, cy, cz]];
            (0..CHUNK_SIZE)
                .rev()
                .map(|y| chunk.get(lx, y, lz))
                .find(|v| !v.is_air())
        })
    }

    pub(in crate::server) fn get(&self, at: [i32; 3]) -> Option<Voxel> {
        let size = CHUNK_SIZE as i32;
        let chunk = self.chunks.get(&at.map(|g| g.div_euclid(size)))?;
        let [x, y, z] = at.map(|g| g.rem_euclid(size) as usize);
        Some(chunk.get(x, y, z))
    }

    /// Where each Material of `of` stands.
    fn of(&self, of: &HashSet<MaterialId>) -> HashMap<MaterialId, Vec<[i32; 3]>> {
        let size = CHUNK_SIZE as i32;
        let mut out: HashMap<MaterialId, Vec<[i32; 3]>> = HashMap::new();
        for (c, chunk) in &self.chunks {
            for z in 0..CHUNK_SIZE {
                for y in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        let material = chunk.get(x, y, z).material();
                        if of.contains(&material) {
                            let at = [x, y, z].map(|l| l as i32);
                            out.entry(material)
                                .or_default()
                                .push([0, 1, 2].map(|a| c[a] * size + at[a]));
                        }
                    }
                }
            }
        }
        out
    }
}

/// A Template found standing whole: which, its lowest corner and its turn.
#[derive(Clone, Copy, Debug)]
pub(in crate::server) struct Found {
    pub template: usize,
    pub corner: [i32; 3],
    pub turn: Turn,
}

/// Every Template of `templates` standing whole in `world`: each of its Voxel cells there, and
/// each Air cell air. Found from its first Voxel cell, by every Voxel of that cell's Material.
pub(in crate::server) fn standing(
    world: &Generated,
    templates: &[Template],
    size: f32,
) -> Vec<Found> {
    standing_of(world, templates, size, true)
}

/// [`standing`], asking Air cells to be air only if `air`: a span leaves its gap as it was.
fn standing_of(world: &Generated, templates: &[Template], size: f32, air: bool) -> Vec<Found> {
    let first = |t: &Template, turn| {
        let sampled = t.sample(size, turn);
        let [sx, sy, sz] = sampled.size();
        (0..sy)
            .flat_map(|y| (0..sz).flat_map(move |z| (0..sx).map(move |x| [x, y, z])))
            .find_map(|at| match sampled.cell(at) {
                Cell::Voxel(v) => Some(v.material()),
                _ => None,
            })
    };
    let anchors: HashSet<MaterialId> = templates
        .iter()
        .flat_map(|t| Turn::all().filter_map(move |turn| first(t, turn)))
        .collect();
    let at = world.of(&anchors);
    let mut found: Vec<Found> = Vec::new();
    for (index, template) in templates.iter().enumerate() {
        let mut corners = HashSet::new();
        for turn in Turn::all() {
            let sampled = template.sample(size, turn);
            let [sx, sy, sz] = sampled.size();
            let cells: Vec<([i32; 3], Cell)> = (0..sy)
                .flat_map(|y| (0..sz).flat_map(move |z| (0..sx).map(move |x| [x, y, z])))
                .map(|at| (at, sampled.cell(at)))
                .filter(|(_, c)| matches!(c, Cell::Voxel(_)) || (air && *c == Cell::Air))
                .collect();
            let Some(&(first, Cell::Voxel(anchor))) =
                cells.iter().find(|(_, c)| matches!(c, Cell::Voxel(_)))
            else {
                continue;
            };
            for &at in at.get(&anchor.material()).into_iter().flatten() {
                let corner = [0, 1, 2].map(|a| at[a] - first[a]);
                if corners.contains(&corner) {
                    continue;
                }
                let whole = cells.iter().all(|&(p, cell)| {
                    let got = world.get([0, 1, 2].map(|a| corner[a] + p[a]));
                    match cell {
                        Cell::Voxel(v) => got == Some(v),
                        _ => got.is_some_and(|g| g.is_air()),
                    }
                });
                if whole {
                    corners.insert(corner);
                    found.push(Found {
                        template: index,
                        corner,
                        turn,
                    });
                }
            }
        }
    }
    found
}

/// The edge the Settlement turns to its road: the one nearest the door's cell.
pub(super) fn road_side(size: [i32; 3], [x, _, z]: [i32; 3]) -> [i32; 2] {
    let [w, _, d] = size;
    [
        (d - 1 - z, [0, 1]),
        (w - 1 - x, [1, 0]),
        (z, [0, -1]),
        (x, [-1, 0]),
    ]
    .into_iter()
    .min_by_key(|&(distance, _)| distance)
    .map(|(_, out)| out)
    .unwrap()
}

/// The demo World tests' seed; the square is wide enough that each Game lays out a Settlement in
/// it.
pub(in crate::server) const SEED: u64 = 7;
const HALF_M: f32 = 320.0;

#[test]
fn a_world_of_each_game_shows_its_templates_and_a_settlement_level_whole_and_furnished() {
    for game in GAMES {
        let (_install, loaded) = loaded(game, 1.0);
        let templates = &loaded.tables.templates;
        let generator = Worldgen::new(SEED, 1.0, loaded.world.unwrap())
            .with_templates(templates)
            .with_settlements(&loaded.tables.settlements);
        let world = Generated::new(&generator, HALF_M);
        let found = standing(&world, templates, 1.0);
        let exported = export(game);
        let kind = |f: &Found| exported[f.template]["kind"].as_str().unwrap();

        let buildings: Vec<&Found> = found.iter().filter(|f| kind(f) != "wild").collect();
        let minimum = loaded
            .tables
            .settlements
            .iter()
            .map(|s| s.minimum)
            .min()
            .unwrap();
        assert!(
            buildings.len() > usize::from(minimum),
            "{game}: {} buildings stand",
            buildings.len()
        );
        let wild = found.iter().filter(|f| kind(f) == "wild").count();
        assert!(wild > 0, "{game}: a wilderness Structure stands");

        let furnishings = ["door", "chest", "workbench", "lamp", "glass"]
            .map(|n| material(&loaded, &format!("{game}:{n}")));
        for furnishing in furnishings {
            assert!(
                buildings.iter().any(|f| {
                    let t = &templates[f.template];
                    let [w, h, d] = t.size();
                    (0..h).any(|y| {
                        (0..d).any(|z| {
                            (0..w).any(|x| {
                                matches!(t.cell([x, y, z]), Cell::Voxel(v) if v.material() == furnishing)
                            })
                        })
                    })
                }),
                "{game}: furnished with {}",
                name(&loaded, Kind::Material, furnishing.into())
            );
        }

        // Each door looks out onto the Settlement's path, where the scan holds what is before it.
        let paths: HashSet<MaterialId> = loaded
            .tables
            .settlements
            .iter()
            .map(|s| s.path.material())
            .collect();
        let door = material(&loaded, &format!("{game}:door"));
        let mut facing = 0;
        for f in &buildings {
            let sampled = templates[f.template].sample(1.0, f.turn);
            let [sx, sy, sz] = sampled.size();
            let Some(at) = (0..sy)
                .flat_map(|y| (0..sz).flat_map(move |z| (0..sx).map(move |x| [x, y, z])))
                .find(|&at| matches!(sampled.cell(at), Cell::Voxel(v) if v.material() == door))
            else {
                continue;
            };
            let [dx, dz] = road_side(sampled.size(), at);
            let from = [f.corner[0] + at[0], f.corner[2] + at[2]];
            let ahead: Vec<Option<Voxel>> = (1..=10)
                .flat_map(|k| (-1..=1).map(move |across| (k, across)))
                .map(|(k, across)| {
                    world.top([
                        from[0] + dx * k + dz * across,
                        from[1] + dz * k + dx * across,
                    ])
                })
                .collect();
            if ahead.iter().any(Option::is_none) {
                continue;
            }
            assert!(
                ahead
                    .iter()
                    .flatten()
                    .any(|v| paths.contains(&v.material())),
                "{game}: the door of {} at {:?} faces no path",
                templates[f.template].name,
                f.corner
            );
            facing += 1;
        }
        assert!(facing > 0, "{game}: no door's road was in the scan");

        let mut cellars = 0;
        for f in &found {
            let sampled = templates[f.template].sample(1.0, f.turn);
            let [sx, _, sz] = sampled.size();
            // Level: nothing under its lowest layer is air, so it stands on its ground.
            for z in 0..sz {
                for x in 0..sx {
                    if !matches!(sampled.cell([x, 0, z]), Cell::Voxel(_)) {
                        continue;
                    }
                    let below = [f.corner[0] + x, f.corner[1] - 1, f.corner[2] + z];
                    assert!(
                        world.get(below).is_none_or(|v| !v.is_air()),
                        "{game}: {} at {:?} stands on its ground",
                        templates[f.template].name,
                        f.corner
                    );
                }
            }
            // Underground: ground stands round every cellar cell on the footprint's rim.
            if sampled.ground() == 0 || kind(f) == "wild" {
                continue;
            }
            cellars += 1;
            let (mut rim, mut buried) = (0, 0);
            for y in 0..sampled.ground() {
                for z in 0..sz {
                    for x in [0, sx - 1] {
                        let out = if x == 0 { -1 } else { sx };
                        if !matches!(sampled.cell([x, y, z]), Cell::Voxel(_)) {
                            continue;
                        }
                        rim += 1;
                        let at = [f.corner[0] + out, f.corner[1] + y, f.corner[2] + z];
                        buried += usize::from(world.get(at).is_some_and(|v| !v.is_air()));
                    }
                }
            }
            assert!(
                buried * 10 >= rim * 9,
                "{game}: {}'s cellar is underground ({buried} of {rim})",
                templates[f.template].name
            );
        }
        assert!(cellars > 0, "{game}: a building with a cellar stands");
    }
}

/// How far from spawn the scan for Settlements of every size looks: a few of the villages'
/// lattice cells, and one of Terra's town's.
const SIZES_SCAN_M: f32 = 768.0;

#[test]
fn a_world_of_each_game_lays_small_and_large_settlements() {
    for game in GAMES {
        let (_install, loaded) = loaded(game, 1.0);
        let templates = &loaded.tables.templates;
        let generator = Worldgen::new(SEED, 1.0, loaded.world.unwrap())
            .with_templates(templates)
            .with_settlements(&loaded.tables.settlements);
        let world = Generated::new(&generator, SIZES_SCAN_M);
        let found = standing(&world, templates, 1.0);
        let exported = export(game);
        let kinds: Vec<&str> = found
            .iter()
            .map(|f| exported[f.template]["kind"].as_str().unwrap())
            .collect();
        let sizes = copy_sizes(&found, &kinds, SIZES_SCAN_M);
        // Tidewater's islands hold only a handful of buildings, whatever a harbour draws.
        let large = if game == "tidewater" { 3 } else { 8 };
        assert!(
            sizes.first().is_some_and(|&n| n < large.min(6))
                && sizes.last().is_some_and(|&n| n >= large),
            "{game}: copies of {sizes:?} buildings, not small and large"
        );
    }
}

/// How many buildings each Settlement whose centre stands in `half_m` of spawn, less a large
/// copy's reach, lays besides its centre, fewest first: each building goes to the nearest centre.
/// `kinds` holds each one's export kind.
fn copy_sizes(found: &[Found], kinds: &[&str], half_m: f32) -> Vec<usize> {
    let middle = |f: &Found| [f.corner[0], f.corner[2]].map(|g| g as f32);
    let all: Vec<[f32; 2]> = found
        .iter()
        .zip(kinds)
        .filter(|(_, k)| **k == "centre")
        .map(|(f, _)| middle(f))
        .collect();
    let centres: Vec<[f32; 2]> = all
        .iter()
        .copied()
        .filter(|c| c.iter().all(|v| v.abs() <= half_m - 64.0))
        .collect();
    let mut sizes = vec![0; centres.len()];
    for (f, _) in found
        .iter()
        .zip(kinds)
        .filter(|(_, k)| !matches!(**k, "centre" | "wild"))
    {
        let at = middle(f);
        let d = |c: &[f32; 2]| (c[0] - at[0]).hypot(c[1] - at[1]);
        let nearest = all.iter().min_by(|a, b| d(a).total_cmp(&d(b)));
        if let Some(i) = nearest.and_then(|n| centres.iter().position(|c| c == n)) {
            sizes[i] += 1;
        }
    }
    sizes.sort_unstable();
    sizes
}

/// Where Terra's town stands at [`SEED`], about 490 m from spawn: the town's lattice is 384 m, so
/// the square the other tests scan need not hold one.
const TERRA_TOWN_M: [f32; 2] = [456.0, -166.0];

#[test]
fn terra_lays_out_a_town_of_cobbled_roads() {
    let (_install, loaded) = loaded("terra", 1.0);
    let templates = &loaded.tables.templates;
    let generator = Worldgen::new(SEED, 1.0, loaded.world.unwrap())
        .with_templates(templates)
        .with_settlements(&loaded.tables.settlements);
    let world = Generated::around(&generator, TERRA_TOWN_M, 160.0);
    let found = standing(&world, templates, 1.0);
    let cobble = material(&loaded, "terra:cobblestone");
    let rects: Vec<[[i32; 2]; 2]> = found
        .iter()
        .map(|f| {
            let [sx, _, sz] = templates[f.template].sample(1.0, f.turn).size();
            [
                [f.corner[0], f.corner[2]],
                [f.corner[0] + sx, f.corner[2] + sz],
            ]
        })
        .collect();
    let wells = found
        .iter()
        .zip(&rects)
        .filter(|(f, _)| templates[f.template].name == "terra:well");
    // Only a town lays cobblestone roads, and no wild Structure stands within its margin.
    let road = |[min, max]: [[i32; 2]; 2]| {
        let centre = [0, 1].map(|a| (min[a] + max[a]) / 2);
        let mut cells = 0;
        for z in centre[1] - 60..centre[1] + 60 {
            for x in centre[0] - 60..centre[0] + 60 {
                let built = rects.iter().any(|[min, max]| {
                    (min[0]..max[0]).contains(&x) && (min[1]..max[1]).contains(&z)
                });
                if !built && world.top([x, z]).is_some_and(|v| v.material() == cobble) {
                    cells += 1;
                }
            }
        }
        cells
    };
    let (town, at) = wells
        .map(|(_, &rect)| (road(rect), rect))
        .max_by_key(|&(cells, _)| cells)
        .unwrap_or_default();
    assert!(town > 100, "{town} cobbled road cells round a well");
    // Its buildings, each nearer its well than the town's smallest radius.
    let centre = [0, 1].map(|a| (at[0][a] + at[1][a]) / 2);
    let buildings = rects
        .iter()
        .zip(&found)
        .filter(|(r, f)| {
            let middle = [0, 1].map(|a| (r[0][a] + r[1][a]) / 2 - centre[a]);
            templates[f.template].name != "terra:well"
                && middle[0].pow(2) + middle[1].pow(2) <= 96 * 96
        })
        .count();
    assert!(buildings >= 18, "the town lays {buildings} buildings");
}

/// Each Game's bridge: a Template that spans. Meadow's footbridge is left out: its soft hills
/// hold no gap, and it has no water.
const BRIDGES: [(&str, &str); 2] = [
    ("terra", "terra:stone_bridge"),
    ("tidewater", "tidewater:rope_bridge"),
];

/// The bridges of `game` standing in `half_m` of spawn at [`SEED`], each with whether the ground
/// under its middle is below the water level.
fn bridges(game: &str, bridge: &str, half_m: f32) -> Vec<(Found, bool)> {
    let (_install, loaded) = loaded(game, 1.0);
    let templates = &loaded.tables.templates;
    let index = templates.iter().position(|t| t.name == bridge).unwrap();
    assert!(templates[index].spans(), "{bridge}");
    let generator = Worldgen::new(SEED, 1.0, loaded.world.unwrap())
        .with_templates(templates)
        .with_settlements(&loaded.tables.settlements);
    let world = Generated::new(&generator, half_m);
    let template = std::slice::from_ref(&templates[index]);
    standing_of(&world, template, 1.0, false)
        .into_iter()
        .map(|f| {
            let [sx, _, sz] = template[0].sample(1.0, f.turn).size();
            let middle = [f.corner[0] + sx / 2, f.corner[2] + sz / 2].map(|g| g as f32);
            let height_m = generator.surface_height_m(middle[0], middle[1]);
            (f, generator.water().is_some_and(|w| height_m < w.level_m))
        })
        .collect()
}

/// How far from spawn the scan for a bridge looks: a few lattice cells of the bridges' rows,
/// since few cells hold a gap.
const BRIDGE_SCAN_M: f32 = 512.0;

#[test]
fn terra_and_tidewater_each_stand_a_bridge_over_a_gap_near_spawn() {
    for (game, bridge) in BRIDGES {
        let found = bridges(game, bridge, BRIDGE_SCAN_M);
        assert!(!found.is_empty(), "{game}: no bridge stands near spawn");
    }
}

#[test]
fn every_game_stocks_its_settlements_buildings_and_wilderness_biomes_with_loot() {
    for game in GAMES {
        let (_install, loaded) = loaded(game, 1.0);
        for settlement in &loaded.tables.settlements {
            assert!(settlement.loot.is_some(), "{}", settlement.name);
            assert!(
                settlement.rules.iter().any(|r| r.loot.is_some()),
                "{}: a building keeps its own",
                settlement.name
            );
        }
        let biomes = loaded.world.as_ref().unwrap().biomes;
        assert_eq!(loaded.biome_loot.len(), biomes.as_slice().len(), "{game}");
        for (biome, loot) in biomes.as_slice().iter().zip(&loaded.biome_loot) {
            if !biome.surface_features.structures().is_empty() {
                assert!(loot.is_some(), "{game}: {}", biome.name.as_str());
            }
        }
    }
}

#[test]
fn every_template_cell_of_a_material_that_fixes_its_shape_is_in_that_shape() {
    for game in GAMES {
        let (_install, loaded) = loaded(game, 1.0);
        let uses = &loaded.tables.uses;
        for template in &loaded.tables.templates {
            let [w, h, d] = template.size();
            for y in 0..h {
                for z in 0..d {
                    for x in 0..w {
                        let Cell::Voxel(v) = template.cell([x, y, z]) else {
                            continue;
                        };
                        if let Some(fixed) = uses.shape(v.material()) {
                            assert_eq!(
                                v.shape(),
                                fixed,
                                "{}: {} at {:?} is not in its row's Shape",
                                template.name,
                                name(&loaded, Kind::Material, usize::from(v.material())),
                                [x, y, z]
                            );
                        }
                    }
                }
            }
        }
    }
}

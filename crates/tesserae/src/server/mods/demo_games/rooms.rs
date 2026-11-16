//! Inside every Game's buildings: each storey and cellar a Player is meant to stand in is walked to
//! from the door with no jump, the door opens a way out to the edge the Settlement turns to its
//! road, a Player's body passes each open Door, the wells hold water, and every Template fits the
//! undo budget at the finest Voxel.

use std::collections::{HashMap, HashSet};

use tesserae_core::collide::{Aabb, VoxelVolume, sweep};
use tesserae_core::mesh::{FACE_STEPS, NEG_X as NEG_X_FACE, POS_X as POS_X_FACE, POS_Z};
use tesserae_core::template::{Cell, Sampled, Turn};
use tesserae_core::{Orientation, SHAPE_POST, SHAPE_SLAB, SHAPE_STAIR, Voxel};

use super::loaded;
use super::structures::{GAMES, export, is_closed_door, road_side};
use crate::server::mods::LoadedMods;
use crate::server::player::{BODY_HALF_WIDTH_M, BODY_HEIGHT_M};
use crate::server::undo::RECORDED_LIMIT;

/// The standing heights, in metres above the bottom of the Ground layer, a Player is meant to
/// reach in each building kind with a door, by its place in each Game's list: the ground floor at
/// 1, an upper storey at 5, a cellar at -3. The smithy and the stall are open-fronted and the well
/// has no inside, so they have none.
const SETTLEMENT_LEVELS: [&[f32]; 10] = [
    &[1.0],
    &[1.0, 5.0],
    &[1.0, -3.0],
    &[1.0, 5.0, -3.0],
    &[1.0],
    &[],
    &[1.0],
    &[],
    &[1.0],
    &[],
];

/// The wilderness Templates with a door, and their levels: a tower's roof deck, a raised house's
/// floor, a crypt under a mound.
const WILD_LEVELS: [(&str, &[f32]); 8] = [
    ("terra:watchtower", &[1.0, 12.0, -3.0]),
    ("tidewater:lighthouse", &[1.0, 18.0]),
    ("tidewater:stilt_house", &[4.0]),
    ("ember:barrow", &[1.0, -3.0]),
    ("lumen:observatory", &[1.0, 10.0, -3.0]),
    ("lumen:glass_dome", &[1.0]),
    ("meadow:burrow", &[1.0, -3.0]),
    ("meadow:treehouse", &[5.0]),
];

/// Clear height a house storey keeps over most of its floor: 2.5 m, in whole cells.
const STOREY_CLEAR_CELLS: i32 = 3;

/// Where a Player stands on a cell: its top in half-metres from its bottom, and the way it rises
/// if it is a stair.
#[derive(Clone, Copy)]
struct Footing {
    top: i32,
    rise: Option<[i32; 2]>,
}

/// A Template unturned at 1 m, read for a Player's walk: a cell outside it, or one it keeps, is
/// ground up to the Ground layer and open air above.
struct Rooms<'a> {
    loaded: &'a LoadedMods,
    sampled: Sampled<'a>,
    size: [i32; 3],
    ground: i32,
}

impl<'a> Rooms<'a> {
    fn new(loaded: &'a LoadedMods, template: usize) -> Self {
        let sampled = loaded.tables.templates[template].sample(1.0, Turn::NONE);
        let (size, ground) = (sampled.size(), sampled.ground());
        Rooms {
            loaded,
            sampled,
            size,
            ground,
        }
    }

    fn cell(&self, at: [i32; 3]) -> Cell {
        if (0..3).all(|a| (0..self.size[a]).contains(&at[a])) {
            self.sampled.cell(at)
        } else {
            Cell::Keep
        }
    }

    fn passable(&self, voxel: Voxel) -> bool {
        is_closed_door(self.loaded, voxel.material())
            || self.loaded.tables.fluids.contains(voxel.material())
    }

    fn open(&self, at: [i32; 3]) -> bool {
        match self.cell(at) {
            Cell::Keep => at[1] > self.ground,
            Cell::Air => true,
            Cell::Ground => false,
            Cell::Voxel(v) => self.passable(v),
        }
    }

    fn footing(&self, at: [i32; 3]) -> Option<Footing> {
        let whole = Footing { top: 2, rise: None };
        match self.cell(at) {
            Cell::Keep if at[1] <= self.ground => Some(whole),
            Cell::Ground => Some(whole),
            Cell::Voxel(v) if !self.passable(v) => Some(match v.shape() {
                SHAPE_SLAB => Footing { top: 1, rise: None },
                SHAPE_STAIR => {
                    let o = Orientation::from_bits(v.orientation()).unwrap_or_default();
                    let step = FACE_STEPS[o.face_of(POS_Z)];
                    Footing {
                        top: 2,
                        rise: Some([step[0], step[2]]),
                    }
                }
                _ => whole,
            }),
            _ => None,
        }
    }

    /// Standing on the cell `at`: its height in half-metres, if a body 2 m tall fits above it.
    fn stand(&self, at: [i32; 3]) -> Option<(i32, Footing)> {
        let footing = self.footing(at)?;
        let [x, y, z] = at;
        (self.open([x, y + 1, z]) && self.open([x, y + 2, z]))
            .then_some((2 * y + footing.top, footing))
    }

    /// Every closed Door cell.
    fn doors(&self) -> Vec<[i32; 3]> {
        let [w, h, d] = self.size;
        (0..h)
            .flat_map(|y| (0..d).flat_map(move |z| (0..w).map(move |x| [x, y, z])))
            .filter(|&at| matches!(self.cell(at), Cell::Voxel(v) if is_closed_door(self.loaded, v.material())))
            .collect()
    }

    /// The cells stood on that a walk from a door reaches: a step of half a metre anywhere, or a
    /// whole metre onto or off a stair along the way it rises.
    fn walk(&self) -> HashSet<[i32; 3]> {
        let mut reached = HashSet::new();
        let mut next: Vec<[i32; 3]> = Vec::new();
        for [x, y, z] in self.doors() {
            let below = [x, y - 1, z];
            if self.stand(below).is_some() && reached.insert(below) {
                next.push(below);
            }
        }
        let [w, _, d] = self.size;
        while let Some(at) = next.pop() {
            let (from, footing) = self.stand(at).expect("only cells stood on are queued");
            for [dx, dz] in [[1, 0], [-1, 0], [0, 1], [0, -1]] {
                for dy in -2..=2 {
                    let to = [at[0] + dx, at[1] + dy, at[2] + dz];
                    if !(-2..=w + 1).contains(&to[0]) || !(-2..=d + 1).contains(&to[2]) {
                        continue;
                    }
                    let Some((height, there)) = self.stand(to) else {
                        continue;
                    };
                    let climb = height - from;
                    let fits = climb.abs() <= 1
                        || (climb == 2 && there.rise == Some([dx, dz]))
                        || (climb == -2 && footing.rise == Some([-dx, -dz]));
                    if fits && reached.insert(to) {
                        next.push(to);
                    }
                }
            }
        }
        reached
    }

    /// Whether no step joins standing on `at` to standing beside it: a crenel between merlons is
    /// stood on, but not walked to.
    fn ledge(&self, [x, y, z]: [i32; 3]) -> bool {
        let Some((height, _)) = self.stand([x, y, z]) else {
            return true;
        };
        ![[1, 0], [-1, 0], [0, 1], [0, -1]].iter().any(|[dx, dz]| {
            (-1..=1).any(|dy| {
                self.stand([x + dx, y + dy, z + dz])
                    .is_some_and(|(there, _)| (there - height).abs() <= 1)
            })
        })
    }

    /// The level in metres above the bottom of the Ground layer of standing on `at`.
    fn level_m(&self, at: [i32; 3]) -> Option<f32> {
        self.stand(at)
            .map(|(height, _)| (height - 2 * self.ground) as f32 / 2.0)
    }
}

fn levels_of(exported: &[serde_json::Value], index: usize, name: &str) -> Option<&'static [f32]> {
    if exported[index]["kind"] != "wild" {
        return Some(SETTLEMENT_LEVELS[index]);
    }
    WILD_LEVELS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|&(_, levels)| levels)
}

#[test]
fn from_every_door_a_player_walks_to_each_storey_and_cellar_with_room_overhead() {
    for game in GAMES {
        let (_install, loaded) = loaded(game, 1.0);
        let exported = export(game);
        for (index, template) in loaded.tables.templates.iter().enumerate() {
            let name = &template.name;
            let rooms = Rooms::new(&loaded, index);
            let doors = rooms.doors();
            let Some(levels) = levels_of(&exported, index, name) else {
                assert!(
                    doors.is_empty(),
                    "{name} has a door, so list the levels it reaches"
                );
                continue;
            };
            assert_eq!(doors.is_empty(), levels.is_empty(), "{name}");
            let reached = rooms.walk();
            let [w, h, d] = rooms.size;
            for &level in levels {
                let (mut walked, mut clear) = (0, 0);
                for y in 0..h {
                    for z in 0..d {
                        for x in 0..w {
                            let at = [x, y, z];
                            if rooms.level_m(at) != Some(level) || rooms.ledge(at) {
                                continue;
                            }
                            assert!(
                                reached.contains(&at),
                                "{name}: {at:?} at {level} m is cut off"
                            );
                            walked += 1;
                            clear += i32::from(
                                (1..=STOREY_CLEAR_CELLS).all(|k| rooms.open([x, y + k, z])),
                            );
                        }
                    }
                }
                assert!(walked > 0, "{name}: nothing stood on at {level} m");
                let house = exported[index]["kind"] != "wild";
                assert!(
                    !house || clear * 4 >= walked * 3,
                    "{name}: {clear} of {walked} cells at {level} m have 3 m clear"
                );
            }
        }
    }
}

#[test]
fn every_door_opens_a_way_out_to_the_edge_the_settlement_turns_to_its_road() {
    for game in GAMES {
        let (_install, loaded) = loaded(game, 1.0);
        for (index, template) in loaded.tables.templates.iter().enumerate() {
            let rooms = Rooms::new(&loaded, index);
            let Some(&door) = rooms.doors().iter().min_by_key(|d| d[1]) else {
                continue;
            };
            let [dx, dz] = road_side(rooms.size, door);
            let mut at = door;
            while (0..rooms.size[0]).contains(&at[0]) && (0..rooms.size[2]).contains(&at[2]) {
                for up in 0..2 {
                    assert!(
                        rooms.open([at[0], at[1] + up, at[2]]),
                        "{}: the way out of the door at {door:?} is shut at {at:?}",
                        template.name
                    );
                }
                at = [at[0] + dx, at[1], at[2] + dz];
            }
        }
    }
}

#[test]
fn every_shipped_template_fits_the_undo_budget_at_a_quarter_metre() {
    let mut largest = (0, String::new());
    for game in GAMES {
        let (_install, loaded) = loaded(game, 0.25);
        for template in &loaded.tables.templates {
            let sampled = template.sample(0.25, Turn::NONE);
            let [w, h, d] = sampled.size();
            // Every cell but Keep may change; a foundation's fill depends on the ground, so not it.
            let changed = (0..h)
                .flat_map(|y| (0..d).flat_map(move |z| (0..w).map(move |x| [x, y, z])))
                .filter(|&at| sampled.cell(at) != Cell::Keep)
                .count();
            assert!(
                changed <= RECORDED_LIMIT,
                "{}: {changed} Voxels",
                template.name
            );
            largest = largest.max((changed, template.name.clone()));
        }
    }
    println!("largest: {} with {} Voxels", largest.1, largest.0);
}

#[test]
fn every_games_well_stands_its_posts_on_its_ring_and_holds_water_a_metre_down() {
    for game in GAMES {
        let (_install, loaded) = loaded(game, 1.0);
        let exported = export(game);
        let index = exported.iter().position(|t| t["kind"] == "centre").unwrap();
        let name = &loaded.tables.templates[index].name;
        let rooms = Rooms::new(&loaded, index);
        let [w, h, d] = rooms.size;
        let all: Vec<[i32; 3]> = (0..h)
            .flat_map(|y| (0..d).flat_map(move |z| (0..w).map(move |x| [x, y, z])))
            .collect();
        let voxel = |at| match rooms.cell(at) {
            Cell::Voxel(v) => Some(v),
            _ => None,
        };
        let water = |at| voxel(at).is_some_and(|v| loaded.tables.fluids.contains(v.material()));
        let solid = |at| voxel(at).is_some_and(|v| !water(at) && v.shape() != SHAPE_POST);

        let posts: Vec<_> = all
            .iter()
            .filter(|&&at| voxel(at).is_some_and(|v| v.shape() == SHAPE_POST))
            .collect();
        assert!(!posts.is_empty(), "{name} has posts");
        for &[x, y, z] in posts {
            let below = voxel([x, y - 1, z]);
            assert!(
                below.is_some_and(|v| v.shape() == SHAPE_POST || v.is_full_cube()),
                "{name}: the post at {:?} stands on nothing whole",
                [x, y, z]
            );
        }

        let wet: Vec<[i32; 3]> = all.iter().copied().filter(|&at| water(at)).collect();
        assert!(!wet.is_empty(), "{name} holds water");
        for &[x, y, z] in &wet {
            let held = |at| water(at) || solid(at);
            assert!(
                held([x, y - 1, z]),
                "{name}: water at {:?} has no floor",
                [x, y, z]
            );
            for [dx, dz] in [[1, 0], [-1, 0], [0, 1], [0, -1]] {
                assert!(
                    held([x + dx, y, z + dz]),
                    "{name}: water at {:?} runs out",
                    [x, y, z]
                );
            }
        }
        let top = *wet.iter().max_by_key(|at| at[1]).unwrap();
        assert_eq!(
            rooms.cell([top[0], top[1] + 1, top[2]]),
            Cell::Air,
            "{name}: open above"
        );
        let surface = top[1] + 1;
        let rim = [[1, 0], [-1, 0], [0, 1], [0, -1]]
            .iter()
            .map(|[dx, dz]| {
                let mut y = top[1];
                while solid([top[0] + dx, y + 1, top[2] + dz]) {
                    y += 1;
                }
                let lip =
                    voxel([top[0] + dx, y, top[2] + dz]).is_some_and(|v| v.shape() == SHAPE_SLAB);
                (2 * (y + 1) - i32::from(lip)) as f32 / 2.0
            })
            .fold(f32::MIN, f32::max);
        let depth = rim - surface as f32;
        assert!(
            (0.5..=1.5).contains(&depth),
            "{name}: the water stands {depth} m under the rim"
        );
    }
}

/// A Template's cells at one Voxel size, on ground under its Ground layer and open air above.
struct Placed {
    voxels: HashMap<[i32; 3], Voxel>,
    floor: i32,
    ground: Voxel,
}

impl VoxelVolume for Placed {
    fn voxel(&self, at: [i32; 3]) -> Option<Voxel> {
        Some(match self.voxels.get(&at) {
            Some(&v) => v,
            None if at[1] < self.floor => self.ground,
            None => Voxel::AIR,
        })
    }
}

#[test]
fn a_player_walks_through_each_leaf_of_an_open_door_at_every_voxel_size() {
    for size in [1.0, 0.5, 0.25] {
        let (_install, loaded) = loaded("terra", size);
        let tables = &loaded.tables;
        for name in ["terra:cottage", "terra:inn"] {
            let template = tables.templates.iter().find(|t| t.name == name).unwrap();
            let sampled = template.sample(size, Turn::NONE);
            let [w, h, d] = sampled.size();
            let per_m = (1.0 / size) as i32;
            let floor = sampled.ground() + per_m;
            let mut placed = Placed {
                voxels: HashMap::new(),
                floor,
                ground: Voxel::cube(template.foundation().unwrap()),
            };
            let mut doors = Vec::new();
            for y in 0..h {
                for z in 0..d {
                    for x in 0..w {
                        let voxel = match sampled.cell([x, y, z]) {
                            Cell::Voxel(v) => v,
                            Cell::Keep => continue,
                            Cell::Air => Voxel::AIR,
                            Cell::Ground => placed.ground,
                        };
                        if is_closed_door(&loaded, voxel.material()) {
                            doors.push([x, y, z]);
                        }
                        placed.voxels.insert([x, y, z], voxel);
                    }
                }
            }
            let front = doors.iter().map(|d| d[2]).max().unwrap();
            let leaf_cells = (2.0 / size) as i32;
            let low = doors.iter().map(|d| d[0]).min().unwrap();
            let high = doors.iter().map(|d| d[0]).max().unwrap();
            let leaves = (high - low + 1) / leaf_cells;
            assert_eq!(leaves, if name == "terra:inn" { 2 } else { 1 }, "{name}");

            let feet = floor as f32 * size + 0.01;
            let outside = (front + 1) as f32 * size + 0.1;
            let walk = |placed: &Placed, leaf: i32| {
                let centre = (low + leaf * leaf_cells) as f32 * size + 1.0;
                let body = Aabb::new(
                    [centre - BODY_HALF_WIDTH_M, feet, outside],
                    [
                        centre + BODY_HALF_WIDTH_M,
                        feet + BODY_HEIGHT_M,
                        outside + 2.0 * BODY_HALF_WIDTH_M,
                    ],
                );
                sweep(placed, tables, size, body, [0.0, 0.0, -2.0]).moved[2]
            };
            for leaf in 0..leaves {
                assert!(
                    walk(&placed, leaf) > -1.0,
                    "{name}: shut, leaf {leaf} at {size}"
                );
            }
            for at in &doors {
                let v = placed.voxels[at];
                let door = tables.uses.door(v.material()).unwrap();
                let open = door.toggle(v);
                // Each leaf opens onto its outer edge: a pair as a mirrored pair.
                let hinge = Orientation::from_bits(open.orientation())
                    .unwrap()
                    .face_of(POS_Z);
                if leaves == 2 {
                    let outer = if at[0] < low + leaf_cells {
                        NEG_X_FACE
                    } else {
                        POS_X_FACE
                    };
                    assert_eq!(hinge, outer, "{name}: the leaf at {at:?} at {size}");
                }
                placed.voxels.insert(*at, open);
            }
            for leaf in 0..leaves {
                assert_eq!(
                    walk(&placed, leaf),
                    -2.0,
                    "{name}: open, leaf {leaf} at {size}"
                );
            }
        }
    }
}

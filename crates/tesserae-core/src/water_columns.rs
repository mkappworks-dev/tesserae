//! The water columns a wet Chunk's mesh carries for its water's look: per column of the Chunk and a
//! one-column apron, the Flow, depth and foam and falling flags its fragments read. They are drawn
//! and move nothing (ADR-0038).

use crate::chunk::CHUNK_SIZE;
use crate::fluid::FluidTable;
use crate::light::{Light, MAX_LIGHT};
use crate::mesh::{LitChunk, NEG_X, NEG_Y, NEG_Z, POS_X, POS_Y, POS_Z, Quad};
use crate::occupants::{Fluid, Occupants};
use crate::shape::SUBDIV;
use crate::voxel::MaterialId;
use crate::worldgen::Fall;

/// Columns along a side of a Chunk's water columns: the Chunk's and its apron.
pub const WATER_SIDE: usize = CHUNK_SIZE + 2;

/// What the World answers of its generated water, by global column, as the water columns ask it.
/// [`crate::Worldgen`] answers it; a test, a double.
pub trait WaterAnswers {
    fn voxel_size(&self) -> f32;
    /// The World's water Material, `None` in a World without.
    fn water_material(&self) -> Option<MaterialId>;
    /// The generated Water surface over a column in metres, and the global Voxel it tops out in.
    fn water_top(&self, column: [i32; 2]) -> Option<(f32, i32)>;
    /// The generated ground's height under a column, in metres.
    fn ground_m(&self, column: [i32; 2]) -> f32;
    /// The Generator's Flow over a column, in m/s along x and z.
    fn flow(&self, column: [i32; 2]) -> [f32; 2];
    /// The Fall over a column, if it is a lip with a lower pool past it.
    fn fall(&self, column: [i32; 2]) -> Option<Fall>;
    /// The sea's Water surface in metres, `None` in a World without a sea.
    fn sea_level_m(&self) -> Option<f32>;
}

/// One column's water, 4 bytes: zero for a dry column.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WaterColumn {
    /// Along x and z, in steps of [`Self::FLOW_STEP_M_S`].
    pub flow: [i8; 2],
    /// In steps of [`Self::DEPTH_STEP_M`], saturating; at least one on a wet column.
    pub depth: u8,
    pub flags: u8,
}

impl WaterColumn {
    pub const FLOW_STEP_M_S: f32 = 4.0 / 127.0;
    /// Sixteen metres saturate, past which water has absorbed the red and green of the light.
    pub const DEPTH_STEP_M: f32 = 1.0 / 16.0;
    /// Its water falls: it is descending, or continues up past what the job sees without a
    /// Source holding it.
    pub const FALLING: u8 = 1;
    /// A Fall lands in it.
    pub const FOAM: u8 = 2;
    /// The Generator's sea, which runs in waves from the Wind; a lake only ripples.
    pub const SEA: u8 = 4;

    fn new(flow_m_s: [f32; 2], depth_m: f32, flags: u8) -> Self {
        let step = |v: f32| (v / Self::FLOW_STEP_M_S).round().clamp(-127.0, 127.0) as i8;
        WaterColumn {
            flow: flow_m_s.map(step),
            depth: (depth_m / Self::DEPTH_STEP_M).round().clamp(1.0, 255.0) as u8,
            flags,
        }
    }

    pub fn is_wet(&self) -> bool {
        self.depth > 0
    }

    pub fn flow_m_s(&self) -> [f32; 2] {
        self.flow.map(|f| f as f32 * Self::FLOW_STEP_M_S)
    }

    pub fn depth_m(&self) -> f32 {
        self.depth as f32 * Self::DEPTH_STEP_M
    }
}

/// The index of column `[x, z]`, each in `-1..=CHUNK_SIZE`, in a Chunk's water columns.
pub fn water_index(x: i32, z: i32) -> usize {
    debug_assert!((-1..=CHUNK_SIZE as i32).contains(&x) && (-1..=CHUNK_SIZE as i32).contains(&z));
    (z + 1) as usize * WATER_SIDE + (x + 1) as usize
}

/// The sheets of water pouring over the Generator's lips in the Chunk at `coord`, for its blended
/// layer: one per lip column with a lower pool past it, on the lip's far face a sixteenth off the
/// rock, from the upper Water surface down to the lower, cut to the Chunk. Drawn only: nothing is
/// added to the World (ADR-0038). Lit as the air past the lip is.
pub fn fall_sheets(
    coord: [i32; 3],
    center: LitChunk<'_>,
    answers: &impl WaterAnswers,
) -> Vec<Quad> {
    let Some(water) = answers.water_material() else {
        return Vec::new();
    };
    let sub = SUBDIV as i32;
    let size = answers.voxel_size();
    let base = [coord[0] * SIDE, coord[1] * SIDE, coord[2] * SIDE];
    let mut sheets = Vec::new();
    for z in 0..SIDE {
        for x in 0..SIDE {
            let Some(fall) = answers.fall([base[0] + x, base[2] + z]) else {
                continue;
            };
            let in_chunk = |m: f32| (((m / size) - base[1] as f32) * sub as f32).round() as i32;
            let low = in_chunk(fall.to_m).max(0);
            let high = in_chunk(fall.from_m).min(SIDE * sub);
            if low >= high {
                continue;
            }
            let [dx, dz] = fall.toward;
            let (face, plane) = match (dx, dz) {
                (1, 0) => (POS_X, (x + 1) * sub + 1),
                (-1, 0) => (NEG_X, x * sub - 1),
                (0, 1) => (POS_Z, (z + 1) * sub + 1),
                _ => (NEG_Z, z * sub - 1),
            };
            let along = |c: i32| [(c * sub) as u16, ((c + 1) * sub) as u16];
            let height = [low as u16, high as u16];
            // A face's u and v are its next two axes in turn: y then z across x, x then y across z.
            let (u, v) = if dx != 0 {
                (height, along(z))
            } else {
                (along(x), height)
            };
            let past = [x + dx, (high - 1) / sub, z + dz];
            let light = if past.iter().all(|&c| (0..SIDE).contains(&c)) {
                center
                    .light
                    .get(past[0] as usize, past[1] as usize, past[2] as usize)
            } else {
                Light::new(MAX_LIGHT, 0)
            };
            sheets.push(Quad::face(face, plane, u, v, u32::from(water), light));
        }
    }
    sheets
}

/// The water columns of a coarse Chunk whose corner is at `origin_m` (x and z) and whose cells
/// are `cell_m` across: over each column's centre, from `level_m`'s Water surface down to
/// `ground_m`, calm, and flagged as the sea at or below `sea_m`. Coarse levels draw the
/// Generator's water and keep none of the World's own, so its answers are the whole rule. Empty
/// where none of the Chunk's own columns is wet.
pub fn coarse_water_columns(
    origin_m: [f32; 2],
    cell_m: f32,
    level_m: impl Fn(f32, f32) -> Option<f32>,
    ground_m: impl Fn(f32, f32) -> f32,
    sea_m: Option<f32>,
) -> Vec<WaterColumn> {
    let mut out = vec![WaterColumn::default(); WATER_SIDE * WATER_SIDE];
    let mut wet = false;
    for z in -1..=SIDE {
        for x in -1..=SIDE {
            let at = [
                origin_m[0] + (x as f32 + 0.5) * cell_m,
                origin_m[1] + (z as f32 + 0.5) * cell_m,
            ];
            let Some(surface_m) = level_m(at[0], at[1]) else {
                continue;
            };
            let depth_m = surface_m - ground_m(at[0], at[1]);
            if depth_m <= 0.0 {
                continue;
            }
            let sea = sea_m.is_some_and(|sea_m| surface_m <= sea_m);
            let flags = if sea { WaterColumn::SEA } else { 0 };
            out[water_index(x, z)] = WaterColumn::new([0.0; 2], depth_m, flags);
            wet |= (0..SIDE).contains(&x) && (0..SIDE).contains(&z);
        }
    }
    if wet { out } else { Vec::new() }
}

/// How fast simulated water runs down a surface sloping one in one.
const SLOPE_FLOW_M_S: f32 = 2.0;
const MAX_FLOW_M_S: f32 = 4.0;
const SIDE: i32 = CHUNK_SIZE as i32;
const CARDINALS: [[i32; 2]; 4] = [[0, 1], [1, 0], [0, -1], [-1, 0]];

/// The water columns of the Chunk at `coord` from it and its six face neighbours, row by row along
/// x then z over [`WATER_SIDE`]; empty for a Chunk with no fluid of `fluids`. Each column is
/// classified on its own: one whose top water is a Source of the World's water in the Voxel the
/// Generator's Water surface tops out in takes the Generator's answers, and any other (poured,
/// simulated, dammed, a Settlement's pond) is read from the Voxels the job sees.
pub fn water_columns(
    coord: [i32; 3],
    center: LitChunk<'_>,
    neighbors: [Option<LitChunk<'_>>; 6],
    fluids: &FluidTable,
    answers: &impl WaterAnswers,
) -> Vec<WaterColumn> {
    let packed = center
        .chunk
        .kinds()
        .iter()
        .any(|v| fluids.contains(v.material()));
    if !packed && !center.overlay.may_contain_fluid() {
        return Vec::new();
    }
    let seen = Seen {
        center,
        neighbors,
        fluids,
    };
    let mut scans = vec![None; WATER_SIDE * WATER_SIDE];
    for z in -1..=SIDE {
        for x in -1..=SIDE {
            scans[water_index(x, z)] = seen.scan(x, z);
        }
    }
    if !(0..SIDE).any(|z| (0..SIDE).any(|x| scans[water_index(x, z)].is_some())) {
        return Vec::new();
    }
    let base = [coord[0] * SIDE, coord[1] * SIDE, coord[2] * SIDE];
    let size = answers.voxel_size();
    let sea_m = answers.sea_level_m();
    let generated = |column: [i32; 2], level_m: f32, top: i32| {
        let depth_m = (top + 1) as f32 * size - answers.ground_m(column);
        let foam = CARDINALS.iter().any(|&[dx, dz]| {
            answers
                .fall([column[0] - dx, column[1] - dz])
                .is_some_and(|f| f.toward == [dx, dz])
        });
        let sea = sea_m.is_some_and(|sea_m| level_m <= sea_m);
        let flags =
            if foam { WaterColumn::FOAM } else { 0 } | if sea { WaterColumn::SEA } else { 0 };
        WaterColumn::new(answers.flow(column), depth_m, flags)
    };
    let mut out = vec![WaterColumn::default(); WATER_SIDE * WATER_SIDE];
    for z in -1..=SIDE {
        for x in -1..=SIDE {
            let Some(scan) = scans[water_index(x, z)] else {
                continue;
            };
            let column = [base[0] + x, base[2] + z];
            let top = base[1] + scan.top;
            let world_water =
                scan.fluid.source && Some(scan.fluid.material) == answers.water_material();
            if world_water
                && let Some((level_m, k)) = answers.water_top(column)
                && (k == top || (scan.ceiling && k > top))
            {
                out[water_index(x, z)] = generated(column, level_m, k);
                continue;
            }
            let height = |x: i32, z: i32| {
                let in_apron = (-1..=SIDE).contains(&x) && (-1..=SIDE).contains(&z);
                in_apron.then(|| scans[water_index(x, z)].map(|s| s.height()))?
            };
            let slope = |h_minus: Option<f32>, h_plus: Option<f32>| match (h_minus, h_plus) {
                (Some(a), Some(b)) => (b - a) / 2.0,
                (Some(a), None) => scan.height() - a,
                (None, Some(b)) => b - scan.height(),
                (None, None) => 0.0,
            };
            let grad = [
                slope(height(x - 1, z), height(x + 1, z)),
                slope(height(x, z - 1), height(x, z + 1)),
            ];
            let flow = grad.map(|g| (-g * SLOPE_FLOW_M_S).clamp(-MAX_FLOW_M_S, MAX_FLOW_M_S));
            let falling = scan.fluid.descent > 0 || (scan.ceiling && !scan.fluid.source);
            let depth_m = if scan.saturated {
                f32::MAX
            } else {
                scan.depth * size
            };
            let flags = if falling { WaterColumn::FALLING } else { 0 };
            out[water_index(x, z)] = WaterColumn::new(flow, depth_m, flags);
        }
    }
    // The job holds only face neighbours, so an apron corner is the Generator's where it has
    // water and otherwise repeats the held column beside it along z.
    for (x, z, held_z) in [
        (-1, -1, 0),
        (SIDE, -1, 0),
        (-1, SIDE, SIDE - 1),
        (SIDE, SIDE, SIDE - 1),
    ] {
        let column = [base[0] + x, base[2] + z];
        out[water_index(x, z)] = answers
            .water_top(column)
            .map_or(out[water_index(x, held_z)], |(level_m, k)| {
                generated(column, level_m, k)
            });
    }
    out
}

/// A column's water as the job sees it.
#[derive(Clone, Copy, Debug)]
struct Scan {
    /// The top water Voxel's y in the Chunk's own Voxels, and its fluid.
    top: i32,
    fluid: Fluid,
    /// From the top water's surface down to the first Voxel under it without water, in Voxels.
    depth: f32,
    /// The water runs past the bottom of what the job sees.
    saturated: bool,
    /// The water runs up past the top of what the job sees.
    ceiling: bool,
}

impl Scan {
    /// The top water's surface, in Voxels up from the Chunk's floor.
    fn height(&self) -> f32 {
        self.top as f32 + self.fluid.surface as f32 / Fluid::FULL_SURFACE as f32
    }
}

/// The Chunk and its six face neighbours, as one span of Voxels.
struct Seen<'a> {
    center: LitChunk<'a>,
    neighbors: [Option<LitChunk<'a>>; 6],
    fluids: &'a FluidTable,
}

impl Seen<'_> {
    /// The fluid at `[x, y, z]` from the Chunk's corner, `None` outside what the job sees and
    /// `Some(None)` where it sees no fluid.
    fn fluid(&self, x: i32, y: i32, z: i32) -> Option<Option<Fluid>> {
        let wrap = |v: i32| v.rem_euclid(SIDE) as usize;
        let out = [x, y, z].map(|v| {
            if v < 0 {
                -1
            } else if v >= SIDE {
                1
            } else {
                0
            }
        });
        let chunk = match out {
            [0, 0, 0] => self.center,
            [-1, 0, 0] => self.neighbors[NEG_X]?,
            [1, 0, 0] => self.neighbors[POS_X]?,
            [0, -1, 0] => self.neighbors[NEG_Y]?,
            [0, 1, 0] => self.neighbors[POS_Y]?,
            [0, 0, -1] => self.neighbors[NEG_Z]?,
            [0, 0, 1] => self.neighbors[POS_Z]?,
            _ => return None,
        };
        if !(-SIDE..2 * SIDE).contains(&y) {
            return None;
        }
        let (x, y, z) = (wrap(x), wrap(y), wrap(z));
        let occupants = Occupants::of(
            chunk.chunk.get(x, y, z),
            chunk.overlay.get(x, y, z),
            self.fluids,
        );
        Some(occupants.fluid)
    }

    /// Column `[x, z]`'s water: its top in the Chunk holding the column, followed up and down as
    /// far as the job sees.
    fn scan(&self, x: i32, z: i32) -> Option<Scan> {
        let mut top = (0..SIDE)
            .rev()
            .find(|&y| matches!(self.fluid(x, y, z), Some(Some(_))))?;
        while let Some(Some(_)) = self.fluid(x, top + 1, z) {
            top += 1;
        }
        let ceiling = self.fluid(x, top + 1, z).is_none();
        let fluid = self.fluid(x, top, z)??;
        let mut floor = top - 1;
        let saturated = loop {
            match self.fluid(x, floor, z) {
                Some(Some(_)) => floor -= 1,
                Some(None) => break false,
                None => break true,
            }
        };
        let depth = (top - floor - 1) as f32 + fluid.surface as f32 / Fluid::FULL_SURFACE as f32;
        Some(Scan {
            top,
            fluid,
            depth,
            saturated,
            ceiling,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::light::ChunkLight;
    use crate::occupants::{ChunkOverlay, Level, Overlay};
    use crate::{Chunk, Voxel};

    static DARK: ChunkLight = ChunkLight::dark();
    const FLOOR: usize = 4;

    /// The Generator's answers, the same over every column.
    struct Double {
        top: Option<(f32, i32)>,
        flow: [f32; 2],
        falls: Vec<([i32; 2], Fall)>,
        sea_m: Option<f32>,
    }

    impl WaterAnswers for Double {
        fn voxel_size(&self) -> f32 {
            1.0
        }
        fn water_material(&self) -> Option<MaterialId> {
            Some(water())
        }
        fn water_top(&self, _: [i32; 2]) -> Option<(f32, i32)> {
            self.top
        }
        fn ground_m(&self, _: [i32; 2]) -> f32 {
            FLOOR as f32
        }
        fn flow(&self, _: [i32; 2]) -> [f32; 2] {
            self.flow
        }
        fn fall(&self, column: [i32; 2]) -> Option<Fall> {
            self.falls
                .iter()
                .find(|(c, _)| *c == column)
                .map(|(_, f)| *f)
        }
        fn sea_level_m(&self) -> Option<f32> {
            self.sea_m
        }
    }

    fn water() -> MaterialId {
        crate::testgen::fixtures().water
    }

    /// Rock under [`FLOOR`] and air over it.
    fn ground() -> Chunk {
        let mut chunk = Chunk::air();
        let rock = Voxel::cube(crate::testgen::fixtures().ground.rock);
        for y in 0..FLOOR {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    chunk.set(x, y, z, rock);
                }
            }
        }
        chunk
    }

    fn flowing(surface: u8, descent: u16) -> Fluid {
        let level = Level {
            surface,
            reach: 3,
            descent,
        };
        Fluid::flowing(water(), [level; crate::shape::MAX_VOIDS]).unwrap()
    }

    /// `fluid(x, z)` in every Voxel of each column from [`FLOOR`] up to `top(x, z)`.
    fn pool(
        top: impl Fn(usize, usize) -> usize,
        fluid: impl Fn(usize, usize) -> Fluid,
    ) -> ChunkOverlay {
        let mut overlay = ChunkOverlay::none();
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                for y in FLOOR..=top(x, z) {
                    overlay.set(
                        x,
                        y,
                        z,
                        Overlay {
                            fluid: Some(fluid(x, z)),
                            burning: None,
                        },
                    );
                }
            }
        }
        overlay
    }

    fn columns(center: &Chunk, overlay: &ChunkOverlay, answers: &Double) -> Vec<WaterColumn> {
        let air = Chunk::air();
        let around = [Some(LitChunk::new(&air, &DARK)); 6];
        water_columns(
            [0; 3],
            LitChunk::new(center, &DARK).with_overlay(overlay),
            around,
            &crate::testgen::fixtures().tables.fluids,
            answers,
        )
    }

    fn generator(top: Option<(f32, i32)>) -> Double {
        Double {
            top,
            flow: [0.5, -0.25],
            falls: Vec::new(),
            sea_m: None,
        }
    }

    #[test]
    fn a_dry_chunk_has_no_water_columns() {
        let chunk = ground();
        assert!(columns(&chunk, &ChunkOverlay::none(), &generator(Some((9.6, 9)))).is_empty());
    }

    #[test]
    fn a_generated_lakes_depth_runs_from_its_drawn_top_to_its_ground() {
        // Generation packs its water as cubes of the Material, which read as Sources.
        let mut chunk = ground();
        for y in FLOOR..=9 {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    chunk.set(x, y, z, Voxel::cube(water()));
                }
            }
        }
        let lake = ChunkOverlay::none();
        let mut answers = generator(Some((9.6, 9)));
        let fall = Fall {
            toward: [1, 0],
            from_m: 11.6,
            to_m: 9.6,
        };
        answers.falls.push(([5, 5], fall));
        let got = columns(&chunk, &lake, &answers);
        assert_eq!(got.len(), WATER_SIDE * WATER_SIDE);
        for z in 0..SIDE {
            for x in 0..SIDE {
                let c = got[water_index(x, z)];
                assert_eq!(c.depth_m(), 10.0 - FLOOR as f32, "{x},{z}");
                let [fx, fz] = c.flow_m_s();
                assert!(
                    (fx - 0.5).abs() < 0.02 && (fz + 0.25).abs() < 0.02,
                    "{x},{z}"
                );
                let foam = [x, z] == [6, 5];
                assert_eq!(c.flags & WaterColumn::FOAM != 0, foam, "{x},{z}");
            }
        }
        assert!(got[water_index(-1, -1)].is_wet(), "the Generator's corner");
    }

    #[test]
    fn generated_water_at_sea_level_is_the_sea_and_a_lake_above_it_is_not() {
        let mut chunk = ground();
        for y in FLOOR..=9 {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    chunk.set(x, y, z, Voxel::cube(water()));
                }
            }
        }
        let none = ChunkOverlay::none();
        let mut sea = generator(Some((9.6, 9)));
        sea.sea_m = Some(9.6);
        let got = columns(&chunk, &none, &sea);
        assert!(got[water_index(3, 3)].flags & WaterColumn::SEA != 0);
        sea.sea_m = Some(5.0);
        let got = columns(&chunk, &none, &sea);
        assert!(
            got[water_index(3, 3)].flags & WaterColumn::SEA == 0,
            "a lake"
        );
    }

    #[test]
    fn a_lip_with_a_lower_pool_past_it_pours_one_sheet_from_surface_to_surface() {
        let chunk = ground();
        let light = ChunkLight::filled(Light::new(MAX_LIGHT, 0));
        let mut answers = generator(Some((9.6, 9)));
        answers.falls.push((
            [5, 7],
            Fall {
                toward: [1, 0],
                from_m: 11.5,
                to_m: 9.25,
            },
        ));
        answers.falls.push((
            [9, 3],
            Fall {
                toward: [0, -1],
                from_m: 12.0,
                to_m: 10.0,
            },
        ));
        let sheets = fall_sheets([0, 0, 0], LitChunk::new(&chunk, &light), &answers);
        assert_eq!(sheets.len(), 2);
        let corners = |q: &Quad| q.corners().map(|c| c.position);
        // Row by row along z, so the lip at z 3 comes first.
        let (north, east) = (corners(&sheets[0]), corners(&sheets[1]));
        assert!(east.iter().all(|p| p[0] == 6.0 + 1.0 / 16.0), "{east:?}");
        let (ys, zs): (Vec<f32>, Vec<f32>) = east.iter().map(|p| (p[1], p[2])).unzip();
        assert_eq!(ys.iter().copied().fold(f32::MAX, f32::min), 9.25);
        assert_eq!(ys.iter().copied().fold(f32::MIN, f32::max), 11.5);
        assert_eq!(zs.iter().copied().fold(f32::MAX, f32::min), 7.0);
        assert_eq!(zs.iter().copied().fold(f32::MIN, f32::max), 8.0);
        assert!(north.iter().all(|p| p[2] == 3.0 - 1.0 / 16.0), "{north:?}");
        assert!(sheets.iter().all(|q| q.material == u32::from(water())));
    }

    #[test]
    fn a_lip_beside_dry_ground_pours_nothing() {
        let chunk = ground();
        let light = ChunkLight::filled(Light::new(MAX_LIGHT, 0));
        let answers = generator(Some((9.6, 9)));
        assert!(fall_sheets([0, 0, 0], LitChunk::new(&chunk, &light), &answers).is_empty());
        let high = fall_sheets([0, 1, 0], LitChunk::new(&chunk, &light), &{
            let mut answers = generator(Some((9.6, 9)));
            answers.falls.push((
                [5, 7],
                Fall {
                    toward: [1, 0],
                    from_m: 11.5,
                    to_m: 9.25,
                },
            ));
            answers
        });
        assert!(high.is_empty(), "a Fall below the Chunk is cut away");
    }

    #[test]
    fn simulated_flow_points_down_the_surface_gradient() {
        let chunk = ground();
        let water = pool(|_, _| FLOOR, |x, _| flowing(16 - (x / 2) as u8, 0));
        let got = columns(&chunk, &water, &generator(None));
        for z in 1..SIDE - 1 {
            for x in 1..SIDE - 1 {
                let [fx, fz] = got[water_index(x, z)].flow_m_s();
                assert!(fx > 0.0 && fz.abs() < 1e-6, "{fx},{fz} at {x},{z}");
            }
        }
        assert_eq!(got[water_index(-1, 5)], WaterColumn::default(), "dry apron");
    }

    #[test]
    fn falling_fluid_is_flagged() {
        let chunk = ground();
        let water = pool(
            |x, _| if x == 7 { 20 } else { FLOOR },
            |x, _| {
                if x == 3 {
                    flowing(12, 8)
                } else {
                    flowing(16, 0)
                }
            },
        );
        let got = columns(&chunk, &water, &generator(None));
        let falling = |x: i32| got[water_index(x, 9)].flags & WaterColumn::FALLING != 0;
        assert!(falling(3), "descending");
        assert!(!falling(5) && !falling(7), "lying");

        // Flowing water running on up into the Chunk above is a stream falling through.
        let above = Chunk::air();
        let column = pool(
            |x, _| if x == 7 { CHUNK_SIZE - 1 } else { 0 },
            |_, _| flowing(16, 0),
        );
        let mut over = column.clone();
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                for y in 0..FLOOR {
                    let fluid = (x == 7).then(|| flowing(16, 0));
                    over.set(
                        x,
                        y,
                        z,
                        Overlay {
                            fluid,
                            burning: None,
                        },
                    );
                }
            }
        }
        let air = Chunk::air();
        let mut around = [Some(LitChunk::new(&air, &DARK)); 6];
        around[POS_Y] = Some(LitChunk::new(&above, &DARK).with_overlay(&over));
        let tall = pool(
            |x, _| if x == 7 { CHUNK_SIZE - 1 } else { FLOOR },
            |_, _| flowing(16, 0),
        );
        let got = water_columns(
            [0; 3],
            LitChunk::new(&chunk, &DARK).with_overlay(&tall),
            around,
            &crate::testgen::fixtures().tables.fluids,
            &generator(None),
        );
        assert_ne!(
            got[water_index(7, 9)].flags & WaterColumn::FALLING,
            0,
            "through the top"
        );
        assert_eq!(got[water_index(5, 9)].flags & WaterColumn::FALLING, 0);
    }

    #[test]
    fn a_dammed_river_and_a_settlement_pond_take_the_scan_rule() {
        let chunk = ground();
        let dammed = pool(|_, _| 9, |_, _| Fluid::source(water()));
        // The Generator's river tops out three Voxels under the dammed water.
        for answers in [generator(Some((6.6, 6))), generator(None)] {
            let got = columns(&chunk, &dammed, &answers);
            let c = got[water_index(12, 12)];
            assert_eq!(c.depth_m(), 6.0, "from the scan, not the drawn top");
            assert_eq!(c.flow_m_s(), [0.0; 2], "level water stands still");
        }
    }
}

//! Seam walls read from the children, against the walls of a mesh that keeps them all.

use super::*;
use bevy::platform::collections::HashSet;
use tesserae_core::RenderClass;

/// Levels 0 to 3 of `source`'s ground, built as the Server builds them.
struct Levels {
    chunks: HashMap<(u32, ChunkCoord), Shown>,
    tables: Tables,
}

impl Levels {
    fn shown(&mut self, level: u32, c: ChunkCoord) -> Shown {
        if let Some(s) = self.chunks.get(&(level, c)) {
            return s.clone();
        }
        let s = if level == 0 {
            shown(packed(source(0).chunk(c.x, c.y, c.z)))
        } else {
            shown(generate_coarse(
                &source(level - 1),
                c,
                &Default::default(),
                &self.tables,
                None,
                seam_margin(level),
            ))
        };
        self.chunks.insert((level, c), s.clone());
        s
    }

    fn opaque(&mut self, level: u32, at: IVec3) -> bool {
        let e = CHUNK_SIZE as i32;
        let c = ChunkCoord::new(at.x.div_euclid(e), at.y.div_euclid(e), at.z.div_euclid(e));
        let l = at.map(|v| v.rem_euclid(e));
        let v = self
            .shown(level, c)
            .chunk
            .get(l.x as usize, l.y as usize, l.z as usize);
        v.is_full_cube() && self.tables.materials.render(v) == RenderClass::Opaque
    }
}

/// The cells of the walls `seamed` has past `plain`, as (axis, whether the wall looks toward
/// negative, the Voxel it belongs to).
fn walls(plain: &ChunkMesh, seamed: &ChunkMesh) -> HashSet<(usize, bool, IVec3)> {
    let mut cells = HashSet::default();
    for (layer, base) in [
        (&seamed.opaque, &plain.opaque),
        (&seamed.cutout, &plain.cutout),
    ] {
        for q in base.positions.len() / 4..layer.positions.len() / 4 {
            let n = layer.normals[q * 4];
            let axis = if n[0] != 0.0 { 0 } else { 2 };
            let negative = n[axis] < 0.0;
            let corners = &layer.positions[q * 4..q * 4 + 4];
            let lo = corners.iter().fold(Vec3::MAX, |a, &p| a.min(Vec3::from(p)));
            let hi = corners.iter().fold(Vec3::MIN, |a, &p| a.max(Vec3::from(p)));
            let other = 2 - axis;
            let plane = lo[axis] as i32;
            for y in lo.y as i32..hi.y as i32 {
                for w in lo[other] as i32..hi[other] as i32 {
                    let mut cell = IVec3::ZERO;
                    cell[axis] = if negative { plane } else { plane - 1 };
                    cell.y = y;
                    cell[other] = w;
                    cells.insert((axis, negative, cell));
                }
            }
        }
    }
    cells
}

/// A wall can be seen where the finer level's Voxels against its plane, across it, are not all
/// opaque cubes; every such wall a mesh keeping them all has must be kept. Levels 2 and 3 read
/// their children with a margin, so this is also the check on it.
#[test]
fn no_seam_wall_the_finer_ground_leaves_open_is_dropped() {
    let mut levels = Levels {
        chunks: HashMap::default(),
        tables: crate::server::fixture_tables(),
    };
    let surface = source(0);
    let (low_m, high_m) = surface.surface_bounds_m();
    let all = Arc::new(SeamOpenings::everywhere());
    let none = Arc::new(SeamOpenings::nowhere());
    let mut seen = 0;
    for level in 1..=3 {
        let cell_m = world().cell_size_at(level);
        let voxel_m = world().voxel_size * 2f32.powi(level as i32);
        let (mut every, mut kept) = (0, 0);
        for y in (low_m / cell_m).floor() as i32..=(high_m / cell_m).floor() as i32 {
            for (x, z) in [(0, 0), (1, 0), (-1, -1), (2, -3)] {
                let coord = ChunkCoord::new(x, y, z);
                let center = levels.shown(level, coord);
                let around: [Shown; 6] =
                    std::array::from_fn(|f| levels.shown(level, face_neighbors(coord)[f]));
                let mesh = |openings: Option<&Arc<SeamOpenings>>| {
                    let with = |s: &Shown| Shown {
                        openings: openings.map_or_else(|| Arc::clone(&s.openings), Arc::clone),
                        ..s.clone()
                    };
                    let tables = &levels.tables;
                    mesh_coarse(
                        &surface,
                        level,
                        voxel_m,
                        coord,
                        &with(&center),
                        &around.each_ref().map(with),
                        tables,
                    )
                };
                let plain = mesh(Some(&none));
                let ungated = walls(&plain, &mesh(Some(&all)));
                let gated = walls(&plain, &mesh(None));
                every += ungated.len();
                kept += gated.len();
                let origin = IVec3::new(coord.x, coord.y, coord.z) * CHUNK_SIZE as i32;
                for &(axis, negative, cell) in &ungated {
                    let other = 2 - axis;
                    let at = origin + cell;
                    let across = at[axis] + if negative { -1 } else { 1 };
                    let mut finer = IVec3::ZERO;
                    finer[axis] = if negative { 2 * across + 1 } else { 2 * across };
                    let open = (0..4).any(|i| {
                        finer.y = 2 * at.y + i / 2;
                        finer[other] = 2 * at[other] + i % 2;
                        !levels.opaque(level - 1, finer)
                    });
                    if open {
                        seen += 1;
                        assert!(
                            gated.contains(&(axis, negative, cell)),
                            "level {level} {coord}: the wall of {cell} across axis {axis}"
                        );
                    }
                }
            }
        }
        assert!(kept < every, "level {level} kept all {every} walls");
        if level == 1 {
            assert!(2 * kept < every, "level 1 kept {kept} of {every} walls");
        }
    }
    assert!(seen > 500, "only {seen} walls could be seen");
}

//! Pass 1 as it was before the bitmask one, which must stay identical to it.

use super::*;

pub(super) fn pass1_reference<const TRANSLUCENT: bool, const SURFACES: bool>(
    n: &Neighborhood,
    tables: &Tables,
    mesh: &mut ChunkMesh,
) {
    let shapes = &tables.shapes;
    let materials = &tables.materials;
    let mut mask = vec![None::<FaceKey>; CHUNK_SIZE * CHUNK_SIZE];

    for d in 0..3usize {
        // (d, u, v) is a cyclic permutation of (x, y, z), so u cross v = d and
        // the winding below is counter-clockwise seen from +d.
        let u = (d + 1) % 3;
        let v = (d + 2) % 3;

        for positive in [false, true] {
            let step = if positive { 1 } else { -1 };
            // Faces come in negative/positive pairs, so flipping the low bit is
            // the face of the neighbour that looks back at this one.
            let face = d * 2 + usize::from(positive);
            let under_surface = SURFACES && face == POS_Y;

            for s in 0..CHUNK_SIZE {
                mask.fill(None);
                for a in 0..CHUNK_SIZE {
                    // Indexed by strides: with the cell's coordinates in an
                    // array indexed by axis, they lived on the stack and the
                    // loop's speed came down to how the optimiser laid out the
                    // function around it.
                    let row = pad_index(0, 0, 0) + s * STRIDES[d] + a * STRIDES[u];
                    for b in 0..CHUNK_SIZE {
                        let i = row + b * STRIDES[v];
                        let here = n.voxels[i];
                        if !here.is_full_cube() {
                            continue;
                        }
                        let j = if positive {
                            i + STRIDES[d]
                        } else {
                            i - STRIDES[d]
                        };
                        let mut c = [0i32; 3];
                        c[d] = s as i32 + step;
                        c[u] = a as i32;
                        c[v] = b as i32;
                        // Hidden only if the Voxel across covers this whole
                        // face. A slab beside a cube hides the cube's face on
                        // the slab's sealed side and nowhere else.
                        //
                        // The cube test comes first and alone because it is
                        // nearly every answer, and `seals` for all of them
                        // measured 2-6% slower on `greedy_mesh/solid` and
                        // `/terrain`. `seals` is the same question, asked of
                        // the table only for a shaped Voxel.
                        let across = n.voxels[j];
                        let hidden = if across.is_full_cube() {
                            !TRANSLUCENT || hides(materials, across, here)
                        } else {
                            across.shape() != 0
                                && (shapes.seals(across, face ^ 1)
                                    && (!TRANSLUCENT || hides(materials, across, here))
                                    || TRANSLUCENT
                                        && n.logged_at(c[0], c[1], c[2])
                                            .is_some_and(|f| f.material == here.material())
                                        && n.fluid_cover(tables, c, face ^ 1, here, here).is_full())
                                // Lower fluid of the same Material standing on
                                // this one covers all of its top.
                                || under_surface
                                    && n.surface_at(c[0], c[1], c[2])
                                        .is_some_and(|(m, _)| m == here.material())
                        };
                        if hidden {
                            continue;
                        }
                        // The light of the Voxel across the face, not of the
                        // one the face belongs to: an opaque Voxel holds 0 by
                        // construction, which is what makes the dimming rule
                        // work, so lighting a face by its own Voxel would make
                        // every face black. Read from the cell this loop has
                        // already fetched to test occlusion.
                        let lit = n.light[j];
                        mask[a * CHUNK_SIZE + b] = Some((here.material(), lit));
                    }
                }

                let mut a = 0;
                while a < CHUNK_SIZE {
                    let mut b = 0;
                    while b < CHUNK_SIZE {
                        let Some(key) = mask[a * CHUNK_SIZE + b] else {
                            b += 1;
                            continue;
                        };
                        let (m, lit) = key;

                        let mut w = 1;
                        while b + w < CHUNK_SIZE && mask[a * CHUNK_SIZE + b + w] == Some(key) {
                            w += 1;
                        }

                        let mut h = 1;
                        'grow: while a + h < CHUNK_SIZE {
                            for k in 0..w {
                                if mask[(a + h) * CHUNK_SIZE + b + k] != Some(key) {
                                    break 'grow;
                                }
                            }
                            h += 1;
                        }

                        let sub = SUBDIV;
                        let plane = (s + usize::from(positive)) * sub;
                        let span =
                            |at: usize, len: usize| [(at * sub) as u16, ((at + len) * sub) as u16];

                        // A run is one Material, so it is one layer.
                        let layer = if TRANSLUCENT {
                            mesh.layer(materials.def(m).render)
                        } else {
                            &mut mesh.opaque
                        };
                        layer.quads.push(Quad::face(
                            face,
                            plane as i32,
                            span(a, h),
                            span(b, w),
                            m as u32,
                            lit,
                        ));

                        for i in 0..h {
                            for j in 0..w {
                                mask[(a + i) * CHUNK_SIZE + b + j] = None;
                            }
                        }
                        b += w;
                    }
                    a += 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::light::MAX_LIGHT;
    use crate::occupants::Overlay;
    use crate::testgen::{self, fixtures};
    use crate::{Orientation, SHAPE_SLAB, SHAPE_STAIR};

    /// Pass 1 alone, both ways, with the instance `greedy_mesh` would pick.
    fn assert_identical(center: LitChunk<'_>, neighbors: [Option<LitChunk<'_>>; 6], what: &str) {
        let tables = &fixtures().tables;
        let n = Neighborhood::new(center, neighbors);
        let (mut fast, mut slow) = (ChunkMesh::default(), ChunkMesh::default());
        if !n.surfaces.is_empty() {
            pass1::<true, true>(&n, tables, &mut fast);
            pass1_reference::<true, true>(&n, tables, &mut slow);
        } else if n.is_translucent(&tables.materials) {
            pass1::<true, false>(&n, tables, &mut fast);
            pass1_reference::<true, false>(&n, tables, &mut slow);
        } else {
            pass1::<false, false>(&n, tables, &mut fast);
            pass1_reference::<false, false>(&n, tables, &mut slow);
        }
        assert!(fast.opaque == slow.opaque, "{what}: opaque");
        assert!(fast.blended == slow.blended, "{what}: blended");
        assert!(fast.cutout == slow.cutout, "{what}: cutout");
    }

    /// xorshift, so the Chunks are the same every run.
    struct Seeded(u64);

    impl Seeded {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
    }

    /// Cubes of every kind the fixtures have, slabs and stairs, fluid in the
    /// overlay and light everywhere, at a density.
    fn random(seed: u64, solid_in_16: u64) -> (Chunk, ChunkLight, ChunkOverlay) {
        let f = fixtures();
        let mut r = Seeded(seed);
        let (mut chunk, mut light, mut overlay) =
            (Chunk::air(), ChunkLight::dark(), ChunkOverlay::none());
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let bits = r.next();
                    light.set(
                        x,
                        y,
                        z,
                        Light::new((bits >> 40) as u8 & 15, (bits >> 44) as u8 & 15),
                    );
                    let voxel = match bits % 16 {
                        k if k >= solid_in_16 => Voxel::AIR,
                        0 => Voxel::cube(f.glass),
                        1 => Voxel::cube(f.leaves),
                        2 => Voxel::cube(f.water),
                        3 => Voxel::shaped(f.ground.rock, SHAPE_SLAB, Orientation::default()),
                        4 => Voxel::shaped(f.ground.rock, SHAPE_STAIR, Orientation::default()),
                        5 => Voxel::cube(f.lantern),
                        _ => Voxel::cube(f.ground.rock),
                    };
                    chunk.set(x, y, z, voxel);
                    if voxel.is_air() && bits >> 20 & 7 == 0 {
                        let surface = (bits >> 24) as u8 % 16 + 1;
                        let fluid = Fluid {
                            source: false,
                            surface,
                            reach: 1,
                            ..Fluid::source(f.water)
                        };
                        overlay.set(
                            x,
                            y,
                            z,
                            Overlay {
                                fluid: Some(fluid),
                                burning: None,
                            },
                        );
                    }
                }
            }
        }
        (chunk, light, overlay)
    }

    #[test]
    fn the_bitmask_pass_1_makes_the_reference_s_quads_in_its_order() {
        let f = fixtures();
        let sky = ChunkLight::filled(Light::new(MAX_LIGHT, 0));
        let dark = ChunkLight::dark();
        let dry = ChunkOverlay::none();
        let warm = ChunkLight::filled(Light::new(3, 9));

        // The meshing benches' kinds of Chunk.
        let solid = Chunk::filled(Voxel::cube(f.ground.rock));
        let checker = testgen::checkerboard_chunk(f.ground.rock);
        let mut ramp = ChunkLight::dark();
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    ramp.set(x, y, z, Light::new((y / 2) as u8 % 16, (x / 2) as u8 % 16));
                }
            }
        }
        for (x, y, z) in [(0, 0, 0), (1, 0, 1), (3, -1, 2), (0, 1, 0), (-2, 0, 5)] {
            let terrain = testgen::terrain_chunk(x, y, z, f.ground);
            for light in [&sky, &dark, &ramp] {
                let lit = LitChunk::new(&terrain, light);
                assert_identical(lit, [None; 6], "terrain");
                assert_identical(
                    lit,
                    [Some(LitChunk::new(&solid, &dark)); 6],
                    "terrain, buried",
                );
            }
        }
        for chunk in [&solid, &checker] {
            assert_identical(LitChunk::new(chunk, &sky), [None; 6], "solid or checker");
        }
        let mut neighbors = [Some(LitChunk::new(&solid, &dark)); 6];
        neighbors[POS_Y] = Some(LitChunk::new(&checker, &sky));
        assert_identical(
            LitChunk::new(&solid, &dark),
            neighbors,
            "solid under a checker",
        );

        // Seeded Chunks, opaque, translucent and wet, alone and among others.
        for seed in 1..=8u64 {
            let (chunk, light, overlay) = random(seed * 0x9e37_79b9, 4 + seed % 12);
            let (other, other_light, other_overlay) = random(seed + 100, 8);
            let lit = LitChunk::new(&chunk, &light);
            let wet = lit.with_overlay(&overlay);
            let around = LitChunk::new(&other, &other_light).with_overlay(&other_overlay);
            assert_identical(lit, [None; 6], &format!("random {seed}"));
            assert_identical(wet, [None; 6], &format!("random {seed}, wet"));
            assert_identical(
                wet,
                [Some(around); 6],
                &format!("random {seed}, among others"),
            );
            let opaque = Chunk::filled(Voxel::cube(f.ground.rock));
            let mut mixed = [Some(LitChunk::new(&opaque, &warm)); 6];
            mixed[seed as usize % 6] = Some(around);
            assert_identical(
                lit.with_overlay(&dry),
                mixed,
                &format!("random {seed}, one side"),
            );
        }
    }
}

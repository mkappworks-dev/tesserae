//! A Chunk layer's quads as the quad buffer holds them: a header, a table of
//! the layer's (Material, light) pairs, then one record per quad, which
//! `chunk_vertex.wesl` expands from the vertex index. See
//! `docs/adr/0030-a-chunk-mesh-is-one-record-per-quad.md`.
//!
//! This conversion lives in `tesserae` and never in `tesserae-core`, which must
//! acquire no `bevy` or `wgpu` dependency even transitively. See
//! `docs/architecture.md` invariant 3.

use bevy::asset::RenderAssetUsages;
use bevy::camera::primitives::Aabb;
use bevy::mesh::{
    Indices, MeshVertexAttribute, PrimitiveTopology, VertexAttributeValues, VertexFormat,
};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use tesserae_core::MeshLayer;
use tesserae_core::light::Light;
#[cfg(test)]
use tesserae_core::mesh::Corner;
use tesserae_core::mesh::{PAINT_SHIFT, PAINT_SLOTS, QUAD_CORNERS, Quad};

/// The placeholder a Chunk layer's mesh is made of: four vertices a quad, which
/// no pipeline reads. Bevy allocates a mesh's vertex range by its vertex count,
/// and this is the smallest attribute that gives it one.
pub const ATTRIBUTE_PLACEHOLDER: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_QuadPlaceholder", 2_084_155_066, VertexFormat::Uint8);

/// Drawn indexed, so the GPU shades each corner once rather than once per
/// triangle it is in: six vertices a quad unindexed cost the depth prepass and
/// the shadow passes about a fifth more.
pub const VERTICES_PER_QUAD: u32 = 4;

/// The Material is in the low bits of a table word, the light above them. The
/// Material needs 17 bits, for [`tesserae_core::mesh::FLAME`].
pub const LIGHT_SHIFT: u32 = 24;

/// The LOD level, the table's length and the words a record takes.
pub const HEADER_WORDS: u32 = 3;

/// The geometry, with the table index in the second word's top bits.
const NARROW: u32 = 2;
/// The geometry and the table word itself, for a layer with more pairs than a
/// table index reaches.
const WIDE: u32 = 3;

/// One Chunk layer's words, ready for the quad buffer. The Client keeps it
/// beside the entity drawing it, as collision and the blended sort read it.
#[derive(Component, Clone, Debug)]
pub struct ChunkQuads {
    words: Vec<u32>,
    aabb: Aabb,
    /// Where `words` starts in the quad buffer, which `MeshTag` carries to the
    /// shader. Set as it lands.
    pub at: u32,
}

impl ChunkQuads {
    /// `None` for a layer with nothing to draw.
    pub fn pack(layer: &MeshLayer, level: u32) -> Option<ChunkQuads> {
        if layer.is_empty() {
            return None;
        }
        let mut slots: HashMap<u32, u32> = HashMap::default();
        let mut table = Vec::new();
        let slot_of: Vec<u32> = layer
            .quads
            .iter()
            .map(|quad| {
                let word = paint(quad.material, quad.light);
                *slots.entry(word).or_insert_with(|| {
                    table.push(word);
                    table.len() as u32 - 1
                })
            })
            .collect();
        let stride = if table.len() > PAINT_SLOTS {
            WIDE
        } else {
            NARROW
        };
        let mut words = Vec::with_capacity(
            (HEADER_WORDS + stride * layer.quad_count() as u32) as usize + table.len(),
        );
        words.extend([level, 0, stride]);
        if stride == NARROW {
            words[1] = table.len() as u32;
            words.extend_from_slice(&table);
        }
        for (quad, &slot) in layer.quads.iter().zip(&slot_of) {
            let [lo, hi] = quad.geometry;
            if stride == NARROW {
                words.extend([lo, hi | slot << PAINT_SHIFT]);
            } else {
                words.extend([lo, hi, table[slot as usize]]);
            }
        }
        let (min, max) = layer
            .quads
            .iter()
            .map(Quad::bounds)
            .fold((Vec3::MAX, Vec3::MIN), |(lo, hi), (least, most)| {
                (lo.min(Vec3::from(least)), hi.max(Vec3::from(most)))
            });
        Some(ChunkQuads {
            words,
            aabb: Aabb::from_min_max(min, max),
            at: 0,
        })
    }

    pub fn words(&self) -> &[u32] {
        &self.words
    }

    #[cfg(any(test, feature = "dev", feature = "profiling"))]
    pub fn level(&self) -> u32 {
        self.words[0]
    }

    pub fn quad_count(&self) -> u32 {
        (self.words.len() as u32 - self.records_at()) / self.stride()
    }

    /// The bounds of every corner, in Voxels: the placeholder has no positions
    /// Bevy could find them from.
    pub fn aabb(&self) -> Aabb {
        self.aabb
    }

    /// What the GPU holds for the layer: its words and its placeholder.
    #[cfg(any(feature = "dev", feature = "profiling"))]
    pub fn gpu_bytes(&self) -> usize {
        let vertices = self.quad_count() * VERTICES_PER_QUAD;
        let index_bytes = if vertices > U16_VERTICES { 4 } else { 2 };
        self.words.len() * 4 + (vertices + self.quad_count() * 6 * index_bytes) as usize
    }

    fn stride(&self) -> u32 {
        self.words[2]
    }

    /// Where the records start, past the header and the table.
    pub fn records_at(&self) -> u32 {
        HEADER_WORDS + self.words[1]
    }

    /// The layer's quads, read back from its words.
    pub fn quads(&self) -> impl Iterator<Item = Quad> + '_ {
        let table = &self.words[HEADER_WORDS as usize..self.records_at() as usize];
        self.words[self.records_at() as usize..]
            .chunks(self.stride() as usize)
            .map(move |record| {
                let geometry = [record[0], record[1] & ((1 << PAINT_SHIFT) - 1)];
                let word = match record {
                    [_, hi] => table[(hi >> PAINT_SHIFT) as usize],
                    _ => record[2],
                };
                Quad {
                    geometry,
                    material: word & ((1 << LIGHT_SHIFT) - 1),
                    light: Light::from_packed((word >> LIGHT_SHIFT) as u8),
                }
            })
    }

    /// The records in `order`, for the blended sort to write over its own.
    pub fn records_in(&self, order: &[u32]) -> Vec<u32> {
        let (at, stride) = (self.records_at() as usize, self.stride() as usize);
        order
            .iter()
            .flat_map(|&q| &self.words[at + q as usize * stride..][..stride])
            .copied()
            .collect()
    }
}

/// A Material and the light on it as one table word.
pub fn paint(material: u32, light: Light) -> u32 {
    debug_assert!(material < 1 << LIGHT_SHIFT);
    material | u32::from(light.packed()) << LIGHT_SHIFT
}

/// The mesh a layer of `quads` quads draws: render-world only, since nothing
/// on the main world reads it.
pub fn placeholder(quads: u32) -> Mesh {
    let vertices = quads * VERTICES_PER_QUAD;
    let indices = (0..quads).flat_map(|q| QUAD_CORNERS.map(|c| q * VERTICES_PER_QUAD + c as u32));
    let indices = if vertices > U16_VERTICES {
        Indices::U32(indices.collect())
    } else {
        Indices::U16(indices.map(|i| i as u16).collect())
    };
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(
        ATTRIBUTE_PLACEHOLDER,
        VertexAttributeValues::Uint8(vec![0; vertices as usize]),
    )
    .with_inserted_indices(indices)
}

/// How many vertices two-byte indices reach.
const U16_VERTICES: u32 = 1 << 16;

/// What `chunk_vertex.wesl` makes of one vertex.
#[cfg(test)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Expanded {
    pub corner: Corner,
    pub material: u32,
    pub light: u32,
    pub level: u32,
}

/// `chunk_vertex.wesl`'s `expand`, step for step, for the test that holds the
/// two equal: `buffer` is the quad buffer, `at` the layer's tag and `vertex`
/// the mesh-local vertex index.
#[cfg(test)]
pub fn expand(buffer: &[u32], at: u32, vertex: u32) -> Expanded {
    use tesserae_core::mesh::{KIND_CROSS, PLANE_BIAS};
    let word = |i: u32| buffer[i as usize];
    let level = word(at);
    let table = at + HEADER_WORDS;
    let quad = vertex / VERTICES_PER_QUAD;
    let corner = vertex % VERTICES_PER_QUAD;
    let record = table + word(at + 1) + quad * word(at + 2);
    let (lo, hi) = (word(record), word(record + 1));
    let paint = if word(at + 2) == WIDE {
        word(record + 2)
    } else {
        word(table + (hi >> PAINT_SHIFT))
    };
    let kind = lo & 7;
    let mut position = [0.0f32; 3];
    let mut normal = [0.0f32; 3];
    let uv;
    if kind < KIND_CROSS {
        let positive = kind & 1 == 1;
        let d = (kind / 2) as usize;
        let (u, v) = ((d + 1) % 3, (d + 2) % 3);
        let plane = ((lo >> 3) & 1023) as i32 - PLANE_BIAS;
        let u0 = (lo >> 13) & 511;
        let v0 = (lo >> 22) & 511;
        let u1 = u0 + (hi & 511) + 1;
        let v1 = v0 + ((hi >> 9) & 511) + 1;
        let (high_u, high_v) = if positive {
            (corner == 1 || corner == 2, corner >= 2)
        } else {
            (corner >= 2, corner == 1 || corner == 2)
        };
        let cu = if high_u { u1 } else { u0 };
        let cv = if high_v { v1 } else { v0 };
        position[d] = plane as f32 / 16.0;
        position[u] = cu as f32 / 16.0;
        position[v] = cv as f32 / 16.0;
        normal[d] = if positive { 1.0 } else { -1.0 };
        uv = [
            (cu - u0 / 16 * 16) as f32 / 16.0,
            (cv - v0 / 16 * 16) as f32 / 16.0,
        ];
    } else {
        let c = lo >> 3;
        let cell = [c & 31, (c >> 5) & 31, (c >> 10) & 31];
        let reversed = (lo >> 20) & 1 == 1;
        let i = if reversed { 3 - corner } else { corner };
        let (high_u, high_v) = (i == 1 || i == 2, i >= 2);
        uv = [f32::from(u8::from(high_u)), f32::from(u8::from(high_v))];
        let side = if reversed { -1.0 } else { 1.0 };
        if kind == KIND_CROSS {
            let rising = (lo >> 18) & 1 == 0;
            let x = u32::from(if rising { high_u } else { !high_u });
            position = [
                (cell[0] + x) as f32,
                (cell[1] + u32::from(high_v)) as f32,
                (cell[2] + u32::from(high_u)) as f32,
            ];
            normal[2] = side;
        } else {
            let d = ((lo >> 18) & 3) as usize;
            let (u, v) = ((d + 1) % 3, (d + 2) % 3);
            let offset = (lo >> 21) & 31;
            let cu = if high_u {
                ((hi >> 4) & 15) + 1
            } else {
                hi & 15
            };
            let cv = if high_v {
                ((hi >> 12) & 15) + 1
            } else {
                (hi >> 8) & 15
            };
            position[d] = (cell[d] * 16 + offset) as f32 / 16.0;
            position[u] = (cell[u] * 16 + cu) as f32 / 16.0;
            position[v] = (cell[v] * 16 + cv) as f32 / 16.0;
            normal[d] = side;
        }
    }
    Expanded {
        corner: Corner {
            position,
            normal,
            uv,
        },
        material: paint & ((1 << LIGHT_SHIFT) - 1),
        light: paint >> LIGHT_SHIFT,
        level,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tesserae_core::light::MAX_LIGHT;
    use tesserae_core::shape::examples;
    use tesserae_core::testgen::fixtures;
    use tesserae_core::{
        Burning, CHUNK_SIZE, Chunk, ChunkLight, ChunkOverlay, Fluid, Form, Level, LitChunk,
        Neighborhood, NodeBox, Orientation, Overlay, ShapeDef, Tables, Voxel, greedy_mesh,
    };

    /// Both channels non-zero, so a nibble lost on the way shows.
    static LIT: ChunkLight = ChunkLight::filled(Light::new(MAX_LIGHT, 7));

    /// The builtin Shapes, then a fence and a pipe, which connect, a Cross and
    /// two Plane forms, one of them of odd thickness.
    fn tables() -> Tables {
        let mut tables = fixtures().tables.clone();
        tables.shapes.register(examples::fence()).unwrap();
        tables.shapes.register(examples::pipe()).unwrap();
        for (form, b) in [
            (Form::Cross, NodeBox::new([4, 0, 4], [12, 16, 12])),
            (Form::Plane, NodeBox::new([1, 1, 0], [15, 15, 2])),
            (Form::Plane, NodeBox::new([0, 3, 2], [16, 4, 14])),
        ] {
            tables
                .shapes
                .register(ShapeDef {
                    form,
                    connect: None,
                    boxes: vec![b],
                    collision: None,
                })
                .unwrap();
        }
        tables
    }

    fn wet(fluid: Fluid) -> Overlay {
        Overlay {
            fluid: Some(fluid),
            burning: None,
        }
    }

    /// A floor the full width of the Chunk, fluid standing at every surface
    /// level, every Shape in every orientation soaked in a source and in flowing
    /// fluid at every level, and flames off both far corners.
    fn every_kind_of_face(tables: &Tables) -> tesserae_core::ChunkMesh {
        let (rock, water) = (fixtures().ground.rock, fixtures().water);
        let mut chunk = Chunk::air();
        let mut overlay = ChunkOverlay::none();
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                chunk.set(x, 0, z, Voxel::cube(rock));
            }
        }
        for surface in 1..=Fluid::FULL_SURFACE {
            let fluid = Fluid {
                source: false,
                surface,
                reach: 1,
                ..Fluid::source(water)
            };
            overlay.set(2 * surface as usize - 1, 2, 1, wet(fluid));
        }
        let mut cell = 0;
        for shape in 1..tables.shapes.len() as u8 {
            for orientation in Orientation::all() {
                for surface in 1..=Fluid::FULL_SURFACE {
                    let at = [
                        1 + 2 * (cell % 15),
                        4 + 2 * (cell / 225),
                        1 + 2 * (cell / 15 % 15),
                    ];
                    cell += 1;
                    let voxel = Voxel::shaped(rock, shape, orientation);
                    chunk.set(at[0], at[1], at[2], voxel);
                    if tables.shapes.form_of(voxel).is_flat() {
                        continue;
                    }
                    let level = Level {
                        surface,
                        reach: 1,
                        descent: 0,
                    };
                    let fluid = if surface == Fluid::FULL_SURFACE {
                        Fluid::source(water)
                    } else {
                        Fluid::flowing(water, [level; 4]).unwrap()
                    };
                    overlay.set(at[0], at[1], at[2], wet(fluid));
                }
            }
        }
        for at in [0, CHUNK_SIZE - 1] {
            chunk.set(at, CHUNK_SIZE - 1, at, Voxel::cube(rock));
            overlay.set(
                at,
                CHUNK_SIZE - 1,
                at,
                Overlay {
                    fluid: None,
                    burning: Some(Burning { fuel: 1 }),
                },
            );
        }
        let lit = LitChunk::new(&chunk, &LIT).with_overlay(&overlay);
        greedy_mesh(&Neighborhood::isolated(lit), tables)
    }

    /// Packs `layer` at an offset into a buffer of other words, as the quad
    /// buffer would hold it, and expands every vertex as the shader does.
    fn assert_expands_exactly(layer: &MeshLayer, what: &str) {
        let level = 3;
        let packed = ChunkQuads::pack(layer, level).expect("a layer with quads");
        let at = 5;
        let mut buffer = vec![u32::MAX; at as usize];
        buffer.extend_from_slice(packed.words());
        buffer.extend([u32::MAX; 4]);
        let drawn = placeholder(packed.quad_count());
        let indices: Vec<usize> = drawn
            .indices()
            .expect("an indexed placeholder")
            .iter()
            .collect();
        assert_eq!(indices.len(), layer.quads.len() * 6, "{what}");
        for ((q, quad), triangles) in layer.quads.iter().enumerate().zip(indices.chunks(6)) {
            let corners = quad.corners();
            for (k, &corner) in tesserae_core::mesh::QUAD_CORNERS.iter().enumerate() {
                let expanded = expand(&buffer, at, triangles[k] as u32);
                assert_eq!(
                    expanded.corner, corners[corner],
                    "{what}: quad {q} vertex {k}"
                );
                assert_eq!(expanded.material, quad.material, "{what}: quad {q}");
                assert_eq!(expanded.light, u32::from(quad.light.packed()), "{what}");
                assert_eq!(expanded.level, level);
            }
        }
        assert_eq!(packed.quads().collect::<Vec<_>>(), layer.quads, "{what}");
    }

    #[test]
    fn every_shape_and_fluid_level_expands_to_the_meshers_own_corners() {
        let tables = tables();
        for shape in 0..tables.shapes.len() as u8 {
            for (orientation, quads) in tables.shapes.preview_geometry(shape).iter().enumerate() {
                let layer = tesserae_core::mesh::mesh_shape(quads, fixtures().ground.rock);
                if !layer.is_empty() {
                    assert_expands_exactly(&layer, &format!("Shape {shape} turned {orientation}"));
                }
            }
        }

        let mesh = every_kind_of_face(&tables);
        for (layer, what) in [
            (&mesh.opaque, "opaque"),
            (&mesh.blended, "blended"),
            (&mesh.cutout, "cutout"),
        ] {
            assert_expands_exactly(layer, what);
        }

        // What the scene is for: the extremes the packing has to hold.
        let corners = || {
            [&mesh.opaque, &mesh.blended, &mesh.cutout]
                .into_iter()
                .flat_map(|l| l.quads.iter().flat_map(Quad::corners))
        };
        let coordinates = || corners().flat_map(|c| c.position);
        assert_eq!(
            coordinates().fold(0.0, f32::min),
            -1.0 / 16.0,
            "a flame below"
        );
        assert_eq!(
            coordinates().fold(0.0, f32::max),
            32.0 + 1.0 / 16.0,
            "and one above"
        );
        let kinds: Vec<_> = corners().map(|c| c.normal).collect();
        assert!(kinds.contains(&[0.0, 0.0, -1.0]), "the back of a Cross");
        assert!(
            mesh.cutout
                .quads
                .iter()
                .any(|q| q.material == tesserae_core::mesh::FLAME)
        );
        let widest = corners().flat_map(|c| c.uv).fold(0.0, f32::max);
        assert_eq!(widest, 32.0, "the floor's UVs span the Chunk");
        for surface in 1..=Fluid::FULL_SURFACE {
            let height = 2.0 + f32::from(surface) / 16.0;
            assert!(
                mesh.blended
                    .quads
                    .iter()
                    .flat_map(Quad::corners)
                    .any(|c| c.position[1] == height),
                "no fluid surface at {height}"
            );
        }
    }

    /// More pairs than a table index reaches: every quad carries its own.
    #[test]
    fn a_layer_with_more_pairs_than_a_table_holds_still_expands_exactly() {
        let quads: Vec<Quad> = (0..PAINT_SLOTS as u32 + 3)
            .map(|i| {
                let light = Light::from_packed(i as u8);
                Quad::face(3, 16, [0, 16], [0, 16], i >> 8, light)
            })
            .collect();
        let layer = MeshLayer { quads };
        let packed = ChunkQuads::pack(&layer, 0).unwrap();
        assert_eq!(packed.stride(), WIDE);
        assert_expands_exactly(&layer, "wide");
    }

    #[test]
    fn the_aabb_is_exactly_the_bounds_of_every_corner() {
        let mesh = every_kind_of_face(&tables());
        let packed = ChunkQuads::pack(&mesh.cutout, 0).unwrap();
        let corners = mesh.cutout.quads.iter().flat_map(Quad::corners);
        let lo = corners
            .clone()
            .fold(Vec3::MAX, |a, c| a.min(Vec3::from(c.position)));
        let hi = corners.fold(Vec3::MIN, |a, c| a.max(Vec3::from(c.position)));
        assert_eq!(Vec3::from(packed.aabb().min()), lo);
        assert_eq!(Vec3::from(packed.aabb().max()), hi);
        assert_eq!(lo.x, -1.0 / 16.0, "a flame outside the Chunk");
        assert!(ChunkQuads::pack(&MeshLayer::default(), 0).is_none());
    }

    #[test]
    fn the_placeholder_has_four_vertices_a_quad_and_two_triangles_over_them() {
        let mesh = placeholder(7);
        assert_eq!(mesh.count_vertices(), 28);
        let Some(Indices::U16(indices)) = mesh.indices() else {
            panic!("two-byte indices while they reach");
        };
        assert_eq!(indices[..12], [0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);
        assert_eq!(indices.len(), 42);
        assert_eq!(indices[41], 27);
    }

    #[test]
    fn a_placeholder_past_two_byte_indices_takes_four() {
        let quads = (1 << 14) + 1;
        let mesh = placeholder(quads);
        let Some(Indices::U32(indices)) = mesh.indices() else {
            panic!("four-byte indices past 65,536 vertices");
        };
        assert_eq!(indices.last(), Some(&(quads * 4 - 1)));
        assert!(matches!(
            placeholder(1 << 14).indices(),
            Some(Indices::U16(_))
        ));
    }
}

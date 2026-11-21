//! A Chunk layer's quads as the quad buffer holds them: a header, a table of
//! the layer's (Material, light) pairs, then one record per quad, which
//! `chunk_vertex.wesl` expands from the vertex index. See
//! `docs/adr/0034-a-chunk-mesh-is-one-record-per-quad.md`.
//!
//! This conversion lives in `tesserae` and never in `tesserae-core`, which must
//! acquire no `bevy` or `wgpu` dependency even transitively. See
//! `docs/architecture.md` invariant 3.

use bevy::asset::RenderAssetUsages;
use bevy::camera::primitives::Aabb;
use bevy::mesh::MeshTag;
use bevy::mesh::{
    Indices, MeshVertexAttribute, PrimitiveTopology, VertexAttributeValues, VertexFormat,
};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use tesserae_core::MeshLayer;
use tesserae_core::light::Light;
#[cfg(test)]
use tesserae_core::mesh::Corner;
use tesserae_core::mesh::layout::{GEOMETRY_MASK, PAINT_SHIFT, PAINT_SLOTS, mask};
use tesserae_core::mesh::{QUAD_CORNERS, Quad};

use crate::render::quad_buffer::QuadBuffer;

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
const MATERIAL_MASK: u32 = mask(LIGHT_SHIFT);

/// The header's words: the LOD level, the table's length, the words a record
/// takes and the layer's quads, past which its bucket's placeholder only pads.
pub const HEADER_WORDS: u32 = 4;
pub const HEADER_LEVEL: u32 = 0;
pub const HEADER_TABLE_LEN: u32 = 1;
pub const HEADER_STRIDE: u32 = 2;
pub const HEADER_QUADS: u32 = 3;

/// The geometry, with the table index in the second word's top bits.
const NARROW: u32 = 2;
/// The geometry and the table word itself, for a layer with more pairs than a
/// table index reaches.
pub const WIDE: u32 = 3;

/// One Chunk layer's words, ready for the quad buffer. The Client keeps it
/// beside the entity drawing it, as collision and the blended sort read it.
#[derive(Component, Clone, Debug)]
pub struct ChunkQuads {
    words: Vec<u32>,
    aabb: Aabb,
    /// Where `words` starts in the quad buffer, which `MeshTag` carries to the
    /// shader. Set as it lands.
    at: u32,
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
        let mut header = [0; HEADER_WORDS as usize];
        header[HEADER_LEVEL as usize] = level;
        header[HEADER_STRIDE as usize] = stride;
        header[HEADER_QUADS as usize] = layer.quad_count() as u32;
        words.extend(header);
        if stride == NARROW {
            words[HEADER_TABLE_LEN as usize] = table.len() as u32;
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

    #[cfg(test)]
    pub fn words(&self) -> &[u32] {
        &self.words
    }

    #[cfg(any(test, feature = "dev", feature = "profiling"))]
    pub fn level(&self) -> u32 {
        self.words[HEADER_LEVEL as usize]
    }

    pub fn quad_count(&self) -> u32 {
        self.words[HEADER_QUADS as usize]
    }

    /// The bounds of every corner, in Voxels: the placeholder has no positions
    /// Bevy could find them from.
    pub fn aabb(&self) -> Aabb {
        self.aabb
    }

    /// What the GPU holds for the layer alone: its placeholder is its
    /// bucket's, shared.
    #[cfg(any(feature = "dev", feature = "profiling"))]
    pub fn gpu_bytes(&self) -> usize {
        self.words.len() * 4
    }

    fn stride(&self) -> u32 {
        self.words[HEADER_STRIDE as usize]
    }

    /// Takes a range of `buffer` and queues the layer's words into it: the tag
    /// the shader finds them by.
    pub fn land(&mut self, buffer: &mut QuadBuffer) -> MeshTag {
        self.at = buffer.alloc(self.words.len() as u32);
        buffer.write(self.at, &self.words);
        MeshTag::new(self.at)
    }

    /// Frees the range the layer landed in.
    pub fn give_back(&self, buffer: &mut QuadBuffer) {
        buffer.release(self.at, self.words.len() as u32);
    }

    /// Rewrites the layer's records in its range in `order`.
    pub fn write_records(&self, buffer: &mut QuadBuffer, order: &[u32]) {
        let (at, stride) = (self.records_at() as usize, self.stride() as usize);
        let records: Vec<u32> = order
            .iter()
            .flat_map(|&q| &self.words[at + q as usize * stride..][..stride])
            .copied()
            .collect();
        buffer.write(self.at + self.records_at(), &records);
    }

    /// Where the records start, past the header and the table.
    fn records_at(&self) -> u32 {
        HEADER_WORDS + self.words[HEADER_TABLE_LEN as usize]
    }

    /// The layer's quads, read back from its words.
    pub fn quads(&self) -> impl Iterator<Item = Quad> + '_ {
        let table = &self.words[HEADER_WORDS as usize..self.records_at() as usize];
        self.words[self.records_at() as usize..]
            .chunks(self.stride() as usize)
            .map(move |record| {
                let geometry = [record[0], record[1] & GEOMETRY_MASK];
                let word = match record {
                    [_, hi] => table[(hi >> PAINT_SHIFT) as usize],
                    _ => record[2],
                };
                Quad {
                    geometry,
                    material: word & MATERIAL_MASK,
                    light: Light::from_packed((word >> LIGHT_SHIFT) as u8),
                }
            })
    }
}

/// A Material and the light on it as one table word.
pub fn paint(material: u32, light: Light) -> u32 {
    debug_assert!(material <= MATERIAL_MASK);
    material | u32::from(light.packed()) << LIGHT_SHIFT
}

/// One placeholder per bucket, shared by every layer whose quads it holds: on
/// Metal each mesh is its own indirect draw in every pass and cascade, so
/// opaque and cutout layers in one bucket are one draw. Blended layers are
/// sorted and drawn one by one, and share only the memory.
#[derive(Resource, Default)]
pub struct Placeholders(HashMap<u32, Handle<Mesh>>);

impl Placeholders {
    /// The placeholder a layer of `quads` quads draws, built the first time
    /// its bucket is asked for and kept for the session.
    pub fn of(&mut self, meshes: &mut Assets<Mesh>, quads: u32) -> Handle<Mesh> {
        self.0
            .entry(bucket(quads))
            .or_insert_with_key(|&size| meshes.add(placeholder(size)))
            .clone()
    }

    /// What the GPU holds for every bucket built.
    #[cfg(any(feature = "dev", feature = "profiling"))]
    pub fn gpu_bytes(&self) -> usize {
        self.0
            .keys()
            .map(|&quads| {
                let index_bytes = if wide_indices(quads) { 4 } else { 2 };
                (quads * VERTICES_PER_QUAD + quads * 6 * index_bytes) as usize
            })
            .sum()
    }
}

/// Each bucket holds about 2^(1/4) times the quads of the one below: about 9%
/// of a layer's vertex work is padding on average.
const BUCKET_GROWTH: f64 = 1.189_207_115_002_721;

/// The quads of the smallest bucket that holds a layer of `quads` quads. The
/// last bucket of two-byte indices is exactly where they run out.
pub fn bucket(quads: u32) -> u32 {
    let mut size = 1;
    while size < quads {
        let next = (size + 1).max((f64::from(size) * BUCKET_GROWTH).ceil() as u32);
        size = if size < U16_QUADS {
            next.min(U16_QUADS)
        } else {
            next
        };
    }
    size
}

/// The mesh a bucket of `quads` quads draws: render-world only, since nothing
/// on the main world reads it.
pub fn placeholder(quads: u32) -> Mesh {
    let vertices = quads * VERTICES_PER_QUAD;
    let indices = (0..quads).flat_map(|q| QUAD_CORNERS.map(|c| q * VERTICES_PER_QUAD + c as u32));
    let indices = if wide_indices(quads) {
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
const U16_QUADS: u32 = U16_VERTICES / VERTICES_PER_QUAD;

fn wide_indices(quads: u32) -> bool {
    quads * VERTICES_PER_QUAD > U16_VERTICES
}

/// What `chunk_vertex.wesl` makes of one vertex.
#[cfg(test)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Expanded {
    pub corner: Corner,
    pub material: u32,
    pub light: u32,
    pub level: u32,
}

/// `chunk_vertex.wesl`'s `transform_vertex` and `expand`, step for step, for
/// the test that holds the two equal: `buffer` is the quad buffer, `at` the
/// layer's tag and `vertex` the mesh-local vertex index. `None` for the
/// bucket's padding, which the shader degenerates.
#[cfg(test)]
pub fn expand(buffer: &[u32], at: u32, vertex: u32) -> Option<Expanded> {
    use tesserae_core::mesh::layout::*;
    use tesserae_core::shape::SUBDIV;
    let sub = SUBDIV as u32;
    let word = |i: u32| buffer[i as usize];
    let field = |word: u32, shift: u32, bits: u32| (word >> shift) & mask(bits);
    let level = word(at + HEADER_LEVEL);
    let stride = word(at + HEADER_STRIDE);
    let table = at + HEADER_WORDS;
    let quad = vertex / VERTICES_PER_QUAD;
    if quad >= word(at + HEADER_QUADS) {
        return None;
    }
    let corner = vertex % VERTICES_PER_QUAD;
    let record = table + word(at + HEADER_TABLE_LEN) + quad * stride;
    let (lo, hi) = (word(record), word(record + 1));
    let paint = if stride == WIDE {
        word(record + 2)
    } else {
        word(table + (hi >> PAINT_SHIFT))
    };
    let kind = field(lo, 0, KIND_BITS);
    let mut position = [0.0f32; 3];
    let mut normal = [0.0f32; 3];
    let uv;
    if kind < FACES {
        let positive = kind & 1 == 1;
        let d = (kind / 2) as usize;
        let (u, v) = ((d + 1) % 3, (d + 2) % 3);
        let plane = field(lo, PLANE_SHIFT, PLANE_BITS) as i32 - PLANE_BIAS;
        let u0 = field(lo, U0_SHIFT, CORNER_BITS);
        let v0 = field(lo, V0_SHIFT, CORNER_BITS);
        let u1 = u0 + field(hi, 0, CORNER_BITS) + 1;
        let v1 = v0 + field(hi, DV_SHIFT, CORNER_BITS) + 1;
        let (high_u, high_v) = if positive {
            (corner == 1 || corner == 2, corner >= 2)
        } else {
            (corner >= 2, corner == 1 || corner == 2)
        };
        let cu = if high_u { u1 } else { u0 };
        let cv = if high_v { v1 } else { v0 };
        position[d] = plane as f32 / sub as f32;
        position[u] = cu as f32 / sub as f32;
        position[v] = cv as f32 / sub as f32;
        normal[d] = if positive { 1.0 } else { -1.0 };
        uv = [
            (cu - u0 / sub * sub) as f32 / sub as f32,
            (cv - v0 / sub * sub) as f32 / sub as f32,
        ];
    } else {
        let cell = [0, 1, 2].map(|a| field(lo, CELL_SHIFT + a * CELL_BITS, CELL_BITS));
        let reversed = field(lo, REVERSED_SHIFT, 1) == 1;
        let i = if reversed { 3 - corner } else { corner };
        let (high_u, high_v) = (i == 1 || i == 2, i >= 2);
        uv = [f32::from(u8::from(high_u)), f32::from(u8::from(high_v))];
        let side = if reversed { -1.0 } else { 1.0 };
        if kind == KIND_CROSS {
            let rising = field(lo, DIAGONAL_SHIFT, 1) == 0;
            let x = u32::from(if rising { high_u } else { !high_u });
            position = [
                (cell[0] + x) as f32,
                (cell[1] + u32::from(high_v)) as f32,
                (cell[2] + u32::from(high_u)) as f32,
            ];
            normal[2] = side;
        } else {
            let d = field(lo, AXIS_SHIFT, AXIS_BITS) as usize;
            let (u, v) = ((d + 1) % 3, (d + 2) % 3);
            let offset = field(lo, OFFSET_SHIFT, OFFSET_BITS);
            let span = |k: u32| field(hi, k * SPAN_BITS, SPAN_BITS);
            let cu = if high_u { span(1) + 1 } else { span(0) };
            let cv = if high_v { span(3) + 1 } else { span(2) };
            position[d] = (cell[d] * sub + offset) as f32 / sub as f32;
            position[u] = (cell[u] * sub + cu) as f32 / sub as f32;
            position[v] = (cell[v] * sub + cv) as f32 / sub as f32;
            normal[d] = side;
        }
    }
    Some(Expanded {
        corner: Corner {
            position,
            normal,
            uv,
        },
        material: paint & MATERIAL_MASK,
        light: paint >> LIGHT_SHIFT,
        level,
    })
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
        // The next bucket up, so the placeholder always has padding to draw.
        let drawn = placeholder(bucket(packed.quad_count() + 1));
        let indices: Vec<usize> = drawn
            .indices()
            .expect("an indexed placeholder")
            .iter()
            .collect();
        let (records, padding) = indices.split_at(layer.quads.len() * 6);
        assert!(!padding.is_empty(), "{what}");
        for &vertex in padding {
            assert_eq!(expand(&buffer, at, vertex as u32), None, "{what}: padding");
        }
        for ((q, quad), triangles) in layer.quads.iter().enumerate().zip(records.chunks(6)) {
            let corners = quad.corners();
            for (k, &corner) in tesserae_core::mesh::QUAD_CORNERS.iter().enumerate() {
                let expanded = expand(&buffer, at, triangles[k] as u32).expect("a record's vertex");
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
    fn a_layer_draws_the_smallest_bucket_that_holds_it() {
        for (quads, drawn) in [(1, 1), (5, 5), (6, 6), (7, 8), (8, 8), (9, 10), (13, 15)] {
            assert_eq!(bucket(quads), drawn, "{quads} quads");
        }
        assert_eq!(
            bucket(1 << 14),
            1 << 14,
            "the last bucket of two-byte indices"
        );
        assert!(bucket((1 << 14) + 1) > 1 << 14);
        let mut buckets = 0;
        let mut last = 0;
        for quads in 1..=100_000 {
            let drawn = bucket(quads);
            assert!(drawn >= quads);
            assert!(
                (drawn - quads) as f64 <= quads as f64 * 0.19 + 1.0,
                "{quads} quads padded to {drawn}"
            );
            if drawn != last {
                buckets += 1;
                last = drawn;
            }
        }
        assert!(buckets < 70, "{buckets} buckets up to 100,000 quads");
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

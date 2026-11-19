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
use tesserae_core::mesh::{PAINT_SHIFT, PAINT_SLOTS, QUAD_CORNERS, Quad, QuadKind};

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

/// A face direction with at least this many quads in a layer gets a draw of
/// its own, hidden while the camera is behind every one of its planes. Fewer
/// are drawn with the layer's other quads: on Metal each draw is a draw call of
/// its own, and a small one saves less vertex work than it costs.
pub const OWN_DRAW_QUADS: u32 = 128;

/// How a layer's quads are drawn.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Split {
    /// Each face direction with [`OWN_DRAW_QUADS`] or more on its own, the
    /// rest together. Only for a layer whose back faces are culled.
    ByFace,
    /// All in one draw, in the mesher's order, which the blended sort indexes.
    Whole,
}

/// The face kind, in `FACE_STEPS` order, every quad of a draw has.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Facing(pub u8);

impl Facing {
    pub fn normal(self) -> Vec3 {
        let mut normal = Vec3::ZERO;
        normal[usize::from(self.0 / 2)] = if self.0 & 1 == 1 { 1.0 } else { -1.0 };
        normal
    }

    /// Whether a camera at `eye` sees none of the draw's quads, which lie in
    /// `aabb` under `transform`: it is behind every one of their planes. The
    /// margin keeps rounding from hiding a face the GPU would still draw.
    pub fn hidden_from(self, eye: Vec3, aabb: &Aabb, transform: &GlobalTransform) -> bool {
        const MARGIN_M: f32 = 1e-3;
        let axis = usize::from(self.0 / 2);
        if self.0 & 1 == 1 {
            eye[axis] < transform.transform_point(aabb.min().into())[axis] - MARGIN_M
        } else {
            eye[axis] > transform.transform_point(aabb.max().into())[axis] + MARGIN_M
        }
    }
}

/// One draw of a layer: `count` records from the `first`.
#[derive(Clone, Debug, PartialEq)]
pub struct LayerDraw {
    /// `None` for flat forms and directions too small to draw alone.
    pub facing: Option<Facing>,
    pub first: u32,
    pub count: u32,
    /// In Voxels: the placeholder has no positions Bevy could find it from.
    pub aabb: Aabb,
}

/// One Chunk layer's words, ready for the quad buffer. The Client keeps it
/// beside the entities drawing it, as collision and the blended sort read it.
#[derive(Component, Clone, Debug)]
pub struct ChunkQuads {
    words: Vec<u32>,
    draws: Vec<LayerDraw>,
    /// Where `words` starts in the quad buffer, which `MeshTag` carries to the
    /// shader. Set as it lands.
    pub at: u32,
}

/// The draw a flat form goes in, after the six face directions.
const MIXED: usize = 6;

fn direction_of(quad: &Quad) -> usize {
    match quad.kind() {
        QuadKind::Face(face) => face,
        QuadKind::Cross | QuadKind::Plane => MIXED,
    }
}

impl ChunkQuads {
    /// `None` for a layer with nothing to draw.
    pub fn pack(layer: &MeshLayer, level: u32, split: Split) -> Option<ChunkQuads> {
        if layer.is_empty() {
            return None;
        }
        let (order, draws) = draws(layer, split);
        let mut slots: HashMap<u32, u32> = HashMap::default();
        let mut table = Vec::new();
        let slot_of: Vec<u32> = order
            .iter()
            .map(|&q| {
                let quad = &layer.quads[q as usize];
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
        for (&q, &slot) in order.iter().zip(&slot_of) {
            let [lo, hi] = layer.quads[q as usize].geometry;
            if stride == NARROW {
                words.extend([lo, hi | slot << PAINT_SHIFT]);
            } else {
                words.extend([lo, hi, table[slot as usize]]);
            }
        }
        Some(ChunkQuads {
            words,
            draws,
            at: 0,
        })
    }

    pub fn draws(&self) -> &[LayerDraw] {
        &self.draws
    }

    pub fn words(&self) -> &[u32] {
        &self.words
    }

    #[cfg(any(test, feature = "dev", feature = "profiling"))]
    pub fn level(&self) -> u32 {
        self.words[0]
    }

    #[cfg(test)]
    pub fn quad_count(&self) -> u32 {
        (self.words.len() as u32 - self.records_at()) / self.stride()
    }

    /// What the GPU holds for the layer: its words and its placeholders.
    #[cfg(any(feature = "dev", feature = "profiling"))]
    pub fn gpu_bytes(&self) -> usize {
        let placeholders: u32 = self
            .draws
            .iter()
            .map(|draw| {
                let wide = (draw.first + draw.count) * VERTICES_PER_QUAD > U16_VERTICES;
                draw.count * (VERTICES_PER_QUAD + 6 * if wide { 4 } else { 2 })
            })
            .sum();
        self.words.len() * 4 + placeholders as usize
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

/// The order a layer's quads are stored in and the draws over it: the quads
/// no direction draws alone first, in the mesher's order, then each
/// direction's.
fn draws(layer: &MeshLayer, split: Split) -> (Vec<u32>, Vec<LayerDraw>) {
    let directions: Vec<usize> = layer.quads.iter().map(direction_of).collect();
    let mut counts = [0u32; MIXED + 1];
    for &d in &directions {
        counts[d] += 1;
    }
    let alone = |d: usize| split == Split::ByFace && d < MIXED && counts[d] >= OWN_DRAW_QUADS;
    let groups = std::iter::once(None).chain((0..MIXED).filter(|&d| alone(d)).map(Some));
    let mut order = Vec::with_capacity(layer.quads.len());
    let mut draws = Vec::new();
    for group in groups {
        let first = order.len();
        order.extend((0..layer.quads.len() as u32).filter(|&q| {
            let d = directions[q as usize];
            group.map_or(!alone(d), |g| g == d)
        }));
        if order.len() == first {
            continue;
        }
        let (min, max) = order[first..]
            .iter()
            .map(|&q| layer.quads[q as usize].bounds())
            .fold((Vec3::MAX, Vec3::MIN), |(lo, hi), (least, most)| {
                (lo.min(Vec3::from(least)), hi.max(Vec3::from(most)))
            });
        draws.push(LayerDraw {
            facing: group.map(|d| Facing(d as u8)),
            first: first as u32,
            count: (order.len() - first) as u32,
            aabb: Aabb::from_min_max(min, max),
        });
    }
    (order, draws)
}

/// A Material and the light on it as one table word.
pub fn paint(material: u32, light: Light) -> u32 {
    debug_assert!(material < 1 << LIGHT_SHIFT);
    material | u32::from(light.packed()) << LIGHT_SHIFT
}

/// The mesh one draw of a layer draws: render-world only, since nothing on
/// the main world reads it. Its indices name the draw's records from the
/// `first`, while it holds vertices for its own `count` only: no pipeline
/// reads a vertex, and wgpu bounds an indexed draw by its indices alone.
pub fn placeholder(first: u32, count: u32) -> Mesh {
    let vertices = count * VERTICES_PER_QUAD;
    let indices =
        (first..first + count).flat_map(|q| QUAD_CORNERS.map(|c| q * VERTICES_PER_QUAD + c as u32));
    let indices = if (first + count) * VERTICES_PER_QUAD > U16_VERTICES {
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

    fn sorted(quads: impl IntoIterator<Item = Quad>) -> Vec<Quad> {
        let mut quads: Vec<Quad> = quads.into_iter().collect();
        quads.sort_by_key(|q| (q.geometry, q.material, q.light.packed()));
        quads
    }

    /// Packs `layer` both ways at an offset into a buffer of other words, as
    /// the quad buffer would hold it, and expands every vertex of every draw
    /// as the shader does.
    fn assert_expands_exactly(layer: &MeshLayer, what: &str) {
        for split in [Split::ByFace, Split::Whole] {
            let what = format!("{what}, {split:?}");
            let level = 3;
            let packed = ChunkQuads::pack(layer, level, split).expect("a layer with quads");
            let at = 5;
            let mut buffer = vec![u32::MAX; at as usize];
            buffer.extend_from_slice(packed.words());
            buffer.extend([u32::MAX; 4]);
            let stored: Vec<Quad> = packed.quads().collect();
            let mut drawn = Vec::new();
            for draw in packed.draws() {
                assert_eq!(draw.first as usize, drawn.len(), "{what}: draws in order");
                let mesh = placeholder(draw.first, draw.count);
                assert_eq!(mesh.count_vertices() as u32, draw.count * VERTICES_PER_QUAD);
                let indices: Vec<usize> = mesh
                    .indices()
                    .expect("an indexed placeholder")
                    .iter()
                    .collect();
                assert_eq!(indices.len(), draw.count as usize * 6, "{what}");
                for triangles in indices.chunks(6) {
                    let quad = expand(&buffer, at, triangles[0] as u32);
                    let corners = [0, 1, 2, 5].map(|k| expand(&buffer, at, triangles[k] as u32));
                    let record = stored[triangles[0] / 4];
                    assert_eq!(
                        corners.map(|c| c.corner),
                        record.corners(),
                        "{what}: {record:?}"
                    );
                    assert!(corners.iter().all(|c| *c
                        == Expanded {
                            corner: c.corner,
                            ..quad
                        }));
                    assert_eq!(quad.material, record.material, "{what}");
                    assert_eq!(quad.light, u32::from(record.light.packed()), "{what}");
                    assert_eq!(quad.level, level);
                    if let Some(Facing(face)) = draw.facing {
                        assert_eq!(record.kind(), QuadKind::Face(usize::from(face)), "{what}");
                    }
                    drawn.push(record);
                }
            }
            match split {
                Split::Whole => assert_eq!(drawn, layer.quads, "{what}: the mesher's order"),
                Split::ByFace => assert_eq!(sorted(drawn), sorted(layer.quads.clone()), "{what}"),
            }
        }
    }

    #[test]
    fn each_well_filled_face_direction_is_drawn_alone_and_the_rest_together() {
        let light = Light::new(3, 1);
        let faces =
            |face, n: u16| (0..n).map(move |i| Quad::face(face, 16, [i, i + 1], [0, 16], 9, light));
        let up = 3;
        let west = 0;
        let quads: Vec<Quad> = faces(up, 200)
            .chain(faces(5, 5))
            .chain([Quad::cross([1, 2, 3], 0, false, 9, light)])
            .chain(faces(west, OWN_DRAW_QUADS as u16))
            .collect();
        let layer = MeshLayer { quads };
        let packed = ChunkQuads::pack(&layer, 0, Split::ByFace).unwrap();
        let draws: Vec<_> = packed.draws().iter().map(|d| (d.facing, d.count)).collect();
        assert_eq!(
            draws,
            [
                (None, 6),
                (Some(Facing(west as u8)), OWN_DRAW_QUADS),
                (Some(Facing(up as u8)), 200)
            ]
        );
        let up_draw = &packed.draws()[2];
        assert_eq!(Vec3::from(up_draw.aabb.min()), Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(
            Vec3::from(up_draw.aabb.max()),
            Vec3::new(1.0, 1.0, 200.0 / 16.0)
        );
        assert_expands_exactly(&layer, "split");

        let whole = ChunkQuads::pack(&layer, 0, Split::Whole).unwrap();
        let draws: Vec<_> = whole.draws().iter().map(|d| (d.facing, d.count)).collect();
        assert_eq!(draws, [(None, layer.quads.len() as u32)]);
    }

    #[test]
    fn a_direction_is_hidden_only_from_behind_all_of_its_planes() {
        let up = Facing(3);
        let aabb = Aabb::from_min_max(Vec3::new(0.0, 4.0, 0.0), Vec3::new(32.0, 9.0, 32.0));
        let chunk = GlobalTransform::from(
            Transform::from_xyz(-64.0, 32.0, 0.0).with_scale(Vec3::splat(2.0)),
        );
        // The lowest plane is at 32 + 4 * 2 metres.
        assert!(up.hidden_from(Vec3::new(0.0, 39.9, 0.0), &aabb, &chunk));
        assert!(!up.hidden_from(Vec3::new(0.0, 40.0, 0.0), &aabb, &chunk));
        assert!(!up.hidden_from(Vec3::new(500.0, 41.0, 0.0), &aabb, &chunk));
        let down = Facing(2);
        assert!(down.hidden_from(Vec3::new(0.0, 50.1, 0.0), &aabb, &chunk));
        assert!(!down.hidden_from(Vec3::new(0.0, 50.0, 0.0), &aabb, &chunk));
        assert!(!down.hidden_from(Vec3::new(0.0, 39.0, 0.0), &aabb, &chunk));
        assert_eq!(Facing(4).normal(), Vec3::NEG_Z);
        assert_eq!(Facing(1).normal(), Vec3::X);
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
        let packed = ChunkQuads::pack(&layer, 0, Split::ByFace).unwrap();
        assert_eq!(packed.stride(), WIDE);
        assert_expands_exactly(&layer, "wide");
    }

    #[test]
    fn each_draws_aabb_is_exactly_the_bounds_of_its_corners() {
        let mesh = every_kind_of_face(&tables());
        let light = Light::new(2, 2);
        let floor = (0..OWN_DRAW_QUADS as u16)
            .map(|i| Quad::face(3, i32::from(i), [0, 1], [0, 16], 4, light));
        let layer = MeshLayer {
            quads: mesh
                .opaque
                .quads
                .iter()
                .chain(&mesh.cutout.quads)
                .copied()
                .chain(floor)
                .collect(),
        };
        for layer in [&mesh.opaque, &mesh.cutout, &layer] {
            let packed = ChunkQuads::pack(layer, 0, Split::ByFace).unwrap();
            let stored: Vec<Quad> = packed.quads().collect();
            for draw in packed.draws() {
                let quads = &stored[draw.first as usize..][..draw.count as usize];
                let corners = quads.iter().flat_map(Quad::corners);
                let lo = corners
                    .clone()
                    .fold(Vec3::MAX, |a, c| a.min(Vec3::from(c.position)));
                let hi = corners.fold(Vec3::MIN, |a, c| a.max(Vec3::from(c.position)));
                assert_eq!(Vec3::from(draw.aabb.min()), lo);
                assert_eq!(Vec3::from(draw.aabb.max()), hi);
            }
        }
        let split = ChunkQuads::pack(&layer, 0, Split::ByFace).unwrap();
        let upward = split.draws().iter().filter(|d| d.facing == Some(Facing(3)));
        assert_eq!(upward.count(), 1, "the floor is drawn alone");
        let packed = ChunkQuads::pack(&mesh.cutout, 0, Split::Whole).unwrap();
        assert_eq!(
            packed.draws()[0].aabb.min().x,
            -1.0 / 16.0,
            "a flame outside"
        );
        assert!(ChunkQuads::pack(&MeshLayer::default(), 0, Split::Whole).is_none());
    }

    #[test]
    fn the_placeholder_has_four_vertices_a_quad_and_two_triangles_over_them() {
        let mesh = placeholder(0, 7);
        assert_eq!(mesh.count_vertices(), 28);
        let Some(Indices::U16(indices)) = mesh.indices() else {
            panic!("two-byte indices while they reach");
        };
        assert_eq!(indices[..12], [0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);
        assert_eq!(indices.len(), 42);
        assert_eq!(indices[41], 27);
    }

    #[test]
    fn a_later_draws_placeholder_indexes_its_own_records_with_its_own_vertices() {
        let mesh = placeholder(5, 2);
        assert_eq!(mesh.count_vertices(), 8);
        let Some(Indices::U16(indices)) = mesh.indices() else {
            panic!("two-byte indices while they reach");
        };
        assert_eq!(
            indices[..],
            [20, 21, 22, 20, 22, 23, 24, 25, 26, 24, 26, 27]
        );
    }

    #[test]
    fn a_placeholder_past_two_byte_indices_takes_four() {
        let quads = (1 << 14) + 1;
        let mesh = placeholder(0, quads);
        let Some(Indices::U32(indices)) = mesh.indices() else {
            panic!("four-byte indices past 65,536 vertices");
        };
        assert_eq!(indices.last(), Some(&(quads * 4 - 1)));
        assert!(matches!(
            placeholder(0, 1 << 14).indices(),
            Some(Indices::U16(_))
        ));
        let late = placeholder((1 << 14) - 1, 2);
        assert!(
            matches!(late.indices(), Some(Indices::U32(_))),
            "by its last index"
        );
    }
}

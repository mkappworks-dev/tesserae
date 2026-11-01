//! Level of detail: producing a coarse Chunk from the eight finer ones it covers.

use crate::chunk::{CHUNK_SIZE, Chunk};
use crate::occupants::{Burning, ChunkOverlay, Fluid, Level, Occupants, Overlay};
use crate::shape::{Connections, SUBDIV, ShapeTable};
use crate::tables::Tables;
use crate::voxel::{MaterialId, Voxel};

use crate::light::{ChunkLight, Light, MAX_LIGHT};
use crate::material::{MaterialTable, RenderClass};
use crate::mesh::{
    FACE_STEPS, NEG_X, NEG_Y, NEG_Z, POS_X, POS_Y, SEAM_EVERY, SEAM_LAYERS, SeamOpenings,
};
use crate::worldgen::Worldgen;

const OCTANT: usize = CHUNK_SIZE / 2;

const CUBE_WEIGHT: u32 = (SUBDIV * SUBDIV * SUBDIV) as u32;

/// Half of eight cubes; a tie is solid.
const SOLID_AT: u32 = 8 * CUBE_WEIGHT / 2;

/// Up to `N` Materials by weight: eight for packed children, sixteen where a
/// Voxel can add a fluid beside its solid.
struct Tally<const N: usize> {
    total: u32,
    len: usize,
    materials: [MaterialId; N],
    weights: [u32; N],
    /// The largest `dy` of any child of this Material that has weight.
    tops: [u8; N],
}

impl<const N: usize> Tally<N> {
    #[inline]
    fn new() -> Self {
        Tally {
            total: 0,
            len: 0,
            materials: [0; N],
            weights: [0; N],
            tops: [0; N],
        }
    }

    #[inline]
    fn add(&mut self, material: MaterialId, weight: u32, dy: u8) {
        if weight == 0 {
            return;
        }
        self.total += weight;
        match self.materials[..self.len]
            .iter()
            .position(|&m| m == material)
        {
            Some(i) => {
                self.weights[i] += weight;
                self.tops[i] = self.tops[i].max(dy);
            }
            None => {
                self.materials[self.len] = material;
                self.weights[self.len] = weight;
                self.tops[self.len] = dy;
                self.len += 1;
            }
        }
    }

    /// The Material with the most volume of those `keep` accepts.
    #[inline]
    fn best(&self, keep: impl Fn(MaterialId) -> bool) -> Option<usize> {
        // Height keeps a one-Voxel grass cap over dirt (a 4-4 tie at every odd surface
        // height); id then makes the answer independent of palette and iteration order.
        (0..self.len)
            .filter(|&i| keep(self.materials[i]))
            .max_by_key(|&i| {
                (
                    self.weights[i],
                    self.tops[i],
                    std::cmp::Reverse(self.materials[i]),
                )
            })
    }

    #[inline]
    fn weight_of(&self, material: MaterialId) -> u32 {
        self.materials[..self.len]
            .iter()
            .position(|&m| m == material)
            .map_or(0, |i| self.weights[i])
    }

    #[inline]
    fn finish(&self) -> Voxel {
        if self.total < SOLID_AT {
            return Voxel::AIR;
        }
        let best = self
            .best(|_| true)
            .expect("a solid total has at least one Material");
        Voxel::cube(self.materials[best])
    }
}

#[inline]
fn weight(shapes: &ShapeTable, voxel: Voxel) -> u32 {
    if voxel.is_air() {
        0
    } else {
        shapes.volume(voxel) as u32
    }
}

/// Child `i` is the octant `x + 2y + 4z`. A finer Voxel weighs its Shape's volume; the
/// coarse Voxel is solid at half of eight cubes and is then a full cube of the Material
/// with the most volume, so the result never holds a shaped Voxel.
pub fn downsample(children: [&Chunk; 8], shapes: &ShapeTable) -> Chunk {
    if let Some(v) = children[0].uniform()
        && children.iter().all(|c| c.uniform() == Some(v))
    {
        return Chunk::filled(uniform_octant(v, shapes));
    }
    downsample_general(children, shapes)
}

fn downsample_general(children: [&Chunk; 8], shapes: &ShapeTable) -> Chunk {
    let mut out = Chunk::air();
    for (i, child) in children.into_iter().enumerate() {
        packed_octant(child, i, shapes, &mut out);
    }
    out
}

/// Where child `i`'s octant starts in its coarse Chunk.
fn octant_origin(i: usize) -> (usize, usize, usize) {
    (i % 2 * OCTANT, i / 2 % 2 * OCTANT, i / 4 * OCTANT)
}

fn packed_octant(child: &Chunk, i: usize, shapes: &ShapeTable, out: &mut Chunk) {
    let (ox, oy, oz) = octant_origin(i);
    if let Some(v) = child.uniform() {
        let coarse = uniform_octant(v, shapes);
        if !coarse.is_air() {
            for z in 0..OCTANT {
                for y in 0..OCTANT {
                    for x in 0..OCTANT {
                        out.set(ox + x, oy + y, oz + z, coarse);
                    }
                }
            }
        }
        return;
    }
    // Row `dy + 2dz` of the 2x2 the current coarse row reads.
    let mut rows = [[Voxel::AIR; CHUNK_SIZE]; 4];
    for z in 0..OCTANT {
        for y in 0..OCTANT {
            for (k, row) in rows.iter_mut().enumerate() {
                child.read_row(2 * y + (k & 1), 2 * z + (k >> 1), row);
            }
            for x in 0..OCTANT {
                let lx = 2 * x;
                let first = rows[0][lx];
                let v = if rows.iter().all(|r| r[lx] == first && r[lx + 1] == first) {
                    // What the tally makes of eight of one Voxel.
                    if 8 * weight(shapes, first) >= SOLID_AT {
                        Voxel::cube(first.material())
                    } else {
                        Voxel::AIR
                    }
                } else {
                    let mut tally = Tally::<8>::new();
                    for (k, row) in rows.iter().enumerate() {
                        for v in [row[lx], row[lx + 1]] {
                            tally.add(v.material(), weight(shapes, v), (k & 1) as u8);
                        }
                    }
                    tally.finish()
                };
                if !v.is_air() {
                    out.set(ox + x, oy + y, oz + z, v);
                }
            }
        }
    }
}

/// Where generation puts its lake, so level 1 can tell the lake it made from
/// fluid that flowed or was placed: a packed cube of the lake's Material in a
/// level-0 Voxel whose bottom is below `level_m`, computed as
/// [`Worldgen::chunk`] computes it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Waterline {
    pub level_m: f32,
    /// A level-0 Voxel's edge.
    pub voxel_m: f32,
    pub material: MaterialId,
}

impl Waterline {
    fn generates(&self, voxel: Voxel, child_y: i32, y: usize) -> bool {
        let floor_m = (child_y * CHUNK_SIZE as i32) as f32 * self.voxel_m;
        voxel.material() == self.material && floor_m + y as f32 * self.voxel_m < self.level_m
    }
}

/// A level-0 child of a level-1 Chunk.
#[derive(Clone, Copy)]
pub enum Child<'a> {
    /// Generated, or held and never changed: its packed Voxels are all of it.
    Packed(&'a Chunk),
    /// Held and changed: read with the fluid and fire beside its Voxels.
    Changed(&'a Chunk, &'a ChunkOverlay),
}

/// A coarse Chunk with the fluid and fire level 1 shows of what the Server
/// holds. Above level 1, and wherever nothing held changed, the overlay is
/// empty.
pub struct Coarse {
    pub chunk: Chunk,
    pub overlay: ChunkOverlay,
    /// Open everywhere from [`downsample_changed`]; [`seam_openings`] reads
    /// them from the children.
    pub openings: SeamOpenings,
}

/// How far past the finer Voxels against a seam plane [`seam_openings`] also
/// looks, in finer Voxels: up, and both ways along the plane.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SeamMargin {
    pub above: usize,
    pub aside: usize,
}

impl SeamMargin {
    /// For children that are the finer level's own Chunks.
    pub const EXACT: SeamMargin = SeamMargin { above: 0, aside: 0 };
}

/// The [`SeamOpenings`] of the coarse Chunk `children` downsample to: a coarse
/// Voxel against a seam plane is open where any of its finer Voxels against
/// the plane, or within `margin` of them, is not an opaque cube. Past the
/// children, which is another coarse Chunk's, counts as open.
pub fn seam_openings(
    children: [&Chunk; 8],
    margin: SeamMargin,
    materials: &MaterialTable,
) -> SeamOpenings {
    const FINE: usize = 2 * CHUNK_SIZE;
    let open = |v: Voxel| !v.is_full_cube() || materials.render(v) != RenderClass::Opaque;
    let mut out = SeamOpenings::nowhere();
    if children
        .iter()
        .all(|c| c.uniform().is_some_and(|v| !open(v)))
    {
        return out;
    }
    for axis in [0, 2] {
        let other = 2 - axis;
        for layer in SEAM_LAYERS {
            // The finer Voxels of `layer` that touch its plane.
            let fine = if layer % SEAM_EVERY == 0 {
                2 * layer
            } else {
                2 * layer + 1
            };
            let mut rows = [0u64; FINE];
            for (fy, row) in rows.iter_mut().enumerate() {
                for fw in 0..FINE {
                    let mut c = [0; 3];
                    c[axis] = fine;
                    c[1] = fy;
                    c[other] = fw;
                    let [cx, cy, cz] = c.map(|v| v / CHUNK_SIZE);
                    let child = children[cx + 2 * cy + 4 * cz];
                    let v = child.get(c[0] % CHUNK_SIZE, c[1] % CHUNK_SIZE, c[2] % CHUNK_SIZE);
                    *row |= u64::from(open(v)) << fw;
                }
            }
            for y in 0..CHUNK_SIZE {
                let top = 2 * y + 2 + margin.above;
                let mut any = if top > FINE { u64::MAX } else { 0 };
                for row in &rows[2 * y..top.min(FINE)] {
                    any |= row;
                }
                for w in 0..CHUNK_SIZE {
                    let (lo, hi) = ((2 * w).checked_sub(margin.aside), 2 * w + 2 + margin.aside);
                    let seen = match lo {
                        Some(lo) if hi <= FINE => any & (u64::MAX >> (64 - (hi - lo))) << lo != 0,
                        _ => true,
                    };
                    if seen {
                        let mut cell = [0; 3];
                        cell[axis] = layer;
                        cell[1] = y;
                        cell[other] = w;
                        out.set_open(axis, cell);
                    }
                }
            }
        }
    }
    out
}

/// Level 1 from eight level-0 Chunks, the changed ones read through
/// [`Occupants`]. A picture of held state, never a simulation of it.
///
/// The approximation is by volume, as [`downsample`]'s is. A Voxel's fluid
/// weighs what it fills: its surface's share of the cube in empty space, the
/// sub-cells below each void's surface in a Shape. It competes with solids by
/// volume exactly as generated water does, so a coarse Voxel is still the
/// Material with the most volume once half of it is occupied. Water the
/// generator could have made, a packed source below the [`Waterline`], stays
/// packed lake water. Any other fluid is dynamic: a coarse Voxel whose fluid is
/// mostly dynamic is left air with that fluid in the overlay, standing at the
/// share of the coarse Voxel its contents fill, and at least a sixteenth. So
/// fluid under half a coarse Voxel still shows where nothing solid wins it, a
/// coarse surface lies in the coarse Voxel whose water it draws, and nothing
/// moves it to the generated waterline. A coarse solid burns when at least half
/// of its Material's volume is burning.
pub fn downsample_changed(
    children: [Child<'_>; 8],
    coord_y: i32,
    tables: &Tables,
    waterline: Option<Waterline>,
) -> Coarse {
    let packed = children.map(|c| match c {
        Child::Packed(chunk) => Some(chunk),
        Child::Changed(..) => None,
    });
    if let Some(all) = packed.iter().copied().collect::<Option<Vec<&Chunk>>>() {
        let all: [&Chunk; 8] = std::array::from_fn(|i| all[i]);
        return Coarse {
            chunk: downsample(all, &tables.shapes),
            overlay: ChunkOverlay::none(),
            openings: SeamOpenings::everywhere(),
        };
    }
    let mut chunk = Chunk::air();
    let mut overlay = ChunkOverlay::none();
    for (i, child) in children.into_iter().enumerate() {
        match child {
            Child::Packed(c) => packed_octant(c, i, &tables.shapes, &mut chunk),
            Child::Changed(c, o) => {
                let child_y = 2 * coord_y + (i / 2 % 2) as i32;
                let read = Changed {
                    chunk: c,
                    overlay: o,
                    child_y,
                    tables,
                    waterline,
                };
                read.octant(octant_origin(i), &mut chunk, &mut overlay);
            }
        }
    }
    Coarse {
        chunk,
        overlay,
        openings: SeamOpenings::everywhere(),
    }
}

struct Changed<'a> {
    chunk: &'a Chunk,
    overlay: &'a ChunkOverlay,
    child_y: i32,
    tables: &'a Tables,
    waterline: Option<Waterline>,
}

impl Changed<'_> {
    fn octant(&self, (ox, oy, oz): (usize, usize, usize), out: &mut Chunk, wet: &mut ChunkOverlay) {
        for z in 0..OCTANT {
            for y in 0..OCTANT {
                for x in 0..OCTANT {
                    let (voxel, overlay) = self.cell([2 * x, 2 * y, 2 * z]);
                    if !voxel.is_air() {
                        out.set(ox + x, oy + y, oz + z, voxel);
                    }
                    if overlay != Overlay::NONE {
                        wet.set(ox + x, oy + y, oz + z, overlay);
                    }
                }
            }
        }
    }

    /// One coarse Voxel from the eight at `[lx, ly, lz]` and above.
    fn cell(&self, [lx, ly, lz]: [usize; 3]) -> (Voxel, Overlay) {
        let shapes = &self.tables.shapes;
        let fluids = &self.tables.fluids;
        // Solids and every fluid, as `downsample` weighs Materials.
        let mut all = Tally::<16>::new();
        let mut lake = Tally::<1>::new();
        let mut dynamic = Tally::<8>::new();
        let mut burning = Tally::<8>::new();
        let mut fuel = 0;
        for dz in 0..2 {
            for dy in 0..2 {
                for dx in 0..2 {
                    let (x, y, z) = (lx + dx, ly + dy, lz + dz);
                    let voxel = self.chunk.get(x, y, z);
                    let here = Occupants::of(voxel, self.overlay.get(x, y, z), fluids);
                    let dy = dy as u8;
                    if !here.solid.is_air() {
                        let w = weight(shapes, here.solid);
                        all.add(here.solid.material(), w, dy);
                        if let Some(b) = here.burning {
                            burning.add(here.solid.material(), w, dy);
                            fuel = fuel.max(b.fuel);
                        }
                    }
                    let Some(fluid) = here.fluid else {
                        continue;
                    };
                    let generated = self
                        .waterline
                        .is_some_and(|w| w.generates(voxel, self.child_y, y));
                    let w = if generated {
                        let w = weight(shapes, voxel);
                        lake.add(voxel.material(), w, dy);
                        w
                    } else {
                        let w = fluid_volume(here.solid, fluid, shapes);
                        dynamic.add(fluid.material, w, dy);
                        w
                    };
                    all.add(fluid.material, w, dy);
                }
            }
        }
        if dynamic.total == 0 {
            let voxel = all.finish();
            return (voxel, burns(&all, &burning, voxel, fuel));
        }
        let level = |total: u32| -> u8 {
            let full = u32::from(Fluid::FULL_SURFACE);
            (total * full).div_ceil(8 * CUBE_WEIGHT).clamp(1, full) as u8
        };
        let flowing = |material: MaterialId| {
            let surface = Level {
                surface: level(all.total),
                ..Level::DRY
            };
            Overlay {
                fluid: Fluid::flowing(material, [surface, Level::DRY, Level::DRY, Level::DRY]),
                burning: None,
            }
        };
        if all.total >= SOLID_AT {
            let best = all
                .best(|_| true)
                .expect("an occupied total has a Material");
            let material = all.materials[best];
            if !fluids.contains(material) {
                let voxel = Voxel::cube(material);
                return (voxel, burns(&all, &burning, voxel, fuel));
            }
            if lake.weight_of(material) >= dynamic.weight_of(material) {
                return (Voxel::cube(material), Overlay::NONE);
            }
            return (Voxel::AIR, flowing(material));
        }
        let best = all
            .best(|m| fluids.contains(m))
            .expect("dynamic fluid is weighed");
        (Voxel::AIR, flowing(all.materials[best]))
    }
}

/// The fire a coarse solid shows: its Material's own, if half of it burns.
fn burns<const N: usize>(all: &Tally<N>, burning: &Tally<8>, voxel: Voxel, fuel: u16) -> Overlay {
    let material = voxel.material();
    let burnt = burning.weight_of(material);
    if voxel.is_air() || burnt == 0 || 2 * burnt < all.weight_of(material) {
        return Overlay::NONE;
    }
    Overlay {
        fluid: None,
        burning: Some(Burning { fuel }),
    }
}

/// The volume `fluid` fills where `solid` stands, in [`ShapeTable::volume`]'s
/// units. Read without arms: a connected Shape's arms are its neighbours' to
/// decide, and a sixteenth of a Voxel either way is below what level 1 draws.
fn fluid_volume(solid: Voxel, fluid: Fluid, shapes: &ShapeTable) -> u32 {
    shapes
        .voids(solid, Connections::NONE)
        .iter()
        .enumerate()
        .map(|(r, void)| void.volume_below(fluid.level(r).surface))
        .sum()
}

fn uniform_octant(v: Voxel, shapes: &ShapeTable) -> Voxel {
    if 8 * weight(shapes, v) >= SOLID_AT {
        Voxel::cube(v.material())
    } else {
        Voxel::AIR
    }
}

/// What a coarse Voxel's faces look into when it is air with open ground far above and
/// stone around it. Level 0 shows the inside of a cave black, so does this.
pub const COVERED_SKY: Light = Light::new(0, 0);

const OPEN_SKY: Light = Light::new(MAX_LIGHT, 0);

/// The terrain over a point, in metres, and the band that height never leaves.
pub trait Surface {
    fn height_m(&self, x_m: f32, z_m: f32) -> f32;
    fn bounds_m(&self) -> (f32, f32);
}

impl Surface for Worldgen {
    fn height_m(&self, x_m: f32, z_m: f32) -> f32 {
        self.surface_height_m(x_m, z_m)
    }

    fn bounds_m(&self) -> (f32, f32) {
        self.surface_bounds_m()
    }
}

/// The light the mesher reads for a coarse Chunk and its six face neighbours, in
/// [`crate::mesh`]'s face order.
pub struct CoarseLight {
    pub center: ChunkLight,
    pub around: [ChunkLight; 6],
}

/// Corner heights over the Chunk's footprint and its one-Voxel shell.
struct Lattice {
    heights: Vec<f32>,
}

/// Corners from `-1` to `CHUNK_SIZE` inclusive: a Voxel of the shell has two.
const CORNERS: usize = CHUNK_SIZE + 3;

impl Lattice {
    /// Positions are a whole count of Voxels times the Voxel size, so two Chunks that share a
    /// corner ask for the same point to the last bit at any Voxel size.
    fn sample(surface: &impl Surface, voxel_m: f32, coord: [i32; 3]) -> Self {
        let first = |axis: usize| coord[axis] * CHUNK_SIZE as i32 - 1;
        let mut heights = Vec::with_capacity(CORNERS * CORNERS);
        for iz in 0..CORNERS {
            let z_m = (first(2) + iz as i32) as f32 * voxel_m;
            for ix in 0..CORNERS {
                let x_m = (first(0) + ix as i32) as f32 * voxel_m;
                heights.push(surface.height_m(x_m, z_m));
            }
        }
        Lattice { heights }
    }

    /// The lowest corner of the footprint of the Voxel at column `(x, z)`, which may be `-1`
    /// or `CHUNK_SIZE`.
    fn lowest(&self, x: i32, z: i32) -> f32 {
        let at = |dx: i32, dz: i32| {
            self.heights[(z + 1 + dz) as usize * CORNERS + (x + 1 + dx) as usize]
        };
        at(0, 0).min(at(1, 0)).min(at(0, 1)).min(at(1, 1))
    }

    /// Whether a Voxel whose top is `y_top` Voxels up, in the column `(x, z)` of the frame the
    /// lattice was sampled in, has open ground more than a Voxel above it.
    fn covered(&self, voxel_m: f32, y_top: i32, x: i32, z: i32) -> bool {
        y_top as f32 * voxel_m < self.lowest(x, z) - voxel_m
    }
}

/// How much of the seven Chunks the surface band can reach, decided without a height.
enum Reach {
    /// No Voxel can be covered.
    Nowhere,
    /// Every Voxel is.
    Everywhere,
    /// Ask the lattice.
    Sampled(Lattice),
}

impl Reach {
    fn of(surface: &impl Surface, voxel_m: f32, coord: [i32; 3], solid_throughout: bool) -> Self {
        let size = CHUNK_SIZE as i32;
        let (lowest_m, highest_m) = surface.bounds_m();
        // The shell's lowest Voxel tops out at the Chunk's floor and its highest at one Voxel
        // above the Chunk's ceiling.
        let floor_m = (coord[1] * size) as f32 * voxel_m;
        let ceiling_m = ((coord[1] + 1) * size + 1) as f32 * voxel_m;
        if solid_throughout || floor_m >= highest_m - voxel_m {
            Reach::Nowhere
        } else if ceiling_m < lowest_m - voxel_m {
            Reach::Everywhere
        } else {
            Reach::Sampled(Lattice::sample(surface, voxel_m, coord))
        }
    }

    fn covered(&self, voxel_m: f32, y_top: i32, x: i32, z: i32) -> bool {
        match self {
            Reach::Nowhere => false,
            Reach::Everywhere => true,
            Reach::Sampled(lattice) => lattice.covered(voxel_m, y_top, x, z),
        }
    }
}

fn is_solid_throughout(chunk: &Chunk) -> bool {
    chunk.uniform().is_some_and(|v| !v.is_air())
}

/// What a coarse Chunk needs to light its lake as level 0 does.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct UnderWater {
    /// The lake's surface, in metres.
    pub level_m: f32,
    pub material: MaterialId,
    /// Levels the lake's Material removes per Voxel it fills.
    pub dimming: u8,
    /// A level-0 Voxel's edge. A light level counts steps, not metres, so depth
    /// is counted in these.
    pub step_m: f32,
}

impl UnderWater {
    /// The sky light level 0 has in its Water Voxel whose bottom is at `bottom_m`.
    /// Full sky falls into the top Voxel and loses only the dimming; below full
    /// strength it no longer falls free, so each Voxel after costs a step too.
    fn light_at(&self, bottom_m: f32) -> Light {
        let steps = ((self.level_m - bottom_m) / self.step_m).ceil().max(1.0) as u32;
        let d = self.dimming as u32;
        let lost = if d == 0 { 0 } else { d + (d + 1) * (steps - 1) };
        Light::new(MAX_LIGHT - lost.min(MAX_LIGHT as u32) as u8, 0)
    }

    /// Lights the Water in `chunk`, whose lowest Voxel row is `base_y`, at the
    /// cells `cells` yields.
    fn light_water(
        &self,
        light: &mut ChunkLight,
        chunk: &Chunk,
        base_y: i32,
        voxel_m: f32,
        cells: impl Iterator<Item = [i32; 3]>,
    ) {
        let floor_m = base_y as f32 * voxel_m;
        if floor_m >= self.level_m
            || chunk
                .uniform()
                .is_some_and(|v| v.material() != self.material)
        {
            return;
        }
        for [x, y, z] in cells {
            let (x, y, z) = (x as usize, y as usize, z as usize);
            if chunk.get(x, y, z).material() == self.material {
                light.set(x, y, z, self.light_at((base_y + y as i32) as f32 * voxel_m));
            }
        }
    }
}

/// [`coarse_light_in`] for a World without water.
pub fn coarse_light(
    surface: &impl Surface,
    voxel_m: f32,
    coord: [i32; 3],
    center: &Chunk,
    around: [&Chunk; 6],
) -> CoarseLight {
    coarse_light_in(surface, voxel_m, coord, center, around, None)
}

/// The light a coarse Chunk is meshed with: open sky, except Air the ground
/// covers, which is dark, and Water, which is dimmed by its depth as at level 0.
pub fn coarse_light_in(
    surface: &impl Surface,
    voxel_m: f32,
    coord: [i32; 3],
    center: &Chunk,
    around: [&Chunk; 6],
    water: Option<UnderWater>,
) -> CoarseLight {
    let solid = is_solid_throughout(center) && around.iter().all(|c| is_solid_throughout(c));
    let reach = Reach::of(surface, voxel_m, coord, solid);
    let size = CHUNK_SIZE as i32;
    let base_y = coord[1] * size;

    // A uniform Chunk is answered whole; only a mixed one is walked.
    let uniform = |chunk: &Chunk| match (&reach, chunk.uniform()) {
        (Reach::Nowhere, _) => Some(OPEN_SKY),
        (_, Some(v)) if !v.is_air() => Some(OPEN_SKY),
        (Reach::Everywhere, Some(_)) => Some(COVERED_SKY),
        _ => None,
    };

    let mut center_light = uniform(center).map_or_else(
        || {
            let mut light = ChunkLight::filled(OPEN_SKY);
            for z in 0..size {
                for x in 0..size {
                    for y in 0..size {
                        if center.get(x as usize, y as usize, z as usize).is_air()
                            && reach.covered(voxel_m, base_y + y + 1, x, z)
                        {
                            light.set(x as usize, y as usize, z as usize, COVERED_SKY);
                        }
                    }
                }
            }
            light
        },
        ChunkLight::filled,
    );

    // The plane of the neighbour across `face` that touches this Chunk, in its own indices.
    let plane_cells = |face: usize| {
        (0..size).flat_map(move |a| {
            (0..size).map(move |b| match face {
                NEG_X => [size - 1, a, b],
                POS_X => [0, a, b],
                NEG_Y => [a, size - 1, b],
                POS_Y => [a, 0, b],
                NEG_Z => [a, b, size - 1],
                _ => [a, b, 0],
            })
        })
    };

    let mut around_light: [ChunkLight; 6] = std::array::from_fn(|face| {
        let step = FACE_STEPS[face];
        let neighbour = around[face];
        if let Some(whole) = uniform(neighbour) {
            return ChunkLight::filled(whole);
        }
        let mut plane = ChunkLight::filled(OPEN_SKY);
        for local in plane_cells(face) {
            let air = neighbour
                .get(local[0] as usize, local[1] as usize, local[2] as usize)
                .is_air();
            let (x, z) = (local[0] + step[0] * size, local[2] + step[2] * size);
            let y_top = base_y + step[1] * size + local[1] + 1;
            if air && reach.covered(voxel_m, y_top, x, z) {
                plane.set(
                    local[0] as usize,
                    local[1] as usize,
                    local[2] as usize,
                    COVERED_SKY,
                );
            }
        }
        plane
    });
    if let Some(water) = water {
        let all =
            (0..size).flat_map(|z| (0..size).flat_map(move |y| (0..size).map(move |x| [x, y, z])));
        water.light_water(&mut center_light, center, base_y, voxel_m, all);
        for (face, light) in around_light.iter_mut().enumerate() {
            let base = base_y + FACE_STEPS[face][1] * size;
            water.light_water(light, around[face], base, voxel_m, plane_cells(face));
        }
    }
    CoarseLight {
        center: center_light,
        around: around_light,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testgen::fixtures;

    use crate::shape::examples::{FENCE_ID, fence};
    use crate::shape::{NodeBox, Orientation, SHAPE_POST, SHAPE_SLAB, SHAPE_STAIR, ShapeDef};

    use crate::worldgen::{Worldgen, WorldgenParams};
    use std::collections::BTreeMap;

    const SEED: u64 = 0x7e55_e2ae_0000_0001;
    const COLUMNS: usize = 60;

    /// Eight children of rock below finer y 20, with a two-Voxel pit at finer
    /// (32, 18..20, 9): against the coarse plane x = 16, in coarse Voxel
    /// (16, 9, 4).
    fn pitted_children() -> [Chunk; 8] {
        let rock = Voxel::cube(fixtures().ground.rock);
        let mut children: [Chunk; 8] = std::array::from_fn(|_| Chunk::air());
        for (i, child) in children.iter_mut().enumerate() {
            if i >> 1 & 1 == 1 {
                continue;
            }
            for y in 0..20 {
                for z in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        child.set(x, y, z, rock);
                    }
                }
            }
        }
        for y in 18..20 {
            children[1].set(0, y, 9, Voxel::AIR);
        }
        children
    }

    #[test]
    fn a_seam_opening_is_where_the_finer_voxels_against_the_plane_let_light_through() {
        let children = pitted_children();
        let refs = std::array::from_fn(|i| &children[i]);
        let materials = &fixtures().tables.materials;
        let exact = seam_openings(refs, SeamMargin::EXACT, materials);
        assert!(exact.is_open(0, [16, 9, 4]), "the pit");
        assert!(!exact.is_open(0, [16, 9, 5]), "beside the pit");
        assert!(!exact.is_open(0, [16, 8, 4]), "under the pit");
        assert!(!exact.is_open(0, [15, 9, 4]), "the far side of the plane");
        assert!(exact.is_open(0, [16, 10, 20]), "air over the ground");
        assert!(
            !exact.is_open(2, [4, 9, 16]),
            "the pit is against no plane across z"
        );

        let wider = SeamMargin { above: 1, aside: 1 };
        let margin = seam_openings(refs, wider, materials);
        assert!(margin.is_open(0, [16, 8, 4]), "a finer Voxel above");
        assert!(margin.is_open(0, [16, 9, 5]), "a finer Voxel aside");
        assert!(!margin.is_open(0, [16, 7, 4]), "two finer Voxels below");
    }

    const LATTICE: usize = 2 * CHUNK_SIZE;

    fn table() -> ShapeTable {
        let mut t = ShapeTable::builtin();
        assert_eq!(t.register(fence()).unwrap(), FENCE_ID);
        t
    }

    /// A Shape of 2047 of 4096 sub-cells: a slab less one.
    fn nearly_a_slab(t: &mut ShapeTable) -> Voxel {
        let b = NodeBox::new;
        let id = t
            .register(ShapeDef {
                connect: None,
                boxes: vec![
                    b([0, 0, 0], [16, 7, 16]),
                    b([0, 7, 0], [15, 8, 16]),
                    b([15, 7, 0], [16, 8, 15]),
                ],
                collision: None,
            })
            .unwrap();
        let v = Voxel::new(fixtures().ground.rock, id, 0);
        assert_eq!(t.volume(v), 2047);
        v
    }

    fn lattice_index(x: usize, y: usize, z: usize) -> usize {
        (z * LATTICE + y) * LATTICE + x
    }

    fn split(at: impl Fn(usize, usize, usize) -> Voxel) -> [Chunk; 8] {
        std::array::from_fn(|i| {
            let (ox, oy, oz) = (
                i % 2 * CHUNK_SIZE,
                i / 2 % 2 * CHUNK_SIZE,
                i / 4 * CHUNK_SIZE,
            );
            let mut c = Chunk::air();
            for z in 0..CHUNK_SIZE {
                for y in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        c.set(x, y, z, at(ox + x, oy + y, oz + z));
                    }
                }
            }
            c
        })
    }

    fn refs(children: &[Chunk; 8]) -> [&Chunk; 8] {
        std::array::from_fn(|i| &children[i])
    }

    fn run(children: &[Chunk; 8], shapes: &ShapeTable) -> Chunk {
        downsample(refs(children), shapes)
    }

    /// Brute force over the flat lattice, sharing nothing with `downsample` but the definition.
    fn reference(children: &[Chunk; 8], shapes: &ShapeTable) -> Vec<Voxel> {
        let mut lattice = vec![Voxel::AIR; LATTICE * LATTICE * LATTICE];
        for (i, child) in children.iter().enumerate() {
            let (ox, oy, oz) = (
                i % 2 * CHUNK_SIZE,
                i / 2 % 2 * CHUNK_SIZE,
                i / 4 * CHUNK_SIZE,
            );
            for z in 0..CHUNK_SIZE {
                for y in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        lattice[lattice_index(ox + x, oy + y, oz + z)] = child.get(x, y, z);
                    }
                }
            }
        }
        let mut out = Vec::with_capacity(CHUNK_SIZE.pow(3));
        for cz in 0..CHUNK_SIZE {
            for cy in 0..CHUNK_SIZE {
                for cx in 0..CHUNK_SIZE {
                    let mut total = 0u32;
                    let mut per: BTreeMap<MaterialId, (u32, usize)> = BTreeMap::new();
                    for dz in 0..2 {
                        for dy in 0..2 {
                            for dx in 0..2 {
                                let v =
                                    lattice[lattice_index(2 * cx + dx, 2 * cy + dy, 2 * cz + dz)];
                                if v.material() == 0 {
                                    continue;
                                }
                                let w = shapes.volume(v) as u32;
                                if w == 0 {
                                    continue;
                                }
                                total += w;
                                let e = per.entry(v.material()).or_insert((0, 0));
                                e.0 += w;
                                e.1 = e.1.max(dy);
                            }
                        }
                    }
                    if 2 * total < 8 * 4096 {
                        out.push(Voxel::AIR);
                        continue;
                    }
                    let mut best: Option<(MaterialId, u32, usize)> = None;
                    for (&m, &(w, top)) in &per {
                        // Ascending id, so a full tie leaves the first found.
                        let better = match best {
                            None => true,
                            Some((_, bw, bt)) => w > bw || (w == bw && top > bt),
                        };
                        if better {
                            best = Some((m, w, top));
                        }
                    }
                    out.push(Voxel::cube(best.unwrap().0));
                }
            }
        }
        out
    }

    fn assert_matches_reference(children: &[Chunk; 8], shapes: &ShapeTable, what: &str) {
        let got = run(children, shapes);
        let want = reference(children, shapes);
        for z in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    assert_eq!(
                        got.get(x, y, z),
                        want[(z * CHUNK_SIZE + y) * CHUNK_SIZE + x],
                        "{what}: at {x},{y},{z}"
                    );
                }
            }
        }
    }

    /// The coarse Voxel at the origin of child 0, from eight finer ones in order `dx + 2dy + 4dz`.
    fn coarse(voxels: [Voxel; 8], shapes: &ShapeTable) -> Voxel {
        let mut c = Chunk::air();
        for (j, v) in voxels.into_iter().enumerate() {
            c.set(j % 2, j / 2 % 2, j / 4, v);
        }
        let air = Chunk::air();
        let mut children = [&air; 8];
        children[0] = &c;
        downsample(children, shapes).get(0, 0, 0)
    }

    fn cube(m: MaterialId) -> Voxel {
        Voxel::cube(m)
    }

    fn slab() -> Voxel {
        Voxel::shaped(fixtures().ground.rock, SHAPE_SLAB, Orientation::IDENTITY)
    }

    /// Hand-rolled to avoid a dependency; the stream must not change between runs.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        fn below(&mut self, n: usize) -> usize {
            (self.next() >> 11) as usize % n
        }
    }

    fn pool(orientations: &[Orientation]) -> Vec<Voxel> {
        let mut pool = vec![Voxel::AIR, Voxel::new(0, SHAPE_STAIR, 5)];
        pool.extend(
            [
                fixtures().ground.rock,
                fixtures().ground.soil,
                fixtures().ground.surface,
                fixtures().glass,
                fixtures().leaves,
                water(),
            ]
            .map(cube),
        );
        for shape in [SHAPE_SLAB, SHAPE_STAIR, SHAPE_POST, FENCE_ID] {
            for &o in orientations {
                for m in [fixtures().ground.rock, fixtures().ground.soil, water()] {
                    pool.push(Voxel::shaped(m, shape, o));
                }
            }
        }
        pool
    }

    /// 2x2x2 blocks of noise, two Materials in halves, or one with a stranger: the ties.
    fn random_children(seed: u64) -> [Chunk; 8] {
        let orientations: Vec<Orientation> = Orientation::all().collect();
        let pool = pool(&orientations);
        let mut rng = Rng(seed);
        let mut lattice = vec![Voxel::AIR; LATTICE * LATTICE * LATTICE];
        let dominant = 1 + rng.below(3);
        for cz in 0..CHUNK_SIZE {
            for cy in 0..CHUNK_SIZE {
                for cx in 0..CHUNK_SIZE {
                    let (a, b) = (pool[rng.below(pool.len())], pool[rng.below(pool.len())]);
                    let kind = rng.below(3 + dominant);
                    for j in 0..8 {
                        let v = match kind {
                            0 => pool[rng.below(pool.len())],
                            1 => {
                                if rng.below(2) == 0 {
                                    a
                                } else {
                                    b
                                }
                            }
                            2 if j == rng.below(8) => b,
                            _ => a,
                        };
                        let (dx, dy, dz) = (j % 2, j / 2 % 2, j / 4);
                        lattice[lattice_index(2 * cx + dx, 2 * cy + dy, 2 * cz + dz)] = v;
                    }
                }
            }
        }
        split(|x, y, z| lattice[lattice_index(x, y, z)])
    }

    #[test]
    fn half_the_volume_is_solid_and_the_bare_post_is_air() {
        let mut t = table();
        let nearly = nearly_a_slab(&mut t);
        let stair = Voxel::shaped(fixtures().ground.rock, SHAPE_STAIR, Orientation::IDENTITY);
        let post = Voxel::shaped(fixtures().ground.rock, SHAPE_POST, Orientation::IDENTITY);
        let fence = Voxel::new(fixtures().ground.rock, FENCE_ID, 0);

        assert_eq!(
            coarse([slab(); 8], &t),
            cube(fixtures().ground.rock),
            "a tie is solid"
        );
        assert_eq!(coarse([stair; 8], &t), cube(fixtures().ground.rock));
        assert_eq!(coarse([post; 8], &t), Voxel::AIR);
        assert_eq!(coarse([fence; 8], &t), Voxel::AIR);
        assert_eq!(coarse([nearly; 8], &t), Voxel::AIR);

        let mut exactly = [
            cube(fixtures().ground.rock),
            cube(fixtures().ground.rock),
            cube(fixtures().ground.rock),
            slab(),
            slab(),
        ]
        .into_iter()
        .chain([Voxel::AIR; 3]);
        let mut at_half = [Voxel::AIR; 8];
        at_half.fill_with(|| exactly.next().unwrap());
        assert_eq!(coarse(at_half, &t), cube(fixtures().ground.rock));
        at_half[4] = nearly;
        assert_eq!(coarse(at_half, &t), Voxel::AIR, "one sub-cell under");
    }

    #[test]
    fn a_shaped_result_is_never_produced() {
        let t = table();
        for seed in [3, 4] {
            let out = run(&random_children(seed), &t);
            assert!(!out.may_contain_shaped());
            assert_eq!(out.shaped_count(), 0);
        }
        let stairs = std::array::from_fn(|_| {
            Chunk::filled(Voxel::new(fixtures().ground.soil, SHAPE_STAIR, 7))
        });
        let out = run(&stairs, &t);
        assert_eq!(out.uniform(), Some(cube(fixtures().ground.soil)));
    }

    /// Grass is the one-Voxel cap over dirt, so every odd surface height is a
    /// four-to-four tie that only the height rule resolves.
    #[test]
    fn grass_over_dirt_stays_grass_at_every_surface_height() {
        let t = table();
        for surface in 0..LATTICE {
            let children = split(|_, y, _| match y.cmp(&surface) {
                std::cmp::Ordering::Less => cube(fixtures().ground.soil),
                std::cmp::Ordering::Equal => cube(fixtures().ground.surface),
                std::cmp::Ordering::Greater => Voxel::AIR,
            });
            let out = run(&children, &t);
            for cy in 0..CHUNK_SIZE {
                let want = match cy.cmp(&(surface / 2)) {
                    std::cmp::Ordering::Less => cube(fixtures().ground.soil),
                    std::cmp::Ordering::Equal => cube(fixtures().ground.surface),
                    std::cmp::Ordering::Greater => Voxel::AIR,
                };
                assert_eq!(out.get(5, cy, 9), want, "surface {surface}, row {cy}");
            }
        }
    }

    #[test]
    fn height_decides_a_tie_whatever_the_ids() {
        let t = table();
        let stacked = |bottom: MaterialId, top: MaterialId| {
            std::array::from_fn(|j| cube(if j / 2 % 2 == 1 { top } else { bottom }))
        };
        assert_eq!(
            coarse(stacked(fixtures().ground.rock, water()), &t),
            cube(water())
        );
        assert_eq!(
            coarse(stacked(water(), fixtures().ground.rock), &t),
            cube(fixtures().ground.rock)
        );
        assert_eq!(
            coarse(
                stacked(fixtures().ground.soil, fixtures().ground.surface),
                &t
            ),
            cube(fixtures().ground.surface)
        );
        assert_eq!(
            coarse(
                stacked(fixtures().ground.surface, fixtures().ground.soil),
                &t
            ),
            cube(fixtures().ground.soil)
        );
    }

    #[test]
    fn the_lowest_id_wins_a_tie_in_height_in_any_order_or_octant() {
        let t = table();
        let mats = [
            fixtures().ground.rock,
            fixtures().ground.soil,
            fixtures().ground.surface,
            fixtures().glass,
            fixtures().leaves,
            water(),
        ];
        for &a in &mats {
            for &b in mats.iter().filter(|&&b| b != a) {
                let want = cube(a.min(b));
                for (left, right) in [(a, b), (b, a)] {
                    let voxels =
                        std::array::from_fn(|j| cube(if j % 2 == 0 { left } else { right }));
                    assert_eq!(coarse(voxels, &t), want, "{a} against {b}");

                    // The palette of the Chunk fills in a different order.
                    for octant in 0..8 {
                        let mut chunk = Chunk::air();
                        let order: Vec<usize> = if left < right {
                            (0..8).collect()
                        } else {
                            (0..8).rev().collect()
                        };
                        for j in order {
                            chunk.set(j % 2 + 4, j / 2 % 2 + 6, j / 4 + 2, voxels[j]);
                        }
                        let air = Chunk::air();
                        let mut children = [&air; 8];
                        children[octant] = &chunk;
                        let out = downsample(children, &t);
                        let (ox, oy, oz) = (
                            octant % 2 * OCTANT,
                            octant / 2 % 2 * OCTANT,
                            octant / 4 * OCTANT,
                        );
                        assert_eq!(out.get(ox + 2, oy + 3, oz + 1), want, "octant {octant}");
                    }
                }
            }
        }
    }

    /// The top solid coarse Voxel of the column at `level`, built as the Server does, and its
    /// top face in metres.
    fn top_of_column(level: u32, x_m: f32, z_m: f32) -> (Voxel, f32) {
        // The terrain's own surface, so a dry World: a lake's top is water.
        let params = WorldgenParams {
            caves: None,
            water: None,
            ..crate::testgen::world_params()
        };
        let below = level - 1;
        // Level 0 keeps its stairs; the sources above it are cubes, as in the Server.
        let source = Worldgen::new(
            SEED,
            2f32.powi(below as i32),
            if below == 0 {
                params
            } else {
                WorldgenParams {
                    smoothing: None,
                    ..params
                }
            },
        );
        let voxel_m = 2f32.powi(level as i32);
        let cell_m = CHUNK_SIZE as f32 * voxel_m;
        let (lowest, highest) = source.surface_bounds_m();
        let (cx, cz) = ((x_m / cell_m).floor() as i32, (z_m / cell_m).floor() as i32);
        let (lx, lz) = (
            ((x_m - cx as f32 * cell_m) / voxel_m) as usize,
            ((z_m - cz as f32 * cell_m) / voxel_m) as usize,
        );
        let shapes = ShapeTable::builtin();
        let band = (lowest / cell_m).floor() as i32..=(highest / cell_m).floor() as i32;
        for cy in band.rev() {
            let children: Vec<Chunk> = (0..8)
                .map(|i| source.chunk(2 * cx + i % 2, 2 * cy + i / 2 % 2, 2 * cz + i / 4))
                .collect();
            let chunk = downsample(std::array::from_fn(|i| &children[i]), &shapes);
            for y in (0..CHUNK_SIZE).rev() {
                let v = chunk.get(lx, y, lz);
                if !v.is_air() {
                    let top_m = (cy * CHUNK_SIZE as i32 + y as i32 + 1) as f32 * voxel_m;
                    return (v, top_m);
                }
            }
        }
        panic!("no solid Voxel in the column at {x_m}, {z_m}, level {level}");
    }

    /// The bound decision 9 of the LOD spec rests on: a level's surface is within one of its
    /// own Voxels of the real one, so a seam is a step of at most that.
    #[test]
    fn the_coarse_surface_is_within_one_coarse_voxel_of_the_terrain_at_every_level() {
        let mut rng = Rng(0x5eed);
        let mut capped = [0usize; 5];
        for level in 1..=5u32 {
            let voxel_m = 2f32.powi(level as i32);
            let reach_m = 4096.0;
            for _ in 0..COLUMNS {
                let unit = |rng: &mut Rng| rng.below(1 << 20) as f32 / (1 << 20) as f32;
                let (x_m, z_m) = (
                    (unit(&mut rng) * 2.0 - 1.0) * reach_m,
                    (unit(&mut rng) * 2.0 - 1.0) * reach_m,
                );
                let (voxel, top_m) = top_of_column(level, x_m, z_m);
                // Terrain at the column's centre, which its Voxels estimate.
                let (cx, cz) = ((x_m / voxel_m).floor() + 0.5, (z_m / voxel_m).floor() + 0.5);
                let surface_m = Worldgen::new(
                    SEED,
                    voxel_m,
                    WorldgenParams {
                        caves: None,
                        ..crate::testgen::world_params()
                    },
                )
                .surface_height_m(cx * voxel_m, cz * voxel_m);
                let signed = (top_m - surface_m) / voxel_m;
                let error = signed.abs();
                assert!(
                    error <= 1.0,
                    "level {level} at {x_m}, {z_m}: top {top_m} m against terrain {surface_m} m"
                );
                // The surface Voxel is lost only when under half solid.
                assert!(
                    signed >= -0.5,
                    "level {level} at {x_m}, {z_m}: top {top_m} m lost its surface Voxel"
                );
                capped[level as usize - 1] +=
                    (voxel.material() == crate::testgen::fixtures().ground.surface) as usize;
            }
        }
        // Not every column: where the surface pokes one finer Voxel into the next coarse
        // Voxel up, that one is under half and the cap is lost. A tie broken by id instead
        // of height would lose it at half of all columns.
        for (i, &n) in capped.iter().enumerate() {
            assert!(
                n * 10 >= COLUMNS * 7,
                "level {}: grass tops {n}/{COLUMNS}",
                i + 1
            );
        }
    }

    #[test]
    fn translucent_materials_keep_their_id_and_count_as_occupied() {
        let t = table();
        for m in [fixtures().glass, fixtures().leaves, water()] {
            assert_eq!(coarse([cube(m); 8], &t), cube(m));
            let mut mostly = [cube(m); 8];
            mostly[..3].fill(cube(fixtures().ground.rock));
            assert_eq!(coarse(mostly, &t), cube(m), "five of eight");

            let mut pane = [Voxel::AIR; 8];
            pane[3] = cube(m);
            assert_eq!(coarse(pane, &t), Voxel::AIR, "one thick vanishes");

            let mut half = [Voxel::AIR; 8];
            half[..4].fill(cube(m));
            assert_eq!(coarse(half, &t), cube(m), "half the volume is solid");
        }

        let mut c = Chunk::air();
        c.set(10, 11, 12, cube(fixtures().glass));
        let air = Chunk::air();
        assert_eq!(
            downsample([&c, &air, &air, &air, &air, &air, &air, &air], &t).uniform(),
            Some(Voxel::AIR)
        );
    }

    #[test]
    fn child_i_is_octant_x_plus_2y_plus_4z() {
        let t = table();
        let children = split(|x, y, z| {
            if (32..36).contains(&x) && (4..6).contains(&y) && (38..40).contains(&z) {
                cube(fixtures().ground.rock)
            } else {
                Voxel::AIR
            }
        });
        let out = run(&children, &t);
        for cz in 0..CHUNK_SIZE {
            for cy in 0..CHUNK_SIZE {
                for cx in 0..CHUNK_SIZE {
                    let want = (16..18).contains(&cx) && cy == 2 && cz == 19;
                    assert_eq!(!out.get(cx, cy, cz).is_air(), want, "at {cx},{cy},{cz}");
                }
            }
        }
    }

    #[test]
    fn it_agrees_with_a_brute_force_lattice_over_random_children() {
        let t = table();
        for seed in 1..=12u64 {
            let children = random_children(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1);
            assert_matches_reference(&children, &t, &format!("seed {seed}"));
        }
    }

    #[test]
    fn every_orientation_of_every_shape_weighs_the_same() {
        let t = table();
        for shape in [SHAPE_SLAB, SHAPE_STAIR, SHAPE_POST] {
            let want = t.volume(Voxel::shaped(
                fixtures().ground.rock,
                shape,
                Orientation::IDENTITY,
            ));
            for o in Orientation::all() {
                let v = Voxel::shaped(fixtures().ground.rock, shape, o);
                assert_eq!(t.volume(v), want);
                assert_eq!(
                    coarse([v; 8], &t),
                    if 8 * want as u32 >= SOLID_AT {
                        cube(fixtures().ground.rock)
                    } else {
                        Voxel::AIR
                    }
                );
            }
        }
    }

    #[test]
    fn the_fast_paths_agree_with_the_general_loop() {
        let t = table();
        let uniform = [
            Voxel::AIR,
            Voxel::new(0, SHAPE_STAIR, 9),
            cube(fixtures().ground.rock),
            slab(),
            Voxel::shaped(fixtures().ground.soil, SHAPE_STAIR, Orientation::new(3, 2)),
            Voxel::shaped(fixtures().ground.rock, SHAPE_POST, Orientation::IDENTITY),
            Voxel::new(fixtures().ground.rock, FENCE_ID, 0),
        ];
        for v in uniform {
            let children = std::array::from_fn(|_| Chunk::filled(v));
            assert!(children.iter().all(|c| c.uniform().is_some()));
            assert_matches_reference(&children, &t, &format!("{v:?}"));
            assert!(run(&children, &t).uniform().is_some(), "{v:?}");
        }

        for a in uniform {
            for b in uniform {
                let mut children: [Chunk; 8] =
                    std::array::from_fn(|i| Chunk::filled(if i % 3 == 0 { a } else { b }));
                assert_matches_reference(&children, &t, &format!("{a:?} and {b:?}"));
                children[5].set(3, 4, 5, cube(water()));
                assert_matches_reference(&children, &t, &format!("{a:?}, {b:?} and a speck"));
            }
        }
    }

    /// The fixture water, and a translucent lava whose dimming is not water's.
    fn fixture() -> (Tables, [MaterialId; 2]) {
        let mut tables = Tables {
            shapes: table(),
            ..fixtures().tables.clone()
        };
        let water = fixtures().water;
        let lava = tables
            .materials
            .register(crate::MaterialDef {
                emission: 10,
                dimming: 4,
                ..crate::testgen::WATER_MATERIAL
            })
            .unwrap();
        let hot = crate::FluidDef {
            waterlogs: false,
            extinguishes: false,
            heat: Some(crate::Heat {
                strength: 10,
                reach_m: 2.0,
            }),
            ..crate::testgen::WATER_FLUID
        };
        tables.register_fluid(lava, hot).unwrap();
        (tables, [water, lava])
    }

    fn tables() -> Tables {
        fixture().0
    }

    fn water() -> MaterialId {
        fixture().1[0]
    }

    fn lava() -> MaterialId {
        fixture().1[1]
    }

    /// Every packed water Voxel counts as generated, so nothing is dynamic.
    fn drowned() -> Option<Waterline> {
        Some(Waterline {
            level_m: f32::MAX,
            voxel_m: 1.0,
            material: water(),
        })
    }

    fn changed<'a>(children: &'a [Chunk; 8], overlays: &'a [ChunkOverlay; 8]) -> [Child<'a>; 8] {
        std::array::from_fn(|i| Child::Changed(&children[i], &overlays[i]))
    }

    fn assert_same(a: &Chunk, b: &Chunk, what: &str) {
        for z in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    assert_eq!(a.get(x, y, z), b.get(x, y, z), "{what} at {x} {y} {z}");
                }
            }
        }
    }

    #[test]
    fn a_changed_child_with_nothing_beside_its_voxels_downsamples_as_a_packed_one() {
        let t = tables();
        let dry: [ChunkOverlay; 8] = Default::default();
        for seed in 0..6u64 {
            let children = random_children(seed.wrapping_mul(0x51_7cc1_b727_220a) | 1);
            let got = downsample_changed(changed(&children, &dry), 0, &t, drowned());
            assert_same(
                &got.chunk,
                &run(&children, &t.shapes),
                &format!("seed {seed}"),
            );
            assert_eq!(got.overlay.uniform(), Some(Overlay::NONE));
        }

        // Generated lake water at its own waterline is lake water, read either way.
        let generator = Worldgen::new(
            SEED,
            1.0,
            crate::testgen::world_params().with_default_lake(water()),
        );
        let level_m = generator.water().expect("the World has a lake").level_m;
        let waterline = Some(Waterline {
            level_m,
            voxel_m: 1.0,
            material: water(),
        });
        let mut lakes = 0;
        for (cx, cz) in [(0, 0), (3, -2), (-4, 5), (7, 7), (-6, -3), (2, 9)] {
            let cy = (level_m / 64.0).floor() as i32;
            let children: [Chunk; 8] = std::array::from_fn(|i| {
                let (x, y, z) = (i as i32 % 2, i as i32 / 2 % 2, i as i32 / 4);
                generator.chunk(2 * cx + x, 2 * cy + y, 2 * cz + z)
            });
            lakes += usize::from(children.iter().any(|c| {
                c.kinds().iter().any(|v| v.material() == water()) && c.uniform().is_none()
            }));
            let got = downsample_changed(changed(&children, &dry), cy, &t, waterline);
            assert_same(&got.chunk, &run(&children, &t.shapes), "lake");
            assert_eq!(got.overlay.uniform(), Some(Overlay::NONE), "lake");
        }
        assert!(lakes > 0, "no lake shore was compared");
    }

    /// The coarse Voxel at the origin of a level-1 Chunk whose child 0 holds
    /// `fine[j]` at octant `j` of its first eight Voxels, under `waterline`.
    fn one_cell(fine: [(Voxel, Overlay); 8], waterline: Option<Waterline>) -> (Voxel, Overlay) {
        let mut chunk = Chunk::air();
        let mut overlay = ChunkOverlay::none();
        for (j, (v, o)) in fine.into_iter().enumerate() {
            chunk.set(j % 2, j / 2 % 2, j / 4, v);
            overlay.set(j % 2, j / 2 % 2, j / 4, o);
        }
        let air = Chunk::air();
        let mut children = [Child::Packed(&air); 8];
        children[0] = Child::Changed(&chunk, &overlay);
        let got = downsample_changed(children, 0, &tables(), waterline);
        (got.chunk.get(0, 0, 0), got.overlay.get(0, 0, 0))
    }

    fn flowing(fluid: MaterialId, surface: u8) -> Overlay {
        Overlay {
            fluid: Fluid::flowing(
                fluid,
                [
                    Level {
                        surface,
                        reach: 3,
                        descent: 0,
                    },
                    Level::DRY,
                    Level::DRY,
                    Level::DRY,
                ],
            ),
            burning: None,
        }
    }

    fn standing(fluid: MaterialId, surface: u8) -> Overlay {
        Overlay {
            fluid: Fluid::flowing(
                fluid,
                [
                    Level {
                        surface,
                        ..Level::DRY
                    },
                    Level::DRY,
                    Level::DRY,
                    Level::DRY,
                ],
            ),
            burning: None,
        }
    }

    const DRY: (Voxel, Overlay) = (Voxel::AIR, Overlay::NONE);

    #[test]
    fn dynamic_fluid_stands_at_the_share_of_the_coarse_voxel_it_fills() {
        let full = (Voxel::AIR, flowing(water(), 16));
        assert_eq!(
            one_cell([full; 8], None),
            (Voxel::AIR, standing(water(), 16))
        );
        assert_eq!(
            one_cell([full, full, DRY, DRY, full, full, DRY, DRY], None),
            (Voxel::AIR, standing(water(), 8)),
            "the bottom half"
        );
        let film = (Voxel::AIR, flowing(water(), 4));
        assert_eq!(
            one_cell([film, DRY, DRY, DRY, DRY, DRY, DRY, DRY], None),
            (Voxel::AIR, standing(water(), 1)),
            "a sixteenth of a coarse Voxel still shows"
        );
        // Stone wins a coarse Voxel that is more stone than water, as it does
        // against a lake.
        let rock = (cube(fixtures().ground.rock), Overlay::NONE);
        assert_eq!(
            one_cell([rock, rock, rock, full, DRY, DRY, DRY, DRY], None),
            (cube(fixtures().ground.rock), Overlay::NONE)
        );
        // Stone under half is dropped and the water stands over what it filled.
        assert_eq!(
            one_cell([rock, rock, film, film, DRY, DRY, DRY, DRY], None),
            (Voxel::AIR, standing(water(), 5)),
        );
    }

    #[test]
    fn only_water_the_generator_could_have_made_stays_packed() {
        for (lake, other) in [(water(), lava()), (lava(), water())] {
            only_the_lake_stays_packed(lake, other);
        }
    }

    /// Packed `lake` under its waterline is generated; packed `other` there is a
    /// fluid that was placed, whatever the two are.
    fn only_the_lake_stays_packed(material: MaterialId, other: MaterialId) {
        let lake = (cube(material), Overlay::NONE);
        let at = |level_m| {
            Some(Waterline {
                level_m,
                voxel_m: 0.5,
                material,
            })
        };
        assert_eq!(
            one_cell([(cube(other), Overlay::NONE); 8], at(1.0)),
            (Voxel::AIR, standing(other, 16)),
            "another fluid under the waterline is dynamic"
        );
        assert_eq!(one_cell([lake; 8], at(1.0)), lake, "every bottom under 1 m");
        assert_eq!(
            one_cell([lake; 8], at(0.0)),
            (Voxel::AIR, standing(material, 16)),
            "a bottom at the waterline is above it"
        );
        assert_eq!(
            one_cell([lake; 8], None),
            (Voxel::AIR, standing(material, 16))
        );
        // Mostly lake: the lake's, and the correction for it still applies.
        let full = (Voxel::AIR, flowing(material, 16));
        let mut mixed = [lake; 8];
        mixed[7] = full;
        mixed[6] = full;
        mixed[5] = full;
        assert_eq!(one_cell(mixed, at(1.0)), lake);
        mixed[4] = full;
        assert_eq!(one_cell(mixed, at(1.0)), lake, "a tie is the lake's");
        mixed[3] = full;
        assert_eq!(
            one_cell(mixed, at(1.0)),
            (Voxel::AIR, standing(material, 16))
        );
    }

    #[test]
    fn waterlogged_water_weighs_what_it_fills_of_the_voids() {
        let slab = Voxel::shaped(fixtures().ground.rock, SHAPE_SLAB, Orientation::IDENTITY);
        let logged = (slab, standing(water(), 16));
        let film = (Voxel::AIR, flowing(water(), 4));
        // Four half-full slabs: stone 4 x 2048, water 4 x 2048 + 4 x 1024 above.
        assert_eq!(
            one_cell(
                [logged, logged, film, film, logged, logged, film, film],
                None
            ),
            (Voxel::AIR, standing(water(), 10))
        );
        // A stair's notch holds a quarter of the cube: stone wins.
        let stair = Voxel::shaped(fixtures().ground.rock, SHAPE_STAIR, Orientation::IDENTITY);
        let notch = Overlay {
            fluid: Some(Fluid::source(water())),
            burning: None,
        };
        assert_eq!(
            one_cell([(stair, notch); 8], None),
            (cube(fixtures().ground.rock), Overlay::NONE)
        );
    }

    #[test]
    fn a_coarse_solid_burns_when_half_its_material_does() {
        let fire = |fuel| Overlay {
            fluid: None,
            burning: Some(Burning { fuel }),
        };
        let burning = |fuel| (cube(fixtures().ground.rock), fire(fuel));
        let cold = (cube(fixtures().ground.rock), Overlay::NONE);
        assert_eq!(
            one_cell(
                [
                    burning(3),
                    burning(9),
                    burning(1),
                    burning(2),
                    cold,
                    cold,
                    cold,
                    cold
                ],
                None
            ),
            (cube(fixtures().ground.rock), fire(9))
        );
        assert_eq!(
            one_cell(
                [
                    burning(3),
                    burning(9),
                    burning(1),
                    cold,
                    cold,
                    cold,
                    cold,
                    cold
                ],
                None
            ),
            (cube(fixtures().ground.rock), Overlay::NONE)
        );
        // A fire too small to keep its Voxel solid is gone with it.
        assert_eq!(
            one_cell(
                [burning(3), burning(9), burning(1), DRY, DRY, DRY, DRY, DRY],
                None
            ),
            DRY
        );
    }

    /// A flat heightfield that counts how often it is asked.
    struct Flat {
        height_m: f32,
        samples: std::cell::Cell<usize>,
    }

    impl Flat {
        fn at(height_m: f32) -> Self {
            Flat {
                height_m,
                samples: std::cell::Cell::new(0),
            }
        }
    }

    impl Surface for Flat {
        fn height_m(&self, _x: f32, _z: f32) -> f32 {
            self.samples.set(self.samples.get() + 1);
            self.height_m
        }

        fn bounds_m(&self) -> (f32, f32) {
            (self.height_m, self.height_m)
        }
    }

    const AIR_CHUNK: Chunk = Chunk::filled(Voxel::AIR);

    fn sky(light: &ChunkLight, x: usize, y: usize, z: usize) -> u8 {
        light.get(x, y, z).sky()
    }

    /// 2 m Voxels, a flat surface at 40 m, rock under 20 m: the air Voxels from 20 m up
    /// to 38 m are covered and the one whose top is at 38 m is the first that is not.
    #[test]
    fn air_more_than_a_voxel_under_the_surface_is_covered_and_the_voxel_at_it_is_not() {
        let mut center = Chunk::filled(Voxel::AIR);
        for y in 0..10 {
            for x in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    center.set(x, y, z, Voxel::cube(fixtures().ground.rock));
                }
            }
        }
        let surface = Flat::at(40.0);
        let lit = coarse_light(&surface, 2.0, [0, 0, 0], &center, [&AIR_CHUNK; 6]);
        for y in 10..CHUNK_SIZE {
            let top_m = (y + 1) as f32 * 2.0;
            let want = if top_m < 40.0 - 2.0 { 0 } else { MAX_LIGHT };
            assert_eq!(sky(&lit.center, 5, y, 7), want, "y {y}, top {top_m} m");
        }
    }

    /// A sloped, rippled heightfield, so the lattice differs from column to column.
    struct Hills;

    impl Surface for Hills {
        fn height_m(&self, x: f32, z: f32) -> f32 {
            48.0 + 0.3 * x + 4.0 * (z * 0.35).sin()
        }

        fn bounds_m(&self) -> (f32, f32) {
            (f32::MIN, f32::MAX)
        }
    }

    /// The same Chunk wherever it is asked for: a third of its Voxels are rock, by a hash of
    /// where they are.
    fn pocked(coord: [i32; 3]) -> Chunk {
        let mut chunk = Chunk::filled(Voxel::AIR);
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let h = (coord[0] * 7919 + x as i32)
                        .wrapping_mul(31)
                        .wrapping_add(coord[1] * 104_729 + y as i32)
                        .wrapping_mul(31)
                        .wrapping_add(coord[2] * 15_485_863 + z as i32)
                        .wrapping_mul(2_654_435_761u32 as i32);
                    if (h >> 7).rem_euclid(3) == 0 {
                        chunk.set(x, y, z, Voxel::cube(fixtures().ground.rock));
                    }
                }
            }
        }
        chunk
    }

    /// The mesher reads a neighbour's light on the plane it shares with the Chunk, so that
    /// plane must say what the neighbour would say of itself.
    #[test]
    fn a_neighbours_shared_plane_agrees_with_the_neighbour_lit_on_its_own() {
        use crate::mesh::FACE_STEPS;
        let (voxel_m, coord) = (1.0, [0, 1, 0]);
        let at = |step: [i32; 3]| [0, 1, 2].map(|a| coord[a] + step[a]);
        let chunks: Vec<Chunk> = FACE_STEPS.iter().map(|&s| pocked(at(s))).collect();
        let lit = coarse_light(
            &Hills,
            voxel_m,
            coord,
            &pocked(coord),
            std::array::from_fn(|f| &chunks[f]),
        );
        let last = CHUNK_SIZE - 1;
        let mut covered = 0;
        for (face, step) in FACE_STEPS.iter().enumerate() {
            let other = at(*step);
            let around: Vec<Chunk> = FACE_STEPS
                .iter()
                .map(|s| pocked([0, 1, 2].map(|a| other[a] + s[a])))
                .collect();
            let alone = coarse_light(
                &Hills,
                voxel_m,
                other,
                &chunks[face],
                std::array::from_fn(|f| &around[f]),
            );
            for a in 0..CHUNK_SIZE {
                for b in 0..CHUNK_SIZE {
                    let (x, y, z) = match face {
                        0 => (last, a, b),
                        1 => (0, a, b),
                        2 => (a, last, b),
                        3 => (a, 0, b),
                        4 => (a, b, last),
                        _ => (a, b, 0),
                    };
                    assert_eq!(
                        lit.around[face].get(x, y, z),
                        alone.center.get(x, y, z),
                        "face {face} at ({x}, {y}, {z})"
                    );
                    covered += usize::from(alone.center.get(x, y, z) == COVERED_SKY);
                }
            }
        }
        assert!(covered > 100, "only {covered} covered on the shared planes");
    }

    /// The Voxels of a neighbour's light that the mesher reads: the plane it shares with the
    /// Chunk across `face`.
    fn shared_plane(light: &ChunkLight, face: usize) -> Vec<Light> {
        let last = CHUNK_SIZE - 1;
        (0..CHUNK_SIZE)
            .flat_map(|a| (0..CHUNK_SIZE).map(move |b| (a, b)))
            .map(|(a, b)| {
                let (x, y, z) = match face {
                    0 => (last, a, b),
                    1 => (0, a, b),
                    2 => (a, last, b),
                    3 => (a, 0, b),
                    4 => (a, b, last),
                    _ => (a, b, 0),
                };
                light.get(x, y, z)
            })
            .collect()
    }

    fn assert_all_uniform(lit: &CoarseLight, want: Light) {
        assert_eq!(lit.center.uniform(), Some(want));
        for (face, plane) in lit.around.iter().enumerate() {
            assert_eq!(plane.uniform(), Some(want), "face {face}");
        }
    }

    #[test]
    fn a_chunk_wholly_above_the_band_is_open_sky_and_asks_for_no_height() {
        let surface = Flat::at(40.0);
        let lit = coarse_light(
            &surface,
            2.0,
            [0, 1, 0],
            &pocked([0, 1, 0]),
            [&AIR_CHUNK; 6],
        );
        assert_all_uniform(&lit, OPEN_SKY);
        assert_eq!(surface.samples.get(), 0);
    }

    #[test]
    fn seven_solid_chunks_are_open_sky_and_ask_for_no_height() {
        let rock = Chunk::filled(Voxel::cube(fixtures().ground.rock));
        let surface = Flat::at(40.0);
        let lit = coarse_light(&surface, 2.0, [0, 0, 0], &rock, [&rock; 6]);
        assert_all_uniform(&lit, OPEN_SKY);
        assert_eq!(surface.samples.get(), 0);
    }

    #[test]
    fn air_wholly_under_the_band_is_covered_throughout_and_asks_for_no_height() {
        let surface = Flat::at(40.0);
        let lit = coarse_light(&surface, 2.0, [0, -2, 0], &AIR_CHUNK, [&AIR_CHUNK; 6]);
        assert_all_uniform(&lit, COVERED_SKY);
        assert_eq!(surface.samples.get(), 0);
    }

    /// The fast paths decide from the band alone, so they must give what sampling gives.
    #[test]
    fn the_fast_paths_agree_with_sampling() {
        // The same heights through a stub whose band is unknown, so nothing is skipped.
        struct Unbounded(Flat);
        impl Surface for Unbounded {
            fn height_m(&self, x: f32, z: f32) -> f32 {
                self.0.height_m(x, z)
            }
            fn bounds_m(&self) -> (f32, f32) {
                (f32::MIN, f32::MAX)
            }
        }
        let rock = Chunk::filled(Voxel::cube(fixtures().ground.rock));
        // The last two sit a fraction of a Voxel either side of the band's two edges, where a
        // bound a Voxel out of place turns a plane of the shell the wrong way.
        let cases: [(f32, i32, &Chunk, &Chunk); 5] = [
            (2.0, 1, &AIR_CHUNK, &AIR_CHUNK),
            (2.0, 0, &rock, &rock),
            (2.0, -2, &AIR_CHUNK, &AIR_CHUNK),
            (1.2, 1, &AIR_CHUNK, &AIR_CHUNK),
            (1.2, 0, &AIR_CHUNK, &AIR_CHUNK),
        ];
        for (voxel_m, cy, centre, around) in cases {
            let fast = coarse_light(&Flat::at(40.0), voxel_m, [0, cy, 0], centre, [around; 6]);
            let slow = coarse_light(
                &Unbounded(Flat::at(40.0)),
                voxel_m,
                [0, cy, 0],
                centre,
                [around; 6],
            );
            assert_eq!(fast.center, slow.center, "centre at y {cy}, {voxel_m} m");
            for face in 0..6 {
                assert_eq!(
                    shared_plane(&fast.around[face], face),
                    shared_plane(&slow.around[face], face),
                    "face {face} at y {cy}, {voxel_m} m"
                );
            }
        }
    }

    /// Falls one metre for every metre in +x, so a Voxel's two corners differ by two.
    struct Ramp;

    impl Surface for Ramp {
        fn height_m(&self, x: f32, _z: f32) -> f32 {
            60.0 - x
        }

        fn bounds_m(&self) -> (f32, f32) {
            (f32::MIN, f32::MAX)
        }
    }

    /// On a slope the footprint's lowest corner is the surface the Voxel has to be under, or a
    /// Voxel at the foot of a slope reads as open ground it cannot see.
    #[test]
    fn the_lowest_corner_of_the_footprint_decides() {
        let lit = coarse_light(&Ramp, 2.0, [0, 0, 0], &AIR_CHUNK, [&AIR_CHUNK; 6]);
        // Column 5 spans x = 10 to 12 m, so its lowest corner is 48 m and a Voxel is covered
        // if its top is under 46 m. The lower corner alone, at 50 m, would say 48 m.
        assert_eq!(sky(&lit.center, 5, 21, 0), 0, "top 44 m is under 46 m");
        assert_eq!(sky(&lit.center, 5, 22, 0), MAX_LIGHT, "top 46 m is not");
        assert_eq!(sky(&lit.center, 5, 23, 0), MAX_LIGHT);
    }

    /// The surface is the heightfield's, before caves, so a pit that is open to the sky is dark
    /// below its top Voxel here and lit at level 0. A column scan of the Chunks above would
    /// fix it; when one does, this test flips.
    #[test]
    fn a_pit_open_to_the_sky_is_covered_below_its_top_voxel() {
        let mut center = Chunk::filled(Voxel::cube(fixtures().ground.rock));
        for y in 5..CHUNK_SIZE {
            center.set(8, y, 8, Voxel::AIR);
        }
        let lit = coarse_light(&Flat::at(40.0), 2.0, [0, 0, 0], &center, [&AIR_CHUNK; 6]);
        assert_eq!(
            sky(&lit.center, 8, 10, 8),
            0,
            "a pit's floor is lit at level 0"
        );
        assert_eq!(sky(&lit.center, 8, 31, 8), MAX_LIGHT);
    }

    /// Level 0's own flood, as the truth: a column of water `depth` Voxels deep
    /// over stone under open sky, and the sky light in each water Voxel, top
    /// first.
    fn level_zero_water(material: MaterialId, depth: usize) -> Vec<u8> {
        let mut c = Chunk::air();
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for y in 0..4 {
                    c.set(x, y, z, Voxel::cube(fixtures().ground.rock));
                }
                for y in 4..4 + depth {
                    c.set(x, y, z, Voxel::cube(material));
                }
            }
        }
        let sky = ChunkLight::filled(OPEN_SKY);
        let air = Chunk::air();
        let mut border = [None; 6];
        border[POS_Y] = Some(crate::mesh::LitChunk::new(&air, &sky));
        let light = crate::light::light_chunk(&c, border, &tables());
        (4..4 + depth)
            .rev()
            .map(|y| light.get(16, y, 16).sky())
            .collect()
    }

    #[test]
    fn coarse_water_is_dimmed_by_its_depth_as_the_level_zero_flood_dims_it() {
        for material in [water(), lava()] {
            coarse_lake_is_dimmed_as_level_zero(material);
        }
    }

    fn coarse_lake_is_dimmed_as_level_zero(material: MaterialId) {
        let truth = level_zero_water(material, 10);
        // Level 1, 2 m Voxels: stone to 30 m, water from 30 m to 40 m, air above.
        let mut c = Chunk::air();
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for y in 0..15 {
                    c.set(x, y, z, Voxel::cube(fixtures().ground.rock));
                }
                for y in 15..20 {
                    c.set(x, y, z, Voxel::cube(material));
                }
            }
        }
        let water = UnderWater {
            level_m: 40.0,
            material,
            dimming: tables().materials.dimming(Voxel::cube(material)),
            step_m: 1.0,
        };
        let lit = coarse_light_in(
            &Flat::at(30.0),
            2.0,
            [0, 0, 0],
            &c,
            [&AIR_CHUNK; 6],
            Some(water),
        );
        // A coarse Voxel's bottom is `2 * (20 - y)` level-0 Voxels under the surface,
        // and level 0's Voxel there is the `2 * (20 - y)`th from the top.
        for y in 15..20 {
            let from_top = 2 * (20 - y) - 1;
            assert_eq!(
                lit.center.get(16, y, 16).sky(),
                truth[from_top],
                "coarse row {y}"
            );
        }
        assert_eq!(lit.center.get(16, 20, 16), OPEN_SKY, "air over the water");
        assert_eq!(
            coarse_light(&Flat::at(30.0), 2.0, [0, 0, 0], &c, [&AIR_CHUNK; 6])
                .center
                .get(16, 17, 16),
            OPEN_SKY,
            "a World without water is lit as before"
        );
    }
}

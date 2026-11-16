//! Generator version 3's terrain: a vertical band around Sea level inside which every field lives.
//! See `docs/adr/0030-generator-version-3-is-a-layered-field-stack-in-a-declared-band.md`.

use std::sync::Arc;

use crate::climate::{BiomeTable, ClimateField, MAX_BIOMES, Site};
use crate::drainage::Drainage;

/// The first Generator version whose terrain is the layered field stack.
pub const LANDFORMS_SINCE: u32 = 3;

/// What a Game tunes of version 3's terrain, in metres. Plain `Copy` data like the rest of
/// [`crate::WorldgenParams`]; read only by version 3.
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct LandformParams {
    /// How far the sea floor may fall below Sea level.
    pub depth_m: f32,
    /// How far the land may rise above Sea level; a peak past it is clamped.
    pub height_m: f32,
    /// How far the 3D density may push solid or air past the base surface.
    pub overhang_m: f32,
    /// The horizontal period of the continent field: how far apart two oceans are.
    pub continent_size_m: f32,
    /// Whether the 3D density runs. Off for the coarse levels that generate columns, which draw
    /// the heightfield alone.
    pub overhangs: bool,
}

impl Default for LandformParams {
    fn default() -> Self {
        LandformParams {
            depth_m: 96.0,
            height_m: 192.0,
            overhang_m: 12.0,
            continent_size_m: 6000.0,
            overhangs: true,
        }
    }
}

/// The fields version 3 reads at a column, each a pure function of the seed and a position in
/// metres.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Fields {
    /// 0 deep ocean to 1 far inland.
    pub continent: f32,
    /// 0 rugged to 1 worn flat.
    pub erosion: f32,
    /// 0 flat to 1 mountainous: where the 3D density may add overhangs.
    pub ruggedness: f32,
}

const SALT_WARP: u32 = 0x1a4d_0001;
const SALT_CONTINENT: u32 = 0x1a4d_0011;
const SALT_EROSION: u32 = 0x1a4d_0021;
const SALT_PEAKS: u32 = 0x1a4d_0031;
const SALT_DETAIL: u32 = 0x1a4d_0041;
const SALT_PLATEAU: u32 = 0x1a4d_0051;
const SALT_CANYON: u32 = 0x1a4d_0061;
const SALT_CANYON_LINE: u32 = 0x1a4d_0071;
const SALT_ISLANDS: u32 = 0x1a4d_0081;
const SALT_VOLCANO: u32 = 0x1a4d_0091;
const SALT_OVERHANG: u32 = 0x1a4d_00a1;
const SALT_CAVERN: u32 = 0x1a4d_00c1;
const SALT_RAVINE: u32 = 0x1a4d_00d1;
const SALT_RAVINE_MASK: u32 = 0x1a4d_00d2;

/// The continent field's value at the coast, and how many metres one unit of it is worth inland:
/// an approximation of distance, good enough to place a Biome.
const COAST_CONTINENT: f32 = 0.47;
const COAST_M_PER_UNIT: f32 = 0.4;
/// Temperature (a unit-square coordinate) lost per metre above Sea level.
const LAPSE_PER_M: f32 = 0.0016;
/// Humidity gained at the coast, fading to nothing this far inland.
const COAST_WET: f32 = 0.3;
const COAST_WET_REACH_M: f32 = 1800.0;

/// How the Climate of the unit square moves with altitude and the coast: `(temperature, humidity)`.
pub(crate) fn climate_shift(altitude_m: f32, coast_m: f32) -> (f32, f32) {
    (
        -LAPSE_PER_M * altitude_m.max(0.0),
        COAST_WET * (1.0 - smooth(0.0, COAST_WET_REACH_M, coast_m)),
    )
}

/// The wavelength of the density noise, in metres: arches and ledges a few Voxels thick.
const OVERHANG_SIZE_M: f32 = 10.0;

/// Continent field to base height, as fractions of the sea floor depth below Sea level (negative
/// knots) and of the land height above it (positive): a deep floor, a slope, a shelf, a coast
/// and then rising inland.
const CONTINENT_CURVE: [(f32, f32); 8] = [
    (0.00, -0.95),
    (0.20, -0.88),
    (0.32, -0.40),
    (0.40, -0.10),
    (0.47, 0.0),
    (0.53, 0.02),
    (0.70, 0.08),
    (1.00, 0.14),
];

const WARP_SIZE_M: f32 = 1500.0;
const WARP_AMOUNT_M: f32 = 400.0;
const EROSION_SIZE_M: f32 = 2800.0;
const PEAKS_SIZE_M: f32 = 1400.0;
const MESA_STEP_M: f32 = 18.0;
const VOLCANO_CELL_M: f32 = 5000.0;
const VOLCANO_RADIUS_M: f32 = 700.0;

/// Version 3's base ground: Sea level, the band, and the detail octaves a Game tunes with
/// `feature_size_m`, `octaves` and `relief_m`.
#[derive(Clone, Debug)]
struct Base {
    seed: u32,
    sea_m: f32,
    depth_m: f32,
    height_m: f32,
    continent_size_m: f32,
    inv_feature_size: f32,
    octaves: u32,
    relief_m: f32,
    overhang_m: f32,
    /// Present only when a Biome declares a relief scale.
    relief_by_biome: Option<(ClimateField, Arc<BiomeTable>, [f32; MAX_BIOMES])>,
}

#[inline]
fn smooth(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn curve(points: &[(f32, f32)], t: f32) -> f32 {
    let i = points
        .partition_point(|p| p.0 <= t)
        .clamp(1, points.len() - 1);
    let ((t0, v0), (t1, v1)) = (points[i - 1], points[i]);
    v0 + (v1 - v0) * smooth(t0, t1, t)
}

impl Base {
    fn new(seed: u32, p: &crate::WorldgenParams) -> Self {
        Base {
            seed,
            sea_m: p.sea_level_m,
            depth_m: p.landforms.depth_m,
            height_m: p.landforms.height_m,
            continent_size_m: p.landforms.continent_size_m,
            inv_feature_size: 1.0 / p.feature_size_m,
            octaves: p.octaves,
            relief_m: p.relief_m,
            relief_by_biome: p.biomes.shapes_relief().then(|| {
                let mut offsets = [0.0; MAX_BIOMES];
                for (o, s) in offsets.iter_mut().zip(p.biomes.selectors()) {
                    *o = s.relief.unwrap_or(1.0) - 1.0;
                }
                (
                    ClimateField::new(seed, p.climate),
                    Arc::new(p.biomes),
                    offsets,
                )
            }),
            overhang_m: if p.landforms.overhangs {
                p.landforms.overhang_m
            } else {
                0.0
            },
        }
    }

    fn warped(&self, x_m: f32, z_m: f32) -> (f32, f32) {
        crate::noise::warp2(
            x_m,
            z_m,
            self.seed.wrapping_add(SALT_WARP),
            WARP_SIZE_M,
            WARP_AMOUNT_M,
        )
    }

    fn fields(&self, x_m: f32, z_m: f32) -> Fields {
        let (wx, wz) = self.warped(x_m, z_m);
        self.fields_warped(wx, wz)
    }

    fn fields_warped(&self, wx: f32, wz: f32) -> Fields {
        let size = self.continent_size_m;
        let continent = crate::noise::fbm2(
            wx / size,
            wz / size,
            self.seed.wrapping_add(SALT_CONTINENT),
            4,
        );
        // fBm bunches around a half, so a contrast curve spreads the fields across their range.
        let erosion = smooth(
            0.25,
            0.75,
            crate::noise::fbm2(
                wx / EROSION_SIZE_M,
                wz / EROSION_SIZE_M,
                self.seed.wrapping_add(SALT_EROSION),
                3,
            ),
        );
        let inland = smooth(0.50, 0.66, continent);
        Fields {
            continent,
            erosion,
            ruggedness: inland * (1.0 - erosion).powf(1.5),
        }
    }

    /// The ground's height in metres before any river carves it.
    fn base_height_m(&self, x_m: f32, z_m: f32) -> f32 {
        let seed = self.seed;
        let (wx, wz) = self.warped(x_m, z_m);
        let f = self.fields_warped(wx, wz);
        let inland = smooth(0.50, 0.66, f.continent);
        let (d, h) = (self.depth_m, self.height_m);

        let base = curve(&CONTINENT_CURVE, f.continent);
        let mut offset = if base < 0.0 { base * d } else { base * h };

        let ridge = crate::noise::ridged2(
            wx / PEAKS_SIZE_M,
            wz / PEAKS_SIZE_M,
            seed.wrapping_add(SALT_PEAKS),
            4,
        );
        offset += h * 0.8 * ridge * ridge * f.ruggedness;

        let n = crate::noise::fbm2(
            x_m * self.inv_feature_size,
            z_m * self.inv_feature_size,
            seed.wrapping_add(SALT_DETAIL),
            self.octaves,
        );
        let worn = 0.35 + 0.65 * (1.0 - f.erosion);
        let scale = self
            .relief_by_biome
            .as_ref()
            .map_or(1.0, |(climate, biomes, offsets)| {
                (1.0 + climate.blend(biomes, x_m, z_m, offsets)).max(0.0)
            });
        offset += (n - 0.5) * self.relief_m * scale * if base < 0.0 { 0.3 } else { worn };

        let plateau =
            crate::noise::fbm2(wx / 3500.0, wz / 3500.0, seed.wrapping_add(SALT_PLATEAU), 2);
        let mesa = smooth(0.58, 0.68, plateau) * inland * f.erosion;
        if mesa > 0.0 {
            let above = offset.max(0.0);
            let step = (above / MESA_STEP_M).floor();
            let frac = above / MESA_STEP_M - step;
            let terraced = (step + smooth(0.75, 1.0, frac)) * MESA_STEP_M;
            offset += (terraced - above) * mesa;
        }

        let canyon =
            crate::noise::fbm2(wx / 2500.0, wz / 2500.0, seed.wrapping_add(SALT_CANYON), 2);
        let line = crate::noise::ridged2(
            wx / 600.0,
            wz / 600.0,
            seed.wrapping_add(SALT_CANYON_LINE),
            2,
        );
        offset -= 70.0
            * smooth(0.72, 0.82, canyon)
            * inland
            * smooth(0.88, 0.97, line)
            * offset.clamp(0.0, 70.0)
            / 70.0;

        if base < 0.0 {
            let band = smooth(0.28, 0.34, f.continent) * (1.0 - smooth(0.40, 0.46, f.continent));
            let islands = smooth(
                0.62,
                0.72,
                crate::noise::fbm2(wx / 500.0, wz / 500.0, seed.wrapping_add(SALT_ISLANDS), 3),
            );
            offset += band * islands * (0.35 * d + 8.0);
        }

        // Narrow slots cut deep into rugged land: ridged noise, thresholded near its crest.
        let slot = crate::noise::ridged2(wx / 450.0, wz / 450.0, seed.wrapping_add(SALT_RAVINE), 2);
        let ravines = smooth(
            0.58,
            0.68,
            crate::noise::fbm2(
                wx / 3000.0,
                wz / 3000.0,
                seed.wrapping_add(SALT_RAVINE_MASK),
                2,
            ),
        );
        offset -= 45.0 * ravines * inland * smooth(0.975, 0.995, slot) * smooth(8.0, 30.0, offset);

        offset += self.volcano(x_m, z_m, inland);

        self.sea_m + offset.clamp(-d, h)
    }

    /// The most the density can push the surface anywhere, which widens the early-out bounds.
    fn overhang_max_m(&self) -> f32 {
        self.overhang_m
    }

    /// How far, in metres, the 3D density pushes the surface up or down over a column: zero on
    /// flat ground, up to `overhang_m` on rugged.
    fn overhang_amp_m(&self, x_m: f32, z_m: f32) -> f32 {
        if self.overhang_m <= 0.0 {
            return 0.0;
        }
        self.overhang_m * smooth(0.2, 0.55, self.fields(x_m, z_m).ruggedness)
    }

    /// The signed push of the density at a point, in `-amp_m..amp_m`: positive is solid.
    fn overhang_push_m(&self, x_m: f32, y_m: f32, z_m: f32, amp_m: f32) -> f32 {
        let n = crate::noise::fbm3(
            x_m / OVERHANG_SIZE_M,
            y_m / OVERHANG_SIZE_M,
            z_m / OVERHANG_SIZE_M,
            self.seed.wrapping_add(SALT_OVERHANG),
            2,
        );
        ((n * 2.0 - 1.0) * 2.0).clamp(-1.0, 1.0) * amp_m
    }

    /// A cone at a hashed spot of some 5 km cells, over land only.
    fn volcano(&self, x_m: f32, z_m: f32, inland: f32) -> f32 {
        if inland <= 0.0 {
            return 0.0;
        }
        let (cx, cz) = (
            (x_m / VOLCANO_CELL_M).floor() as i32,
            (z_m / VOLCANO_CELL_M).floor() as i32,
        );
        let mut best = 0.0f32;
        for dz in -1..=1 {
            for dx in -1..=1 {
                let (ix, iz) = (cx + dx, cz + dz);
                let h = crate::noise::cell_hash(ix, iz, self.seed.wrapping_add(SALT_VOLCANO));
                if h[0] > 0.22 {
                    continue;
                }
                let (px, pz) = (
                    (ix as f32 + 0.2 + 0.6 * h[1]) * VOLCANO_CELL_M,
                    (iz as f32 + 0.2 + 0.6 * h[2]) * VOLCANO_CELL_M,
                );
                let r = ((x_m - px).powi(2) + (z_m - pz).powi(2)).sqrt();
                if r >= VOLCANO_RADIUS_M {
                    continue;
                }
                let cone = (1.0 - r / VOLCANO_RADIUS_M).powf(1.3) * self.height_m * 0.55;
                let crater = 40.0 * (1.0 - smooth(0.0, 90.0, r));
                best = best.max(cone - crater);
            }
        }
        best * inland
    }
}

/// Version 3's ground and water: the base landforms, and the lakes and rivers drained over them.
#[derive(Clone)]
pub(crate) struct Terrain {
    base: Base,
    drainage: Arc<Drainage>,
}

impl Terrain {
    pub(crate) fn new(seed: u32, p: &crate::WorldgenParams) -> Self {
        let base = Base::new(seed, p);
        Terrain {
            base: base.clone(),
            drainage: Arc::new(Drainage::new(p.sea_level_m, {
                let base = base.clone();
                move |x, z| base.base_height_m(x, z)
            })),
        }
    }

    pub(crate) fn fields(&self, x_m: f32, z_m: f32) -> Fields {
        self.base.fields(x_m, z_m)
    }

    /// The ground's height in metres: inside `sea - depth ..= sea + height`, with rivers' valleys
    /// cut into it.
    pub(crate) fn height_m(&self, x_m: f32, z_m: f32) -> f32 {
        let h = self.base.base_height_m(x_m, z_m);
        match self.drainage.wet(x_m, z_m).valley {
            Some(v) if v.ceiling_m < h => h - (h - v.ceiling_m) * v.fade,
            _ => h,
        }
    }

    /// The level lake or river water stands at over a column, if any reaches it; the sea is the
    /// caller's.
    pub(crate) fn inland_water_m(&self, x_m: f32, z_m: f32) -> Option<f32> {
        self.drainage.wet(x_m, z_m).level_m
    }

    /// What a Biome's selectors ask of the column, and how its Climate shifts there: the air is
    /// colder by altitude and wetter toward the sea.
    pub(crate) fn site(&self, x_m: f32, z_m: f32) -> (Site, (f32, f32)) {
        let f = self.base.fields(x_m, z_m);
        let altitude_m = self.height_m(x_m, z_m) - self.base.sea_m;
        let coast_m =
            (f.continent - COAST_CONTINENT) * self.base.continent_size_m * COAST_M_PER_UNIT;
        (
            Site {
                altitude_m,
                coast_m,
                erosion: f.erosion,
            },
            climate_shift(altitude_m, coast_m),
        )
    }

    pub(crate) fn overhang_max_m(&self) -> f32 {
        self.base.overhang_max_m()
    }

    pub(crate) fn overhang_amp_m(&self, x_m: f32, z_m: f32) -> f32 {
        self.base.overhang_amp_m(x_m, z_m)
    }

    pub(crate) fn overhang_push_m(&self, x_m: f32, y_m: f32, z_m: f32, amp_m: f32) -> f32 {
        self.base.overhang_push_m(x_m, y_m, z_m, amp_m)
    }

    pub(crate) fn forget_regions(&self) {
        self.drainage.forget();
    }
}

impl std::fmt::Debug for Terrain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Terrain")
            .field("base", &self.base)
            .finish_non_exhaustive()
    }
}

/// Large chambers in the rock, a thresholded low-frequency 3D field squashed vertically. They stay
/// a depth under the ground so none breaches the surface, and above the band's floor so the
/// uniform deep rock below it stays uniform.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Caverns {
    seed: u32,
    /// No cavern below this height, the band's floor.
    pub(crate) floor_m: f32,
}

/// The field value above which rock is cavern.
const CAVERN_CUT: f32 = 0.67;
const CAVERN_SIZE_M: [f32; 3] = [200.0, 90.0, 200.0];
/// A cavern's roof stays this far under the ground.
pub(crate) const CAVERN_MIN_DEPTH_M: f32 = 22.0;

impl Caverns {
    pub(crate) fn new(seed: u32, p: &crate::WorldgenParams) -> Self {
        Caverns {
            seed: seed.wrapping_add(SALT_CAVERN),
            floor_m: p.sea_level_m - p.landforms.depth_m - p.landforms.overhang_m,
        }
    }

    pub(crate) fn carves(&self, x_m: f32, y_m: f32, z_m: f32) -> bool {
        y_m >= self.floor_m
            && crate::noise::fbm3(
                x_m / CAVERN_SIZE_M[0],
                y_m / CAVERN_SIZE_M[1],
                z_m / CAVERN_SIZE_M[2],
                self.seed,
                2,
            ) > CAVERN_CUT
    }

    /// Whether anything in the box `lo..=hi` (metres) could be a cavern; `false` is a proof.
    pub(crate) fn may_carve(&self, lo: [f32; 3], hi: [f32; 3]) -> bool {
        if hi[1] < self.floor_m {
            return false;
        }
        let scale = |p: [f32; 3]| [0, 1, 2].map(|a| p[a] / CAVERN_SIZE_M[a]);
        crate::noise::fbm3_range(scale(lo), scale(hi), self.seed, 2).1 > CAVERN_CUT
    }
}

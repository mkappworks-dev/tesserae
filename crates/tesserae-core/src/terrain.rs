//! The terrain: a vertical band around Sea level inside which every field lives.
//! See `docs/adr/0030-generator-version-3-is-a-layered-field-stack-in-a-declared-band.md`.

use std::sync::Arc;

use wide::f32x4;

use crate::climate::{BiomeTable, ClimateField, MAX_BIOMES, Site};
use crate::drainage::Drainage;
use crate::noise::{Fbm3Cache, Lanes};

/// What a Game tunes of version 3's terrain, in metres. Plain `Copy` data like the rest of
/// [`crate::WorldgenParams`].
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
const SALT_SHAFT: u32 = 0x1a4d_00e1;
const SALT_CALM: u32 = 0x1a4d_00f1;
const SALT_CRAG: u32 = 0x1a4d_0101;
const SALT_TERRACE: u32 = 0x1a4d_0111;
const SALT_OUTCROP: u32 = 0x1a4d_0121;
const SALT_SEABED: u32 = 0x1a4d_0131;

/// The continent field's value at the coast, and how many metres one unit of it is worth inland:
/// an approximation of distance, good enough to place a Biome.
const COAST_CONTINENT: f32 = 0.47;
const COAST_M_PER_UNIT: f32 = 0.4;
/// Temperature (a unit-square coordinate) lost per metre above Sea level.
const LAPSE_PER_M: f32 = 0.0016;
/// Humidity gained at the coast, fading to nothing this far inland.
const COAST_WET: f32 = 0.1;
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
/// The crags on a range's upper flanks and summits: their spacing, and their height as a share
/// of the land's.
const CRAG_SIZE_M: f32 = 100.0;
const CRAG_HEIGHT: f32 = 0.22;
/// A range's benches: the rise from one to the next, the share of each step that is level, how
/// far the benches drift before they meet as a ramp, and how fully the rise is stepped.
const TERRACE_STEP_M: f32 = 10.0;
const TERRACE_BENCH: f32 = 0.4;
const TERRACE_DRIFT_M: f32 = 260.0;
const TERRACE_STRENGTH: f32 = 0.8;
/// The overhang density's outcrops: how far apart, and the field value above which one stands.
const OUTCROP_SIZE_M: f32 = 90.0;
const OUTCROP_CUT: f32 = 0.55;
/// How far below the coast, as a share of the sea's depth, the sea floor's gentler detail, its
/// islands and its formations fade out, so the coast has no step.
const COAST_EASE: f32 = 0.05;
/// Sea-floor formations: the water always kept over them, a reef's height, and one rock outcrop
/// or pillar in some cells this wide.
const SEABED_CLEAR_M: f32 = 2.0;
const REEF_M: f32 = 4.0;
const OUTCROP_CELL_M: f32 = 56.0;
const OUTCROP_CHANCE: f32 = 0.3;
/// The share of the land's height a range's body approaches; its crags rise above that.
const SUMMIT_SHARE: f32 = 0.85;
const MESA_STEP_M: f32 = 18.0;
/// How far the mountain field may lie from its middle and still be a level valley floor, and how
/// far it must for a massif's top.
const VALLEY_FLOOR: f32 = 0.05;
const MASSIF_TOP: f32 = 0.45;
/// The calm patches' period, and the band of their field that fades them in.
const CALM_SIZE_M: f32 = 450.0;
const CALM_CUT: [f32; 2] = [0.36, 0.48];
/// Where on the worn-to-rugged range the land starts and stops rising into hills and mountains.
const RUGGED_CUT: [f32; 2] = [0.4, 0.85];
const VOLCANO_CELL_M: f32 = 2600.0;
/// One island may stand in each cell this wide, at that chance, its radius in that range.
const ISLAND_CELL_M: f32 = 3200.0;
const ISLAND_CHANCE: f32 = 0.55;
const ISLAND_RADIUS_M: [f32; 2] = [300.0, 1100.0];
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

/// Version 3's large-scale fields from their raw noise.
fn fields_of(FieldNoise { continent, erosion }: FieldNoise<f32>) -> Fields {
    // fBm bunches around a half, so a contrast curve spreads the fields across their range.
    let erosion = smooth(0.25, 0.75, erosion);
    let inland = smooth(0.50, 0.66, continent);
    Fields {
        continent,
        erosion,
        ruggedness: inland * smooth(RUGGED_CUT[0], RUGGED_CUT[1], 1.0 - erosion),
    }
}

/// The continent and erosion fields before [`fields_of`] shapes them.
#[derive(Clone, Copy)]
struct FieldNoise<L> {
    continent: L,
    erosion: L,
}

impl FieldNoise<f32x4> {
    fn lanes(self) -> [FieldNoise<f32>; 4] {
        let (continent, erosion) = (self.continent.to_array(), self.erosion.to_array());
        std::array::from_fn(|k| FieldNoise {
            continent: continent[k],
            erosion: erosion[k],
        })
    }
}

/// A cut into the land: the regions it may run through, and the ridged line it follows there.
#[derive(Clone, Copy)]
struct Cut<L> {
    region: L,
    line: L,
}

impl Cut<f32x4> {
    fn lanes(self) -> [Cut<f32>; 4] {
        let (region, line) = (self.region.to_array(), self.line.to_array());
        std::array::from_fn(|k| Cut {
            region: region[k],
            line: line[k],
        })
    }
}

/// What [`Base::samples`] asks of a column.
#[derive(Clone, Copy)]
struct Samples<L> {
    warped: (L, L),
    fields: FieldNoise<L>,
    calm: L,
    peaks: L,
    /// Every detail octave.
    rough: L,
    /// The two broadest detail octaves.
    broad: L,
    plateau: L,
    canyon: Cut<L>,
    ravine: Cut<L>,
}

impl Samples<f32x4> {
    fn lanes(self) -> [Samples<f32>; 4] {
        let lane = |v: f32x4| v.to_array();
        let (wx, wz) = (lane(self.warped.0), lane(self.warped.1));
        let fields = self.fields.lanes();
        let (calm, peaks) = (lane(self.calm), lane(self.peaks));
        let (rough, broad, plateau) = (lane(self.rough), lane(self.broad), lane(self.plateau));
        let (canyon, ravine) = (self.canyon.lanes(), self.ravine.lanes());
        std::array::from_fn(|k| Samples {
            warped: (wx[k], wz[k]),
            fields: fields[k],
            calm: calm[k],
            peaks: peaks[k],
            rough: rough[k],
            broad: broad[k],
            plateau: plateau[k],
            canyon: canyon[k],
            ravine: ravine[k],
        })
    }
}

/// 0 to 1: how far a mountain range rises, from its field `n`, before ruggedness scales it.
/// Massifs stand where the field is far from its middle, and the valleys between them follow the
/// lines where it crosses its middle: those lines run on and join, so a valley drains to lower
/// land instead of being a basin the drainage fills with a lake. Near those lines the ground is a
/// level floor. Not capped at 1: the rare far reaches of the field are the highest peaks, which
/// the band clamps.
fn mountain_of(n: f32) -> f32 {
    // Squared, so a foot rises gently off the floor and the upper flanks steepen to the top.
    let t = (((2.0 * n - 1.0).abs() - VALLEY_FLOOR) / (MASSIF_TOP - VALLEY_FLOOR)).max(0.0);
    t * t
}

/// 0 to 1: the patches of calm ground, before the coast fades them.
fn calm_of(n: f32) -> f32 {
    smooth(CALM_CUT[0], CALM_CUT[1], n)
}

/// 0 to 1: where a canyon cuts, a winding line through some regions of the land.
fn canyon_of(Cut { region, line }: Cut<f32>) -> f32 {
    smooth(0.6, 0.7, region) * smooth(0.88, 0.97, crate::noise::ridge(line))
}

/// 0 to 1: narrow slots cut deep into the land, ridged noise thresholded near its crest.
fn ravine_of(Cut { region, line }: Cut<f32>) -> f32 {
    smooth(0.52, 0.62, region) * smooth(0.975, 0.995, crate::noise::ridge(line))
}

/// The overhang field `n` as a signed push in `-amp_m..amp_m`.
fn push_from(n: f32, amp_m: f32) -> f32 {
    ((n * 2.0 - 1.0) * 2.0).clamp(-1.0, 1.0) * amp_m
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

    fn warped<L: Lanes>(&self, x_m: L, z_m: L) -> (L, L) {
        crate::noise::warp2_lanes(
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
        fields_of(self.field_noise(wx, wz))
    }

    fn field_noise<L: Lanes>(&self, wx: L, wz: L) -> FieldNoise<L> {
        let size = L::splat(self.continent_size_m);
        let erosion_size = L::splat(EROSION_SIZE_M);
        FieldNoise {
            continent: L::fbm2(
                wx / size,
                wz / size,
                self.seed.wrapping_add(SALT_CONTINENT),
                4,
            ),
            erosion: L::fbm2(
                wx / erosion_size,
                wz / erosion_size,
                self.seed.wrapping_add(SALT_EROSION),
                3,
            ),
        }
    }

    /// The noise every column's base height asks for, whatever the column: apart from the
    /// arithmetic on it, so four columns can be sampled at once.
    fn samples<L: Lanes>(&self, x_m: L, z_m: L) -> Samples<L> {
        let seed = self.seed;
        let (wx, wz) = self.warped(x_m, z_m);
        let at = |size_m: f32, salt: u32, octaves: u32| {
            let size = L::splat(size_m);
            L::fbm2(wx / size, wz / size, seed.wrapping_add(salt), octaves)
        };
        let detail = |octaves| {
            let k = L::splat(self.inv_feature_size);
            L::fbm2(x_m * k, z_m * k, seed.wrapping_add(SALT_DETAIL), octaves)
        };
        Samples {
            warped: (wx, wz),
            fields: self.field_noise(wx, wz),
            calm: at(CALM_SIZE_M, SALT_CALM, 2),
            peaks: self.peaks_noise(wx, wz),
            rough: detail(self.octaves),
            broad: detail(self.octaves.min(2)),
            plateau: at(3500.0, SALT_PLATEAU, 2),
            canyon: Cut {
                region: at(2500.0, SALT_CANYON, 2),
                line: at(600.0, SALT_CANYON_LINE, 2),
            },
            ravine: Cut {
                region: at(3000.0, SALT_RAVINE_MASK, 2),
                line: at(450.0, SALT_RAVINE, 2),
            },
        }
    }

    /// The ground's height in metres before any river carves it.
    fn base_height_m(&self, x_m: f32, z_m: f32) -> f32 {
        self.base_height_of(x_m, z_m, self.samples(x_m, z_m))
    }

    /// [`Self::base_height_m`] at four columns, bit for bit.
    fn base_height_x4(&self, x_m: [f32; 4], z_m: [f32; 4]) -> [f32; 4] {
        let samples = self.samples(f32x4::new(x_m), f32x4::new(z_m)).lanes();
        std::array::from_fn(|k| self.base_height_of(x_m[k], z_m[k], samples[k]))
    }

    fn base_height_of(&self, x_m: f32, z_m: f32, s: Samples<f32>) -> f32 {
        let seed = self.seed;
        let (wx, wz) = s.warped;
        let f = fields_of(s.fields);
        let inland = smooth(0.50, 0.66, f.continent);
        let (d, h) = (self.depth_m, self.height_m);

        let base = curve(&CONTINENT_CURVE, f.continent);
        let mut offset = if base < 0.0 { base * d } else { base * h };

        // Patches of calm ground a few hundred metres across, everywhere inland: level places to
        // build never lie far off. Mountains rise from them rather than being cut by them.
        let calm = calm_of(s.calm) * inland;
        let mountain = mountain_of(s.peaks) * f.ruggedness;
        // Eased toward the band's top rather than cut flat by the clamp, so a high range still
        // ends in peaks.
        let crest_m = h * SUMMIT_SHARE;
        let rise_m = crest_m * (1.0 - libm::expf(-h * 1.1 * mountain / crest_m));
        // High up a range the rock breaks into crags and knolls rather than a smooth dome; below
        // them the flanks are benched.
        let summit = smooth(0.12, 0.45, mountain);
        let crown = smooth(0.3, 0.45, mountain);
        offset += rise_m + (self.terraced_m(wx, wz, rise_m) - rise_m) * (1.0 - crown);
        if summit > 0.0 {
            // Knolls standing on ground that joins up, not ridges walling off hollows: a ridge
            // network would leave pits between its crests for the drainage to fill as tarns.
            let n = crate::noise::fbm2(
                wx / CRAG_SIZE_M,
                wz / CRAG_SIZE_M,
                seed.wrapping_add(SALT_CRAG),
                3,
            );
            // Rising straight from the threshold, so a knoll ends in a point and not a flat top.
            let crag = ((n - 0.42) / 0.38).max(0.0);
            offset += h * CRAG_HEIGHT * summit * crag;
        }

        // Worn inland ground is a plain, and so is a valley floor between mountains: the fine
        // octaves that make it a field of small bumps fade out and the rest is gentler, so there
        // are wide level places to build and farm. A mountain's flank keeps only the broad
        // octaves, so it is a smooth slope and not a heap of knobs.
        let floor = f.ruggedness * (1.0 - smooth(0.0, 0.35, mountain / f.ruggedness.max(1e-3)));
        let plain = (smooth(0.2, 0.5, f.erosion) * inland).max(calm).max(floor);
        let (rough, broad) = (s.rough, s.broad);
        let flank = smooth(0.2, 0.6, f.ruggedness) * (1.0 - summit);
        let n = rough + (broad - rough) * plain.max(flank);
        // Detail also dies out at the waterline, so noise does not litter the coast with islets.
        let shore = 0.2 + 0.8 * smooth(0.0, 14.0, offset.abs());
        let worn =
            (0.35 + 0.65 * (1.0 - f.erosion)) * (1.0 - 0.8 * plain) * (1.0 - 0.6 * calm) * shore;
        let scale = self
            .relief_by_biome
            .as_ref()
            .map_or(1.0, |(climate, biomes, offsets)| {
                (1.0 + climate.blend(biomes, x_m, z_m, offsets)).max(0.0)
            });
        // The sea floor's detail is gentler, eased across the coast so it has no step.
        let land = smooth(-COAST_EASE, 0.0, base);
        offset += (n - 0.5) * self.relief_m * scale * (0.3 + (worn - 0.3) * land);

        let plateau = s.plateau;
        let mesa = smooth(0.58, 0.68, plateau) * inland * f.erosion;
        if mesa > 0.0 {
            let above = offset.max(0.0);
            let step = (above / MESA_STEP_M).floor();
            let frac = above / MESA_STEP_M - step;
            let terraced = (step + smooth(0.75, 1.0, frac)) * MESA_STEP_M;
            offset += (terraced - above) * mesa;
        }

        offset -= 70.0 * canyon_of(s.canyon) * inland * offset.clamp(0.0, 70.0) / 70.0;

        if land < 1.0 {
            offset += (self.island(x_m, z_m, offset) - offset) * (1.0 - land);
        }

        offset -= 45.0 * ravine_of(s.ravine) * inland * smooth(8.0, 30.0, offset);

        offset += self.volcano(x_m, z_m);

        if land < 1.0 {
            offset += self.seabed_rise_m(wx, wz, offset) * (1.0 - land);
        }

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
        // Only where a range rises well above its valleys: on a valley floor or a gentle foot the
        // density would only scatter knobs over a smooth slope.
        let (wx, wz) = self.warped(x_m, z_m);
        let rugged = self.fields_warped(wx, wz).ruggedness;
        if rugged <= 0.0 {
            return 0.0;
        }
        self.overhang_amp_of(
            rugged,
            [self.peaks_noise(wx, wz), self.outcrop_noise(wx, wz)],
        )
    }

    /// The mountain ranges' field, which the base height and the overhang density's reach ask.
    fn peaks_noise<L: Lanes>(&self, wx: L, wz: L) -> L {
        let size = L::splat(PEAKS_SIZE_M);
        L::fbm2(wx / size, wz / size, self.seed.wrapping_add(SALT_PEAKS), 3)
    }

    /// The outcrop field the overhang density's reach asks of a rugged column.
    fn outcrop_noise<L: Lanes>(&self, wx: L, wz: L) -> L {
        let size = L::splat(OUTCROP_SIZE_M);
        L::fbm2(
            wx / size,
            wz / size,
            self.seed.wrapping_add(SALT_OUTCROP),
            2,
        )
    }

    fn overhang_amp_of(&self, rugged: f32, [peaks, outcrop]: [f32; 2]) -> f32 {
        // And there only in scattered outcrops: everywhere, it walled whole flanks.
        self.overhang_m
            * smooth(0.04, 0.2, mountain_of(peaks) * rugged)
            * smooth(OUTCROP_CUT, OUTCROP_CUT + 0.08, outcrop)
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
        push_from(n, amp_m)
    }

    /// [`Self::overhang_push_m`] at four heights of one column, bit for bit.
    fn overhang_push_x4(&self, x_m: f32, y_m: [f32; 4], z_m: f32, amp_m: f32) -> [f32; 4] {
        let size = crate::noise::fill(OVERHANG_SIZE_M);
        let n = crate::noise::fbm3_lanes(
            crate::noise::fill(x_m) / size,
            f32x4::new(y_m) / size,
            crate::noise::fill(z_m) / size,
            self.seed.wrapping_add(SALT_OVERHANG),
            2,
        );
        n.to_array().map(|n| push_from(n, amp_m))
    }

    /// A range's rise broken into benches: level ground every [`TERRACE_STEP_M`] with a slope up
    /// to the next. Each step's height drifts across the range, so one bench runs down into the
    /// next as a ramp somewhere along it, and there is always a way up that is not a climb. The
    /// foot is left alone, so the valley floor meets the range without a ledge.
    fn terraced_m(&self, wx: f32, wz: f32, rise_m: f32) -> f32 {
        let foot = smooth(0.0, TERRACE_STEP_M, rise_m);
        if foot <= 0.0 {
            return rise_m;
        }
        let drift_m = crate::noise::fbm2(
            wx / TERRACE_DRIFT_M,
            wz / TERRACE_DRIFT_M,
            self.seed.wrapping_add(SALT_TERRACE),
            2,
        ) * 3.0
            * TERRACE_STEP_M;
        let t = (rise_m + drift_m) / TERRACE_STEP_M;
        let step = t.floor();
        let terraced_m = (step + smooth(TERRACE_BENCH, 1.0, t - step)) * TERRACE_STEP_M - drift_m;
        rise_m + (terraced_m - rise_m) * foot * TERRACE_STRENGTH
    }

    /// How far rock formations raise the sea floor at `offset_m` below Sea level: outcrops and
    /// pillars standing off the floor, and reef ridges along the shelf. Always [`SEABED_CLEAR_M`]
    /// or more under the surface, so water stands over every one of them.
    fn seabed_rise_m(&self, wx: f32, wz: f32, offset_m: f32) -> f32 {
        let room_m = (-SEABED_CLEAR_M - offset_m).max(0.0);
        if room_m <= 0.0 {
            return 0.0;
        }
        self.formation_m(wx, wz, offset_m).min(room_m)
    }

    /// [`Self::seabed_rise_m`] before it is kept under the surface.
    fn formation_m(&self, wx: f32, wz: f32, offset_m: f32) -> f32 {
        let seed = self.seed.wrapping_add(SALT_SEABED);
        let reef = crate::noise::ridged2(wx / 220.0, wz / 220.0, seed, 2);
        let shelf = smooth(-30.0, -18.0, offset_m) * (1.0 - smooth(-6.0, -3.0, offset_m));
        let mut rise_m = REEF_M * shelf * smooth(0.86, 0.96, reef);
        let (cx, cz) = (
            (wx / OUTCROP_CELL_M).floor() as i32,
            (wz / OUTCROP_CELL_M).floor() as i32,
        );
        let fray = crate::noise::fbm2(wx / 7.0, wz / 7.0, seed.wrapping_add(1), 2);
        for dz in -1..=1 {
            for dx in -1..=1 {
                let (ix, iz) = (cx + dx, cz + dz);
                let h = crate::noise::cell_hash(ix, iz, seed.wrapping_add(2));
                if h[0] > OUTCROP_CHANCE {
                    continue;
                }
                let k = h[0] / OUTCROP_CHANCE;
                let centre = [
                    (ix as f32 + 0.2 + 0.6 * h[1]) * OUTCROP_CELL_M,
                    (iz as f32 + 0.2 + 0.6 * h[2]) * OUTCROP_CELL_M,
                ];
                let radius_m = (3.0 + 7.0 * h[1]) * (0.8 + 0.4 * fray);
                let (dx, dz) = (wx - centre[0], wz - centre[1]);
                let r = (dx * dx + dz * dz).sqrt() / radius_m;
                if r >= 1.0 {
                    continue;
                }
                // Flat-topped with steep sides: a pillar when narrow, a table when wide.
                let height_m = 3.0 + 11.0 * k;
                rise_m = rise_m.max(height_m * (1.0 - smooth(0.55, 1.0, r)));
            }
        }
        rise_m
    }

    /// The sea floor `offset_m` (below Sea level) raised into an island where one stands: a
    /// hashed disc in some cells, its edge frayed by noise, rising to a broad low crown so there
    /// is room on it for trees and Structures.
    fn island(&self, x_m: f32, z_m: f32, offset_m: f32) -> f32 {
        let (cx, cz) = (
            (x_m / ISLAND_CELL_M).floor() as i32,
            (z_m / ISLAND_CELL_M).floor() as i32,
        );
        let fray = crate::noise::fbm2(
            x_m / 500.0,
            z_m / 500.0,
            self.seed.wrapping_add(SALT_ISLANDS),
            3,
        );
        let mut out = offset_m;
        for dz in -1..=1 {
            for dx in -1..=1 {
                let (ix, iz) = (cx + dx, cz + dz);
                let h = crate::noise::cell_hash(ix, iz, self.seed.wrapping_add(SALT_ISLANDS));
                if h[0] > ISLAND_CHANCE {
                    continue;
                }
                let k = h[0] / ISLAND_CHANCE;
                let (px, pz) = (
                    (ix as f32 + 0.25 + 0.5 * h[1]) * ISLAND_CELL_M,
                    (iz as f32 + 0.25 + 0.5 * h[2]) * ISLAND_CELL_M,
                );
                let radius_m = ISLAND_RADIUS_M[0] + (ISLAND_RADIUS_M[1] - ISLAND_RADIUS_M[0]) * k;
                let (dx, dz) = (x_m - px, z_m - pz);
                let r = (dx * dx + dz * dz).sqrt() / (radius_m * (0.7 + 0.6 * fray));
                if r >= 1.3 {
                    continue;
                }
                let crown_m = 6.0 + 22.0 * h[1];
                // Inside the disc a low dome; past its edge the shore falls back to the floor.
                let top = 2.0 + crown_m * (1.0 - r * r).max(0.0);
                let land = 1.0 - smooth(0.85, 1.3, r);
                out = out.max(offset_m + (top - offset_m) * land);
            }
        }
        out
    }

    /// A cone at a hashed spot of some cells; one off the coast rises from the sea as an island.
    fn volcano(&self, x_m: f32, z_m: f32) -> f32 {
        let (cx, cz) = (
            (x_m / VOLCANO_CELL_M).floor() as i32,
            (z_m / VOLCANO_CELL_M).floor() as i32,
        );
        let mut best = 0.0f32;
        for dz in -1..=1 {
            for dx in -1..=1 {
                let (ix, iz) = (cx + dx, cz + dz);
                let h = crate::noise::cell_hash(ix, iz, self.seed.wrapping_add(SALT_VOLCANO));
                if h[0] > 0.3 {
                    continue;
                }
                let (px, pz) = (
                    (ix as f32 + 0.2 + 0.6 * h[1]) * VOLCANO_CELL_M,
                    (iz as f32 + 0.2 + 0.6 * h[2]) * VOLCANO_CELL_M,
                );
                let (dx, dz) = (x_m - px, z_m - pz);
                let r = (dx * dx + dz * dz).sqrt();
                if r >= VOLCANO_RADIUS_M {
                    continue;
                }
                let cone = libm::powf(1.0 - r / VOLCANO_RADIUS_M, 1.3) * self.height_m * 0.55;
                let crater = 40.0 * (1.0 - smooth(0.0, 90.0, r));
                best = best.max(cone - crater);
            }
        }
        best
    }
}

/// A column's height and water, and what asking them learnt that its Biome, sea-floor rock and
/// overhang reach read again.
#[derive(Clone, Copy)]
pub(crate) struct Column {
    pub(crate) height_m: f32,
    pub(crate) inland_m: Option<f32>,
    river_cut_m: f32,
    warped: (f32, f32),
    fields: FieldNoise<f32>,
    peaks: f32,
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
                move |x, z| base.base_height_x4(x, z)
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
        h - self.drainage.wet(x_m, z_m, h).cut_m
    }

    /// A column's height and inland water from one drainage ask, which the water's level allows
    /// because it never reads the ground.
    pub(crate) fn column(&self, x_m: f32, z_m: f32) -> Column {
        self.column_of(x_m, z_m, self.base.samples(x_m, z_m))
    }

    /// [`Self::column`] at four columns, bit for bit.
    pub(crate) fn columns_x4(&self, x_m: [f32; 4], z_m: [f32; 4]) -> [Column; 4] {
        let samples = self.base.samples(f32x4::new(x_m), f32x4::new(z_m)).lanes();
        std::array::from_fn(|k| self.column_of(x_m[k], z_m[k], samples[k]))
    }

    fn column_of(&self, x_m: f32, z_m: f32, s: Samples<f32>) -> Column {
        let base_m = self.base.base_height_of(x_m, z_m, s);
        let wet = self.drainage.wet(x_m, z_m, base_m);
        Column {
            height_m: base_m - wet.cut_m,
            inland_m: wet.level_m,
            river_cut_m: wet.river_cut_m,
            warped: s.warped,
            fields: s.fields,
            peaks: s.peaks,
        }
    }

    /// The level lake or river water stands at over a column, if any reaches it; the sea is the
    /// caller's.
    pub(crate) fn inland_water_m(&self, x_m: f32, z_m: f32) -> Option<f32> {
        self.drainage.wet(x_m, z_m, f32::MIN).level_m
    }

    /// The Flow of the lake or river water over a column, in m/s along x and z.
    pub(crate) fn flow(&self, x_m: f32, z_m: f32) -> [f32; 2] {
        self.drainage.flow(x_m, z_m)
    }

    /// What a Biome's selectors ask of the column, and how its Climate shifts there: the air is
    /// colder by altitude and wetter toward the sea.
    pub(crate) fn site(&self, x_m: f32, z_m: f32) -> (Site, (f32, f32)) {
        self.site_of(&self.column(x_m, z_m))
    }

    /// [`Self::site`] of a column already asked.
    pub(crate) fn site_of(&self, c: &Column) -> (Site, (f32, f32)) {
        let f = fields_of(c.fields);
        let altitude_m = c.height_m - self.base.sea_m;
        let coast_m =
            (f.continent - COAST_CONTINENT) * self.base.continent_size_m * COAST_M_PER_UNIT;
        (
            Site {
                altitude_m,
                coast_m,
                erosion: f.erosion,
                river_m: c.river_cut_m,
            },
            climate_shift(altitude_m, coast_m),
        )
    }

    pub(crate) fn overhang_max_m(&self) -> f32 {
        self.base.overhang_max_m()
    }

    /// Whether a sea-floor formation stands at least a metre proud of a column, so its face is
    /// bare rock and not the sea's sand.
    pub(crate) fn seabed_rock_of(&self, c: &Column) -> bool {
        // The continent field is the one `fields_of` leaves as it is.
        if curve(&CONTINENT_CURVE, c.fields.continent) >= 0.0 {
            return false;
        }
        let floor_m = c.height_m - self.base.sea_m;
        let (wx, wz) = c.warped;
        floor_m <= -SEABED_CLEAR_M && self.base.formation_m(wx, wz, floor_m) >= 1.0
    }

    pub(crate) fn overhang_amp_m(&self, x_m: f32, z_m: f32) -> f32 {
        self.base.overhang_amp_m(x_m, z_m)
    }

    /// [`Self::overhang_amp_m`] at four columns already asked, bit for bit.
    pub(crate) fn overhang_amp_x4_of(&self, c: &[Column; 4]) -> [f32; 4] {
        let b = &self.base;
        let rugged = c.map(|c| fields_of(c.fields).ruggedness);
        if b.overhang_m <= 0.0 || rugged.iter().all(|&r| r <= 0.0) {
            return [0.0; 4];
        }
        let lanes = |m: fn(&Column) -> f32| f32x4::new(std::array::from_fn(|k| m(&c[k])));
        let outcrop = b
            .outcrop_noise(lanes(|c| c.warped.0), lanes(|c| c.warped.1))
            .to_array();
        std::array::from_fn(|k| {
            if rugged[k] <= 0.0 {
                0.0
            } else {
                b.overhang_amp_of(rugged[k], [c[k].peaks, outcrop[k]])
            }
        })
    }

    pub(crate) fn overhang_push_m(&self, x_m: f32, y_m: f32, z_m: f32, amp_m: f32) -> f32 {
        self.base.overhang_push_m(x_m, y_m, z_m, amp_m)
    }

    pub(crate) fn overhang_push_x4(
        &self,
        x_m: f32,
        y_m: [f32; 4],
        z_m: f32,
        amp_m: f32,
    ) -> [f32; 4] {
        self.base.overhang_push_x4(x_m, y_m, z_m, amp_m)
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
/// How far under the cut the shell round a flooded cavern reaches: more than the field moves in a
/// Voxel, so at least one Voxel of rock stands between its water and a dry tunnel.
const CAVERN_SHELL: f32 = 0.06;
const CAVERN_SIZE_M: [f32; 3] = [200.0, 90.0, 200.0];
/// One shaft may open in each cell this wide, about every other one.
const SHAFT_CELL_M: f32 = 320.0;
const SHAFT_CHANCE: f32 = 0.5;
/// How far under its mouth a shaft looks for a cavern's roof.
const SHAFT_REACH_M: f32 = 140.0;
/// The widest a shaft is.
const SHAFT_PAD_M: f32 = 4.5;

/// A vertical way down from the ground into a cavern, which otherwise never breaches the surface.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Shaft {
    pub(crate) centre_m: [f32; 2],
    pub(crate) radius_m: f32,
    /// The cavern's roof it opens into.
    pub(crate) bottom_m: f32,
}

impl Shaft {
    pub(crate) fn cuts(&self, x_m: f32, y_m: f32, z_m: f32) -> bool {
        let [dx, dz] = [x_m - self.centre_m[0], z_m - self.centre_m[1]];
        y_m >= self.bottom_m && dx * dx + dz * dz <= self.radius_m * self.radius_m
    }
}
/// A cavern's roof stays this far under the ground.
pub(crate) const CAVERN_MIN_DEPTH_M: f32 = 22.0;

impl Caverns {
    pub(crate) fn new(seed: u32, p: &crate::WorldgenParams) -> Self {
        Caverns {
            seed: seed.wrapping_add(SALT_CAVERN),
            floor_m: p.sea_level_m - p.landforms.depth_m - p.landforms.overhang_m,
        }
    }

    fn field(&self, x_m: f32, y_m: f32, z_m: f32) -> f32 {
        crate::noise::fbm3(
            x_m / CAVERN_SIZE_M[0],
            y_m / CAVERN_SIZE_M[1],
            z_m / CAVERN_SIZE_M[2],
            self.seed,
            2,
        )
    }

    /// [`Caverns::field`] for a run of nearby samples sharing `cache`.
    fn field_near(&self, cache: &mut Fbm3Cache, x_m: f32, y_m: f32, z_m: f32) -> f32 {
        crate::noise::fbm3_cached(
            x_m / CAVERN_SIZE_M[0],
            y_m / CAVERN_SIZE_M[1],
            z_m / CAVERN_SIZE_M[2],
            self.seed,
            2,
            cache,
        )
    }

    pub(crate) fn carves(&self, x_m: f32, y_m: f32, z_m: f32) -> bool {
        y_m >= self.floor_m && self.field(x_m, y_m, z_m) > CAVERN_CUT
    }

    /// [`Caverns::carves`] for a run of nearby samples sharing `cache`.
    pub(crate) fn carves_near(&self, cache: &mut Fbm3Cache, x_m: f32, y_m: f32, z_m: f32) -> bool {
        y_m >= self.floor_m && self.field_near(cache, x_m, y_m, z_m) > CAVERN_CUT
    }

    /// Whether the rock at a point is a cavern's or the shell of rock kept round it, which a
    /// tunnel or a shaft must not cut below the water table, where the cavern is a lake.
    pub(crate) fn walls_in(&self, cache: &mut Fbm3Cache, x_m: f32, y_m: f32, z_m: f32) -> bool {
        self.field_near(cache, x_m, y_m, z_m) > CAVERN_CUT - CAVERN_SHELL
    }

    /// The shafts whose mouths lie within `pad_m` of the box `min..max` on the ground plane, each
    /// over a cavern; `ground_m` is the surface height, and a shaft whose mouth `opens` refuses is
    /// not dug.
    pub(crate) fn shafts(
        &self,
        min: [f32; 2],
        max: [f32; 2],
        ground_m: impl Fn(f32, f32) -> f32,
        opens: impl Fn(f32, f32) -> bool,
    ) -> Vec<Shaft> {
        let cell = |m: f32| (m / SHAFT_CELL_M).floor() as i32;
        let mut out = Vec::new();
        for iz in cell(min[1] - SHAFT_PAD_M)..=cell(max[1] + SHAFT_PAD_M) {
            for ix in cell(min[0] - SHAFT_PAD_M)..=cell(max[0] + SHAFT_PAD_M) {
                let h = crate::noise::cell_hash(ix, iz, self.seed.wrapping_add(SALT_SHAFT));
                if h[0] > SHAFT_CHANCE {
                    continue;
                }
                let centre_m = [
                    (ix as f32 + 0.15 + 0.7 * h[1]) * SHAFT_CELL_M,
                    (iz as f32 + 0.15 + 0.7 * h[2]) * SHAFT_CELL_M,
                ];
                let radius_m = 2.5 + 2.0 * h[0] / SHAFT_CHANCE;
                let near = (0..2)
                    .all(|a| centre_m[a] + radius_m >= min[a] && centre_m[a] - radius_m <= max[a]);
                if !near || !opens(centre_m[0], centre_m[1]) {
                    continue;
                }
                let top_m = ground_m(centre_m[0], centre_m[1]);
                let mut y = top_m - CAVERN_MIN_DEPTH_M;
                let lowest = self.floor_m.max(top_m - SHAFT_REACH_M);
                while y >= lowest {
                    if self.carves(centre_m[0], y, centre_m[1]) {
                        out.push(Shaft {
                            centre_m,
                            radius_m,
                            bottom_m: y,
                        });
                        break;
                    }
                    y -= 2.0;
                }
            }
        }
        out
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Columns from a few metres to a hundred kilometres out, so the sea, islands, volcanoes,
    /// mountains and plains all occur.
    fn columns(i: u32) -> [[f32; 4]; 2] {
        let at = |k: u32| {
            let h = crate::noise::cell_hash(i as i32, k as i32, 0x0c01);
            let scale = [50.0, 3e3, 4e4, 2e5][(h[0] * 4.0) as usize];
            (h[1] - 0.5) * scale + h[2]
        };
        [
            std::array::from_fn(|k| at(k as u32)),
            std::array::from_fn(|k| at(k as u32 + 4)),
        ]
    }

    #[test]
    fn four_columns_at_once_are_four_single_columns_bit_for_bit() {
        let p = crate::testgen::world_params();
        for seed in 0..24u32 {
            let base = Base::new(seed.wrapping_mul(0x9e37_79b9), &p);
            for i in 0..400u32 {
                let [x, z] = columns(seed * 1000 + i);
                let height = base.base_height_x4(x, z);
                for k in 0..4 {
                    let at = format!("lane {k} at {},{} seed {seed}", x[k], z[k]);
                    let want = base.base_height_m(x[k], z[k]);
                    assert_eq!(height[k].to_bits(), want.to_bits(), "height {at}");
                }
            }
        }
    }

    fn site_bits((s, (t, h)): (Site, (f32, f32))) -> [u32; 6] {
        [s.altitude_m, s.coast_m, s.erosion, s.river_m, t, h].map(f32::to_bits)
    }

    /// [`Terrain::site`] from its own asks of the fields, the base height and the valley cut.
    fn site_asked_apart(t: &Terrain, x_m: f32, z_m: f32) -> (Site, (f32, f32)) {
        let f = t.base.fields(x_m, z_m);
        let h = t.base.base_height_m(x_m, z_m);
        let wet = t.drainage.wet(x_m, z_m, h);
        let river_m = wet.river_cut_m;
        let altitude_m = h - wet.cut_m - t.base.sea_m;
        let coast_m = (f.continent - COAST_CONTINENT) * t.base.continent_size_m * COAST_M_PER_UNIT;
        (
            Site {
                altitude_m,
                coast_m,
                erosion: f.erosion,
                river_m,
            },
            climate_shift(altitude_m, coast_m),
        )
    }

    /// [`Terrain::seabed_rock_of`] from its own asks of the fields and the height.
    fn seabed_rock_asked_apart(t: &Terrain, x_m: f32, z_m: f32) -> bool {
        let b = &t.base;
        let (wx, wz) = b.warped(x_m, z_m);
        if curve(&CONTINENT_CURVE, b.fields_warped(wx, wz).continent) >= 0.0 {
            return false;
        }
        let floor_m = t.height_m(x_m, z_m) - b.sea_m;
        floor_m <= -SEABED_CLEAR_M && b.formation_m(wx, wz, floor_m) >= 1.0
    }

    /// Each region solves its rivers over a window of its own, so a river's width comes from two
    /// catchments either side of a border. The coarse solve's rivers fed in at a window's edge
    /// bring border stream nodes more than a fifth apart from 5.3% to 3.5% on these worlds; the
    /// rest are valleys its 128 m lattice cannot see.
    #[test]
    fn a_river_keeps_its_width_across_a_region_border() {
        let p = crate::testgen::world_params();
        let (mut nodes, mut apart) = (0, 0);
        for seed in 0..8u32 {
            let t = Terrain::new(seed.wrapping_mul(0x9e37_79b9), &p);
            for key in [
                [0, 0],
                [-1, 0],
                [0, -1],
                [-1, -1],
                [1, 1],
                [2, -2],
                [-3, 2],
                [4, 1],
            ] {
                let (n, a, _) = t.drainage.border_widths(key);
                (nodes, apart) = (nodes + n, apart + a);
            }
        }
        assert!(nodes > 1000, "{nodes} stream nodes on the borders");
        assert!(
            apart * 100 <= nodes * 4,
            "{apart} of {nodes} border stream nodes are a fifth apart in width"
        );
    }

    /// D8 draws streams side by side down an even slope: 9.2% of stream nodes on these worlds
    /// before one joined the other, 1.4% after.
    #[test]
    fn few_streams_run_side_by_side() {
        let p = crate::testgen::world_params();
        let (mut streams, mut beside) = (0, 0);
        for seed in 0..8u32 {
            let t = Terrain::new(seed.wrapping_mul(0x9e37_79b9), &p);
            for key in [
                [0, 0],
                [-1, 0],
                [0, -1],
                [-1, -1],
                [1, 1],
                [2, -2],
                [-3, 2],
                [4, 1],
            ] {
                let (s, b) = t.drainage.parallel_streams(key);
                (streams, beside) = (streams + s, beside + b);
            }
        }
        assert!(streams > 5000, "{streams} stream nodes");
        assert!(
            beside * 100 <= streams * 2,
            "{beside} of {streams} stream nodes run beside another"
        );
    }

    #[test]
    fn one_ask_of_a_column_is_its_separate_asks_bit_for_bit() {
        let p = crate::testgen::world_params();
        let (mut cut, mut wet, mut rock, mut pushed) = (0, 0, 0, 0);
        for seed in 0..8u32 {
            let t = Terrain::new(seed.wrapping_mul(0x9e37_79b9), &p);
            for i in 0..40u32 {
                // A patch of columns round each point, so a valley's cut and a formation's rise
                // are crossed and not only stepped over.
                let [x0, z0] = columns(seed * 1000 + i).map(|c| c[0]);
                for row in 0..16 {
                    let x = std::array::from_fn(|k| x0 + (row % 4 * 4 + k) as f32 * 3.7);
                    let z = [z0 + (row / 4) as f32 * 9.1; 4];
                    let four = t.columns_x4(x, z);
                    let amp = t.overhang_amp_x4_of(&four);
                    for k in 0..4 {
                        let (x, z) = (x[k], z[k]);
                        let at = format!("lane {k} at {x},{z} seed {seed}");
                        let one = t.column(x, z);
                        for (c, how) in [(&four[k], "four-wide"), (&one, "scalar")] {
                            let at = format!("{how} {at}");
                            assert_eq!(c.height_m.to_bits(), t.height_m(x, z).to_bits(), "{at}");
                            let level = t.inland_water_m(x, z).map(f32::to_bits);
                            assert_eq!(c.inland_m.map(f32::to_bits), level, "level {at}");
                            let site = site_bits(site_asked_apart(&t, x, z));
                            assert_eq!(site_bits(t.site_of(c)), site, "site {at}");
                            let bare = seabed_rock_asked_apart(&t, x, z);
                            assert_eq!(t.seabed_rock_of(c), bare, "rock {at}");
                        }
                        assert_eq!(site_bits(t.site(x, z)), site_bits(t.site_of(&one)));
                        let want = t.overhang_amp_m(x, z);
                        assert_eq!(amp[k].to_bits(), want.to_bits(), "overhang {at}");
                        cut += usize::from(t.site_of(&one).0.river_m > 0.0);
                        wet += usize::from(one.inland_m.is_some());
                        rock += usize::from(t.seabed_rock_of(&one));
                        pushed += usize::from(amp[k] > 0.0);
                    }
                }
            }
        }
        let crossed = [cut, wet, rock, pushed];
        assert!(
            crossed.iter().all(|&n| n > 20),
            "cut, wet, rock, pushed: {crossed:?}"
        );
    }

    /// Chunk columns walked at every LOD level's Voxel size, each Voxel also asking a neighbour as
    /// a spring does, so one cache steps across the field's cells and jumps back.
    #[test]
    fn the_cached_cavern_field_is_the_plain_one_bit_for_bit() {
        let p = crate::testgen::world_params();
        for seed in 0..48u32 {
            let caverns = Caverns::new(seed.wrapping_mul(0x9e37_79b9), &p);
            let mut cache = Fbm3Cache::default();
            for column in 0..40u32 {
                let h = crate::noise::cell_hash(seed as i32, column as i32, 0x0ca7);
                let voxel_m = [0.5, 1.0, 2.0, 4.0, 8.0, 16.0, 32.0][(h[0] * 7.0) as usize];
                let [x_m, z_m] = [h[1], h[2]].map(|v| (v - 0.5) * 3000.0);
                let floor_m = caverns.floor_m + (column % 8) as f32 * 24.0 * voxel_m;
                for y in 0..32 {
                    let y_m = floor_m + y as f32 * voxel_m;
                    let beside = [x_m + voxel_m * (y % 3) as f32, y_m - voxel_m, z_m];
                    for [x, y, z] in [[x_m, y_m, z_m], beside] {
                        let want = caverns.field(x, y, z);
                        let got = caverns.field_near(&mut cache, x, y, z);
                        assert_eq!(got.to_bits(), want.to_bits(), "{x},{y},{z} seed {seed}");
                    }
                }
            }
        }
    }
}

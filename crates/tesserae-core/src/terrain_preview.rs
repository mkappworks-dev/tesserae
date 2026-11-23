//! A small top-down picture of a World before it exists, drawn from the Generator the World will
//! use, so the two never disagree. It reads no World state and needs no Server (ADR-0002).

use crate::biome_edits::BiomeEdits;
use crate::settlement::Settlement;
use crate::template::Template;
use crate::voxel::MaterialId;
use crate::worldgen::{ColumnTop, Worldgen, WorldgenParams};

/// Drawn where nothing is generated: a World without ground, or a Material with no colour.
pub const BLANK: [u8; 4] = [40, 44, 48, 255];

/// Where the picture looks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Centre {
    /// Where the Player first stands, as the Server finds it.
    Spawn,
    At([f32; 2]),
}

/// Where the picture looks and how many pixels it has.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub centre: Centre,
    /// The width of ground the picture covers, in metres; its height follows the aspect.
    pub span_m: f32,
    pub width: u32,
    pub height: u32,
}

impl Frame {
    /// The Create World preview: 16:9 around the spawn, wide enough at the default Climate size to
    /// show several Biomes.
    pub const CREATE: Frame = Frame {
        centre: Centre::Spawn,
        span_m: 3072.0,
        width: 320,
        height: 180,
    };

    fn metres_per_pixel(&self) -> f32 {
        self.span_m / self.width as f32
    }

    /// The global Voxel column under the centre of pixel `[px, py]` when the picture is centred on
    /// `centre_m`; north (−Z) is up.
    pub fn column(&self, centre_m: [f32; 2], [px, py]: [u32; 2], voxel_size: f32) -> [i32; 2] {
        let step = self.metres_per_pixel();
        let x_m = centre_m[0] + (px as f32 + 0.5 - self.width as f32 / 2.0) * step;
        let z_m = centre_m[1] + (py as f32 + 0.5 - self.height as f32 / 2.0) * step;
        [x_m, z_m].map(|m| (m / voxel_size).floor() as i32)
    }
}

/// Straight sRGBA8, row by row from the north-west corner.
#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    /// The point the picture is centred on, in metres.
    pub centre_m: [f32; 2],
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// What a Game (with its Mods) declares that the picture needs: read once, drawn for any seed.
#[derive(Clone, Debug, PartialEq)]
pub struct Source {
    pub params: WorldgenParams,
    /// Registered Material names, so Biome edits that name ground can apply.
    pub materials: Vec<(String, MaterialId)>,
    /// Each Material's appearance colour.
    pub colours: Vec<(MaterialId, [u8; 4])>,
    /// What the Server's Generator stands, because a Settlement moves the spawn.
    pub templates: Vec<Template>,
    pub settlements: Vec<Settlement>,
}

impl Source {
    /// The picture of the World made with `seed` at `voxel_size` and the player's Biome `edits`,
    /// or why those edits cannot apply. The same arguments always give the same picture. Each
    /// pixel is its column's [`ColumnTop`]: what stands on the ground and cave mouths are not drawn.
    pub fn render(
        &self,
        seed: u64,
        voxel_size: f32,
        edits: Option<&BiomeEdits>,
        frame: &Frame,
    ) -> Result<Image, String> {
        let params = match edits {
            Some(edits) => edits.apply(self.params, |name| {
                self.materials
                    .iter()
                    .find(|(n, _)| n == name)
                    .map(|(_, id)| *id)
            })?,
            None => self.params,
        };
        let generator = Worldgen::new(seed, voxel_size, params)
            .with_templates(&self.templates)
            .with_settlements(&self.settlements);
        let centre_m = match frame.centre {
            Centre::Spawn => generator.spawn_m(),
            Centre::At(at) => at,
        };
        let water = generator.water().map(|w| w.material);
        Ok(paint(
            &columns(&generator, frame, centre_m),
            frame,
            centre_m,
            water,
            |m| {
                self.colours
                    .iter()
                    .find(|(id, _)| *id == m)
                    .map_or(BLANK, |(_, c)| *c)
            },
        ))
    }
}

/// [`Worldgen::column_top`] under each pixel, row by row.
pub fn columns(generator: &Worldgen, frame: &Frame, centre_m: [f32; 2]) -> Vec<Option<ColumnTop>> {
    let size = generator.voxel_size();
    (0..frame.height)
        .flat_map(|py| (0..frame.width).map(move |px| [px, py]))
        .map(|pixel| {
            let [gx, gz] = frame.column(centre_m, pixel, size);
            generator.column_top(gx, gz)
        })
        .collect()
}

/// How much a slope of one metre per metre facing away from the light darkens a pixel.
const SHADE: f32 = 1.2;
/// Water darkens with depth until this many metres.
const DEEP_M: f32 = 24.0;

/// Colours each column by what tops it, lit from the north-west by its slope, and water darker
/// the deeper it stands. Only ever scales a colour, so a Material's hue survives.
fn paint(
    tops: &[Option<ColumnTop>],
    frame: &Frame,
    centre_m: [f32; 2],
    water: Option<MaterialId>,
    colour: impl Fn(MaterialId) -> [u8; 4],
) -> Image {
    let (w, h) = (frame.width as i64, frame.height as i64);
    let height = |px: i64, py: i64| {
        let at = (py.clamp(0, h - 1) * w + px.clamp(0, w - 1)) as usize;
        tops[at].map(|t| t.level_m.filter(|_| t.wet).unwrap_or(t.surface_m))
    };
    let step = frame.metres_per_pixel();
    let mut rgba = Vec::with_capacity(tops.len() * 4);
    for (at, top) in tops.iter().enumerate() {
        let (px, py) = (at as i64 % w, at as i64 / w);
        let Some(top) = top else {
            rgba.extend_from_slice(&BLANK);
            continue;
        };
        let (base, lift) = match (top.wet, top.level_m) {
            (true, Some(level_m)) => {
                let depth = ((level_m - top.surface_m) / DEEP_M).clamp(0.0, 1.0);
                (water.map_or(BLANK, &colour), 1.0 - 0.45 * depth)
            }
            _ => {
                let slope = match (height(px - 1, py - 1), height(px + 1, py + 1)) {
                    (Some(nw), Some(se)) => (nw - se) / (2.0 * std::f32::consts::SQRT_2 * step),
                    _ => 0.0,
                };
                (colour(top.top), 1.0 - (slope * SHADE).clamp(-0.25, 0.3))
            }
        };
        let [r, g, b, a] = base;
        rgba.extend([r, g, b].map(|c| (c as f32 * lift).round().clamp(0.0, 255.0) as u8));
        rgba.push(a);
    }
    Image {
        centre_m,
        width: frame.width,
        height: frame.height,
        rgba,
    }
}

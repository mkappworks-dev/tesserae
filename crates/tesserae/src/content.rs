//! Plain content sent by the Server to the Client.

use bevy::prelude::*;
use tesserae_core::{MaterialId, ShapeId};

#[derive(Message, Clone, Debug, Default, PartialEq, Eq)]
pub struct MaterialPalette(pub Vec<(MaterialId, [u8; 4])>);

/// Shared RGBA texture atlases and how each varies, keyed by registered Material id.
#[derive(Message, Clone, Debug, Default, PartialEq, Eq)]
pub struct MaterialTextures(pub Vec<(MaterialId, Vec<u8>, tesserae_mods::TileVariation)>);

/// How the Client draws each Item, declared or implicit, in the Server's item
/// order.
#[derive(Message, Clone, Debug, Default, PartialEq)]
pub struct ItemLooks(pub Vec<ItemLook>);

#[derive(Clone, Debug, PartialEq)]
pub struct ItemLook {
    /// `<mod id>:<name>`.
    pub name: String,
    pub size_m: f32,
    pub color: [u8; 4],
    /// Its Material's appearance is drawn when set.
    pub material: Option<MaterialId>,
    pub max_stack: u32,
    /// RGBA pixels of one `MATERIAL_TEXTURE_EDGE` tile: the Item is a flat sprite of it.
    pub texture: Option<std::sync::Arc<[u8]>>,
}

impl ItemLook {
    /// The name without its Mod's id, for the Player to read.
    pub fn label(&self) -> &str {
        self.name.split_once(':').map_or(&self.name, |(_, l)| l)
    }
}

/// Every Recipe in the World, in the Server's Recipe order, so a Craft names
/// one by its index.
#[derive(Message, Clone, Debug, Default, PartialEq, Eq)]
pub struct RecipeBook(pub Vec<tesserae_core::craft::Recipe>);

/// Every Sound in the World, indexed by `SoundId`, with its recordings' checked
/// Ogg Vorbis bytes: what the Client plays a Sound event with.
#[derive(Message, Clone, Debug, Default, PartialEq)]
pub struct SoundLibrary(pub std::sync::Arc<[LibrarySound]>);

#[derive(Clone, Debug, PartialEq)]
pub struct LibrarySound {
    pub def: tesserae_core::audio::SoundDef,
    pub recordings: Vec<std::sync::Arc<[u8]>>,
}

impl SoundLibrary {
    pub fn new(sounds: tesserae_mods::Sounds) -> Self {
        SoundLibrary(
            sounds
                .sounds
                .into_iter()
                .map(|s| LibrarySound {
                    def: s.def,
                    recordings: s.recordings.into_iter().map(Into::into).collect(),
                })
                .collect(),
        )
    }
}

/// Every Soundscape in the World, indexed by `SoundscapeId`: what the Client
/// plays while the Server names one.
#[derive(Message, Clone, Debug, Default, PartialEq)]
pub struct SoundscapeLibrary(pub std::sync::Arc<[tesserae_core::audio::SoundscapeDef]>);

impl SoundscapeLibrary {
    pub fn new(sounds: &tesserae_mods::Sounds) -> Self {
        SoundscapeLibrary(sounds.soundscapes.iter().map(|s| s.def.clone()).collect())
    }
}

/// The Game's player figure, or `None` for the engine's plain box.
#[derive(Message, Clone, Debug, Default, PartialEq)]
pub struct CharacterModel(pub Option<std::sync::Arc<tesserae_mods::Character>>);

/// What the registered content is called and what a player can do with it.
/// Content only: it holds no Voxel, Chunk or Occupants, so a Client reading it
/// reads nothing of the World (ADR-0002).
#[derive(Message, Clone, Debug, Default, PartialEq)]
pub struct ContentCatalogue {
    /// Stable root Game id, supplied by the Server.
    pub game: Option<String>,
    /// Every registered Material, in id order.
    pub materials: Vec<CatalogueMaterial>,
    /// Every registered Shape, in id order.
    pub shapes: Vec<CatalogueShape>,
    /// The fluid the Game's lakes are made of.
    pub lake: Option<MaterialId>,
    pub rock: Option<MaterialId>,
    /// The Game's tools, in declared order.
    pub tools: Vec<CatalogueTool>,
    /// The World's Templates in load order: what the `place` Tool offers.
    pub templates: Vec<CatalogueTemplate>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CatalogueTemplate {
    /// `<mod id>:<name>`, as a Structure intent names it.
    pub name: String,
    /// Along x, y and z before any turn; a quarter turn swaps x and z.
    pub size_m: [f32; 3],
    /// [`template_thumbnail`] at each quarter turn, in turn order; empty when not drawn.
    pub thumbnails: [std::sync::Arc<[u8]>; 4],
}

/// Pixels along each side of a Template's thumbnail.
pub const THUMBNAIL_PX: usize = 64;

/// RGBA pixels of a small isometric picture of `template` turned by `turn`: a cube for each cell
/// that is not Keep or Air, in its Material's `colour` (a `ground` cell in `ground`), its top lit
/// and its two near sides shaded, and clear where nothing is drawn. Drawn from content alone, so
/// the place Tool's list shows it with no World and no render pass.
pub fn template_thumbnail(
    template: &tesserae_core::template::Template,
    turn: tesserae_core::template::Turn,
    colour: impl Fn(MaterialId) -> [u8; 4],
    ground: [u8; 4],
) -> std::sync::Arc<[u8]> {
    use tesserae_core::template::Cell;
    let sampled = template.sample(template.cell_m(), turn);
    let [w, h, d] = sampled.size();
    let mut cubes = Vec::new();
    for y in 0..h {
        for z in 0..d {
            for x in 0..w {
                match sampled.cell([x, y, z]) {
                    Cell::Voxel(v) if !v.is_air() => cubes.push(([x, y, z], colour(v.material()))),
                    Cell::Ground => cubes.push(([x, y, z], ground)),
                    _ => {}
                }
            }
        }
    }
    let mut pixels = vec![0u8; THUMBNAIL_PX * THUMBNAIL_PX * 4];
    if cubes.is_empty() {
        return pixels.into();
    }
    let at = |[x, y, z]: [i32; 3]| [(x - z) as f32, (x + z) as f32 / 2.0 - y as f32];
    let (mut low, mut high) = ([f32::MAX; 2], [f32::MIN; 2]);
    for &([x, y, z], _) in &cubes {
        for corner in [
            [x, y + 1, z],
            [x + 1, y, z],
            [x, y, z + 1],
            [x + 1, y + 1, z + 1],
        ] {
            let p = at(corner);
            for a in 0..2 {
                low[a] = low[a].min(p[a]);
                high[a] = high[a].max(p[a]);
            }
        }
    }
    let margin = 2.0;
    let side = THUMBNAIL_PX as f32 - 2.0 * margin;
    let scale = side / (high[0] - low[0]).max(high[1] - low[1]);
    let offset =
        [0, 1].map(|a| margin + (side - (high[a] - low[a]) * scale) / 2.0 - low[a] * scale);
    let px = |corner: [i32; 3]| {
        let p = at(corner);
        [p[0] * scale + offset[0], p[1] * scale + offset[1]]
    };
    // Far cubes first, so nearer ones paint over them.
    cubes.sort_by_key(|&([x, y, z], _)| (x + z, y));
    for ([x, y, z], rgba) in cubes {
        let faces = [
            (
                [
                    [x, y, z + 1],
                    [x + 1, y, z + 1],
                    [x + 1, y + 1, z + 1],
                    [x, y + 1, z + 1],
                ],
                0.8,
            ),
            (
                [
                    [x + 1, y, z],
                    [x + 1, y, z + 1],
                    [x + 1, y + 1, z + 1],
                    [x + 1, y + 1, z],
                ],
                0.65,
            ),
            (
                [
                    [x, y + 1, z],
                    [x + 1, y + 1, z],
                    [x + 1, y + 1, z + 1],
                    [x, y + 1, z + 1],
                ],
                1.0,
            ),
        ];
        for (corners, light) in faces {
            fill_quad(&mut pixels, corners.map(px), rgba, light);
        }
    }
    pixels.into()
}

/// Paints the convex quad `corners` in pixel space with `rgba` scaled by `light`, over what is
/// there.
fn fill_quad(pixels: &mut [u8], corners: [[f32; 2]; 4], rgba: [u8; 4], light: f32) {
    let n = THUMBNAIL_PX as i32;
    let low = [0, 1].map(|a| {
        corners
            .iter()
            .map(|c| c[a])
            .fold(f32::MAX, f32::min)
            .floor() as i32
    });
    let high = [0, 1].map(|a| corners.iter().map(|c| c[a]).fold(f32::MIN, f32::max).ceil() as i32);
    let alpha = f32::from(rgba[3]) / 255.0;
    for py in low[1].max(0)..high[1].min(n) {
        for px in low[0].max(0)..high[0].min(n) {
            let p = [px as f32 + 0.5, py as f32 + 0.5];
            let sides = (0..4).map(|i| {
                let [a, b] = [corners[i], corners[(i + 1) % 4]];
                (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
            });
            let (mut neg, mut pos) = (false, false);
            for side in sides {
                neg |= side < 0.0;
                pos |= side > 0.0;
            }
            if neg && pos {
                continue;
            }
            let i = ((py * n + px) * 4) as usize;
            for c in 0..3 {
                let lit = f32::from(rgba[c]) * light;
                pixels[i + c] =
                    (lit * alpha + f32::from(pixels[i + c]) * (1.0 - alpha)).round() as u8;
            }
            pixels[i + 3] = pixels[i + 3].max(rgba[3]);
        }
    }
}

impl CatalogueTool {
    /// The name without its Mod's id, for the Player to read.
    pub fn label(&self) -> &str {
        self.name
            .split_once(':')
            .map_or(&self.name, |(_, local)| local)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogueTool {
    pub id: crate::server::tool::ToolId,
    /// `<mod id>:<name>`, the stable name a saved slot keeps.
    pub name: String,
    pub kind: tesserae_mods::ToolKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CatalogueMaterial {
    pub remove_time: std::time::Duration,
    pub id: MaterialId,
    pub name: String,
    pub solid: bool,
    /// Its fluid row's rank, which is the order the rows were declared in.
    pub fluid: Option<u16>,
    /// Whether a combustion row makes it fuel.
    pub burns: bool,
    /// The Shape its `voxel-uses` row always places it in.
    pub shape: Option<ShapeId>,
    pub door: Option<tesserae_core::Door>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogueShape {
    pub id: ShapeId,
    pub name: String,
    /// Whether turning it changes anything: a cube looks the same however it
    /// is turned and a connected Shape ignores its orientation.
    pub turnable: bool,
    /// A Usable Voxel's row places its Material in it, so it is that row's
    /// alone and the Player is not offered it.
    pub fixed: bool,
    /// How it is drawn: an Item of a flat form is a sprite, and its ghost is
    /// the form's quads.
    pub form: tesserae_core::Form,
    pub geometry: Vec<Vec<tesserae_core::shape::ShapeQuad>>,
}

impl ContentCatalogue {
    pub fn material(&self, id: MaterialId) -> Option<&CatalogueMaterial> {
        self.materials.iter().find(|m| m.id == id)
    }

    pub fn tool_label(&self, id: crate::server::tool::ToolId) -> &str {
        self.tool(id).map_or("unknown", CatalogueTool::label)
    }

    pub fn tool(&self, id: crate::server::tool::ToolId) -> Option<&CatalogueTool> {
        self.tools.iter().find(|t| t.id == id)
    }

    pub fn shape(&self, id: ShapeId) -> Option<&CatalogueShape> {
        self.shapes.iter().find(|s| s.id == id)
    }

    pub fn anything_burns(&self) -> bool {
        self.materials.iter().any(|m| m.burns)
    }
}

/// sRGB in `[0, 1]`. Never sent in a World with no Biomes; the Client holds no generator to work
/// it out itself (ADR-0002).
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct SkyFogTarget {
    pub sky: [f32; 3],
    pub fog: [f32; 3],
}

/// The Biome at the Player's Eye, as an index into [`WorldFacts::biomes`], sent as it changes;
/// `None` where the World has no Biomes. The Client holds no generator to work it out itself
/// (ADR-0002).
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct HereBiome(pub Option<usize>);

/// The cave Biome round the Player's Eye, as an index into [`WorldFacts::cave_biomes`], sent as it
/// changes; `None` above ground and where the World has none. It names the place in place of
/// [`HereBiome`] while it is `Some`.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct HereCave(pub Option<usize>);

/// Every Marker kind of the World and which one marks a death: what the Client names and draws
/// the Map's and the Compass's marks from, sent once at session start.
#[derive(Message, Clone, Debug, Default, PartialEq)]
pub struct MarkerKinds(pub tesserae_core::marker::MarkerKinds);

/// Whether the World's Game permits the Map and the Compass, sent once at session start; the Client
/// draws neither while it is `false`.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapPermitted(pub bool);

/// What the Server says about the open World when a session starts, for the
/// Menu's World details.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct WorldFacts {
    pub name: Option<String>,
    /// The Game's id and version.
    pub game: Option<(String, String)>,
    pub voxel_size_m: f32,
    pub seed: u64,
    /// The snapshotted Biome table's declared names, in table order.
    pub biomes: Vec<String>,
    /// The cave Biomes' names, in table order.
    pub cave_biomes: Vec<String>,
    /// The mode a Player new to this World starts in.
    pub default_mode: tesserae_core::save::GameMode,
    /// The generator version this World was made with and keeps generating by.
    pub generator: u32,
    /// How far apart the Climate map's temperature and humidity change, in metres.
    pub temperature_size_m: f32,
    pub humidity_size_m: f32,
}

#[cfg(test)]
mod thumbnail_tests {
    use super::*;
    use tesserae_core::Voxel;
    use tesserae_core::template::{Cell, Template, Turn};

    const RED: [u8; 4] = [200, 0, 0, 255];
    const BLUE: [u8; 4] = [0, 0, 200, 255];

    /// Three red cells along x, and a blue one a layer up over the last.
    fn ell() -> Template {
        let (r, b, k) = (
            Cell::Voxel(Voxel::cube(1)),
            Cell::Voxel(Voxel::cube(2)),
            Cell::Keep,
        );
        Template::new(
            "t:ell".into(),
            1.0,
            vec![vec![vec![r, r, r]], vec![vec![k, k, b]]],
            0,
            None,
            1.0,
        )
        .unwrap()
    }

    fn drawn(turn: Turn) -> std::sync::Arc<[u8]> {
        let colour = |m| if m == 1 { RED } else { BLUE };
        template_thumbnail(&ell(), turn, colour, [0, 0, 0, 255])
    }

    #[test]
    fn a_templates_thumbnail_shows_its_materials_on_a_clear_ground_and_turns_with_it() {
        let thumb = drawn(Turn::NONE);
        assert_eq!(thumb.len(), THUMBNAIL_PX * THUMBNAIL_PX * 4);
        let pixels: Vec<[u8; 4]> = thumb.chunks(4).map(|p| [p[0], p[1], p[2], p[3]]).collect();
        assert!(pixels.contains(&RED), "a red top");
        assert!(pixels.contains(&BLUE), "a blue top");
        assert_eq!(pixels[0][3], 0, "clear where nothing is drawn");
        assert_ne!(drawn(Turn::new(1)), thumb, "a turn turns it");
    }
}

//! Plain content sent by the Server to the Client.

use bevy::prelude::*;
use tesserae_core::{MaterialId, ShapeId};

#[derive(Message, Clone, Debug, Default, PartialEq, Eq)]
pub struct MaterialPalette(pub Vec<(MaterialId, [u8; 4])>);

/// Shared RGBA texture atlases, keyed by registered Material id.
#[derive(Message, Clone, Debug, Default, PartialEq, Eq)]
pub struct MaterialTextures(pub Vec<(MaterialId, Vec<u8>)>);

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

/// sRGB in `[0, 1]`. Never sent in a World whose Biomes carry no colours; the Client holds no
/// generator to work it out itself (ADR-0002).
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
    /// The mode a Player new to this World starts in.
    pub default_mode: tesserae_core::save::GameMode,
    /// The generator version this World was made with and keeps generating by.
    pub generator: u32,
    /// How far apart the Climate map's temperature and humidity change, in metres.
    pub temperature_size_m: f32,
    pub humidity_size_m: f32,
}

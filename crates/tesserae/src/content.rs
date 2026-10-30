//! Plain content sent by the Server to the Client.

use bevy::prelude::*;
use tesserae_core::{MaterialId, ShapeId};

#[derive(Message, Clone, Debug, Default, PartialEq, Eq)]
pub struct MaterialPalette(pub Vec<(MaterialId, [u8; 4])>);

/// Shared RGBA texture atlases, keyed by registered Material id.
#[derive(Message, Clone, Debug, Default, PartialEq, Eq)]
pub struct MaterialTextures(pub Vec<(MaterialId, Vec<u8>)>);

/// The Game's player figure, or `None` for the engine's plain box.
#[derive(Message, Clone, Debug, Default, PartialEq)]
pub struct CharacterModel(pub Option<std::sync::Arc<tesserae_mods::Character>>);

/// What the registered content is called and what a player can do with it.
/// Content only: it holds no Voxel, Chunk or Occupants, so a Client reading it
/// reads nothing of the World (ADR-0002).
#[derive(Message, Clone, Debug, Default, PartialEq, Eq)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogueMaterial {
    pub remove_time: std::time::Duration,
    pub id: MaterialId,
    pub name: String,
    pub solid: bool,
    /// Its fluid row's rank, which is the order the rows were declared in.
    pub fluid: Option<u16>,
    /// Whether a combustion row makes it fuel.
    pub burns: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogueShape {
    pub id: ShapeId,
    pub name: String,
    /// Whether turning it changes anything: a cube looks the same however it
    /// is turned and a connected Shape ignores its orientation.
    pub turnable: bool,
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

    /// The fluid declared first.
    pub fn first_fluid(&self) -> Option<MaterialId> {
        self.materials
            .iter()
            .filter_map(|m| Some((m.fluid?, m.id)))
            .min()
            .map(|(_, id)| id)
    }

    pub fn anything_burns(&self) -> bool {
        self.materials.iter().any(|m| m.burns)
    }
}

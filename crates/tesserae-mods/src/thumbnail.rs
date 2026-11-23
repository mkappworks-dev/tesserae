//! What a Mod or Game shows in menus: its picture and what it adds, read without running it.

use crate::error::ModError;
use crate::manifest::Found;

pub const THUMBNAIL_WIDTH: u32 = 512;
pub const THUMBNAIL_HEIGHT: u32 = 288;
/// Well above a detailed 512 by 288 illustration, which compresses to tens of kilobytes.
pub const MAX_THUMBNAIL_BYTES: u64 = 512 * 1024;

/// Straight-alpha sRGBA8 pixels, row by row from the top.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thumbnail {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// A Declaration larger than this is not read for a summary; loading still reports on it.
pub const MAX_SUMMARY_DECLARATION_BYTES: u64 = 4 << 20;

/// The swatch of a Material that declares no appearance.
pub(crate) const UNCOLOURED: [u8; 4] = [128, 128, 128, 255];

/// What a Mod adds, counted from its Declaration, so a picture and its counts cannot disagree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub materials: usize,
    pub shapes: usize,
    pub fluids: usize,
    /// One sRGBA colour per declared Material, in Declaration order.
    pub swatches: Vec<[u8; 4]>,
}

impl Found {
    /// What a data Mod declares, read without Trust: parsing data runs no Mod code. `None` for a
    /// component, which must run to declare anything, or a Declaration that does not parse.
    pub fn summary(&self) -> Option<Summary> {
        let name = self.manifest.declaration.as_ref()?;
        let bytes = crate::artwork::read(&self.dir, name, MAX_SUMMARY_DECLARATION_BYTES).ok()?;
        crate::data::summary(&self.manifest.id, &bytes)
    }

    /// The decoded `thumbnail` the Manifest names, or `None` when it names none. The error names
    /// the file and why it was refused.
    pub fn thumbnail(&self) -> Result<Option<Thumbnail>, ModError> {
        let Some(name) = &self.manifest.thumbnail else {
            return Ok(None);
        };
        let refused = |reason| refused(&self.manifest.id, reason);
        let bytes = crate::artwork::read(&self.dir, name, MAX_THUMBNAIL_BYTES).map_err(refused)?;
        Thumbnail::decode(&bytes, THUMBNAIL_WIDTH, THUMBNAIL_HEIGHT)
            .map(Some)
            .map_err(|reason| refused(format!("`{name}` {reason}")))
    }
}

impl Thumbnail {
    /// Decodes a PNG that must be exactly `width` by `height`, under the same bounds as a Mod's.
    pub fn decode(bytes: &[u8], width: u32, height: u32) -> Result<Self, String> {
        let rgba = crate::artwork::decode(bytes, width, height)?;
        Ok(Self {
            width,
            height,
            rgba,
        })
    }
}

/// A thumbnail the named Mod's Manifest points at but that cannot be used.
pub(crate) fn refused(who: &str, reason: String) -> ModError {
    ModError::Manifest {
        who: who.to_owned(),
        reason: format!("thumbnail {reason}"),
    }
}

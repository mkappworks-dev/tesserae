//! The picture a Mod or Game shows in menus: display data, read without running or trusting it.

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

impl Found {
    /// The decoded `thumbnail` the Manifest names, or `None` when it names none. The error names
    /// the file and why it was refused.
    pub fn thumbnail(&self) -> Result<Option<Thumbnail>, ModError> {
        let Some(name) = &self.manifest.thumbnail else {
            return Ok(None);
        };
        let refused = |reason: String| ModError::Manifest {
            who: self.manifest.id.clone(),
            reason: format!("thumbnail {reason}"),
        };
        let bytes = crate::artwork::read(&self.dir, name, MAX_THUMBNAIL_BYTES).map_err(refused)?;
        let rgba = crate::artwork::decode(&bytes, THUMBNAIL_WIDTH, THUMBNAIL_HEIGHT)
            .map_err(|reason| refused(format!("`{name}` {reason}")))?;
        Ok(Some(Thumbnail {
            width: THUMBNAIL_WIDTH,
            height: THUMBNAIL_HEIGHT,
            rgba,
        }))
    }
}

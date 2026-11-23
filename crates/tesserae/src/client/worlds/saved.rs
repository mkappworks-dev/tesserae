//! A World's thumbnail: its terrain drawn once at creation and kept in its folder, so the list
//! never runs a Generator.

use std::collections::HashMap;
use std::io::{self, Cursor};
use std::sync::Arc;
use std::time::SystemTime;

use bevy::tasks::AsyncComputeTaskPool;
use tesserae_core::terrain_preview::{Frame, Image as Picture, Source};

use super::*;

/// Well above any `Frame` a thumbnail is drawn at, so a huge file is refused before allocating.
const MAX_PIXELS: usize = 1024 * 1024;

/// Draws the new World's terrain and saves it beside the sidecar. Detached: the World opens
/// meanwhile, and a quit before it lands leaves the neutral picture.
pub(super) fn draw(dir: PathBuf, source: Arc<Source>, sidecar: &Sidecar) {
    let (seed, voxel_size_m) = (sidecar.seed, sidecar.voxel_size_m);
    let edits = sidecar.biomes.clone();
    AsyncComputeTaskPool::get_or_init(Default::default)
        .spawn(async move {
            let saved = source
                .render(seed, voxel_size_m, edits.as_ref(), &Frame::CREATE)
                .and_then(|picture| write(&dir, &picture).map_err(|e| e.to_string()));
            if let Err(e) = saved {
                warn!("No thumbnail for {}: {e}", dir.display());
            }
        })
        .detach();
}

fn write(dir: &Path, picture: &Picture) -> io::Result<()> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, picture.width, picture.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(io::Error::other)?;
    writer
        .write_image_data(&picture.rgba)
        .map_err(io::Error::other)?;
    writer.finish().map_err(io::Error::other)?;
    // The list may read it meanwhile: a rename shows it the whole file or none.
    let partial = dir.join(format!("{}.partial", library::THUMBNAIL));
    std::fs::write(&partial, bytes)?;
    std::fs::rename(partial, dir.join(library::THUMBNAIL))
}

fn read(file: &Path) -> Result<Picture, String> {
    let bytes = std::fs::read(file).map_err(|e| e.to_string())?;
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_limits(png::Limits {
        bytes: MAX_PIXELS * 4,
    });
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::ALPHA);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut rgba = vec![0; reader.output_buffer_size().ok_or("too large")?];
    let frame = reader.next_frame(&mut rgba).map_err(|e| e.to_string())?;
    if (frame.color_type, frame.bit_depth) != (png::ColorType::Rgba, png::BitDepth::Eight) {
        return Err("not 8-bit RGBA".into());
    }
    rgba.truncate(frame.buffer_size());
    Ok(Picture {
        centre_m: [0.0; 2],
        width: frame.width,
        height: frame.height,
        rgba,
    })
}

/// Each listed World's decoded thumbnail, with the file time it was read at.
#[derive(Default)]
pub(in crate::client) struct Pictures(HashMap<PathBuf, (SystemTime, Option<Handle<Image>>)>);

impl Pictures {
    pub(super) fn get(&self, folder: &Path) -> Option<&Handle<Image>> {
        self.0.get(folder)?.1.as_ref()
    }
}

/// Rereads only a thumbnail whose file changed, so a page rebuilt on every click costs a stat per
/// World. A folder reused after a Delete has a newer file, never the deleted World's picture.
pub(super) fn load(
    launcher: Option<Res<Launcher>>,
    images: Option<ResMut<Assets<Image>>>,
    mut thumbnails: ResMut<Thumbnails>,
    mut read_at: Local<Option<u64>>,
) {
    let (Some(l), Some(mut images)) = (launcher, images) else {
        return;
    };
    if *read_at == Some(l.version) {
        return;
    }
    *read_at = Some(l.version);
    let pictures = &mut thumbnails.1.0;
    pictures.retain(|folder, _| l.entries.iter().any(|e| &e.path == folder));
    for entry in &l.entries {
        let file = entry.path.join(library::THUMBNAIL);
        let Ok(modified) = std::fs::metadata(&file).and_then(|m| m.modified()) else {
            pictures.remove(&entry.path);
            continue;
        };
        if pictures
            .get(&entry.path)
            .is_some_and(|(at, _)| *at == modified)
        {
            continue;
        }
        let handle = read(&file)
            .inspect_err(|e| warn!("Thumbnail {} unreadable: {e}", file.display()))
            .ok()
            .map(|picture| images.add(terrain::image(picture)));
        pictures.insert(entry.path.clone(), (modified, handle));
    }
}

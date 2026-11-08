//! Local PNG artwork referenced by a data Declaration, read and decoded under fixed bounds.

use std::fs;
use std::io::{Cursor, Read};
use std::path::{Component, Path};

use png::{ColorType, Decoder, Limits, Transformations};

/// Far above a 64 by 64 image's worst case, so no honest file is refused.
pub(crate) const MAX_FILE_BYTES: u64 = 1 << 20;

/// Reads one referenced file of at most `max_bytes` from inside `dir`, refusing paths that
/// resolve outside it.
pub(crate) fn read(dir: &Path, name: &str, max_bytes: u64) -> Result<Vec<u8>, String> {
    let path = Path::new(name);
    if name.is_empty() || !path.components().all(|c| matches!(c, Component::Normal(_))) {
        return Err(format!(
            "`{name}` must be a relative path inside the mod's directory"
        ));
    }
    let real = dir
        .join(path)
        .canonicalize()
        .map_err(|e| format!("`{name}`: {e}"))?;
    let base = dir.canonicalize().map_err(|e| e.to_string())?;
    if !real.starts_with(&base) {
        return Err(format!("`{name}` resolves outside the mod's directory"));
    }
    let too_large = || format!("`{name}` is larger than {max_bytes} bytes");
    let file = fs::File::open(&real).map_err(|e| format!("`{name}`: {e}"))?;
    if file.metadata().is_ok_and(|m| m.len() > max_bytes) {
        return Err(too_large());
    }
    // The take still bounds a file that grows after the size check, or reports none.
    let mut bytes = Vec::new();
    file.take(max_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("`{name}`: {e}"))?;
    if bytes.len() as u64 > max_bytes {
        return Err(too_large());
    }
    Ok(bytes)
}

/// Straight-alpha sRGBA8 pixels of a PNG that is exactly `width` by `height`.
pub(crate) fn decode(bytes: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    let mut decoder = Decoder::new(Cursor::new(bytes));
    // Before any allocation: a 64 by 64 target never needs more than its own RGBA bytes plus slack.
    decoder.set_limits(Limits {
        bytes: (width as usize * height as usize * 4) * 2 + 4096,
    });
    decoder.set_transformations(Transformations::EXPAND | Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e| format!("not a PNG: {e}"))?;
    let info = reader.info();
    if (info.width, info.height) != (width, height) {
        return Err(format!(
            "is {} by {} pixels, expected {width} by {height}",
            info.width, info.height
        ));
    }
    let mut raw = vec![0; reader.output_buffer_size().ok_or("too large to decode")?];
    let frame = reader
        .next_frame(&mut raw)
        .map_err(|e| format!("invalid PNG: {e}"))?;
    let raw = &raw[..frame.buffer_size()];
    let rgba: Vec<u8> = match frame.color_type {
        ColorType::Rgba => raw.to_vec(),
        ColorType::Rgb => raw
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|&[r, g, b]| [r, g, b, 255])
            .collect(),
        ColorType::GrayscaleAlpha => raw
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|&[g, a]| [g, g, g, a])
            .collect(),
        ColorType::Grayscale => raw.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        ColorType::Indexed => return Err("indexed colour was not expanded".into()),
    };
    Ok(rgba)
}

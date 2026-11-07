//! What an Ogg Vorbis recording is, read from its first page by hand so the
//! crate needs no decoder: Bevy's already decodes it on the Client.

use std::ops::RangeInclusive;

/// The Vorbis identification header's channel count and sample rate.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Format {
    pub channels: u8,
    pub sample_rate: u32,
}

pub(crate) const CHANNELS: RangeInclusive<u8> = 1..=2;
/// In hertz.
pub(crate) const SAMPLE_RATE: RangeInclusive<u32> = 8_000..=96_000;
/// The largest Effects or Ambient recording.
pub(crate) const MAX_SHORT_BYTES: u64 = 512 << 10;
/// The largest Music recording.
pub(crate) const MAX_MUSIC_BYTES: u64 = 8 << 20;
/// All recordings a World holds, each reference counted, since each is a copy in memory.
pub(crate) const MAX_WORLD_BYTES: u64 = 32 << 20;

const NOT_VORBIS: &str = "is not an Ogg Vorbis file";

/// The format of an Ogg Vorbis file, from the identification header that must
/// open its first page.
pub(crate) fn identify(bytes: &[u8]) -> Result<Format, &'static str> {
    // The Ogg page header: capture pattern, version, the beginning-of-stream
    // flag, then the segment table after byte 26.
    if bytes.len() < 27 || &bytes[..4] != b"OggS" || bytes[4] != 0 || bytes[5] & 0x02 == 0 {
        return Err(NOT_VORBIS);
    }
    let segments = usize::from(bytes[26]);
    let table = bytes.get(27..27 + segments).ok_or(NOT_VORBIS)?;
    let packet = &bytes[27 + segments..];
    let header = packet.get(..30).ok_or(NOT_VORBIS)?;
    if table.first().is_none_or(|&lacing| lacing < 30)
        || header[0] != 1
        || &header[1..7] != b"vorbis"
        || header[7..11] != [0; 4]
        || header[29] & 1 == 0
    {
        return Err(NOT_VORBIS);
    }
    let sample_rate = u32::from_le_bytes(header[12..16].try_into().expect("four bytes"));
    if header[11] == 0 || sample_rate == 0 {
        return Err(NOT_VORBIS);
    }
    Ok(Format {
        channels: header[11],
        sample_rate,
    })
}

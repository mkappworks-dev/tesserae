//! A Player's Map in a commit, additive to the World header like Containers: a
//! commit without it opens with an empty Map, and nothing here touches the
//! format or generator version.

use std::collections::{BTreeSet, HashMap};

use std::io::{Read, Write};

use base64::{Engine, engine::general_purpose::STANDARD};
use flate2::Compression;
use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;
use serde::{Deserialize, Serialize};

use super::Error;
use crate::BiomeId;
use crate::map::{MAX_TILES, Map, Sample, TILE_SAMPLES, TileData};

const TILE_LEN: usize = TILE_SAMPLES * TILE_SAMPLES;
const PLANES: usize = 6;
const ABSENT: u8 = 0;
const NO_BIOME: u8 = 1;
const WITH_BIOME: u8 = 2;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedMap {
    pub tiles: Vec<SavedTile>,
}

/// One tile as deflated byte planes, base64 in the JSON. Planes, not
/// interleaved samples, so neighbouring ground makes long runs for deflate;
/// heights are stored as the change from the sample before, since ground is smooth.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedTile {
    pub at: [i32; 2],
    pub planes: String,
}

fn encode(samples: &[Option<Sample>]) -> String {
    let n = samples.len();
    let mut raw = vec![0u8; n * PLANES];
    let mut last = 0i16;
    for (i, sample) in samples.iter().enumerate() {
        let Some(s) = sample else { continue };
        raw[i] = if s.biome.is_some() {
            WITH_BIOME
        } else {
            NO_BIOME
        };
        raw[n + i] = s.biome.map_or(0, |b| b.index() as u8);
        let [m_lo, m_hi] = s.material.to_le_bytes();
        raw[2 * n + i] = m_lo;
        raw[3 * n + i] = m_hi;
        let [h_lo, h_hi] = s.height_m.wrapping_sub(last).to_le_bytes();
        raw[4 * n + i] = h_lo;
        raw[5 * n + i] = h_hi;
        last = s.height_m;
    }
    let mut deflater = DeflateEncoder::new(Vec::new(), Compression::default());
    deflater.write_all(&raw).expect("writing to a Vec");
    STANDARD.encode(deflater.finish().expect("writing to a Vec"))
}

fn decode(planes: &str) -> Option<Vec<Option<Sample>>> {
    let packed = STANDARD.decode(planes).ok()?;
    let mut raw = Vec::with_capacity(TILE_LEN * PLANES);
    // One byte past the expected length, so an overlong stream is refused without being inflated whole.
    DeflateDecoder::new(packed.as_slice())
        .take(TILE_LEN as u64 * PLANES as u64 + 1)
        .read_to_end(&mut raw)
        .ok()?;
    if raw.len() != TILE_LEN * PLANES {
        return None;
    }
    let n = TILE_LEN;
    let mut last = 0i16;
    (0..n)
        .map(|i| match raw[i] {
            ABSENT => Some(None),
            tag @ (NO_BIOME | WITH_BIOME) => {
                let height_m =
                    last.wrapping_add(i16::from_le_bytes([raw[4 * n + i], raw[5 * n + i]]));
                last = height_m;
                Some(Some(Sample {
                    height_m,
                    material: u16::from_le_bytes([raw[2 * n + i], raw[3 * n + i]]),
                    biome: (tag == WITH_BIOME).then(|| BiomeId::new(raw[n + i])),
                }))
            }
            _ => None,
        })
        .collect()
}

/// Tiles already encoded, by coordinate, with the Map revision each was encoded at.
pub type EncodedTiles = HashMap<[i32; 2], (u64, SavedTile)>;

impl SavedMap {
    pub fn capture(map: &Map) -> Self {
        Self::capture_cached(map, &mut EncodedTiles::new()).0
    }

    /// Re-encodes only the tiles whose revision differs from the cache's; returns how many that was.
    pub fn capture_cached(map: &Map, cache: &mut EncodedTiles) -> (Self, usize) {
        let mut encoded = 0;
        let tiles = map
            .tiles_revised()
            .map(|(at, revision, samples)| {
                let entry = cache
                    .entry(*at)
                    .or_insert_with(|| (0, SavedTile::default()));
                if entry.0 != revision || entry.1.planes.is_empty() {
                    *entry = (
                        revision,
                        SavedTile {
                            at: *at,
                            planes: encode(samples),
                        },
                    );
                    encoded += 1;
                }
                entry.1.clone()
            })
            .collect();
        (SavedMap { tiles }, encoded)
    }

    /// Seeds `cache` with tiles just restored into `map`, which are already encoded.
    pub fn seed(&self, map: &Map, cache: &mut EncodedTiles) {
        for tile in &self.tiles {
            if let Some(revision) = map.tile_revision(tile.at) {
                cache.insert(tile.at, (revision, tile.clone()));
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }

    pub fn validate(&self) -> Result<(), Error> {
        let bad = |why: &str| Err(Error::Corrupt(format!("map: {why}")));
        if self.tiles.len() > MAX_TILES {
            return bad("more tiles than a Map holds");
        }
        let mut seen = BTreeSet::new();
        for tile in &self.tiles {
            if !seen.insert(tile.at) {
                return bad("two tiles at one coordinate");
            }
            if decode(&tile.planes).is_none() {
                return bad("a tile that does not decode to a whole tile of samples");
            }
        }
        Ok(())
    }

    /// Every tile is queued to be sent, since no Client has seen a saved Map.
    pub fn restore(&self) -> Map {
        let mut map = Map::default();
        for tile in &self.tiles {
            if let Some(samples) = decode(&tile.planes) {
                map.load_tile(TileData {
                    coord: tile.at,
                    samples,
                });
            }
        }
        map.mark_all_changed();
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(height_m: i16, biome: Option<u8>) -> Sample {
        Sample {
            height_m,
            material: 300,
            biome: biome.map(BiomeId::new),
        }
    }

    fn explored() -> Map {
        let mut map = Map::default();
        for x in -70..70 {
            for z in 0..10 {
                map.record([x, z], sample((x / 8) as i16, Some((z % 3) as u8)));
            }
        }
        map.record([5, 40], sample(-4, None));
        map
    }

    #[test]
    fn a_map_round_trips_through_json_sample_for_sample() {
        let map = explored();
        let json = serde_json::to_vec(&SavedMap::capture(&map)).unwrap();
        let saved: SavedMap = serde_json::from_slice(&json).unwrap();
        saved.validate().unwrap();
        let back = saved.restore();
        assert_eq!(back.tile_count(), map.tile_count());
        for ((ca, a), (cb, b)) in map.tiles().zip(back.tiles()) {
            assert_eq!((ca, a), (cb, b));
        }
    }

    #[test]
    fn a_restored_map_queues_every_tile_and_an_empty_map_saves_nothing() {
        let back = SavedMap::capture(&explored()).restore();
        assert_eq!(back.changed_count(), back.tile_count());
        assert!(SavedMap::capture(&Map::default()).is_empty());
    }

    #[test]
    fn a_tile_is_far_smaller_than_its_raw_samples() {
        let json = serde_json::to_vec(&SavedMap::capture(&explored())).unwrap();
        let raw = explored().tile_count() * TILE_LEN * 4;
        assert!(json.len() * 10 < raw, "{} of {raw}", json.len());
    }

    #[test]
    fn only_a_changed_sample_moves_the_revision() {
        let mut map = Map::default();
        map.record([0, 0], sample(1, None));
        let before = map.revision();
        map.record([0, 0], sample(1, None));
        assert_eq!(map.revision(), before);
        map.record([0, 0], sample(2, None));
        assert_ne!(map.revision(), before);
    }

    #[test]
    fn a_cached_capture_re_encodes_only_the_tile_that_changed() {
        let mut map = Map::default();
        map.record([0, 0], sample(1, None));
        map.record([TILE_SAMPLES as i32, 0], sample(2, None));
        let mut cache = EncodedTiles::new();
        let (first, encoded) = SavedMap::capture_cached(&map, &mut cache);
        assert_eq!(encoded, 2);
        assert_eq!(SavedMap::capture_cached(&map, &mut cache).1, 0);
        map.record([0, 0], sample(9, None));
        let (second, encoded) = SavedMap::capture_cached(&map, &mut cache);
        assert_eq!(encoded, 1);
        assert_eq!(second, SavedMap::capture(&map));
        assert_eq!(second.tiles[1], first.tiles[1]);
    }

    fn refused(change: impl FnOnce(&mut SavedMap)) -> String {
        let mut saved = SavedMap::capture(&explored());
        change(&mut saved);
        saved.validate().unwrap_err().to_string()
    }

    #[test]
    fn a_corrupt_map_is_refused_with_a_clear_error() {
        assert!(refused(|m| m.tiles[0].planes = "!!".into()).contains("map:"));
        assert!(refused(|m| m.tiles[0].planes = encode(&[None; 3])).contains("map:"));
        assert!(refused(|m| m.tiles[1].at = m.tiles[0].at).contains("two tiles"));
        assert!(
            refused(|m| m.tiles = vec![m.tiles[0].clone(); MAX_TILES + 1]).contains("more tiles")
        );
        let overlong = encode(&vec![None; TILE_LEN + 1]);
        assert!(refused(|m| m.tiles[0].planes = overlong).contains("map:"));
    }
}

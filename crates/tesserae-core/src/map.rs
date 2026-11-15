//! A Player's Map: the surface they have seen, one sample per column of
//! [`SAMPLE_M`] metres, kept in fixed square tiles.

use std::collections::{BTreeSet, HashMap};

use crate::{BiomeId, MaterialId};

pub const SAMPLE_M: f32 = 2.0;
pub const REACH_M: f32 = 96.0;
pub const TILE_SAMPLES: usize = 64;
pub const TILE_M: f32 = SAMPLE_M * TILE_SAMPLES as f32;
/// About 16 MB of samples at four bytes each, 8 km of ground in a square.
pub const MAX_TILES: usize = 1024;

/// A column's surface as it was when seen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Sample {
    pub height_m: i16,
    pub material: MaterialId,
    pub biome: Option<BiomeId>,
}

/// Which column sample a point in metres falls in; a floor, so negative
/// positions do not fold onto positive ones.
pub fn cell_of(x_m: f32, z_m: f32) -> [i32; 2] {
    [
        (x_m / SAMPLE_M).floor() as i32,
        (z_m / SAMPLE_M).floor() as i32,
    ]
}

pub fn cell_centre_m(cell: [i32; 2]) -> (f32, f32) {
    (
        (cell[0] as f32 + 0.5) * SAMPLE_M,
        (cell[1] as f32 + 0.5) * SAMPLE_M,
    )
}

/// The tile holding a cell and the cell's index within the tile's samples.
pub fn locate(cell: [i32; 2]) -> ([i32; 2], usize) {
    let n = TILE_SAMPLES as i32;
    let tile = [cell[0].div_euclid(n), cell[1].div_euclid(n)];
    let at = cell[1].rem_euclid(n) as usize * TILE_SAMPLES + cell[0].rem_euclid(n) as usize;
    (tile, at)
}

#[derive(Clone, Debug)]
struct Tile(Vec<Option<Sample>>, u64);

/// One tile as it crosses to the Client: `TILE_SAMPLES` squared samples, row by row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TileData {
    pub coord: [i32; 2],
    pub samples: Vec<Option<Sample>>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Recorded {
    Stored,
    /// The Map holds [`Map::max_tiles`] tiles and this sample needs a new one.
    Full,
}

#[derive(Clone, Debug)]
pub struct Map {
    tiles: HashMap<[i32; 2], Tile>,
    max_tiles: usize,
    /// Tiles not yet handed out by [`Map::take_changed`].
    changed: BTreeSet<[i32; 2]>,
    /// Counts every change to the samples, so a save can tell an unchanged Map from a changed one.
    revision: u64,
}

impl Default for Map {
    fn default() -> Self {
        Map::with_max_tiles(MAX_TILES)
    }
}

impl Map {
    pub fn with_max_tiles(max_tiles: usize) -> Self {
        Map {
            tiles: HashMap::new(),
            max_tiles,
            changed: BTreeSet::new(),
            revision: 0,
        }
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Every tile, lowest coordinate first, so equal Maps iterate alike.
    pub fn tiles(&self) -> impl Iterator<Item = (&[i32; 2], &[Option<Sample>])> {
        let mut all: Vec<_> = self.tiles.iter().map(|(c, t)| (c, &t.0[..])).collect();
        all.sort_unstable_by_key(|(c, _)| **c);
        all.into_iter()
    }

    /// Like [`Map::tiles`], with the Map revision at which each tile last changed, so a save re-encodes
    /// only the tiles that moved.
    pub fn tiles_revised(&self) -> impl Iterator<Item = (&[i32; 2], u64, &[Option<Sample>])> {
        let mut all: Vec<_> = self.tiles.iter().map(|(c, t)| (c, t.1, &t.0[..])).collect();
        all.sort_unstable_by_key(|(c, _, _)| **c);
        all.into_iter()
    }

    /// One tile's samples, row by row, for a reader that walks many neighbouring cells.
    pub fn tile(&self, coord: [i32; 2]) -> Option<&[Option<Sample>]> {
        self.tiles.get(&coord).map(|t| &t.0[..])
    }

    pub fn tile_revision(&self, coord: [i32; 2]) -> Option<u64> {
        self.tiles.get(&coord).map(|t| t.1)
    }

    pub fn max_tiles(&self) -> usize {
        self.max_tiles
    }

    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    /// Replaces any earlier sample of the column.
    pub fn record(&mut self, cell: [i32; 2], sample: Sample) -> Recorded {
        let (tile, at) = locate(cell);
        if !self.tiles.contains_key(&tile) && self.tiles.len() >= self.max_tiles {
            return Recorded::Full;
        }
        let slot = self
            .tiles
            .entry(tile)
            .or_insert_with(|| Tile(vec![None; TILE_SAMPLES * TILE_SAMPLES], 0));
        // A re-scan of unchanged ground must not resend its tile.
        if slot.0[at] != Some(sample) {
            slot.0[at] = Some(sample);
            self.changed.insert(tile);
            self.revision += 1;
            slot.1 = self.revision;
        }
        Recorded::Stored
    }

    /// Queues every tile again, for a Map loaded from a save or a Client that starts empty.
    pub fn mark_all_changed(&mut self) {
        self.changed.extend(self.tiles.keys().copied());
    }

    pub fn changed_count(&self) -> usize {
        self.changed.len()
    }

    /// At most `max` changed tiles, lowest coordinate first, each cleared as it is handed out.
    pub fn take_changed(&mut self, max: usize) -> Vec<TileData> {
        let coords: Vec<_> = self.changed.iter().take(max).copied().collect();
        coords
            .into_iter()
            .map(|coord| {
                self.changed.remove(&coord);
                TileData {
                    coord,
                    samples: self.tiles[&coord].0.clone(),
                }
            })
            .collect()
    }

    /// Stores a tile as received, past the cap and without queueing it to be sent on.
    pub fn load_tile(&mut self, tile: TileData) {
        if tile.samples.len() == TILE_SAMPLES * TILE_SAMPLES {
            self.revision += 1;
            self.tiles
                .insert(tile.coord, Tile(tile.samples, self.revision));
        }
    }

    pub fn sample(&self, cell: [i32; 2]) -> Option<Sample> {
        let (tile, at) = locate(cell);
        self.tiles.get(&tile)?.0[at]
    }

    /// Explored is a sample having been recorded, so fog is the absence of one.
    pub fn explored(&self, x_m: f32, z_m: f32) -> bool {
        self.sample(cell_of(x_m, z_m)).is_some()
    }

    pub fn explored_count(&self) -> usize {
        self.tiles
            .values()
            .map(|t| t.0.iter().flatten().count())
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(height_m: i16) -> Sample {
        Sample {
            height_m,
            material: 3,
            biome: Some(BiomeId::new(1)),
        }
    }

    #[test]
    fn a_point_in_metres_falls_in_the_floor_cell_on_both_sides_of_zero() {
        assert_eq!(cell_of(0.0, 1.99), [0, 0]);
        assert_eq!(cell_of(2.0, -0.01), [1, -1]);
        assert_eq!(cell_of(-2.0, -2.01), [-1, -2]);
    }

    #[test]
    fn a_cell_centre_is_in_its_own_cell() {
        for cell in [[0, 0], [-1, 5], [-65, -64]] {
            let (x, z) = cell_centre_m(cell);
            assert_eq!(cell_of(x, z), cell);
        }
    }

    #[test]
    fn a_recorded_column_is_explored_and_its_neighbour_is_not() {
        let mut map = Map::default();
        assert!(!map.explored(3.0, 3.0));
        assert_eq!(map.record(cell_of(3.0, 3.0), sample(7)), Recorded::Stored);
        assert!(map.explored(2.5, 3.9));
        assert!(!map.explored(4.5, 3.0));
        assert_eq!(map.explored_count(), 1);
    }

    #[test]
    fn a_later_sample_replaces_an_earlier_one() {
        let mut map = Map::default();
        map.record([-3, 9], sample(4));
        map.record([-3, 9], sample(9));
        assert_eq!(map.sample([-3, 9]), Some(sample(9)));
        assert_eq!(map.explored_count(), 1);
    }

    #[test]
    fn samples_across_a_tile_edge_share_no_tile() {
        let mut map = Map::default();
        let edge = TILE_SAMPLES as i32;
        map.record([edge - 1, 0], sample(1));
        map.record([edge, 0], sample(2));
        map.record([-1, 0], sample(3));
        assert_eq!(map.tile_count(), 3);
    }

    #[test]
    fn a_full_map_keeps_recording_into_its_tiles_but_takes_no_new_one() {
        let mut map = Map::with_max_tiles(1);
        assert_eq!(map.record([0, 0], sample(1)), Recorded::Stored);
        assert_eq!(map.record([1, 0], sample(2)), Recorded::Stored);
        assert_eq!(
            map.record([TILE_SAMPLES as i32, 0], sample(3)),
            Recorded::Full
        );
        assert_eq!(map.tile_count(), 1);
        assert!(!map.explored(TILE_M + 1.0, 1.0));
    }

    #[test]
    fn only_changed_tiles_are_taken_and_an_identical_rescan_changes_nothing() {
        let mut map = Map::default();
        map.record([0, 0], sample(1));
        map.record([TILE_SAMPLES as i32, 0], sample(2));
        assert_eq!(map.changed_count(), 2);
        let first = map.take_changed(1);
        assert_eq!(first.len(), 1);
        assert_eq!(map.changed_count(), 1);
        assert_eq!(map.take_changed(8).len(), 1);
        map.record([0, 0], sample(1));
        assert_eq!(map.changed_count(), 0);
        map.record([0, 0], sample(5));
        assert_eq!(map.take_changed(8)[0].coord, [0, 0]);
    }

    #[test]
    fn marking_all_changed_queues_every_tile_again() {
        let mut map = Map::default();
        map.record([0, 0], sample(1));
        map.record([-1, -1], sample(2));
        map.take_changed(8);
        map.mark_all_changed();
        assert_eq!(map.take_changed(8).len(), 2);
    }

    #[test]
    fn a_loaded_tile_is_read_back_past_the_cap_and_is_not_queued() {
        let mut sender = Map::default();
        sender.record([3, 4], sample(6));
        let tile = sender.take_changed(1).remove(0);
        let mut receiver = Map::with_max_tiles(0);
        receiver.load_tile(tile);
        assert_eq!(receiver.sample([3, 4]), Some(sample(6)));
        assert_eq!(receiver.changed_count(), 0);
    }

    #[test]
    fn a_tile_of_the_wrong_length_is_refused() {
        let mut map = Map::default();
        map.load_tile(TileData {
            coord: [0, 0],
            samples: vec![None; 3],
        });
        assert_eq!(map.tile_count(), 0);
    }
}

//! The Server's record of when each level-0 Chunk entered the mesh extent,
//! joined with the Client's of when it was drawn.

use std::time::{Duration, Instant};

use bevy::platform::collections::HashMap;

use crate::server::streaming::stats::Entered;
use crate::world::ChunkCoord;

/// A level-0 Chunk the Client started (`true`) or stopped drawing.
pub(super) type Drawn = (Instant, ChunkCoord, bool);

/// Per entry, how long after it the Chunk was drawn: zero if it already was,
/// `None` if it never was.
pub(super) fn drawn_after(entered: &[Entered], drawn: &[Drawn]) -> Vec<Option<Duration>> {
    let mut by_chunk: HashMap<ChunkCoord, Vec<(Instant, bool)>> = HashMap::new();
    for &(at, coord, on) in drawn {
        by_chunk.entry(coord).or_default().push((at, on));
    }
    entered
        .iter()
        .map(|entry| {
            let changes = by_chunk.get(&entry.coord)?;
            let next = changes.partition_point(|&(at, _)| at <= entry.at);
            if next > 0 && changes[next - 1].1 {
                return Some(Duration::ZERO);
            }
            changes[next..]
                .iter()
                .find(|&&(_, on)| on)
                .map(|&(at, _)| at - entry.at)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(start: Instant, ms: u64) -> Instant {
        start + Duration::from_millis(ms)
    }

    fn entry(at: Instant, x: i32) -> Entered {
        Entered {
            at,
            coord: ChunkCoord::new(x, 0, 0),
            distance_m: 0.0,
        }
    }

    #[test]
    fn a_chunk_is_drawn_after_entering_when_its_mesh_first_lands() {
        let t = Instant::now();
        let a = ChunkCoord::new(0, 0, 0);
        let drawn = [
            (at(t, 30), a, true),
            (at(t, 50), a, false),
            (at(t, 90), a, true),
        ];
        let entered = [
            entry(at(t, 10), 0),
            entry(at(t, 60), 0),
            entry(at(t, 100), 1),
        ];
        assert_eq!(
            drawn_after(&entered, &drawn),
            [
                Some(Duration::from_millis(20)),
                Some(Duration::from_millis(30)),
                None
            ]
        );
    }

    #[test]
    fn a_chunk_still_drawn_as_it_enters_is_drawn_at_once() {
        let t = Instant::now();
        let a = ChunkCoord::new(0, 0, 0);
        let drawn = [(at(t, 30), a, true)];
        assert_eq!(
            drawn_after(&[entry(at(t, 40), 0)], &drawn),
            [Some(Duration::ZERO)]
        );
    }
}

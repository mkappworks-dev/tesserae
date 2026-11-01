//! Meshes being built off the main thread, and which of them may land.

use std::collections::VecDeque;
use std::hash::Hash;

use bevy::platform::collections::HashMap;
use bevy::tasks::futures::check_ready;

/// Builds per key, oldest first.
pub(super) struct InFlight<K, F> {
    queues: HashMap<K, VecDeque<F>>,
}

impl<K, F> Default for InFlight<K, F> {
    fn default() -> Self {
        Self {
            queues: HashMap::default(),
        }
    }
}

impl<K: Copy + Eq + Hash, F: Future + Unpin> InFlight<K, F> {
    pub(super) fn send(&mut self, key: K, build: F) {
        self.queues.entry(key).or_default().push_back(build);
    }

    pub(super) fn cancel(&mut self, key: K) {
        self.queues.remove(&key);
    }

    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.queues.is_empty()
    }

    /// Each key's newest finished build. Every older one is dropped, finished
    /// or not: landing it would draw an older mesh over a newer one.
    pub(super) fn landed(&mut self) -> Vec<(K, F::Output)> {
        let mut landed = Vec::new();
        self.queues.retain(|&key, queue| {
            let newest = (0..queue.len())
                .rev()
                .find_map(|i| check_ready(&mut queue[i]).map(|built| (i, built)));
            if let Some((i, built)) = newest {
                queue.drain(..=i);
                landed.push((key, built));
            }
            !queue.is_empty()
        });
        landed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::pin::Pin;
    use std::rc::Rc;
    use std::task::{Context, Poll};

    /// A build the test finishes by hand.
    #[derive(Clone, Default)]
    struct Build(Rc<Cell<Option<&'static str>>>);

    impl Build {
        fn finish(&self, mesh: &'static str) {
            self.0.set(Some(mesh));
        }
    }

    impl Future for Build {
        type Output = &'static str;
        fn poll(self: Pin<&mut Self>, _: &mut Context) -> Poll<&'static str> {
            self.0.take().map_or(Poll::Pending, Poll::Ready)
        }
    }

    #[test]
    fn a_mesh_never_lands_after_a_newer_one_for_the_same_chunk() {
        let mut in_flight = InFlight::default();
        let (older, newer) = (Build::default(), Build::default());
        in_flight.send('a', older.clone());
        in_flight.send('a', newer.clone());

        newer.finish("newer");
        assert_eq!(in_flight.landed(), [('a', "newer")]);
        older.finish("older");
        assert_eq!(in_flight.landed(), []);
    }

    /// A Chunk sent a mesh every frame still lands each one that finishes
    /// first, rather than waiting for the stream to pause.
    #[test]
    fn an_older_mesh_that_finishes_first_lands_and_the_newer_one_follows() {
        let mut in_flight = InFlight::default();
        let (older, newer, other) = (Build::default(), Build::default(), Build::default());
        in_flight.send('a', older.clone());
        in_flight.send('a', newer.clone());
        in_flight.send('b', other.clone());

        older.finish("older");
        assert_eq!(in_flight.landed(), [('a', "older")]);
        newer.finish("newer");
        other.finish("other");
        let mut landed = in_flight.landed();
        landed.sort();
        assert_eq!(landed, [('a', "newer"), ('b', "other")]);
    }
}

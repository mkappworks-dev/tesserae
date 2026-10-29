use super::*;

const SIZE: i32 = CHUNK_SIZE as i32;

fn bounded(capacity: usize, reserve: usize, scan_per_step: usize) -> Agenda {
    Agenda::new(Limits {
        capacity,
        reserve,
        scan_per_step,
    })
}

/// Runs steps that do nothing with what they take until the agenda is empty,
/// returning every Voxel each step took.
fn drain(agenda: &mut Agenda, mut step: Step) -> Vec<(Step, Vec<Cell>)> {
    let mut steps = Vec::new();
    while !agenda.is_empty() {
        let taken = agenda.take_due(step);
        steps.push((step, taken.cells));
        agenda.end_step();
        step += 1;
        assert!(step < 100_000, "never drained");
    }
    steps
}

#[test]
fn a_voxel_woken_twice_is_due_once_at_the_earlier_step() {
    let mut agenda = Agenda::default();
    agenda.wake([1, 2, 3], 9);
    agenda.wake([1, 2, 3], 4);
    agenda.wake([1, 2, 3], 7);
    assert_eq!(agenda.len(), 1);
    assert_eq!(agenda.next_due(), Some(4));
    assert!(agenda.take_due(3).cells.is_empty());
    assert_eq!(agenda.take_due(4).cells, vec![[1, 2, 3]]);
}

#[test]
fn due_voxels_come_out_in_coordinate_order() {
    let mut agenda = Agenda::default();
    for cell in [[3, 0, 0], [-1, 5, 2], [0, 0, 0], [-1, 5, 1]] {
        agenda.wake(cell, 0);
    }
    assert_eq!(
        agenda.take_due(0).cells,
        vec![[-1, 5, 1], [-1, 5, 2], [0, 0, 0], [3, 0, 0]]
    );
}

/// Past capacity a wake is coalesced, never lost: the agenda never holds more
/// than its bound, and every Voxel woken is taken by some later step.
#[test]
fn wakes_past_capacity_become_rescans_and_none_is_lost() {
    let mut agenda = bounded(16, 0, 8);
    let woken: Vec<Cell> = (0..40).map(|i| [i % 7, i / 7, -3]).collect();
    for &cell in &woken {
        agenda.wake(cell, 1);
        assert!(agenda.len() <= 16);
    }
    assert_eq!(agenda.overflowed(), 24);
    assert_eq!(agenda.rescans(), 1);
    let mut seen = BTreeSet::new();
    for (_, cells) in drain(&mut agenda, 1) {
        assert!(cells.len() <= 16, "{} in one step", cells.len());
        seen.extend(cells);
    }
    for cell in woken {
        assert!(seen.contains(&cell), "{cell:?} was dropped");
    }
}

/// A rescan visits every Voxel of its spans, a budget's worth per step,
/// resuming where the cursor stopped.
#[test]
fn a_rescan_is_served_a_budget_per_step_from_a_cursor() {
    let mut agenda = bounded(usize::MAX, 0, 100);
    agenda.rescan(Span::chunk([-1, 0, 2]));
    let steps = drain(&mut agenda, 0);
    assert_eq!(steps.len(), (SIZE * SIZE * SIZE) as usize / 100 + 1);
    let all: BTreeSet<Cell> = steps.iter().flat_map(|(_, c)| c.iter().copied()).collect();
    assert_eq!(all.len(), (SIZE * SIZE * SIZE) as usize);
    assert!(all.iter().all(|&c| chunk_of(c).0 == [-1, 0, 2]));
    assert!(steps.iter().all(|(_, c)| c.len() <= 100));
}

/// Three Chunks rescanning share each step's budget, and the turn carries on
/// from the last one served, so a budget smaller than the Chunks still reaches
/// every one.
#[test]
fn rescans_take_turns_round_the_chunks() {
    let mut agenda = bounded(usize::MAX, 0, 6);
    let chunks = [[0, 0, 0], [5, 0, 0], [-2, 1, 0]];
    for c in chunks {
        agenda.rescan(Span::chunk(c));
    }
    let taken = agenda.take_due(0).cells;
    for c in chunks {
        assert_eq!(taken.iter().filter(|&&t| chunk_of(t).0 == c).count(), 2);
    }
    agenda.end_step();

    let mut agenda = bounded(usize::MAX, 0, 1);
    for c in chunks {
        agenda.rescan(Span::chunk(c));
    }
    let served: Vec<[i32; 3]> = (0..6)
        .map(|step| {
            let taken = agenda.take_due(step).cells;
            agenda.end_step();
            chunk_of(taken[0]).0
        })
        .collect();
    assert_eq!(
        served,
        vec![
            [-2, 1, 0],
            [0, 0, 0],
            [5, 0, 0],
            [-2, 1, 0],
            [0, 0, 0],
            [5, 0, 0]
        ]
    );
}

/// When wakes hold every ordinary entry, rescanned Voxels still get the
/// reserve, and only the reserve.
#[test]
fn the_reserve_is_open_to_rescans_alone() {
    let mut agenda = bounded(8, 4, 100);
    for i in 0..8 {
        agenda.wake([i, 0, 0], 50);
    }
    agenda.wake([0, 9, 0], 1);
    assert_eq!(agenda.len(), 8, "a wake took the reserve");
    agenda.rescan(Span::chunk([3, 3, 3]));
    let taken = agenda.take_due(1).cells;
    assert_eq!(taken.len(), 4, "exactly the reserve");
    assert!(taken.contains(&[0, 9, 0]), "the overflowed wake came back");
    // Taken Voxels keep their entries until the step ends, so a rewake fits.
    for &cell in &taken {
        agenda.rewake(cell, 2);
    }
    assert_eq!(agenda.len(), 12);
    agenda.end_step();
}

/// A step's Voxels hold their entries while it runs, so what it wakes them
/// for again is never refused, however full the agenda got meanwhile.
#[test]
fn a_rewake_of_a_voxel_the_step_took_always_fits() {
    let mut agenda = bounded(4, 0, 0);
    for i in 0..4 {
        agenda.wake([i, 0, 0], 0);
    }
    let taken = agenda.take_due(0).cells;
    agenda.wake([9, 9, 9], 1);
    assert_eq!(agenda.overflowed(), 1, "the step's Voxels still count");
    for &cell in &taken {
        agenda.rewake(cell, 5);
    }
    agenda.end_step();
    assert_eq!(agenda.len(), 4);
    assert_eq!(agenda.overflowed(), 1);
}

/// Parked Voxels sleep: they cost nothing until released, then are rescanned.
#[test]
fn parked_voxels_sleep_until_released() {
    let mut agenda = Agenda::default();
    agenda.wake([1, 1, 1], 0);
    agenda.wake([4, 1, 2], 0);
    let taken = agenda.take_due(0).cells;
    for cell in taken {
        agenda.park(cell);
    }
    agenda.end_step();
    assert!(agenda.is_empty());
    assert!(drain(&mut agenda, 1).is_empty());
    agenda.unpark([0, 0, 0]);
    let woken: BTreeSet<Cell> = drain(&mut agenda, 10)
        .into_iter()
        .flat_map(|(_, c)| c)
        .collect();
    assert!(woken.contains(&[1, 1, 1]) && woken.contains(&[4, 1, 2]));
    assert!(woken.iter().all(|c| {
        Span {
            min: [1, 1, 1],
            max: [4, 1, 2],
        }
        .contains_cell(*c)
    }));
}

#[test]
fn forgetting_a_chunk_drops_all_of_its_work_and_nothing_else() {
    let mut agenda = Agenda::default();
    agenda.wake([-1, 0, 0], 3);
    agenda.wake([0, 0, 0], 3);
    agenda.rescan(Span::chunk([-1, 0, 0]));
    agenda.park([-5, 0, 0]);
    agenda.forget([-1, 0, 0]);
    assert_eq!(agenda.len(), 1);
    assert_eq!(agenda.rescans(), 0);
    agenda.unpark([-1, 0, 0]);
    assert_eq!(drain(&mut agenda, 3), vec![(3, vec![[0, 0, 0]])]);
}

#[test]
fn a_box_splits_along_chunk_boundaries_including_negative_ones() {
    let spans = Span::split([-2, 30, 5], [1, 33, 5]);
    assert_eq!(spans.len(), 4);
    let total: usize = spans.iter().map(Span::len).sum();
    assert_eq!(total, 4 * 4);
    assert!(spans.iter().all(|s| chunk_of(s.min).0 == chunk_of(s.max).0));
    assert_eq!(spans[0].home(), [-1, 0, 0]);
    assert_eq!(spans[3].home(), [0, 1, 0]);
}

/// Spans pile up into one covering box rather than without bound.
#[test]
fn a_chunks_spans_stay_bounded() {
    let mut agenda = Agenda::default();
    for i in 0..100 {
        agenda.rescan(Span::cell([i % 30, i / 30, 0]));
    }
    let rescan = &agenda.rescans[&[0, 0, 0]];
    assert!(rescan.next.len() <= MAX_SPANS);
    for i in 0..100 {
        let cell = [i % 30, i / 30, 0];
        assert!(rescan.next.iter().any(|s| s.contains_cell(cell)));
    }
}

/// The scanner moves to the next span lazily, so a checkpoint can catch the
/// cursor exactly at the end of one; that state must save and reload.
#[test]
fn a_cursor_at_the_end_of_a_span_saves_and_resumes() {
    let mut agenda = bounded(usize::MAX, 0, 4);
    let first = Span {
        min: [0, 0, 0],
        max: [1, 1, 0],
    };
    let second = Span {
        min: [8, 0, 0],
        max: [9, 1, 0],
    };
    agenda.rescan(first);
    agenda.rescan(second);
    let taken = agenda.take_due(0);
    assert_eq!(taken.cells.len(), 4);
    agenda.end_step();
    let saved = agenda.checkpoint(1).unwrap();
    let mut restored = Agenda::restore(&saved, 1).unwrap();
    let rest: BTreeSet<Cell> = drain(&mut restored, 1)
        .into_iter()
        .flat_map(|(_, cells)| cells)
        .collect();
    let expected: BTreeSet<Cell> = (0..4).map(|i| second.nth(i)).collect();
    assert_eq!(rest, expected);
}

//! The ring-buffer contract for the rendered trail.

use std::collections::VecDeque;

use attractor_core::{RandomSource, SplitMix64, State, TraceBuffer};

fn point(value: f64) -> State {
    State::new(value, 0.0, 0.0)
}

fn coordinates(buffer: &TraceBuffer) -> Vec<f64> {
    buffer.iter().map(|state| state.x).collect()
}

#[test]
fn holds_points_in_insertion_order_until_it_is_full() {
    let mut buffer = TraceBuffer::new(3, point(0.0));
    buffer.append(point(1.0));
    buffer.append(point(2.0));
    assert_eq!(buffer.len(), 3);
    assert_eq!(coordinates(&buffer), [0.0, 1.0, 2.0]);
}

#[test]
fn drops_the_oldest_point_once_full_and_stays_in_order() {
    let mut buffer = TraceBuffer::new(3, point(0.0));
    for value in 1..=5 {
        buffer.append(point(f64::from(value)));
    }
    assert_eq!(buffer.len(), 3);
    assert_eq!(coordinates(&buffer), [3.0, 4.0, 5.0]);
    assert_eq!(buffer.first(), Some(point(3.0)));
    assert_eq!(buffer.last(), Some(point(5.0)));
}

#[test]
fn capacity_is_clamped_to_at_least_one_point() {
    let mut buffer = TraceBuffer::new(0, point(1.0));
    assert_eq!(buffer.capacity(), 1);
    buffer.append(point(2.0));
    assert_eq!(buffer.len(), 1);
    assert_eq!(buffer.last(), Some(point(2.0)));
}

#[test]
fn indexes_stay_correct_after_several_wraps() {
    let mut buffer = TraceBuffer::new(2, point(0.0));
    for value in 1..=9 {
        buffer.append(point(f64::from(value)));
    }
    let values: Vec<f64> = buffer.iter().map(|state| state.x).collect();
    assert_eq!(values, [8.0, 9.0]);
    assert_eq!(buffer[0], point(8.0));
    assert_eq!(buffer[1], point(9.0));
}

#[test]
fn a_buffer_always_holds_at_least_one_point() {
    let buffer = TraceBuffer::new(10, point(7.0));
    assert_eq!(buffer.len(), 1);
    assert!(!buffer.is_empty());
    assert_eq!(buffer.first(), Some(point(7.0)));
    assert_eq!(buffer.last(), Some(point(7.0)));
}

#[test]
fn matches_a_reference_deque_for_random_operations() {
    let mut rng = SplitMix64::new(2026);
    for capacity in 1..=8 {
        let mut buffer = TraceBuffer::new(capacity, point(0.0));
        let mut reference = VecDeque::from([point(0.0)]);

        for _ in 0..200 {
            let value = rng.random_f64_inclusive(-1000.0..=1000.0);
            buffer.append(point(value));
            reference.push_back(point(value));
            if reference.len() > capacity {
                reference.pop_front();
            }

            assert_eq!(buffer.len(), reference.len());
            let actual: Vec<State> = buffer.iter().collect();
            let expected: Vec<State> = reference.iter().copied().collect();
            assert_eq!(actual, expected, "capacity {capacity} diverged");
        }
    }
}

#[test]
#[should_panic(expected = "index out of range")]
fn indexing_past_the_oldest_point_panics() {
    let mut buffer = TraceBuffer::new(2, point(0.0));
    buffer.append(point(1.0));
    let _ = buffer[2];
}

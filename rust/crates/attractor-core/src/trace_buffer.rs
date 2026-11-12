//! A fixed-capacity ring buffer of trace points, oldest first.

use std::ops::Index;

use crate::State;

/// A fixed-capacity ring buffer of trace points, oldest first.
///
/// The engine appends one state per integration step. Once `capacity` is
/// reached the oldest point is overwritten in place, so appending costs O(1)
/// instead of the O(n) shift that removing the first element of a growing
/// vector would cost (the catalog allows trails of tens of thousands of
/// points).
#[derive(Debug, Clone, PartialEq)]
pub struct TraceBuffer {
    capacity: usize,
    count: usize,
    storage: Vec<State>,
    /// Index in `storage` of the oldest point.
    head: usize,
}

impl TraceBuffer {
    /// Create a buffer holding a single initial point.
    ///
    /// `capacity` is clamped to at least one point.
    pub fn new(capacity: usize, initial: State) -> Self {
        let capacity = capacity.max(1);
        Self {
            capacity,
            count: 1,
            storage: vec![initial; capacity],
            head: 0,
        }
    }

    /// Maximum number of points retained.
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Number of points currently stored.
    pub const fn len(&self) -> usize {
        self.count
    }

    /// Whether the buffer holds no points.
    ///
    /// Always `false`: the buffer starts with one point and [`Self::append`]
    /// never removes the last one.
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// The oldest retained point.
    pub fn first(&self) -> Option<State> {
        (self.count > 0).then(|| self.storage[self.head])
    }

    /// The newest retained point.
    pub fn last(&self) -> Option<State> {
        (self.count > 0).then(|| self.storage[(self.head + self.count - 1) % self.capacity])
    }

    /// Append a point, dropping the oldest one when the buffer is full.
    pub fn append(&mut self, state: State) {
        if self.count < self.capacity {
            self.storage[(self.head + self.count) % self.capacity] = state;
            self.count += 1;
        } else {
            self.storage[self.head] = state;
            self.head = (self.head + 1) % self.capacity;
        }
    }

    /// Iterate over the points, oldest first.
    pub fn iter(&self) -> impl Iterator<Item = State> + '_ {
        (0..self.count).map(move |position| self.storage[(self.head + position) % self.capacity])
    }
}

impl Index<usize> for TraceBuffer {
    type Output = State;

    /// The point at `position`, counting from the oldest retained point.
    ///
    /// # Panics
    ///
    /// Panics when `position` is not less than [`Self::len`].
    fn index(&self, position: usize) -> &Self::Output {
        assert!(
            position < self.count,
            "TraceBuffer index out of range: {position} not in 0..{}",
            self.count
        );
        &self.storage[(self.head + position) % self.capacity]
    }
}

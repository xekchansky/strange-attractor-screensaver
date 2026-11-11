import Foundation

/// A fixed-capacity ring buffer of trace points, oldest first.
///
/// The engine appends one state per integration step. Once `capacity` is
/// reached the oldest point is overwritten in place, so appending costs O(1)
/// instead of the O(n) shift that `Array.removeFirst()` performs on a long
/// trail (the catalog allows trails of tens of thousands of points).
public struct TraceBuffer: RandomAccessCollection, Sendable {
    public typealias Element = State
    public typealias Index = Int

    /// Maximum number of points retained.
    public let capacity: Int
    /// Number of points currently stored.
    public private(set) var count: Int
    /// Backing storage, used as a ring: slot `head` holds the oldest point.
    private var storage: [State]
    /// Index in `storage` of the oldest point.
    private var head: Int

    /// Create a buffer holding a single initial point.
    ///
    /// - Parameters:
    ///   - capacity: Maximum number of points to retain; clamped to at least 1.
    ///   - initial: The first point, which is the only one in the buffer.
    public init(capacity: Int, initial: State) {
        let capacity = Swift.max(1, capacity)
        self.capacity = capacity
        self.storage = Array(repeating: initial, count: capacity)
        self.count = 1
        self.head = 0
    }

    public var startIndex: Int { 0 }

    public var endIndex: Int { count }

    /// The point at `position`, counting from the oldest retained point.
    public subscript(position: Int) -> State {
        precondition(position >= 0 && position < count, "TraceBuffer index out of range")
        return storage[(head + position) % capacity]
    }

    /// Append a point, dropping the oldest one when the buffer is full.
    public mutating func append(_ state: State) {
        if count < capacity {
            storage[(head + count) % capacity] = state
            count += 1
        } else {
            storage[head] = state
            head = (head + 1) % capacity
        }
    }
}

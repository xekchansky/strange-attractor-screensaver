import Testing
@testable import AttractorCore

@Suite("TraceBuffer")
struct TraceBufferTests {
    private func state(_ value: Double) -> State {
        State(x: value, y: 0, z: 0)
    }

    @Test("Holds points in insertion order until it is full")
    func insertionOrder() {
        var buffer = TraceBuffer(capacity: 3, initial: state(0))
        buffer.append(state(1))
        buffer.append(state(2))
        #expect(buffer.count == 3)
        #expect(buffer.map(\.x) == [0, 1, 2])
    }

    @Test("Drops the oldest point once full and stays in order")
    func wrapsAround() {
        var buffer = TraceBuffer(capacity: 3, initial: state(0))
        for value in 1...5 {
            buffer.append(state(Double(value)))
        }
        #expect(buffer.count == 3)
        #expect(buffer.map(\.x) == [3, 4, 5])
        #expect(buffer.first?.x == 3)
        #expect(buffer.last?.x == 5)
    }

    @Test("Capacity is clamped to at least one point")
    func capacityClamp() {
        var buffer = TraceBuffer(capacity: 0, initial: state(1))
        #expect(buffer.capacity == 1)
        buffer.append(state(2))
        #expect(buffer.count == 1)
        #expect(buffer.last?.x == 2)
    }

    @Test("Subscripts stay correct after several wraps")
    func repeatedWraps() {
        var buffer = TraceBuffer(capacity: 2, initial: state(0))
        for value in 1...9 {
            buffer.append(state(Double(value)))
        }
        #expect(Array(buffer).map(\.x) == [8, 9])
    }
}

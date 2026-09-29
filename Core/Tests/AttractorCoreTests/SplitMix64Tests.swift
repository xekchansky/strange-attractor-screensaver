import Foundation
import Testing
@testable import AttractorCore

@Suite("SplitMix64")
struct SplitMix64Tests {
    @Test("Same seed produces the same sequence")
    func sameSeedSameSequence() {
        var a = SplitMix64(seed: 42)
        var b = SplitMix64(seed: 42)
        for _ in 0..<100 {
            #expect(a.next() == b.next())
        }
    }

    @Test("Different seeds produce different sequences")
    func differentSeedsDiffer() {
        var a = SplitMix64(seed: 1)
        var b = SplitMix64(seed: 2)
        let seqA = (0..<8).map { _ in a.next() }
        let seqB = (0..<8).map { _ in b.next() }
        #expect(seqA != seqB)
    }

    @Test("Zero seed is allowed and deterministic")
    func zeroSeedWorks() {
        var a = SplitMix64(seed: 0)
        var b = SplitMix64(seed: 0)
        #expect(a.next() == b.next())
    }
}

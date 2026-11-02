import Testing
@testable import AttractorCore

@Suite("Restart limiter")
struct RestartLimiterTests {
    @Test("Allows a burst up to the configured limit")
    func allowsBurst() {
        var limiter = RestartLimiter(maximumRestarts: 3, window: 10)
        let first = limiter.allowRestart(at: 0)
        let second = limiter.allowRestart(at: 1)
        let third = limiter.allowRestart(at: 2)
        let fourth = limiter.allowRestart(at: 3)
        #expect(first)
        #expect(second)
        #expect(third)
        #expect(!fourth)
    }

    @Test("Suggests a retry time only while the limit blocks")
    func retryTime() {
        var limiter = RestartLimiter(maximumRestarts: 2, window: 10)
        #expect(limiter.nextRetryTime(after: 0) == nil)

        let first = limiter.allowRestart(at: 0)
        #expect(first)
        let retryAfterFirst = limiter.nextRetryTime(after: 1)
        #expect(retryAfterFirst == nil)

        let second = limiter.allowRestart(at: 1)
        #expect(second)
        #expect(limiter.nextRetryTime(after: 2) == 10)
    }

    @Test("Allows a new restart after the window elapses")
    func allowsAfterWindow() {
        var limiter = RestartLimiter(maximumRestarts: 1, window: 10)
        let first = limiter.allowRestart(at: 0)
        let blocked = limiter.allowRestart(at: 5)
        let afterWindow = limiter.allowRestart(at: 10)
        #expect(first)
        #expect(!blocked)
        #expect(afterWindow)
    }

    @Test("Only restarts inside the sliding window count")
    func slidingWindow() {
        var limiter = RestartLimiter(maximumRestarts: 2, window: 10)
        let first = limiter.allowRestart(at: 0)
        let second = limiter.allowRestart(at: 9)
        // The first restart expired at t=10, so one slot opened up again.
        let third = limiter.allowRestart(at: 10)
        // Both remaining restarts are still inside the window.
        let blocked = limiter.allowRestart(at: 11)
        let afterWindow = limiter.allowRestart(at: 19)
        #expect(first)
        #expect(second)
        #expect(third)
        #expect(!blocked)
        #expect(afterWindow)
    }

    @Test("Reset clears the recorded restarts")
    func reset() {
        var limiter = RestartLimiter(maximumRestarts: 1, window: 10)
        let first = limiter.allowRestart(at: 0)
        let blocked = limiter.allowRestart(at: 1)
        limiter.reset()
        let afterReset = limiter.allowRestart(at: 2)
        #expect(first)
        #expect(!blocked)
        #expect(afterReset)
    }

    @Test("Rejects a non-positive window and limit")
    func sanitizesConfiguration() {
        var limiter = RestartLimiter(maximumRestarts: 0, window: 0)
        #expect(limiter.maximumRestarts == 1)
        #expect(limiter.window > 0)
        let first = limiter.allowRestart(at: 0)
        let second = limiter.allowRestart(at: 1)
        #expect(first)
        #expect(!second)
    }
}

import Foundation

/// Limits how often a failed simulation may be replaced within a short window.
///
/// A run that fails immediately (for example, a formula that cannot be
/// evaluated) would otherwise be restarted on every frame, producing a flicker
/// and a CPU storm. The limiter allows a small burst of restarts, then asks the
/// caller to wait until the oldest one leaves the window.
public struct RestartLimiter: Equatable, Sendable {
    /// Maximum number of restarts allowed inside ``window``.
    public let maximumRestarts: Int
    /// Sliding window length, in seconds.
    public let window: TimeInterval

    private var timestamps: [TimeInterval] = []

    /// Create a limiter.
    ///
    /// - Parameters:
    ///   - maximumRestarts: Restart budget per window; values below 1 are
    ///     raised to 1.
    ///   - window: Window length in seconds; non-positive or non-finite values
    ///     fall back to 10 seconds.
    public init(maximumRestarts: Int = 3, window: TimeInterval = 10) {
        self.maximumRestarts = max(1, maximumRestarts)
        self.window = window.isFinite && window > 0 ? window : 10
    }

    /// Record a restart at `now` if the limit allows it.
    ///
    /// - Returns: `true` when the restart may proceed; `false` when the caller
    ///   should wait until ``nextRetryTime(after:)``.
    public mutating func allowRestart(at now: TimeInterval) -> Bool {
        discardExpired(at: now)
        guard timestamps.count < maximumRestarts else { return false }
        timestamps.append(now)
        return true
    }

    /// Earliest time a blocked restart may be retried.
    ///
    /// - Returns: `nil` when a restart is allowed right now, otherwise the
    ///   moment the oldest recorded restart leaves the window.
    public func nextRetryTime(after now: TimeInterval) -> TimeInterval? {
        var candidate = self
        candidate.discardExpired(at: now)
        guard candidate.timestamps.count >= maximumRestarts else { return nil }
        return candidate.timestamps.first.map { $0 + window }
    }

    /// Forget every recorded restart (e.g. when a new animation session starts).
    public mutating func reset() {
        timestamps.removeAll()
    }

    private mutating func discardExpired(at now: TimeInterval) {
        timestamps.removeAll { now - $0 >= window }
    }
}

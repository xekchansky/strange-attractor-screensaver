//! The restart storm limiter contract.

use attractor_core::RestartLimiter;

#[test]
fn allows_a_burst_up_to_the_configured_limit() {
    let mut limiter = RestartLimiter::new(3, 10.0);
    assert!(limiter.allow_restart(0.0));
    assert!(limiter.allow_restart(1.0));
    assert!(limiter.allow_restart(2.0));
    assert!(!limiter.allow_restart(3.0));
}

#[test]
fn suggests_a_retry_time_only_while_the_limit_blocks() {
    let mut limiter = RestartLimiter::new(2, 10.0);
    assert_eq!(limiter.next_retry_time(0.0), None);

    assert!(limiter.allow_restart(0.0));
    assert_eq!(limiter.next_retry_time(1.0), None);

    assert!(limiter.allow_restart(1.0));
    assert_eq!(limiter.next_retry_time(2.0), Some(10.0));
}

#[test]
fn allows_a_new_restart_after_the_window_elapses() {
    let mut limiter = RestartLimiter::new(1, 10.0);
    assert!(limiter.allow_restart(0.0));
    assert!(!limiter.allow_restart(5.0));
    assert!(limiter.allow_restart(10.0));
}

#[test]
fn only_restarts_inside_the_sliding_window_count() {
    let mut limiter = RestartLimiter::new(2, 10.0);
    assert!(limiter.allow_restart(0.0));
    assert!(limiter.allow_restart(9.0));
    // The first restart expired at t = 10, so one slot opened up again.
    assert!(limiter.allow_restart(10.0));
    // Both remaining restarts are still inside the window.
    assert!(!limiter.allow_restart(11.0));
    assert!(limiter.allow_restart(19.0));
}

#[test]
fn reset_clears_the_recorded_restarts() {
    let mut limiter = RestartLimiter::new(1, 10.0);
    assert!(limiter.allow_restart(0.0));
    assert!(!limiter.allow_restart(1.0));
    limiter.reset();
    assert!(limiter.allow_restart(2.0));
}

#[test]
fn rejects_a_non_positive_window_and_limit() {
    let mut limiter = RestartLimiter::new(0, 0.0);
    assert_eq!(limiter.maximum_restarts(), 1);
    assert_eq!(limiter.window(), 10.0);
    assert!(limiter.allow_restart(0.0));
    assert!(!limiter.allow_restart(1.0));
}

#[test]
fn a_non_finite_window_falls_back_to_ten_seconds() {
    for window in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(RestartLimiter::new(2, window).window(), 10.0);
    }
}

#[test]
fn defaults_match_the_swift_limiter() {
    let limiter = RestartLimiter::default();
    assert_eq!(limiter.maximum_restarts(), 3);
    assert_eq!(limiter.window(), 10.0);
}

//! The wall-clock-to-steps scheduler contract.

use attractor_core::SimulationStepScheduler;

const FRAME: f64 = 1.0 / 60.0;

#[test]
fn converts_a_normal_frame_interval_into_whole_steps() {
    let mut scheduler = SimulationStepScheduler::new(0.005, 2.0);
    // 1/60 s × 2 = 0.0333… simulated time → six whole 0.005 steps.
    assert_eq!(scheduler.steps_for_frame_interval(FRAME), 6);
}

#[test]
fn accumulates_fractional_steps_across_frames() {
    let mut scheduler = SimulationStepScheduler::new(0.01, 1.0);
    assert_eq!(scheduler.steps_for_frame_interval(0.004), 0);
    assert_eq!(scheduler.steps_for_frame_interval(0.004), 0);
    assert_eq!(scheduler.steps_for_frame_interval(0.004), 1);
}

#[test]
fn clamps_a_long_stall_to_the_maximum_frame_interval() {
    let mut scheduler = SimulationStepScheduler::with_limits(0.005, 2.0, 0.1, 4096);
    // A 60-second stall (sleep/wake) is treated as 0.1 s → 40 steps.
    assert_eq!(scheduler.steps_for_frame_interval(60.0), 40);
}

#[test]
fn never_exceeds_the_per_frame_step_cap() {
    let mut scheduler = SimulationStepScheduler::with_limits(0.001, 3.0, 0.1, 100);
    assert_eq!(scheduler.steps_for_frame_interval(0.1), 100);
}

#[test]
fn drops_the_backlog_after_a_capped_frame() {
    let mut scheduler = SimulationStepScheduler::with_limits(0.01, 1.0, 1.0, 10);
    assert_eq!(scheduler.steps_for_frame_interval(0.5), 10);
    assert_eq!(scheduler.accumulated_time(), 0.0);
    assert_eq!(scheduler.steps_for_frame_interval(0.05), 5);
}

#[test]
fn a_non_positive_step_size_produces_no_steps() {
    let mut zero = SimulationStepScheduler::new(0.0, 1.0);
    assert_eq!(zero.steps_for_frame_interval(FRAME), 0);

    let mut negative = SimulationStepScheduler::new(-0.01, 1.0);
    assert_eq!(negative.steps_for_frame_interval(FRAME), 0);
}

#[test]
fn non_finite_inputs_produce_no_steps() {
    let mut scheduler = SimulationStepScheduler::new(0.01, 1.0);
    assert_eq!(scheduler.steps_for_frame_interval(f64::NAN), 0);
    assert_eq!(scheduler.steps_for_frame_interval(f64::INFINITY), 0);

    let mut fast = SimulationStepScheduler::new(0.01, f64::NAN);
    assert_eq!(fast.steps_for_frame_interval(FRAME), 0);

    let mut broken = SimulationStepScheduler::new(f64::INFINITY, 1.0);
    assert_eq!(broken.steps_for_frame_interval(FRAME), 0);
}

#[test]
fn non_positive_frame_intervals_produce_no_steps() {
    let mut scheduler = SimulationStepScheduler::new(0.01, 1.0);
    assert_eq!(scheduler.steps_for_frame_interval(0.0), 0);
    assert_eq!(scheduler.steps_for_frame_interval(-1.0), 0);
}

#[test]
fn a_non_positive_simulation_speed_produces_no_steps() {
    let mut stopped = SimulationStepScheduler::new(0.01, 0.0);
    assert_eq!(stopped.steps_for_frame_interval(FRAME), 0);

    let mut reversed = SimulationStepScheduler::new(0.01, -2.0);
    assert_eq!(reversed.steps_for_frame_interval(FRAME), 0);
}

#[test]
fn sanitizes_the_limits() {
    let scheduler = SimulationStepScheduler::with_limits(0.01, 1.0, f64::NAN, 0);
    assert_eq!(scheduler.maximum_frame_interval(), 0.1);
    assert_eq!(scheduler.maximum_steps_per_frame(), 1);

    let scheduler = SimulationStepScheduler::with_limits(0.01, 1.0, -1.0, 5_000_000);
    assert_eq!(scheduler.maximum_frame_interval(), 0.1);
    assert_eq!(scheduler.maximum_steps_per_frame(), 1_000_000);
}

#[test]
fn exposes_the_configured_values() {
    let mut scheduler = SimulationStepScheduler::new(0.005, 2.0);
    assert_eq!(scheduler.step_size(), 0.005);
    assert_eq!(scheduler.simulation_speed(), 2.0);
    assert_eq!(scheduler.maximum_frame_interval(), 0.1);
    assert_eq!(scheduler.maximum_steps_per_frame(), 4096);

    scheduler.set_step_size(0.01);
    scheduler.set_simulation_speed(1.0);
    assert_eq!(scheduler.step_size(), 0.01);
    assert_eq!(scheduler.simulation_speed(), 1.0);
}

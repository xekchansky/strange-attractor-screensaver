//! Command-line parsing: flag parity with the Swift demo plus the headless
//! capture flags (`--seed`, `--frames`, `--width`, `--height`).

use std::collections::HashMap;
use std::path::PathBuf;

use attractor_cli::{CaptureRequest, CommandLineArguments, CommandLineArgumentsError};
use attractor_core::{State, TraceColorMode};

fn parse(args: &[&str]) -> CommandLineArguments {
    CommandLineArguments::parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
        .expect("arguments parse")
}

fn parse_error(args: &[&str]) -> CommandLineArgumentsError {
    CommandLineArguments::parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
        .expect_err("arguments should be rejected")
}

#[test]
fn an_empty_command_line_keeps_every_default() {
    let arguments = parse(&[]);
    assert_eq!(arguments.attractor_id, None);
    assert!(arguments.parameters.is_empty());
    assert_eq!(arguments.capture, None);
    assert_eq!(arguments.warmup_steps, None);
    assert_eq!(arguments.capture_scale, 2);
    assert_eq!(arguments.camera_azimuth_degrees, None);
    assert_eq!(arguments.camera_elevation_degrees, None);
    assert_eq!(arguments.initial_state, None);
    assert_eq!(arguments.seed, None);
    assert_eq!(arguments.frames, None);
    assert_eq!(arguments.width, None);
    assert_eq!(arguments.height, None);
    assert!(!arguments.show_help);
    assert_eq!(arguments.overrides, Default::default());
}

#[test]
fn flags_are_order_independent() {
    let arguments = parse(&[
        "--screenshot",
        "out.png",
        "--trail-length",
        "20000",
        "--attractor",
        "lorenz",
        "--seed",
        "42",
        "--frames",
        "120",
        "-p",
        "sigma=10",
        "--speed",
        "0.5",
        "--step",
        "0.004",
        "--cam-speed",
        "0.1",
        "--trail-color-speed",
        "0.05",
        "--color-mode",
        "depth",
        "--warmup-steps",
        "1000",
        "--capture-scale",
        "1",
        "--cam-azimuth",
        "270",
        "--cam-elevation",
        "85",
        "--initial-state",
        "0.6, 0.6, -0.86",
        "--width",
        "640",
        "--height",
        "480",
    ]);
    assert_eq!(arguments.attractor_id.as_deref(), Some("lorenz"));
    assert_eq!(
        arguments.parameters,
        HashMap::from([("sigma".to_string(), "10".to_string())])
    );
    assert_eq!(
        arguments.capture,
        Some(CaptureRequest::Screenshot(PathBuf::from("out.png")))
    );
    assert_eq!(arguments.seed, Some(42));
    assert_eq!(arguments.frames, Some(120));
    assert_eq!(arguments.warmup_steps, Some(1000));
    assert_eq!(arguments.capture_scale, 1);
    assert_eq!(arguments.width, Some(640));
    assert_eq!(arguments.height, Some(480));
    assert_eq!(arguments.camera_azimuth_degrees, Some(270.0));
    assert_eq!(arguments.camera_elevation_degrees, Some(85.0));
    assert_eq!(arguments.initial_state, Some(State::new(0.6, 0.6, -0.86)));
    let overrides = arguments.overrides;
    assert_eq!(overrides.trail_length, Some(20000));
    assert_eq!(overrides.simulation_speed, Some(0.5));
    assert_eq!(overrides.step_size, Some(0.004));
    assert_eq!(overrides.camera_rotation_speed, Some(0.1));
    assert_eq!(overrides.trail_color_speed, Some(0.05));
    assert_eq!(overrides.color_mode, Some(TraceColorMode::Depth));
}

#[test]
fn bare_name_value_tokens_are_parameters() {
    let arguments = parse(&["--attractor", "rossler", "a=0.2", "b=0.2", "c=5.7"]);
    assert_eq!(
        arguments.parameters,
        HashMap::from([
            ("a".to_string(), "0.2".to_string()),
            ("b".to_string(), "0.2".to_string()),
            ("c".to_string(), "5.7".to_string()),
        ])
    );
}

#[test]
fn a_broken_parameter_pair_is_rejected() {
    assert_eq!(
        parse_error(&["=value"]),
        CommandLineArgumentsError::InvalidParameter("=value".to_string())
    );
    assert_eq!(
        parse_error(&["--param", "name"]),
        CommandLineArgumentsError::InvalidParameter("name".to_string())
    );
}

#[test]
fn an_unknown_flag_is_rejected() {
    assert_eq!(
        parse_error(&["--turbo"]),
        CommandLineArgumentsError::UnknownArgument("--turbo".to_string())
    );
}

#[test]
fn a_flag_without_its_value_is_rejected() {
    assert_eq!(
        parse_error(&["--attractor"]),
        CommandLineArgumentsError::MissingValue("--attractor".to_string())
    );
}

#[test]
fn numeric_and_state_values_are_validated() {
    assert_eq!(
        parse_error(&["--speed", "fast"]),
        CommandLineArgumentsError::InvalidParameter("--speed fast".to_string())
    );
    assert_eq!(
        parse_error(&["--initial-state", "1,2"]),
        CommandLineArgumentsError::InvalidParameter("--initial-state 1,2".to_string())
    );
    assert_eq!(
        parse_error(&["--trail-length", "0"]),
        CommandLineArgumentsError::InvalidParameter("--trail-length 0".to_string())
    );
    assert_eq!(
        parse_error(&["--capture-scale", "5"]),
        CommandLineArgumentsError::InvalidParameter("--capture-scale 5".to_string())
    );
    assert_eq!(
        parse_error(&["--cam-elevation", "90"]),
        CommandLineArgumentsError::InvalidParameter("--cam-elevation 90".to_string())
    );
    assert_eq!(
        parse_error(&["--color-mode", "rainbow"]),
        CommandLineArgumentsError::InvalidParameter("\"--color-mode rainbow\"".to_string())
    );
}

#[test]
fn every_color_mode_name_is_accepted() {
    for (name, mode) in [
        ("trail", TraceColorMode::Trail),
        ("age", TraceColorMode::Age),
        ("depth", TraceColorMode::Depth),
        ("spatial", TraceColorMode::Spatial),
    ] {
        assert_eq!(
            parse(&["--color-mode", name]).overrides.color_mode,
            Some(mode)
        );
    }
}

#[test]
fn screenshot_and_record_are_mutually_exclusive() {
    assert_eq!(
        parse_error(&["--screenshot", "a.png", "--record", "dir"]),
        CommandLineArgumentsError::ConflictingOptions(
            "--screenshot and --record are mutually exclusive".to_string()
        )
    );
}

#[test]
fn record_modifiers_require_record() {
    assert_eq!(
        parse_error(&["--record-seconds", "2"]),
        CommandLineArgumentsError::OptionRequiresRecord("--record-seconds".to_string())
    );
    assert_eq!(
        parse_error(&["--frame-stride", "2"]),
        CommandLineArgumentsError::OptionRequiresRecord("--frame-stride".to_string())
    );
}

#[test]
fn a_record_request_uses_the_swift_defaults() {
    let arguments = parse(&["--record", "frames"]);
    assert_eq!(
        arguments.capture,
        Some(CaptureRequest::Record {
            output_directory: PathBuf::from("frames"),
            seconds: 4.0,
            frame_stride: 2,
        })
    );
}

#[test]
fn record_seconds_and_stride_are_kept() {
    let arguments = parse(&[
        "--record",
        "frames",
        "--record-seconds",
        "40",
        "--frame-stride",
        "1",
    ]);
    assert_eq!(
        arguments.capture,
        Some(CaptureRequest::Record {
            output_directory: PathBuf::from("frames"),
            seconds: 40.0,
            frame_stride: 1,
        })
    );
}

#[test]
fn frames_and_record_seconds_cannot_both_set_the_duration() {
    assert_eq!(
        parse_error(&[
            "--record",
            "frames",
            "--frames",
            "10",
            "--record-seconds",
            "2"
        ]),
        CommandLineArgumentsError::ConflictingOptions(
            "--frames and --record-seconds are mutually exclusive".to_string()
        )
    );
}

#[test]
fn the_seed_accepts_the_full_u64_range() {
    assert_eq!(parse(&["--seed", "0"]).seed, Some(0));
    assert_eq!(
        parse(&["--seed", "18446744073709551615"]).seed,
        Some(u64::MAX)
    );
    assert_eq!(
        parse_error(&["--seed", "-1"]),
        CommandLineArgumentsError::InvalidParameter("--seed -1".to_string())
    );
}

#[test]
fn width_and_height_must_be_usable() {
    assert_eq!(
        parse_error(&["--width", "0"]),
        CommandLineArgumentsError::InvalidParameter("--width 0".to_string())
    );
    assert_eq!(
        parse_error(&["--height", "abc"]),
        CommandLineArgumentsError::InvalidParameter("--height abc".to_string())
    );
    assert_eq!(
        parse_error(&["--width", "20000"]),
        CommandLineArgumentsError::InvalidParameter("--width 20000".to_string())
    );
}

#[test]
fn help_is_recognised() {
    assert!(parse(&["-h"]).show_help);
    assert!(parse(&["--help"]).show_help);
}

#[test]
fn usage_lists_the_headless_capture_flags() {
    for flag in [
        "--attractor",
        "--seed",
        "--screenshot",
        "--record",
        "--record-seconds",
        "--frames",
        "--frame-stride",
        "--width",
        "--height",
        "--capture-scale",
        "--param",
        "--initial-state",
        "--cam-azimuth",
        "--cam-elevation",
        "--cam-speed",
        "--trail-color-speed",
        "--trail-length",
        "--warmup-steps",
        "--step",
        "--speed",
        "--color-mode",
    ] {
        assert!(
            CommandLineArguments::USAGE.contains(flag),
            "usage text misses {flag}"
        );
    }
}

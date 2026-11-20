//! The command-line colour speed must reach the rendered gradient.

use attractor_cli::{CommandLineArguments, prepare};

const FRAME_INTERVAL: f64 = 1.0 / 60.0;
const WARMUP_STEPS: usize = 1_000;
const FRAMES: usize = 90;

fn gradient(speed: &str) -> Vec<attractor_view::Rgb> {
    let arguments: Vec<String> = [
        "--attractor",
        "moore_spiegel",
        "--seed",
        "17528556314502435919",
        "--cam-azimuth",
        "346.8283",
        "--cam-elevation",
        "36",
        "--cam-speed",
        "0",
        "--trail-length",
        "12000",
        // The seed's random selection draws Spatial, whose stable axis colors
        // deliberately ignore the offset; the flag only moves the fading ramp.
        "--color-mode",
        "trail",
        "--trail-color-speed",
        speed,
        "--warmup-steps",
        "1000",
        "--frames",
        "90",
        "--screenshot",
        "unused.png",
    ]
    .iter()
    .map(|argument| argument.to_string())
    .collect();
    let parsed = CommandLineArguments::parse(&arguments).expect("the flags parse");
    let mut run = prepare(&parsed).expect("the run prepares");
    run.player
        .warm_up(WARMUP_STEPS)
        .expect("warm-up stays healthy");
    for _ in 0..FRAMES {
        run.player.tick(FRAME_INTERVAL).expect("frame advances");
    }
    run.player.geometry().colors().to_vec()
}

#[test]
fn the_command_line_colour_speed_reaches_the_gradient() {
    assert_ne!(
        gradient("0"),
        gradient("0.2"),
        "a non-zero --trail-color-speed must shift the gradient"
    );
}

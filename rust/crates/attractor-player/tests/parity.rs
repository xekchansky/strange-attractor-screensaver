//! The player and the CLI capture must produce the same frame for the same
//! flags: the CLI is one more shell over the same runtime.

use std::path::Path;

use attractor_cli::{CommandLineArguments, run};
use attractor_core::{
    AttractorSelection, SimulationSettings, SplitMix64, TraceColorMode, catalog, randomizer,
};
use attractor_gpu::{GpuContext, lit_pixels};
use attractor_player::{Player, PlayerOptions};

const WARMUP_STEPS: usize = 5_000;
const FRAMES: usize = 60;
const FRAME_INTERVAL: f64 = 1.0 / 60.0;
/// Logical 320x240 captured at the CLI's default scale of two.
const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const OVERLAY_SCALE: f32 = 2.0;

fn arguments(output: &Path) -> CommandLineArguments {
    let values = [
        "--attractor",
        "lorenz",
        "--seed",
        "42",
        "--cam-azimuth",
        "0",
        "--cam-elevation",
        "0",
        "--cam-speed",
        "0",
        "--trail-color-speed",
        "0",
        "--color-mode",
        "trail",
        "--warmup-steps",
        "5000",
        "--frames",
        "60",
        "--width",
        "320",
        "--height",
        "240",
        "--screenshot",
    ];
    let mut arguments: Vec<String> = values.iter().map(|value| value.to_string()).collect();
    arguments.push(output.to_string_lossy().into_owned());
    CommandLineArguments::parse(&arguments).expect("the parity arguments parse")
}

/// Build the same run the CLI's `prepare` builds, without going through it.
fn manual_player() -> Player {
    let validation = catalog::bundled_validation().expect("the catalog validates");
    let descriptor = validation
        .valid
        .iter()
        .find(|descriptor| descriptor.id == "lorenz")
        .expect("lorenz is in the catalog")
        .clone();
    let mut rng = SplitMix64::new(42);
    let selection =
        randomizer::select_attractor(&descriptor, &mut rng).expect("the seeded selection builds");

    let base = selection.settings;
    let settings = SimulationSettings::new(
        base.step_size,
        base.simulation_speed,
        0.0,
        0.0,
        base.trail_length,
        TraceColorMode::Trail,
    );
    let selection = AttractorSelection::new(
        selection.descriptor.clone(),
        selection.parameters.clone(),
        settings,
        selection.initial_state,
    );

    Player::new(
        selection,
        validation.valid,
        PlayerOptions {
            camera_azimuth: Some(0.0),
            camera_elevation: Some(0.0),
            heal_automatically: false,
            width: WIDTH,
            height: HEIGHT,
            overlay_scale: OVERLAY_SCALE,
            ..PlayerOptions::default()
        },
        rng,
    )
    .expect("the manual player builds")
}

fn read_png(path: &Path) -> (u32, u32, Vec<u8>) {
    let file = std::fs::File::open(path).expect("the capture file exists");
    let decoder = png::Decoder::new(std::io::BufReader::new(file));
    let mut reader = decoder.read_info().expect("the PNG header parses");
    let mut buffer = vec![
        0;
        reader
            .output_buffer_size()
            .expect("the PNG size is representable")
    ];
    let info = reader.next_frame(&mut buffer).expect("the PNG decodes");
    buffer.truncate(info.buffer_size());
    (info.width, info.height, buffer)
}

#[test]
fn a_player_frame_matches_the_cli_capture() {
    let context = match GpuContext::new() {
        Ok(context) => context,
        Err(error) => {
            eprintln!("skipping the parity test: {error}");
            return;
        }
    };

    let output = std::env::temp_dir().join(format!(
        "attractor-player-parity-{}.png",
        std::process::id()
    ));
    let report = run(&arguments(&output)).expect("the CLI capture succeeds");
    let (cli_width, cli_height, cli_pixels) = read_png(&output);
    let _ = std::fs::remove_file(&output);

    let mut player = manual_player();
    player
        .warm_up(WARMUP_STEPS)
        .expect("the warm-up stays healthy");
    for _ in 0..FRAMES {
        player.tick(FRAME_INTERVAL).expect("the frame advances");
    }
    let pixels = player.render(&context).expect("the player frame renders");

    assert_eq!(report.frames_written, 1);
    assert_eq!((cli_width, cli_height), (WIDTH, HEIGHT));
    assert_eq!(player.size(), (WIDTH, HEIGHT));
    assert_eq!(
        player.frames_simulated(),
        FRAMES,
        "both paths must simulate the same frames"
    );
    assert_eq!(pixels.len(), cli_pixels.len());
    assert_eq!(pixels, cli_pixels, "the player and CLI frames drifted");
    assert!(lit_pixels(&pixels) > 0, "the frame must not be blank");
}

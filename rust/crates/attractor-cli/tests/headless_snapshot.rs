//! Deterministic geometry snapshot of a seeded headless run.
//!
//! The primary renderer test (design I6): the same flags, seed and frame count
//! must produce the same ribbon buffers bit for bit. The fixture is generated
//! by this test with `ATTRACTOR_DUMP_FIXTURES=1` and covers the whole chain —
//! seeded selection, RK4 warm-up, frame stepping, camera framing and ribbon
//! building — without touching the GPU.

use std::path::{Path, PathBuf};

use attractor_cli::{CaptureRequest, CommandLineArguments, prepare};
use attractor_view::{TrailGeometry, Vector3};

const WARMUP_STEPS: usize = 5_000;
const FRAMES: usize = 60;
const FRAME_INTERVAL: f64 = 1.0 / 60.0;

fn arguments() -> CommandLineArguments {
    // Azimuth and elevation zero pin the camera on the +x axis with exact
    // trigonometric values, so the buffers stay bit-identical across libm
    // implementations and the snapshot can compare exact f32 bits.
    let arguments = [
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
        "out.png",
    ];
    CommandLineArguments::parse(&arguments.iter().map(|s| s.to_string()).collect::<Vec<_>>())
        .expect("snapshot arguments parse")
}

fn fnv_bytes(hash: &mut u64, bytes: &[u8]) {
    for &byte in bytes {
        *hash ^= u64::from(byte);
        *hash = hash.wrapping_mul(0x100_0000_01b3);
    }
}

fn geometry_digest(geometry: &TrailGeometry) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for position in geometry.positions() {
        for component in [position.x, position.y, position.z] {
            fnv_bytes(&mut hash, &(component as f32).to_bits().to_le_bytes());
        }
    }
    for color in geometry.colors() {
        for component in [color.r, color.g, color.b] {
            fnv_bytes(&mut hash, &component.to_bits().to_le_bytes());
        }
    }
    hash
}

fn bits(vector: Vector3) -> Vec<u32> {
    [vector.x, vector.y, vector.z]
        .into_iter()
        .map(|component| (component as f32).to_bits())
        .collect()
}

fn fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cli_lorenz_snapshot.json")
}

#[test]
fn the_seeded_lorenz_geometry_matches_the_snapshot() {
    let mut prepared = prepare(&arguments()).expect("the run prepares");
    prepared
        .session
        .warm_up(prepared.warmup_steps)
        .expect("the warm-up stays healthy");
    assert_eq!(prepared.warmup_steps, WARMUP_STEPS);
    assert_eq!(prepared.frames, Some(FRAMES));
    assert_eq!(prepared.pixel_size(), (640, 480));
    assert!(matches!(prepared.capture, CaptureRequest::Screenshot(_)));
    for _ in 0..FRAMES {
        prepared
            .session
            .advance_frame(FRAME_INTERVAL)
            .expect("the frame advances");
    }

    let geometry = prepared.session.geometry();
    let first = geometry.positions().first().copied().expect("non-empty");
    let last = geometry.positions().last().copied().expect("non-empty");
    let first_color = geometry.colors().first().copied().expect("non-empty");
    let last_color = geometry.colors().last().copied().expect("non-empty");
    let actual = serde_json::json!({
        "attractor": prepared.session.selection().descriptor.id,
        "seed": 42,
        "warmup_steps": WARMUP_STEPS,
        "frames": FRAMES,
        // Bits, not decimals: serde_json's default float parser is fast but
        // not always correctly rounded, and this fixture is meant to be exact.
        "parameters_bits": prepared
            .session
            .selection()
            .parameters
            .iter()
            .map(|(name, value)| (name.clone(), serde_json::json!(value.to_bits())))
            .collect::<serde_json::Map<String, serde_json::Value>>(),
        "initial_state_bits": [
            prepared.session.selection().initial_state.x.to_bits(),
            prepared.session.selection().initial_state.y.to_bits(),
            prepared.session.selection().initial_state.z.to_bits(),
        ],
        "point_count": geometry.point_count(),
        "vertex_count": geometry.vertex_count(),
        "triangle_count": geometry.triangle_count(),
        "ribbon_radius_bits": geometry.ribbon_radius().to_bits(),
        "digest": geometry_digest(geometry),
        "first_vertex": {
            "position_bits": bits(first),
            "color_bits": [
                first_color.r.to_bits(),
                first_color.g.to_bits(),
                first_color.b.to_bits(),
            ],
        },
        "last_vertex": {
            "position_bits": bits(last),
            "color_bits": [
                last_color.r.to_bits(),
                last_color.g.to_bits(),
                last_color.b.to_bits(),
            ],
        },
    });

    if std::env::var("ATTRACTOR_DUMP_FIXTURES").as_deref() == Ok("1") {
        let text = serde_json::to_string_pretty(&actual).expect("snapshot serialises");
        if let Some(parent) = fixture_path().parent() {
            std::fs::create_dir_all(parent).expect("fixture directory");
        }
        std::fs::write(fixture_path(), format!("{text}\n")).expect("snapshot written");
        eprintln!("wrote {}", fixture_path().display());
        return;
    }

    let text = std::fs::read_to_string(fixture_path()).expect("snapshot fixture exists");
    let expected: serde_json::Value = serde_json::from_str(&text).expect("snapshot parses");
    assert_eq!(actual, expected, "the headless geometry drifted");
}

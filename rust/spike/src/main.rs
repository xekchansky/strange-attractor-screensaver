#[cfg(not(target_arch = "wasm32"))]
fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let output = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "spike.png".to_string());
    let width = 1280;
    let height = 720;

    let offscreen = wgpu_spike::native::init(width, height);
    let pixels = offscreen.render(0.0);
    let lit = wgpu_spike::native::lit_pixels(&pixels);
    let path = std::path::Path::new(&output);
    wgpu_spike::native::write_png(path, width, height, &pixels).expect("write png");

    let total = (width * height) as usize;
    println!(
        "wrote {} ({}x{}, {} of {} pixels lit, {:.2}%)",
        path.display(),
        width,
        height,
        lit,
        total,
        100.0 * lit as f64 / total as f64,
    );
    if lit * 100 < total {
        eprintln!("frame looks blank");
        std::process::exit(1);
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {}

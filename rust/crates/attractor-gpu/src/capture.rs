//! PNG encoding and the cheap "not blank" frame metric.

use std::io;
use std::path::Path;

/// Brightness difference from the black background above which a channel
/// counts as drawn ink.
const LIT_THRESHOLD: u8 = 4;

/// Write tightly packed RGBA8 pixels as a PNG file.
pub fn write_png(path: &Path, width: u32, height: u32, pixels: &[u8]) -> io::Result<()> {
    let file = std::fs::File::create(path)?;
    let writer = io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(io::Error::other)?;
    writer.write_image_data(pixels).map_err(io::Error::other)?;
    writer.finish().map_err(io::Error::other)?;
    Ok(())
}

/// Count pixels that differ from the black background.
///
/// The sRGB target stores the clear color as `(0, 0, 0)`, so a pixel with any
/// channel above [`LIT_THRESHOLD`] is ribbon or text ink. This is the cheap
/// regression signal for a headless capture; it is deliberately not a
/// perceptual comparison.
pub fn lit_pixels(pixels: &[u8]) -> usize {
    pixels
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|pixel| {
            pixel[0] > LIT_THRESHOLD || pixel[1] > LIT_THRESHOLD || pixel[2] > LIT_THRESHOLD
        })
        .count()
}

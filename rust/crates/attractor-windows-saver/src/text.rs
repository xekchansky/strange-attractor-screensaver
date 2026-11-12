//! UTF-16 conversion for the Win32 entry points.
//!
//! Every `windows-sys` string parameter is a NUL-terminated UTF-16 pointer, so
//! the shell builds one `Vec<u16>` per constant at startup and keeps it alive
//! for the pointer's lifetime.

/// Encode `text` as a NUL-terminated UTF-16 buffer.
///
/// The buffer owns its memory; the caller keeps it alive while Win32 holds the
/// pointer. Characters outside the basic plane become surrogate pairs, which is
/// what the `W` APIs expect.
pub fn wide(text: &str) -> Vec<u16> {
    let mut buffer: Vec<u16> = text.encode_utf16().collect();
    buffer.push(0);
    buffer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_text_is_terminated() {
        assert_eq!(wide("abc"), vec![0x61, 0x62, 0x63, 0x00]);
    }

    #[test]
    fn the_empty_string_is_a_single_terminator() {
        assert_eq!(wide(""), vec![0x00]);
    }

    #[test]
    fn characters_outside_the_basic_plane_become_surrogate_pairs() {
        // '🎇' U+1F387 is a firework, a natural fit for an attractor.
        let buffer = wide("🎇");
        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer[0], 0xD83C);
        assert_eq!(buffer[1], 0xDF87);
        assert_eq!(buffer[2], 0x00);
    }
}

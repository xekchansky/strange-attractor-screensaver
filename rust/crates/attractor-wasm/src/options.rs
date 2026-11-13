//! Query-string options of the web demo.
//!
//! The page keeps its whole state in the URL (`?attractor=lorenz&seed=42`),
//! so a captured frame is reproducible by opening the same link. Parsing is
//! tolerant: an unknown key, an unparsable seed or an unknown backend is
//! ignored, because a malformed link must never stop the demo from starting.

/// Which GPU backend the page asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Backend {
    /// Let wgpu detect WebGPU and fall back to WebGL2 when it is unavailable.
    #[default]
    Auto,
    /// Force the WebGL2 fallback, which is what the browser test covers.
    WebGl2,
}

/// Options parsed from the page URL.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DemoOptions {
    /// Attractor id to run; `None` draws a random one from the catalog.
    pub attractor: Option<String>,
    /// Seed for the selection and the camera; `None` uses a fresh seed.
    pub seed: Option<u64>,
    /// Requested rendering backend.
    pub backend: Backend,
}

impl DemoOptions {
    /// Parse a query string, with or without its leading `?`.
    pub fn parse(query: &str) -> Self {
        let mut options = Self::default();
        for pair in query.trim_start_matches('?').split('&') {
            if pair.is_empty() {
                continue;
            }
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            let key = decode(key);
            let value = decode(value);
            match key.as_str() {
                // An empty `attractor` means "random", like a missing key.
                "attractor" if !value.is_empty() => options.attractor = Some(value),
                "seed" => options.seed = value.parse().ok(),
                "backend" if value.eq_ignore_ascii_case("webgl2") => {
                    options.backend = Backend::WebGl2;
                }
                _ => {}
            }
        }
        options
    }
}

/// Decode `%XX` escapes and `+` the way `URLSearchParams` writes them.
fn decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                decoded.push(b' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                match (hex_value(bytes[index + 1]), hex_value(bytes[index + 2])) {
                    (Some(high), Some(low)) => {
                        decoded.push((high << 4) | low);
                        index += 3;
                    }
                    _ => {
                        decoded.push(b'%');
                        index += 1;
                    }
                }
            }
            byte => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_query_keeps_the_defaults() {
        assert_eq!(DemoOptions::parse(""), DemoOptions::default());
        assert_eq!(DemoOptions::parse("?"), DemoOptions::default());
        assert_eq!(DemoOptions::parse("&&"), DemoOptions::default());
    }

    #[test]
    fn the_leading_question_mark_is_optional() {
        let with = DemoOptions::parse("?seed=42");
        let without = DemoOptions::parse("seed=42");
        assert_eq!(with, without);
        assert_eq!(with.seed, Some(42));
    }

    #[test]
    fn attractor_and_seed_are_read_together() {
        let options = DemoOptions::parse("?attractor=lorenz&seed=7");
        assert_eq!(options.attractor.as_deref(), Some("lorenz"));
        assert_eq!(options.seed, Some(7));
        assert_eq!(options.backend, Backend::Auto);
    }

    #[test]
    fn a_malformed_seed_is_ignored_instead_of_failing() {
        assert_eq!(DemoOptions::parse("?seed=abc").seed, None);
        assert_eq!(DemoOptions::parse("?seed=-1").seed, None);
        assert_eq!(DemoOptions::parse("?seed=").seed, None);
        assert_eq!(
            DemoOptions::parse("?seed=18446744073709551616").seed,
            None,
            "a value above u64::MAX does not fit"
        );
        assert_eq!(
            DemoOptions::parse("?seed=18446744073709551615").seed,
            Some(u64::MAX)
        );
    }

    #[test]
    fn an_empty_attractor_means_random() {
        assert_eq!(DemoOptions::parse("?attractor=").attractor, None);
        assert_eq!(DemoOptions::parse("?attractor").attractor, None);
    }

    #[test]
    fn the_webgl2_backend_is_requested_explicitly() {
        assert_eq!(
            DemoOptions::parse("?backend=webgl2").backend,
            Backend::WebGl2
        );
        assert_eq!(
            DemoOptions::parse("?backend=WEBGL2").backend,
            Backend::WebGl2
        );
        assert_eq!(DemoOptions::parse("?backend=webgpu").backend, Backend::Auto);
        assert_eq!(DemoOptions::parse("?backend=").backend, Backend::Auto);
    }

    #[test]
    fn percent_escapes_are_decoded() {
        let options = DemoOptions::parse("?attractor=moore%5Fspiegel&seed=1");
        assert_eq!(options.attractor.as_deref(), Some("moore_spiegel"));
        assert_eq!(
            DemoOptions::parse("?attractor=Sprott%20A")
                .attractor
                .as_deref(),
            Some("Sprott A")
        );
        assert_eq!(
            DemoOptions::parse("?attractor=a%2Bb").attractor.as_deref(),
            Some("a+b"),
            "a plus sign written as an escape stays a plus"
        );
    }

    #[test]
    fn a_plus_sign_is_a_space() {
        assert_eq!(
            DemoOptions::parse("?attractor=Sprott+A")
                .attractor
                .as_deref(),
            Some("Sprott A")
        );
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let options = DemoOptions::parse("?utm_source=readme&seed=3&quality=high");
        assert_eq!(options.seed, Some(3));
        assert_eq!(options.attractor, None);
    }

    #[test]
    fn a_truncated_escape_is_kept_verbatim() {
        assert_eq!(
            DemoOptions::parse("?attractor=sprott%2")
                .attractor
                .as_deref(),
            Some("sprott%2")
        );
        assert_eq!(
            DemoOptions::parse("?attractor=%zz").attractor.as_deref(),
            Some("%zz")
        );
    }
}

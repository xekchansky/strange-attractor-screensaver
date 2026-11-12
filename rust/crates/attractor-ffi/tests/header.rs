//! The hand-maintained header must declare exactly the exported symbols.

use std::collections::BTreeSet;

const HEADER: &str = include_str!("../include/attractor.h");
const SOURCE: &str = include_str!("../src/lib.rs");

fn header_functions() -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for line in HEADER.lines() {
        let line = line.trim();
        if line.is_empty()
            || line.starts_with("/*")
            || line.starts_with('*')
            || line.starts_with('#')
            || line.starts_with("typedef")
            || line.starts_with('}')
        {
            continue;
        }
        let Some(start) = line.find("attractor_") else {
            continue;
        };
        let tail = &line[start..];
        let Some(end) = tail.find('(') else {
            continue;
        };
        let name: String = tail[..end]
            .chars()
            .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
            .collect();
        if name.starts_with("attractor_") {
            names.insert(name);
        }
    }
    names
}

fn exported_functions() -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut lines = SOURCE.lines();
    while let Some(line) = lines.next() {
        if line.trim() != "#[unsafe(no_mangle)]" {
            continue;
        }
        for line in lines.by_ref() {
            let Some(start) = line.find("fn ") else {
                continue;
            };
            let tail = &line[start + 3..];
            let name: String = tail
                .chars()
                .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
                .collect();
            if name.starts_with("attractor_") {
                names.insert(name);
            }
            break;
        }
    }
    names
}

#[test]
fn the_header_declares_every_exported_symbol() {
    let exported = exported_functions();
    assert!(
        exported.contains("attractor_player_create"),
        "the export scan found no entry points: {exported:?}"
    );
    assert_eq!(
        header_functions(),
        exported,
        "include/attractor.h and the exported symbols drifted apart"
    );
}

#[test]
fn the_header_documents_the_options_layout() {
    for field in [
        "seed",
        "width",
        "height",
        "overlay_scale",
        "use_noop_backend",
    ] {
        assert!(
            HEADER.contains(field),
            "the header must document the '{field}' option"
        );
    }
    assert!(HEADER.contains("AttractorPlayerOptions"));
    assert!(HEADER.contains("ATTRACTOR_PLAYER_INVALID_ARGUMENT"));
}

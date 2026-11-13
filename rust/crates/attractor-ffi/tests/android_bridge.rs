//! Keeps the JNI symbols and the Kotlin declarations in step.
//!
//! A JNI name is spelled by hand on both sides, and a mismatch only shows up
//! as an `UnsatisfiedLinkError` on a device. These tests compare the two lists
//! on the host, where the normal workspace run covers them.

/// The Kotlin half of the bridge.
const NATIVE_BRIDGE_KT: &str = include_str!(
    "../../../shells/android/app/src/main/kotlin/com/xekchansky/strangeattractor/NativeBridge.kt"
);
/// The Rust half of the bridge.
const ANDROID_RS: &str = include_str!("../src/android.rs");

/// The JNI symbol prefix of the `NativeBridge` class.
const PREFIX: &str = "Java_com_xekchansky_strangeattractor_NativeBridge_";

/// The names Kotlin declares as `external fun`.
fn kotlin_externals() -> Vec<String> {
    NATIVE_BRIDGE_KT
        .lines()
        .filter_map(|line| line.trim().strip_prefix("external fun "))
        .filter_map(|rest| rest.split('(').next())
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty())
        .collect()
}

/// The names Rust exports with a `Java_com_…_NativeBridge_` symbol.
fn rust_symbols() -> Vec<String> {
    ANDROID_RS
        .match_indices(PREFIX)
        .map(|(start, _)| {
            ANDROID_RS[start + PREFIX.len()..]
                .chars()
                .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
                .collect::<String>()
        })
        .filter(|name| !name.is_empty())
        .collect()
}

#[test]
fn every_kotlin_external_is_exported_by_rust() {
    let externals = kotlin_externals();
    assert_eq!(externals.len(), 6, "the bridge keeps its six entry points");
    let symbols = rust_symbols();
    for name in &externals {
        assert!(
            symbols.contains(name),
            "Kotlin declares `{name}`, but Rust exports no `{PREFIX}{name}` symbol"
        );
    }
}

#[test]
fn every_rust_symbol_is_declared_in_kotlin() {
    let externals = kotlin_externals();
    for symbol in rust_symbols() {
        assert!(
            externals.contains(&symbol),
            "Rust exports `{PREFIX}{symbol}`, but Kotlin declares no `external fun {symbol}`"
        );
    }
}

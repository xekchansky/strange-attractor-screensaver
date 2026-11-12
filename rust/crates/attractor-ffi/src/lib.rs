//! C ABI used by the macOS screensaver host and the Android JNI bridge.

#[cfg(test)]
mod tests {
    #[test]
    fn crate_is_configured() {
        assert_eq!(env!("CARGO_PKG_NAME"), "attractor-ffi");
    }
}

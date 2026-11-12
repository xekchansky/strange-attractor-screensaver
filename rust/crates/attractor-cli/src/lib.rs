//! Cross-platform demo application with screenshot and recording capture.

#[cfg(test)]
mod tests {
    #[test]
    fn crate_is_configured() {
        assert_eq!(env!("CARGO_PKG_NAME"), "attractor-cli");
    }
}

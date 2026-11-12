//! WebAssembly glue for the GitHub Pages demo, added in I9.

#[cfg(test)]
mod tests {
    #[test]
    fn crate_is_configured() {
        assert_eq!(env!("CARGO_PKG_NAME"), "attractor-wasm");
    }
}

//! WebAssembly glue for the GitHub Pages demo.
//!
//! The native half — [`options`], [`selection`] and [`catalog_view`] — is
//! plain Rust and unit-tested on every platform, so the URL contract and the
//! seeded selection stay covered without a browser. The browser half lives in
//! `web`, builds only for `wasm32` and owns the canvas surface, the player and
//! the per-page state; the page's JavaScript drives the frame loop and renders
//! the overlay as DOM text (design decision D8).

pub mod catalog_view;
pub mod options;
pub mod selection;

#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(target_arch = "wasm32")]
pub use web::{Demo, catalog_json, start};

#[cfg(test)]
mod tests {
    #[test]
    fn crate_is_configured() {
        assert_eq!(env!("CARGO_PKG_NAME"), "attractor-wasm");
    }
}

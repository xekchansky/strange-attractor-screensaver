//! Orchestration of engine, view and GPU surface: tick, render, resize and restart.

#[cfg(test)]
mod tests {
    #[test]
    fn crate_is_configured() {
        assert_eq!(env!("CARGO_PKG_NAME"), "attractor-player");
    }
}

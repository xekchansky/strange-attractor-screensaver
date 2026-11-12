//! Data model, catalog, formula parser, ODE solver, RNG, scheduler, trace buffer, health monitor and engine.

#[cfg(test)]
mod tests {
    #[test]
    fn crate_is_configured() {
        assert_eq!(env!("CARGO_PKG_NAME"), "attractor-core");
    }
}

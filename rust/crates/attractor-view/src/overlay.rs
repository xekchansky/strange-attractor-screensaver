//! Corner-overlay content: attractor name, formulas, parameters and position.

use attractor_core::{AttractorSelection, State};

/// The multi-line text shown in the corner of the screen: the attractor name,
/// its formulas, the concrete parameter values and the current state (the
/// position of the newest trace point).
///
/// The native renderer draws it; the web demo renders the same lines as a DOM
/// overlay, so the content model lives here, free of any text API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayModel {
    lines: Vec<String>,
}

impl OverlayModel {
    /// Build the overlay for `selection` at the live `state`.
    pub fn new(selection: &AttractorSelection, state: State) -> Self {
        let descriptor = &selection.descriptor;

        let mut names: Vec<&String> = selection.parameters.keys().collect();
        names.sort();
        let parameters = names
            .into_iter()
            .map(|name| format!("{name} = {}", Self::format(selection.parameters[name])))
            .collect::<Vec<_>>()
            .join("   ");

        let position = format!(
            "(x, y, z) = ({}, {}, {})",
            Self::format(state.x),
            Self::format(state.y),
            Self::format(state.z)
        );

        Self {
            lines: vec![
                descriptor.name.clone(),
                format!("dX/dt = {}", descriptor.dx),
                format!("dY/dt = {}", descriptor.dy),
                format!("dZ/dt = {}", descriptor.dz),
                parameters,
                position,
            ],
        }
    }

    /// The overlay lines, top to bottom.
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// The overlay text, one line per entry.
    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    /// Format a value the way the Swift overlay does (`%.3f`).
    ///
    /// Non-finite values are spelled the way C's `printf` spells them, so a
    /// briefly diverged state shows `nan` rather than Rust's `NaN`.
    fn format(value: f64) -> String {
        if value.is_nan() {
            "nan".to_string()
        } else if value.is_infinite() {
            if value > 0.0 { "inf" } else { "-inf" }.to_string()
        } else {
            format!("{value:.3}")
        }
    }
}

//! Corner-overlay content: name, formulas, parameters and the live state.

use attractor_core::{DebugConfig, State, catalog};
use attractor_view::OverlayModel;

fn lorenz_selection() -> attractor_core::AttractorSelection {
    let entries = catalog::bundled().expect("bundled catalog");
    DebugConfig::selection("lorenz", &entries).expect("lorenz entry")
}

#[test]
fn the_overlay_lists_the_name_formulas_parameters_and_position() {
    let selection = lorenz_selection();
    let overlay = OverlayModel::new(&selection, State::new(0.1, -0.25, 1.5));
    assert_eq!(
        overlay.text(),
        "Lorenz\n\
         dX/dt = a * (y - x)\n\
         dY/dt = x * (r - z) - y\n\
         dZ/dt = x * y - b * z\n\
         a = 9.500   b = 2.750   r = 27.000\n\
         (x, y, z) = (0.100, -0.250, 1.500)"
    );
    assert_eq!(
        overlay.lines(),
        [
            "Lorenz",
            "dX/dt = a * (y - x)",
            "dY/dt = x * (r - z) - y",
            "dZ/dt = x * y - b * z",
            "a = 9.500   b = 2.750   r = 27.000",
            "(x, y, z) = (0.100, -0.250, 1.500)",
        ]
    );
}

#[test]
fn parameters_are_sorted_by_name_and_separated_by_three_spaces() {
    let mut selection = lorenz_selection();
    selection.parameters.clear();
    selection.parameters.insert("r".to_string(), 28.0);
    selection.parameters.insert("a".to_string(), 10.0);
    selection.parameters.insert("b".to_string(), 8.0 / 3.0);
    let overlay = OverlayModel::new(&selection, State::new(0.0, 0.0, 0.0));
    assert_eq!(overlay.lines()[4], "a = 10.000   b = 2.667   r = 28.000");
}

#[test]
fn the_state_line_formats_every_coordinate_with_three_decimals() {
    let selection = lorenz_selection();
    let overlay = OverlayModel::new(&selection, State::new(-0.0, 12.3456, -7.0));
    assert_eq!(overlay.lines()[5], "(x, y, z) = (-0.000, 12.346, -7.000)");
}

#[test]
fn a_selection_without_parameters_leaves_the_parameter_line_empty() {
    let mut selection = lorenz_selection();
    selection.parameters.clear();
    let overlay = OverlayModel::new(&selection, State::new(0.0, 0.0, 0.0));
    assert_eq!(overlay.lines()[4], "");
    assert_eq!(overlay.lines().len(), 6);
}

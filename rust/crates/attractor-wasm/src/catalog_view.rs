//! The catalog as the picker needs it.

use attractor_core::AttractorDescriptor;
use serde_json::{Value, json};

/// Serialize the catalog for the page's picker: `[{"id": …, "name": …}, …]`.
///
/// The page builds its `<select>` from this list, so adding an attractor to
/// `attractors.json` updates the demo without touching the web code.
pub fn catalog_json(catalog: &[AttractorDescriptor]) -> String {
    let entries: Vec<Value> = catalog
        .iter()
        .map(|descriptor| json!({ "id": descriptor.id, "name": descriptor.name }))
        .collect();
    Value::Array(entries).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_catalog_entry_reaches_the_picker() {
        let catalog = attractor_core::catalog::bundled().expect("the bundled catalog decodes");
        let text = catalog_json(&catalog);
        let parsed: Value = serde_json::from_str(&text).expect("the output is valid JSON");
        let entries = parsed.as_array().expect("the output is an array");
        assert_eq!(entries.len(), catalog.len());
        assert_eq!(entries[0]["id"], json!("lorenz"));
        assert_eq!(entries[0]["name"], json!("Lorenz"));
    }

    #[test]
    fn an_empty_catalog_serializes_to_an_empty_array() {
        assert_eq!(catalog_json(&[]), "[]");
    }
}

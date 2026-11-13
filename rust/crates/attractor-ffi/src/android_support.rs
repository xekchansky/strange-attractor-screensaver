//! Platform-neutral pieces of the Android JNI bridge.
//!
//! The JNI entry points themselves only exist on Android, where they need the
//! NDK's window functions and the `jni` crate. Everything that can be decided
//! without a device lives here, so the normal workspace test run on a desktop
//! covers the settings rule, the frame interval and the catalog the picker
//! lists.

use std::error::Error;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use attractor_shell::{Settings, selection};
use serde::{Deserialize, Serialize};

/// A failure while preparing the arguments for the Android bridge.
#[derive(Debug)]
pub enum BridgeError {
    /// The bundled catalog cannot be read.
    Catalog(selection::SelectionError),
    /// The catalog cannot be written as JSON.
    Encode(serde_json::Error),
}

impl fmt::Display for BridgeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Catalog(error) => write!(formatter, "{error}"),
            Self::Encode(error) => write!(formatter, "cannot encode the catalog: {error}"),
        }
    }
}

impl Error for BridgeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Catalog(error) => Some(error),
            Self::Encode(error) => Some(error),
        }
    }
}

/// One entry of the catalog the settings activity lists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogChoice<'a> {
    /// Stable identifier the settings store.
    pub id: &'a str,
    /// Human-readable name.
    pub name: &'a str,
}

/// The catalog the settings activity lists, as a JSON array of
/// `{"id", "name"}` objects in catalog order.
///
/// The list comes from the same validated catalog the player draws from, so
/// the picker cannot offer an attractor the player would refuse.
pub fn catalog_json() -> Result<String, BridgeError> {
    let catalog = selection::validated_catalog().map_err(BridgeError::Catalog)?;
    let choices: Vec<CatalogChoice<'_>> = catalog
        .iter()
        .map(|descriptor| CatalogChoice {
            id: &descriptor.id,
            name: &descriptor.name,
        })
        .collect();
    serde_json::to_string(&choices).map_err(BridgeError::Encode)
}

/// The settings a wallpaper asks for from the id the picker stored.
///
/// An empty or blank id is the "random each time" choice, which the settings
/// type represents as no pin; an unknown id is handled the same way when the
/// selection is built.
pub fn settings_from_id(id: &str) -> Settings {
    let id = id.trim();
    Settings {
        attractor: (!id.is_empty()).then(|| id.to_owned()),
    }
}

/// Turn the frame interval the wallpaper thread measured into seconds.
///
/// A non-positive interval — the first frame, or a clock that jumped
/// backwards — becomes zero, so the player never steps backwards.
pub fn seconds_from_nanos(elapsed_nanos: i64) -> f64 {
    elapsed_nanos.max(0) as f64 / 1_000_000_000.0
}

/// A seed for one wallpaper session.
///
/// The Kotlin side passes the value it stored or zero; zero means "invent
/// one", exactly like the C ABI's `seed` field, so every activation draws a
/// new attractor without the Android shell keeping a counter.
pub fn resolve_seed(seed: i64) -> u64 {
    if seed == 0 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos() as u64)
            .unwrap_or(0x5EED)
    } else {
        seed as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blank_id_means_random() {
        assert_eq!(settings_from_id(""), Settings::default());
        assert_eq!(settings_from_id("   "), Settings::default());
    }

    #[test]
    fn an_id_is_trimmed_into_the_pin() {
        assert_eq!(
            settings_from_id(" lorenz ").attractor.as_deref(),
            Some("lorenz")
        );
    }

    #[test]
    fn the_catalog_lists_every_runnable_attractor() {
        let json = catalog_json().expect("the bundled catalog is runnable");
        let choices: Vec<serde_json::Value> =
            serde_json::from_str(&json).expect("the list is JSON");
        assert_eq!(choices.len(), 12);
        assert!(
            choices
                .iter()
                .any(|choice| choice["id"] == "lorenz" && choice["name"] == "Lorenz"),
            "the picker offers Lorenz with its display name"
        );
    }

    #[test]
    fn the_catalog_json_keeps_catalog_order() {
        let json = catalog_json().expect("the bundled catalog is runnable");
        let choices: Vec<CatalogChoice<'_>> =
            serde_json::from_str(&json).expect("the list decodes back");
        let catalog = selection::validated_catalog().expect("the bundled catalog is runnable");
        let ids: Vec<&str> = choices.iter().map(|choice| choice.id).collect();
        let expected: Vec<&str> = catalog
            .iter()
            .map(|descriptor| descriptor.id.as_str())
            .collect();
        assert_eq!(ids, expected);
    }

    #[test]
    fn a_frame_interval_becomes_seconds() {
        assert_eq!(seconds_from_nanos(16_666_667), 0.016_666_667);
        assert_eq!(seconds_from_nanos(1_000_000_000), 1.0);
    }

    #[test]
    fn a_non_positive_interval_is_zero() {
        assert_eq!(seconds_from_nanos(0), 0.0);
        assert_eq!(seconds_from_nanos(-5), 0.0);
    }

    #[test]
    fn an_explicit_seed_is_kept() {
        assert_eq!(resolve_seed(42), 42);
        assert_eq!(resolve_seed(-1), u64::MAX);
    }

    #[test]
    fn a_zero_seed_invents_one() {
        assert_ne!(resolve_seed(0), 0);
    }
}

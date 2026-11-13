//! Turning the settings file into a runnable attractor.

use std::error::Error;
use std::fmt;

use attractor_core::catalog::{self, CatalogError};
use attractor_core::{
    AttractorDescriptor, AttractorSelection, RandomizerError, SplitMix64, randomizer,
};

use crate::settings::Settings;

/// Why the catalog could not produce a selection.
#[derive(Debug)]
pub enum SelectionError {
    /// The bundled catalog does not decode.
    Catalog(CatalogError),
    /// Every descriptor failed validation, so there is nothing to run.
    EmptyCatalog,
    /// The randomizer rejected the chosen descriptor.
    Randomizer(RandomizerError),
}

impl fmt::Display for SelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Catalog(error) => write!(formatter, "{error}"),
            Self::EmptyCatalog => write!(formatter, "the catalog contains no runnable attractor"),
            Self::Randomizer(error) => write!(formatter, "cannot select an attractor: {error}"),
        }
    }
}

impl Error for SelectionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Catalog(error) => Some(error),
            Self::Randomizer(error) => Some(error),
            Self::EmptyCatalog => None,
        }
    }
}

/// Load the bundled catalog and drop the descriptors that failed validation.
///
/// A single malformed entry must not blank the screen saver, so invalid
/// descriptors are logged and skipped instead of aborting the catalog.
pub fn validated_catalog() -> Result<Vec<AttractorDescriptor>, SelectionError> {
    let validation = catalog::bundled_validation().map_err(SelectionError::Catalog)?;
    for issue in &validation.issues {
        log::warn!(
            "dropping attractor '{}' ({}): {}",
            issue.descriptor_id,
            issue.field,
            issue.message
        );
    }
    if validation.valid.is_empty() {
        return Err(SelectionError::EmptyCatalog);
    }
    Ok(validation.valid)
}

/// Build the selection a player runs.
///
/// A pinned attractor keeps its identity but still draws fresh parameters,
/// hyperparameters and an initial state from the seeded generator; without a
/// usable pin the whole catalog is in play. The draw order is the randomizer's,
/// so the same seed and settings reproduce the same run.
pub fn choose(
    settings: &Settings,
    catalog: &[AttractorDescriptor],
    rng: &mut SplitMix64,
) -> Result<AttractorSelection, SelectionError> {
    match settings.resolve(catalog) {
        Some(descriptor) => {
            randomizer::select_attractor(descriptor, rng).map_err(SelectionError::Randomizer)
        }
        None => randomizer::select_from_catalog(catalog, rng).map_err(SelectionError::Randomizer),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_catalog_is_runnable() {
        let catalog = validated_catalog().expect("the bundled catalog validates");
        assert_eq!(catalog.len(), 12);
    }

    #[test]
    fn a_pinned_attractor_is_the_one_that_runs() {
        let catalog = validated_catalog().expect("the bundled catalog validates");
        let settings = Settings {
            attractor: Some("aizawa".to_owned()),
        };
        let mut rng = SplitMix64::new(7);
        let selection = choose(&settings, &catalog, &mut rng).expect("the selection is built");
        assert_eq!(selection.descriptor.id, "aizawa");
    }

    #[test]
    fn a_stale_pin_falls_back_to_the_catalog() {
        let catalog = validated_catalog().expect("the bundled catalog validates");
        let settings = Settings {
            attractor: Some("no-such-attractor".to_owned()),
        };
        let mut rng = SplitMix64::new(7);
        let selection = choose(&settings, &catalog, &mut rng).expect("the selection is built");
        assert!(
            catalog
                .iter()
                .any(|descriptor| descriptor.id == selection.descriptor.id)
        );
    }

    #[test]
    fn the_same_seed_produces_the_same_selection() {
        let catalog = validated_catalog().expect("the bundled catalog validates");
        let settings = Settings::default();
        let first =
            choose(&settings, &catalog, &mut SplitMix64::new(42)).expect("the selection is built");
        let second =
            choose(&settings, &catalog, &mut SplitMix64::new(42)).expect("the selection is built");
        assert_eq!(first, second);
    }

    #[test]
    fn an_empty_catalog_is_an_error() {
        let mut rng = SplitMix64::new(1);
        let error = choose(&Settings::default(), &[], &mut rng).expect_err("nothing to run");
        assert!(matches!(error, SelectionError::Randomizer(_)));
    }
}

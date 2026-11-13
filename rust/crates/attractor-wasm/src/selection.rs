//! Turns the URL options into one concrete run.

use attractor_core::randomizer::{self, RandomizerError};
use attractor_core::{AttractorDescriptor, AttractorSelection, SplitMix64};

use crate::options::DemoOptions;

/// A demo run: the concrete selection and the seed that produced it.
#[derive(Debug, Clone)]
pub struct DemoRun {
    /// Seed that drives the parameter sampling and the camera.
    pub seed: u64,
    /// Everything the player needs to start.
    pub selection: AttractorSelection,
}

/// Build the run described by `options`.
///
/// A named attractor samples its parameters, hyperparameters and initial
/// state from the descriptor's declared ranges, exactly like the screensaver's
/// random selection — the seed decides which point of the range is used. An
/// unknown or missing id draws a whole random attractor from `catalog`.
/// `fallback_seed` is used when the URL carries no seed.
pub fn build(
    catalog: &[AttractorDescriptor],
    options: &DemoOptions,
    fallback_seed: u64,
) -> Result<DemoRun, RandomizerError> {
    let seed = options.seed.unwrap_or(fallback_seed);
    let mut rng = SplitMix64::new(seed);
    let named = options
        .attractor
        .as_deref()
        .and_then(|id| catalog.iter().find(|descriptor| descriptor.id == id));
    let selection = match named {
        Some(descriptor) => randomizer::select_attractor(descriptor, &mut rng)?,
        None => randomizer::select_from_catalog(catalog, &mut rng)?,
    };
    Ok(DemoRun { seed, selection })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::{Backend, DemoOptions};

    fn catalog() -> Vec<AttractorDescriptor> {
        attractor_core::catalog::bundled().expect("the bundled catalog decodes")
    }

    #[test]
    fn a_named_attractor_is_selected_from_its_own_descriptor() {
        let options = DemoOptions {
            attractor: Some("lorenz".to_string()),
            seed: Some(42),
            backend: Backend::Auto,
        };
        let run = build(&catalog(), &options, 0).expect("the run builds");
        assert_eq!(run.selection.descriptor.id, "lorenz");
        assert_eq!(run.seed, 42);
    }

    #[test]
    fn the_same_seed_reproduces_the_same_run() {
        let options = DemoOptions {
            attractor: Some("chua".to_string()),
            seed: Some(7),
            backend: Backend::Auto,
        };
        let first = build(&catalog(), &options, 0).expect("the run builds");
        let second = build(&catalog(), &options, 0).expect("the run builds");
        assert_eq!(first.selection, second.selection);
    }

    #[test]
    fn different_seeds_sample_different_parameters() {
        let catalog = catalog();
        let mut seeds = (0..8).map(|seed| {
            let options = DemoOptions {
                attractor: Some("lorenz".to_string()),
                seed: Some(seed),
                backend: Backend::Auto,
            };
            build(&catalog, &options, 0)
                .expect("the run builds")
                .selection
                .parameters
        });
        let first = seeds.next().expect("one run");
        assert!(
            seeds.any(|parameters| parameters != first),
            "eight seeds all produced the same parameters"
        );
    }

    #[test]
    fn an_unknown_id_falls_back_to_a_random_attractor() {
        let options = DemoOptions {
            attractor: Some("no-such-attractor".to_string()),
            seed: Some(3),
            backend: Backend::Auto,
        };
        let run = build(&catalog(), &options, 0).expect("the run builds");
        assert!(!run.selection.descriptor.id.is_empty());
    }

    #[test]
    fn the_fallback_seed_is_used_when_the_url_has_none() {
        let options = DemoOptions {
            attractor: Some("aizawa".to_string()),
            seed: None,
            backend: Backend::Auto,
        };
        let run = build(&catalog(), &options, 99).expect("the run builds");
        assert_eq!(run.seed, 99);
        let explicit = DemoOptions {
            seed: Some(99),
            ..options
        };
        assert_eq!(
            run.selection,
            build(&catalog(), &explicit, 0)
                .expect("the run builds")
                .selection
        );
    }
}

//! The settings file the `/c` window writes.
//!
//! Windows has no registry-free convention for a screen saver, so the file
//! lives next to the user's application data: `%APPDATA%\StrangeAttractor\
//! settings.json`. Only the attractor choice is stored today; the format is
//! plain JSON with unknown keys ignored, so a later build may add fields
//! without breaking an older one.

use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use attractor_core::AttractorDescriptor;
use serde::{Deserialize, Serialize};

/// Directory under `%APPDATA%` that holds the settings file.
pub const DIRECTORY_NAME: &str = "StrangeAttractor";
/// Name of the settings file inside [`DIRECTORY_NAME`].
pub const FILE_NAME: &str = "settings.json";

/// Everything the user can choose.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Attractor id to pin; `None` (or an unknown id) draws a random one on
    /// every launch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attractor: Option<String>,
}

impl Settings {
    /// Path of the settings file under `appdata`.
    ///
    /// `None` means the environment did not provide an application-data
    /// directory, so the saver runs with defaults and `/c` reports the
    /// problem.
    pub fn path_in(appdata: Option<&Path>) -> Option<PathBuf> {
        appdata.map(|root| root.join(DIRECTORY_NAME).join(FILE_NAME))
    }

    /// Path of the settings file for the current user.
    pub fn environment_path() -> Option<PathBuf> {
        let appdata = std::env::var_os("APPDATA").map(PathBuf::from);
        Self::path_in(appdata.as_deref())
    }

    /// Read the file, returning defaults when it does not exist yet.
    pub fn load(path: &Path) -> Result<Self, SettingsError> {
        match fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).map_err(SettingsError::Decode),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(SettingsError::Io(error)),
        }
    }

    /// Write the file, creating its directory when needed.
    ///
    /// The write goes to a sibling temporary file and is renamed into place,
    /// so an interrupted save cannot leave a half-written settings file.
    pub fn save(&self, path: &Path) -> Result<(), SettingsError> {
        if let Some(directory) = path.parent() {
            fs::create_dir_all(directory).map_err(SettingsError::Io)?;
        }
        let text = serde_json::to_string_pretty(self).map_err(SettingsError::Encode)?;
        let temporary = path.with_extension("json.tmp");
        fs::write(&temporary, text).map_err(SettingsError::Io)?;
        fs::rename(&temporary, path).map_err(SettingsError::Io)
    }

    /// The pinned descriptor, when the id names one in the catalog.
    ///
    /// An empty or stale id is treated as "random", so uninstalling an
    /// attractor from the catalog does not break the saver.
    pub fn resolve<'a>(
        &self,
        catalog: &'a [AttractorDescriptor],
    ) -> Option<&'a AttractorDescriptor> {
        let id = self
            .attractor
            .as_deref()
            .map(str::trim)
            .filter(|id| !id.is_empty())?;
        catalog.iter().find(|descriptor| descriptor.id == id)
    }
}

/// A settings file that could not be read, decoded or written.
#[derive(Debug)]
pub enum SettingsError {
    /// The file system rejected the operation.
    Io(io::Error),
    /// The file is not valid settings JSON.
    Decode(serde_json::Error),
    /// The settings could not be encoded.
    Encode(serde_json::Error),
}

impl fmt::Display for SettingsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "cannot access the settings file: {error}"),
            Self::Decode(error) => write!(formatter, "cannot decode the settings file: {error}"),
            Self::Encode(error) => write!(formatter, "cannot encode the settings: {error}"),
        }
    }
}

impl Error for SettingsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Decode(error) | Self::Encode(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    /// A unique empty directory under the system temporary directory.
    ///
    /// The crate has no `tempfile` dependency, and the tests only need a path
    /// that no other test uses.
    fn temporary_directory() -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique = format!(
            "attractor-windows-settings-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        std::env::temp_dir().join(unique)
    }

    fn descriptor(id: &str) -> AttractorDescriptor {
        let mut descriptor = attractor_core::catalog::bundled()
            .expect("the bundled catalog decodes")
            .into_iter()
            .next()
            .expect("the catalog is not empty");
        descriptor.id = id.to_owned();
        descriptor
    }

    #[test]
    fn the_path_lives_under_the_application_data_directory() {
        let path = Settings::path_in(Some(Path::new("C:\\Users\\me\\AppData\\Roaming")))
            .expect("the path is derived");
        assert!(path.ends_with(Path::new(DIRECTORY_NAME).join(FILE_NAME)));
        assert_eq!(Settings::path_in(None), None);
    }

    #[test]
    fn a_missing_file_loads_the_defaults() {
        let path = temporary_directory().join(FILE_NAME);
        assert_eq!(
            Settings::load(&path).expect("missing is not an error"),
            Settings::default()
        );
    }

    #[test]
    fn saving_and_loading_round_trips() {
        let directory = temporary_directory();
        let path = directory.join(FILE_NAME);
        let settings = Settings {
            attractor: Some("lorenz".to_owned()),
        };
        settings.save(&path).expect("the settings are written");
        assert_eq!(
            Settings::load(&path).expect("the settings are read"),
            settings
        );
        fs::remove_dir_all(directory).expect("the temporary directory is removed");
    }

    #[test]
    fn a_random_choice_writes_an_empty_object() {
        let directory = temporary_directory();
        let path = directory.join(FILE_NAME);
        Settings::default()
            .save(&path)
            .expect("the settings are written");
        let text = fs::read_to_string(&path).expect("the file exists");
        assert_eq!(text.trim(), "{}");
        fs::remove_dir_all(directory).expect("the temporary directory is removed");
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let settings: Settings =
            serde_json::from_str(r#"{"attractor":"aizawa","speed":2}"#).expect("the JSON decodes");
        assert_eq!(settings.attractor.as_deref(), Some("aizawa"));
    }

    #[test]
    fn a_corrupt_file_is_an_error() {
        let directory = temporary_directory();
        fs::create_dir_all(&directory).expect("the directory is created");
        let path = directory.join(FILE_NAME);
        fs::write(&path, "{not json}").expect("the file is written");
        assert!(matches!(
            Settings::load(&path),
            Err(SettingsError::Decode(_))
        ));
        fs::remove_dir_all(directory).expect("the temporary directory is removed");
    }

    #[test]
    fn a_pinned_id_resolves_only_when_it_is_in_the_catalog() {
        let catalog = vec![descriptor("lorenz"), descriptor("aizawa")];
        let pinned = Settings {
            attractor: Some(" lorenz ".to_owned()),
        };
        assert_eq!(
            pinned
                .resolve(&catalog)
                .map(|descriptor| descriptor.id.as_str()),
            Some("lorenz")
        );
        let stale = Settings {
            attractor: Some("thomas".to_owned()),
        };
        assert!(stale.resolve(&catalog).is_none());
        let empty = Settings {
            attractor: Some("   ".to_owned()),
        };
        assert!(empty.resolve(&catalog).is_none());
        assert!(Settings::default().resolve(&catalog).is_none());
    }
}

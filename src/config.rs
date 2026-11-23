use std::borrow::Cow;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use pumpkin_plugin_api::Result;
use serde::de::DeserializeOwned;

/// Loads a TOML config, writing the bundled default if the file is missing.
pub(crate) fn load<T: DeserializeOwned>(path: &Path, default: &str) -> Result<T> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => Cow::Owned(contents),
        Err(error) if error.kind() == ErrorKind::NotFound => {
            fs::write(path, default)
                .map_err(|error| format!("Cannot create {}: {error}", path.display()))?;
            Cow::Borrowed(default)
        }
        Err(error) => return Err(format!("Cannot read {}: {error}", path.display())),
    };
    toml::from_str(&contents).map_err(|error| format!("Cannot parse {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn loads_existing_config_and_writes_defaults_only_when_missing() {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let path = std::env::temp_dir().join(format!(
            "rummo-config-{}-{}.toml",
            std::process::id(),
            timestamp.as_nanos()
        ));
        let default = "amount = 1\n";
        let config: HashMap<String, u32> = load(&path, default).unwrap();
        assert_eq!(config["amount"], 1);
        assert_eq!(fs::read_to_string(&path).unwrap(), default);

        let custom = "amount = 2\n";
        fs::write(&path, custom).unwrap();
        let config: HashMap<String, u32> = load(&path, default).unwrap();
        assert_eq!(config["amount"], 2);
        assert_eq!(fs::read_to_string(&path).unwrap(), custom);

        let malformed = "amount =\n";
        fs::write(&path, malformed).unwrap();
        let error = load::<HashMap<String, u32>>(&path, default).unwrap_err();
        assert!(error.starts_with(&format!("Cannot parse {}:", path.display())));
        assert_eq!(fs::read_to_string(&path).unwrap(), malformed);
        fs::remove_file(&path).unwrap();
    }
}

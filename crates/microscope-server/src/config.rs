//! Reading the server's configuration.
//!
//! The file is `teta-wot`'s (LabThings') format. The application's own
//! settings live under its `application_config` key ([`AppConfig`]).

use std::path::PathBuf;

use serde::Deserialize;
use teta_wot::server::{ServerConfig, cli::CliArgs};

/// The application's own configuration: the `application_config` key of the
/// configuration file. Missing keys take their defaults.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// Where captures, scans and other data are saved.
    pub data_folder: PathBuf,
    /// Where log files are written.
    pub log_folder: PathBuf,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            data_folder: PathBuf::from(microscope_things::data::DEFAULT_DATA_FOLDER),
            log_folder: PathBuf::from(".microscope/logs"),
        }
    }
}

/// A configuration that has been read and checked.
#[derive(Debug, Clone)]
pub struct Config {
    /// The server's configuration, as `teta-wot` reads it.
    pub server: ServerConfig,
    /// The application's own configuration.
    pub app: AppConfig,
}

/// Why a configuration couldn't be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// There is no configuration to read: none was given, both `-c` and `-j`
    /// were given, or the file can't be read.
    Missing(String),
    /// The configuration was read but isn't valid.
    Invalid(String),
}

impl ConfigError {
    /// What went wrong, for the user.
    pub fn message(&self) -> &str {
        match self {
            ConfigError::Missing(message) | ConfigError::Invalid(message) => message,
        }
    }
}

/// The configuration's text, from `-c` (a file) or `-j` (inline JSON), with
/// the same messages as `teta-wot`'s command line.
pub fn text(args: &CliArgs) -> Result<String, ConfigError> {
    match (&args.config, &args.json) {
        (Some(_), Some(_)) => Err(ConfigError::Missing(
            "Can't use both --config and --json simultaneously.".into(),
        )),
        (Some(path), None) => std::fs::read_to_string(path).map_err(|error| {
            ConfigError::Missing(format!(
                "Could not find configuration file {} ({error})",
                path.display()
            ))
        }),
        (None, Some(json)) => Ok(json.clone()),
        (None, None) => Err(ConfigError::Missing(
            "No configuration (or empty configuration) provided".into(),
        )),
    }
}

/// Parses and checks a configuration's text.
pub fn parse(text: &str) -> Result<Config, ConfigError> {
    let server = ServerConfig::from_json(text).map_err(|e| ConfigError::Invalid(e.to_string()))?;
    let app = match &server.application_config {
        Some(value) => AppConfig::deserialize(value)
            .map_err(|e| ConfigError::Invalid(format!("application_config: {e}")))?,
        None => AppConfig::default(),
    };
    Ok(Config { server, app })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn application_config_is_read_with_defaults_for_missing_keys() {
        let config = parse(
            r#"{"things": {}, "application_config": {"data_folder": "D:/data", "extra": 1}}"#,
        )
        .expect("valid");
        assert_eq!(config.app.data_folder, PathBuf::from("D:/data"));
        assert_eq!(config.app.log_folder, AppConfig::default().log_folder);
    }

    #[test]
    fn a_missing_application_config_takes_the_defaults() {
        let config = parse(r#"{"things": {}}"#).expect("valid");
        assert_eq!(config.app, AppConfig::default());
    }

    #[test]
    fn a_malformed_application_config_is_invalid() {
        let error = parse(r#"{"things": {}, "application_config": {"data_folder": 3}}"#)
            .expect_err("invalid");
        assert!(matches!(error, ConfigError::Invalid(m) if m.starts_with("application_config")));
    }

    #[test]
    fn the_shipped_simulation_configuration_is_valid() {
        let text = include_str!("../../../configs/simulation.json");
        let config = parse(text).expect("valid");
        assert_eq!(config.server.api_prefix, "/api/v1");
        assert!(config.server.enable_global_lock);
    }
}

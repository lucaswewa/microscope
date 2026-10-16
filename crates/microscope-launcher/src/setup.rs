//! What the launcher prepares before it starts the server: its folder, the
//! configuration it gives the server, its single-instance lock and a port.

use std::fs::{File, OpenOptions, TryLockError};
use std::net::TcpListener;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

/// The configuration used without `--config`: the simulated microscope's.
pub const SIMULATION: &str = include_str!("../../../configs/simulation.json");

/// `%LOCALAPPDATA%\Microscope`, where the launcher keeps the server's
/// settings, data and logs (plan §5.9), or `.microscope` without it.
pub fn default_root() -> PathBuf {
    std::env::var_os("LOCALAPPDATA").map_or_else(
        || PathBuf::from(".microscope"),
        |local| PathBuf::from(local).join("Microscope"),
    )
}

/// The configuration `text`, with its settings, data and log folders made
/// absolute: those it gives as absolute paths are kept, and the others are
/// `settings`, `data` and `logs` in `root`.
pub fn resolve(text: &str, root: &Path) -> Result<Value, String> {
    let mut config: Value =
        serde_json::from_str(text).map_err(|e| format!("the configuration isn't JSON: {e}"))?;
    let object = (config.as_object_mut()).ok_or("the configuration isn't a JSON object")?;
    let absolute = |given: Option<&Value>, name: &str| match given.and_then(Value::as_str) {
        Some(path) if Path::new(path).is_absolute() => json!(path),
        _ => json!(root.join(name)),
    };
    let settings = absolute(object.get("settings_folder"), "settings");
    object.insert("settings_folder".to_owned(), settings);
    let application = (object
        .entry("application_config")
        .or_insert_with(|| json!({})))
    .as_object_mut()
    .ok_or("`application_config` isn't a JSON object")?;
    for (key, name) in [("data_folder", "data"), ("log_folder", "logs")] {
        let folder = absolute(application.get(key), name);
        application.insert(key.to_owned(), folder);
    }
    Ok(config)
}

/// Takes the lock that lets one launcher at a time use `root`, which is
/// held until the file returned is dropped.
pub fn lock(root: &Path) -> Result<File, String> {
    let path = root.join("launcher.lock");
    let file = (OpenOptions::new().create(true).truncate(false).write(true))
        .open(&path)
        .map_err(|e| format!("can't open {}: {e}", path.display()))?;
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(TryLockError::WouldBlock) => Err(format!(
            "another launcher is already running for {}",
            root.display()
        )),
        Err(TryLockError::Error(e)) => Err(format!("can't lock {}: {e}", path.display())),
    }
}

/// `preferred` if it's free on the loopback address, or else 0, for the
/// server to choose one.
pub fn port(preferred: u16) -> u16 {
    if TcpListener::bind(("127.0.0.1", preferred)).is_ok() {
        preferred
    } else {
        0
    }
}

/// `microscope-server`, beside the launcher.
pub fn default_server() -> Result<PathBuf, String> {
    let launcher = std::env::current_exe().map_err(|e| format!("can't find the launcher: {e}"))?;
    Ok(launcher.with_file_name(format!("microscope-server{}", std::env::consts::EXE_SUFFIX)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_folders_are_absolute_and_in_the_root_unless_given_so() {
        // Absolute paths on every platform.
        let base = std::env::temp_dir();
        let root = &base.join("Microscope");
        let config = resolve(SIMULATION, root).expect("resolved");
        assert_eq!(config["settings_folder"], json!(root.join("settings")));
        assert_eq!(
            config["application_config"]["data_folder"],
            json!(root.join("data"))
        );
        assert_eq!(
            config["application_config"]["log_folder"],
            json!(root.join("logs"))
        );
        // The rest is as given.
        assert_eq!(config["things"]["sequence"], "microscope.sequence:Sequence");

        let elsewhere = base.join("lab").join("settings");
        let given = json!({"things": {}, "settings_folder": elsewhere}).to_string();
        let config = resolve(&given, root).expect("resolved");
        assert_eq!(config["settings_folder"], json!(elsewhere));
        assert_eq!(
            config["application_config"]["data_folder"],
            json!(root.join("data"))
        );
        assert!(resolve("[1]", root).is_err());
        assert!(resolve("{", root).is_err());
    }

    #[test]
    fn one_launcher_at_a_time_holds_the_lock() {
        let root = tempfile::tempdir().expect("a folder");
        let first = lock(root.path()).expect("the first locks");
        let refused = lock(root.path()).expect_err("the second is refused");
        assert!(refused.contains("already running"), "{refused}");
        drop(first);
        lock(root.path()).expect("free again");
    }

    #[test]
    fn a_taken_port_leaves_the_choice_to_the_server() {
        let taken = TcpListener::bind("127.0.0.1:0").expect("binds");
        let number = taken.local_addr().expect("an address").port();
        assert_eq!(port(number), 0);
        drop(taken);
        assert_eq!(port(number), number);
    }
}

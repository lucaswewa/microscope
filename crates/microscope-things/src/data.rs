//! The data folder, where captures and other data are saved. P26 lays it
//! out; for now, Things find it and resolve paths in it safely.

use std::path::{Component, Path, PathBuf};

use teta_wot::prelude::*;

/// The data folder when the configuration doesn't name one.
pub const DEFAULT_DATA_FOLDER: &str = ".microscope/data";

/// The data folder: `data_folder` in the configuration's
/// `application_config`, or [`DEFAULT_DATA_FOLDER`].
pub fn data_folder(server: &Server) -> PathBuf {
    let configured = server.application_config().and_then(|config| {
        let folder = config.get("data_folder")?.as_str()?;
        Some(PathBuf::from(folder))
    });
    configured.unwrap_or_else(|| PathBuf::from(DEFAULT_DATA_FOLDER))
}

/// `relative` inside `folder`, if it stays there: a relative path to a file,
/// with no root, drive or `..`.
pub fn resolve(folder: &Path, relative: &str) -> Result<PathBuf, String> {
    let path = Path::new(relative);
    let inside = path
        .components()
        .all(|part| matches!(part, Component::Normal(_) | Component::CurDir));
    if !inside || path.file_name().is_none() {
        return Err(format!(
            "`{relative}` isn't a file path inside the data folder"
        ));
    }
    Ok(folder.join(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_must_stay_inside_the_data_folder() {
        let folder = Path::new("data");
        assert_eq!(
            resolve(folder, "captures/a.jpg"),
            Ok(folder.join("captures/a.jpg"))
        );
        assert_eq!(resolve(folder, "./b.png"), Ok(folder.join("./b.png")));
        for outside in [
            "",
            ".",
            "..",
            "../a.jpg",
            "captures/../../a.jpg",
            "/a.jpg",
            "C:/a.jpg",
            "C:a.jpg",
            r"\\pc\share\a.jpg",
        ] {
            assert!(resolve(folder, outside).is_err(), "{outside}");
        }
    }
}

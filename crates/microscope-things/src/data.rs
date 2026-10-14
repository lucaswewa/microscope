//! The data folder, where captures and other data are saved (ADR-0025):
//! Things find it here, resolve paths in it safely, and pack its files into
//! ZIP downloads.

use std::fs::File;
use std::io::{Seek, Write};
use std::path::{Component, Path, PathBuf};

use teta_wot::prelude::*;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

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

/// A new folder in `parent` for a run of `kind`, such as a scan: called
/// `requested`, or `<kind>-<UTC time>` without it, with `-2`, `-3`, … if
/// that's taken. Its name and its path.
pub fn new_folder(
    parent: &Path,
    requested: &str,
    kind: &str,
) -> Result<(String, PathBuf), ActionError> {
    let stem = match requested.trim() {
        "" => format!("{kind}-{}", chrono::Utc::now().format("%Y-%m-%dT%H-%M-%SZ")),
        name => name.to_owned(),
    };
    let allowed = |c: char| c.is_alphanumeric() || " -_.".contains(c);
    if !stem.chars().all(allowed) || stem.starts_with('.') {
        return Err(ActionError::handled(format!(
            "a {kind}'s name can have letters, digits, spaces, `-`, `_` and `.`, and can't start with `.`"
        )));
    }
    std::fs::create_dir_all(parent).map_err(ActionError::handled)?;
    for n in 1.. {
        let name = match n {
            1 => stem.clone(),
            n => format!("{stem}-{n}"),
        };
        let folder = parent.join(&name);
        match std::fs::create_dir(&folder) {
            Ok(()) => return Ok((name, folder)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(ActionError::handled(error)),
        }
    }
    unreachable!("a free name is found")
}

/// Writes a ZIP archive of `files`, each a name in the archive and the file
/// to put there, to `out`, which it returns. The files are stored as they
/// are, since images are compressed already.
pub fn zip<W: Write + Seek>(files: &[(String, PathBuf)], out: W) -> zip::result::ZipResult<W> {
    let mut archive = ZipWriter::new(out);
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    for (name, path) in files {
        archive.start_file(name.as_str(), stored)?;
        std::io::copy(&mut File::open(path)?, &mut archive)?;
    }
    archive.finish()
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

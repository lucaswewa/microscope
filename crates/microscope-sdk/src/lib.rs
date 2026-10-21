//! The camera vendors' SDKs, through their C APIs, loaded at run time
//! (ADR-0041): [`pylon`] for Basler cameras, and [`vimba`] (Vimba X) for
//! Allied Vision's.
//!
//! One server binary runs with or without them. An SDK that isn't installed
//! is an [`SdkError`] that says where it was looked for and how to install
//! it, not a server that can't be built. The user installs the SDKs; the
//! project never redistributes them.
//!
//! This is the workspace's one crate with unsafe code, which calling C
//! needs. What it offers is safe, and each unsafe block says why it's sound.

use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};

pub mod pylon;
pub mod vimba;

/// Why an SDK can't be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SdkError {
    /// It isn't installed: its library is in none of the places tried.
    NotInstalled {
        /// The SDK.
        sdk: &'static str,
        /// Where its library was looked for.
        tried: Vec<PathBuf>,
        /// The variable that names its library, overriding the search.
        variable: &'static str,
        /// Where to get it.
        install: &'static str,
    },
    /// Its library is there, but can't be loaded, or lacks a function.
    Unusable {
        /// The SDK.
        sdk: &'static str,
        /// Its library.
        path: PathBuf,
        /// Why.
        reason: String,
    },
    /// One of its functions returned an error.
    Failed {
        /// The SDK.
        sdk: &'static str,
        /// The function.
        function: &'static str,
        /// Its error code.
        code: i32,
        /// What the SDK said of it, if anything.
        detail: String,
    },
}

impl fmt::Display for SdkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SdkError::NotInstalled {
                sdk,
                tried,
                variable,
                install,
            } => {
                let tried: Vec<_> = tried
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect();
                write!(
                    f,
                    "{sdk} isn't installed: it isn't at {}. Install it from {install}, \
                     or set {variable} to its library's path",
                    tried.join(", ")
                )
            }
            SdkError::Unusable { sdk, path, reason } => {
                write!(f, "{sdk} at {} can't be used: {reason}", path.display())
            }
            SdkError::Failed {
                sdk,
                function,
                code,
                detail,
            } => {
                write!(
                    f,
                    "{sdk}'s {function} failed with error {code} ({code:#010x})"
                )?;
                match detail.is_empty() {
                    true => Ok(()),
                    false => write!(f, ": {detail}"),
                }
            }
        }
    }
}

impl std::error::Error for SdkError {}

/// Where an SDK's C library is looked for.
struct Search {
    sdk: &'static str,
    /// The variable that names the library, in place of the search.
    variable: &'static str,
    install: &'static str,
    /// Paths, and then bare file names, which the system's search finds.
    places: Vec<PathBuf>,
}

/// An SDK's loaded library, and where it is.
struct Loaded {
    library: libloading::Library,
    path: PathBuf,
    sdk: &'static str,
}

impl Search {
    /// Loads the library from the first place that has it, or from the
    /// path `var(variable)` gives.
    fn load(self, var: impl Fn(&str) -> Option<OsString>) -> Result<Loaded, SdkError> {
        let places = match var(self.variable).filter(|path| !path.is_empty()) {
            Some(path) => vec![PathBuf::from(path)],
            None => self.places,
        };
        for path in &places {
            let bare = path
                .parent()
                .is_none_or(|parent| parent.as_os_str().is_empty());
            if !bare && !path.is_file() {
                continue;
            }
            match open(path) {
                Ok(library) => {
                    let (path, sdk) = (path.clone(), self.sdk);
                    return Ok(Loaded { library, path, sdk });
                }
                // A bare name not on the search path.
                Err(_) if bare => {}
                Err(error) => {
                    let reason = error.to_string();
                    let (path, sdk) = (path.clone(), self.sdk);
                    return Err(SdkError::Unusable { sdk, path, reason });
                }
            }
        }
        Err(SdkError::NotInstalled {
            sdk: self.sdk,
            tried: places,
            variable: self.variable,
            install: self.install,
        })
    }
}

/// Loads the library at `path`, searching its own folder for the libraries
/// it needs.
#[cfg(windows)]
fn open(path: &Path) -> Result<libloading::Library, libloading::Error> {
    use libloading::os::windows::{LOAD_WITH_ALTERED_SEARCH_PATH, Library};
    let flags = if path.is_absolute() {
        LOAD_WITH_ALTERED_SEARCH_PATH
    } else {
        0
    };
    // SAFETY: loading runs the library's initialisers. It's the SDK's own
    // library, where its installer put it or where the user said, and it's
    // made to be loaded by programs this way.
    unsafe { Library::load_with_flags(path.as_os_str(), flags) }.map(Into::into)
}

/// Loads the library at `path`.
#[cfg(not(windows))]
fn open(path: &Path) -> Result<libloading::Library, libloading::Error> {
    // SAFETY: as on Windows: it's the SDK's own library, made to be loaded.
    unsafe { libloading::Library::new(path.as_os_str()) }
}

impl Loaded {
    /// The function `name`.
    ///
    /// # Safety
    ///
    /// `F` must be the function's type, as the SDK's header declares it.
    unsafe fn function<F: Copy>(&self, name: &'static str) -> Result<F, SdkError> {
        // SAFETY: the caller gives the function's type. The pointer copied
        // out stays valid while the library is loaded, and each SDK's
        // struct keeps its library with its functions.
        let function = unsafe { self.library.get::<F>(name) };
        function
            .map(|function| *function)
            .map_err(|error| SdkError::Unusable {
                sdk: self.sdk,
                path: self.path.clone(),
                reason: format!("it has no `{name}`: {error}"),
            })
    }
}

/// Held while an SDK starts or stops. Started at the same moment on two
/// threads, pylon and Vimba X can miss each other's cameras: Vimba X uses
/// pylon's GenTL producers too.
fn one_at_a_time() -> std::sync::MutexGuard<'static, ()> {
    static STARTING: std::sync::Mutex<()> = std::sync::Mutex::new(());
    STARTING.lock().unwrap_or_else(|error| error.into_inner())
}

/// `Ok` if `code`, from `function`, is 0, success in both SDKs.
fn check(sdk: &'static str, function: &'static str, code: i32) -> Result<(), SdkError> {
    match code {
        0 => Ok(()),
        code => Err(SdkError::Failed {
            sdk,
            function,
            code,
            detail: String::new(),
        }),
    }
}

/// The folder `var(name)` names, if it's set.
fn folder(var: &impl Fn(&str) -> Option<OsString>, name: &str) -> Option<PathBuf> {
    var(name).filter(|path| !path.is_empty()).map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn search(places: Vec<PathBuf>) -> Search {
        Search {
            sdk: "an SDK",
            variable: "AN_SDK_LIBRARY",
            install: "its maker",
            places,
        }
    }

    #[test]
    fn a_missing_library_says_where_it_was_looked_for_and_what_to_do() {
        let folder = tempfile::tempdir().expect("a folder");
        let missing = folder.path().join("missing.dll");
        let bare = PathBuf::from("no-such-sdk-library");
        let places = vec![missing.clone(), bare.clone()];
        let Err(error) = search(places.clone()).load(|_| None) else {
            panic!("loaded");
        };
        assert_eq!(
            error,
            SdkError::NotInstalled {
                sdk: "an SDK",
                tried: places,
                variable: "AN_SDK_LIBRARY",
                install: "its maker"
            }
        );
        let message = error.to_string();
        assert!(
            message.contains(&missing.display().to_string()),
            "{message}"
        );
        assert!(message.contains("Install it from its maker"), "{message}");
        assert!(message.contains("set AN_SDK_LIBRARY"), "{message}");
    }

    #[test]
    fn the_variable_replaces_the_search() {
        let folder = tempfile::tempdir().expect("a folder");
        let given = folder.path().join("given.dll");
        let var = |name: &str| (name == "AN_SDK_LIBRARY").then(|| given.clone().into());
        let Err(SdkError::NotInstalled { tried, .. }) = search(vec![]).load(var) else {
            panic!("loaded");
        };
        assert_eq!(tried, [given]);
    }

    #[test]
    fn a_file_that_isnt_a_library_is_unusable() {
        let folder = tempfile::tempdir().expect("a folder");
        let path = folder.path().join("fake.dll");
        std::fs::write(&path, "not a library").expect("written");
        let Err(error) = search(vec![path.clone()]).load(|_| None) else {
            panic!("loaded");
        };
        assert!(
            matches!(&error, SdkError::Unusable { path: at, .. } if *at == path),
            "{error}"
        );
    }

    #[test]
    fn a_failed_call_gives_its_code() {
        assert_eq!(check("pylon", "PylonInitialize", 0), Ok(()));
        let error = check("pylon", "PylonInitialize", 0xC200_0001_u32 as i32);
        assert_eq!(
            error.expect_err("failed").to_string(),
            "pylon's PylonInitialize failed with error -1040187391 (0xc2000001)"
        );
    }
}

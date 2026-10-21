//! Basler's pylon, through its C API (PylonC).
//!
//! pylon's installer puts the library in `pylon\Runtime\x64` on Windows and
//! `/opt/pylon/lib` on Linux. `MICROSCOPE_PYLON_LIBRARY` names it elsewhere.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::{SdkError, Search, check, folder, one_at_a_time};

const SDK: &str = "pylon";

/// pylon's C library, loaded.
pub struct Pylon {
    initialize: unsafe extern "system" fn() -> i32,
    terminate: unsafe extern "system" fn(shut_down_logging: bool) -> i32,
    enumerate_devices: unsafe extern "system" fn(count: *mut usize) -> i32,
    path: PathBuf,
    // Kept loaded while its functions are.
    _library: libloading::Library,
}

impl Pylon {
    /// Finds and loads pylon's C library.
    pub fn load() -> Result<Pylon, SdkError> {
        Pylon::load_from(|name| std::env::var_os(name))
    }

    fn load_from(var: impl Fn(&str) -> Option<OsString>) -> Result<Pylon, SdkError> {
        let loaded = search(&var).load(var)?;
        // SAFETY: here and below, the type is the function's in `PylonC.h`,
        // with GENAPIC_RESULT an HRESULT (32 bits), size_t a usize and _Bool
        // a bool. `extern "system"` is its PYLONC_CC: `__stdcall` on
        // Windows, C elsewhere.
        let initialize = unsafe { loaded.function("PylonInitialize") }?;
        // SAFETY: as above.
        let terminate = unsafe { loaded.function("PylonTerminate") }?;
        // SAFETY: as above.
        let enumerate_devices = unsafe { loaded.function("PylonEnumerateDevices") }?;
        Ok(Pylon {
            initialize,
            terminate,
            enumerate_devices,
            path: loaded.path,
            _library: loaded.library,
        })
    }

    /// The library loaded.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// How many cameras pylon finds.
    pub fn count_cameras(&self) -> Result<usize, SdkError> {
        let _starting = one_at_a_time();
        // SAFETY: it takes no arguments, and the PylonTerminate below
        // matches it, as pylon counts them.
        check(SDK, "PylonInitialize", unsafe { (self.initialize)() })?;
        let mut count = 0;
        // SAFETY: pylon is initialised, and `count` is a size_t to write.
        let code = unsafe { (self.enumerate_devices)(&mut count) };
        // SAFETY: it matches the PylonInitialize above; logging stays up,
        // for the next initialisation.
        let _ = unsafe { (self.terminate)(false) };
        check(SDK, "PylonEnumerateDevices", code).map(|()| count)
    }
}

/// Where pylon's installer puts its C library, then its name for the
/// system's search.
fn search(var: &impl Fn(&str) -> Option<OsString>) -> Search {
    let mut places = Vec::new();
    if cfg!(windows) {
        let name = "PylonC_v10.dll";
        let runtime = |pylon: &Path| pylon.join("Runtime").join("x64").join(name);
        // PYLON_DEV_DIR is the `Development` folder in pylon's.
        let development = folder(var, "PYLON_DEV_DIR");
        if let Some(pylon) = development.as_deref().and_then(Path::parent) {
            places.push(runtime(pylon));
        }
        if let Some(programs) = folder(var, "ProgramFiles") {
            places.push(runtime(&programs.join("Basler").join("pylon")));
        }
        places.push(PathBuf::from(name));
    } else {
        let name = "libpylonc.so";
        if let Some(root) = folder(var, "PYLON_ROOT") {
            places.push(root.join("lib").join(name));
        }
        places.push(PathBuf::from("/opt/pylon/lib").join(name));
        places.push(PathBuf::from(name));
    }
    Search {
        sdk: SDK,
        variable: "MICROSCOPE_PYLON_LIBRARY",
        install: "https://www.baslerweb.com/pylon",
        places,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pylon_is_looked_for_where_its_installer_puts_it() {
        let var = |name: &str| match name {
            "PYLON_DEV_DIR" => Some(OsString::from("D:/pylon/Development/")),
            "PYLON_ROOT" => Some(OsString::from("/home/me/pylon")),
            _ => None,
        };
        let places = search(&var).places;
        if cfg!(windows) {
            let runtime = Path::new("D:/pylon/Runtime/x64");
            assert_eq!(places[0], runtime.join("PylonC_v10.dll"));
        } else {
            assert_eq!(places[0], Path::new("/home/me/pylon/lib/libpylonc.so"));
            assert_eq!(places[1], Path::new("/opt/pylon/lib/libpylonc.so"));
        }
        assert!(places.last().expect("a name").parent() == Some(Path::new("")));
    }

    #[test]
    fn without_pylon_the_error_says_how_to_get_it() {
        let folder = tempfile::tempdir().expect("a folder");
        let missing = folder.path().join("PylonC_v10.dll");
        let var = |name: &str| (name == "MICROSCOPE_PYLON_LIBRARY").then(|| missing.clone().into());
        let message = Pylon::load_from(var).err().expect("not loaded").to_string();
        assert!(message.starts_with("pylon isn't installed"), "{message}");
        assert!(
            message.contains("https://www.baslerweb.com/pylon"),
            "{message}"
        );
    }

    /// Where pylon is installed, such as on the owner's PC; skipped where
    /// it isn't, such as in CI.
    #[test]
    fn an_installed_pylon_counts_its_cameras() {
        let pylon = match Pylon::load() {
            Ok(pylon) => pylon,
            Err(error) => return eprintln!("skipped: {error}"),
        };
        let count = pylon.count_cameras().expect("counted");
        eprintln!("pylon at {} finds {count} cameras", pylon.path().display());
    }
}

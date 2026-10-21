//! Allied Vision's Vimba X, through its C API (VmbC).
//!
//! Vimba X keeps the library in its `api\bin` folder on Windows and
//! `api/lib` on Linux, where `VIMBA_X_HOME` says it's installed.
//! `MICROSCOPE_VIMBA_X_LIBRARY` names it elsewhere.

use std::ffi::{OsString, c_void};
use std::path::{Path, PathBuf};

use crate::{SdkError, Search, check, folder, one_at_a_time};

const SDK: &str = "Vimba X";

/// `VmbVersionInfo_t`.
#[repr(C)]
#[derive(Default)]
struct VersionInfo {
    major: u32,
    minor: u32,
    patch: u32,
}

/// Vimba X's C library, loaded.
pub struct VimbaX {
    version_query: unsafe extern "system" fn(info: *mut VersionInfo, size: u32) -> i32,
    startup: unsafe extern "system" fn(configuration: *const c_void) -> i32,
    shutdown: unsafe extern "system" fn(),
    cameras_list: unsafe extern "system" fn(
        cameras: *mut c_void,
        length: u32,
        found: *mut u32,
        size: u32,
    ) -> i32,
    path: PathBuf,
    // Kept loaded while its functions are.
    _library: libloading::Library,
}

impl VimbaX {
    /// Finds and loads Vimba X's C library.
    pub fn load() -> Result<VimbaX, SdkError> {
        VimbaX::load_from(|name| std::env::var_os(name))
    }

    fn load_from(var: impl Fn(&str) -> Option<OsString>) -> Result<VimbaX, SdkError> {
        let loaded = search(&var).load(var)?;
        // SAFETY: here and below, the type is the function's in Vimba X's
        // `VmbC.h`, with VmbError_t an int32 and VmbUint32_t a u32, and
        // pointers for its pointers. `extern "system"` is its VMB_CALL:
        // `__stdcall` on Windows, C elsewhere.
        let version_query = unsafe { loaded.function("VmbVersionQuery") }?;
        // SAFETY: as above.
        let startup = unsafe { loaded.function("VmbStartup") }?;
        // SAFETY: as above.
        let shutdown = unsafe { loaded.function("VmbShutdown") }?;
        // SAFETY: as above.
        let cameras_list = unsafe { loaded.function("VmbCamerasList") }?;
        Ok(VimbaX {
            version_query,
            startup,
            shutdown,
            cameras_list,
            path: loaded.path,
            _library: loaded.library,
        })
    }

    /// The library loaded.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Its version, such as `1.2.0`.
    pub fn version(&self) -> Result<String, SdkError> {
        let mut info = VersionInfo::default();
        let size = size_of::<VersionInfo>() as u32;
        // SAFETY: `info` is a VmbVersionInfo_t of `size` bytes to write,
        // and the query needs no startup.
        check(SDK, "VmbVersionQuery", unsafe {
            (self.version_query)(&mut info, size)
        })?;
        Ok(format!("{}.{}.{}", info.major, info.minor, info.patch))
    }

    /// How many cameras Vimba X finds.
    pub fn count_cameras(&self) -> Result<u32, SdkError> {
        let _starting = one_at_a_time();
        // SAFETY: a null configuration path is its default; the
        // VmbShutdown below matches it.
        check(SDK, "VmbStartup", unsafe {
            (self.startup)(std::ptr::null())
        })?;
        let mut count = 0;
        // SAFETY: Vimba X is started; with no list, only `count`, a
        // VmbUint32_t to write, is used.
        let code = unsafe { (self.cameras_list)(std::ptr::null_mut(), 0, &mut count, 0) };
        // SAFETY: it matches the VmbStartup above.
        unsafe { (self.shutdown)() };
        check(SDK, "VmbCamerasList", code).map(|()| count)
    }
}

/// Where Vimba X keeps its C library, then its name for the system's
/// search.
fn search(var: &impl Fn(&str) -> Option<OsString>) -> Search {
    let (folder_name, name) = if cfg!(windows) {
        ("bin", "VmbC.dll")
    } else {
        ("lib", "libVmbC.so")
    };
    let mut places = Vec::new();
    if let Some(home) = folder(var, "VIMBA_X_HOME") {
        places.push(home.join("api").join(folder_name).join(name));
    }
    if cfg!(windows)
        && let Some(programs) = folder(var, "ProgramFiles")
    {
        let home = programs.join("Allied Vision").join("Vimba X");
        places.push(home.join("api").join(folder_name).join(name));
    }
    places.push(PathBuf::from(name));
    Search {
        sdk: SDK,
        variable: "MICROSCOPE_VIMBA_X_LIBRARY",
        install: "https://www.alliedvision.com/en/products/software/vimba-x-sdk/",
        places,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vimba_x_is_looked_for_in_its_home() {
        let var = |name: &str| (name == "VIMBA_X_HOME").then(|| OsString::from("/opt/VimbaX"));
        let places = search(&var).places;
        let api = Path::new("/opt/VimbaX").join("api");
        if cfg!(windows) {
            assert_eq!(places[0], api.join("bin").join("VmbC.dll"));
        } else {
            assert_eq!(places[0], api.join("lib").join("libVmbC.so"));
        }
        assert!(places.last().expect("a name").parent() == Some(Path::new("")));
    }

    #[test]
    fn without_vimba_x_the_error_says_how_to_get_it() {
        let folder = tempfile::tempdir().expect("a folder");
        let missing = folder.path().join("VmbC.dll");
        let var =
            |name: &str| (name == "MICROSCOPE_VIMBA_X_LIBRARY").then(|| missing.clone().into());
        let message = VimbaX::load_from(var)
            .err()
            .expect("not loaded")
            .to_string();
        assert!(message.starts_with("Vimba X isn't installed"), "{message}");
        assert!(message.contains("MICROSCOPE_VIMBA_X_LIBRARY"), "{message}");
    }

    /// Where Vimba X is installed, such as on the owner's PC; skipped where
    /// it isn't, such as in CI.
    #[test]
    fn an_installed_vimba_x_gives_its_version_and_counts_its_cameras() {
        let vimba = match VimbaX::load() {
            Ok(vimba) => vimba,
            Err(error) => return eprintln!("skipped: {error}"),
        };
        let version = vimba.version().expect("a version");
        assert!(version.split('.').count() == 3, "{version}");
        let count = vimba.count_cameras().expect("counted");
        eprintln!("Vimba X {version} finds {count} cameras");
    }
}

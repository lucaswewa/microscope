//! Records where the build came from, for the `system` Thing's
//! `version_data`: the output of `git describe --tags --always`, or
//! `unknown` outside a Git checkout.

use std::path::Path;
use std::process::Command;

fn main() {
    let describe = Command::new("git")
        .args(["describe", "--tags", "--always"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "unknown".to_owned());
    println!("cargo:rustc-env=MICROSCOPE_GIT_DESCRIBE={describe}");

    // Run again when the checked-out commit changes. Cargo re-runs a build
    // script on every build if a listed path doesn't exist, so only list the
    // ones that do.
    let git = Path::new("../../.git");
    let mut watched = false;
    for path in ["HEAD", "refs", "packed-refs"].map(|name| git.join(name)) {
        if path.exists() {
            println!("cargo:rerun-if-changed={}", path.display());
            watched = true;
        }
    }
    if !watched {
        println!("cargo:rerun-if-changed=build.rs");
    }
}

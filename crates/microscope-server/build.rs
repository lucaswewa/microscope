//! Rebuilds the server when the web app's build changes, so that a release
//! build embeds the current one (`src/webapp.rs`). Cargo scans the folder for
//! changes; if it doesn't exist, the script runs, cheaply, on every build.

fn main() {
    println!("cargo:rerun-if-changed=../../web/dist");
}

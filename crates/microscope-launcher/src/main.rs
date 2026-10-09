//! `microscope-launcher`: starts and supervises a local microscope server,
//! and opens the web app.
//!
//! For now it only reports its version. The launcher itself arrives in P45.

fn main() {
    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
}

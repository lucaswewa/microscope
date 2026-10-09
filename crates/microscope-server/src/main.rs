//! `microscope-server`: serves the microscope's Things and the web app over
//! HTTP.
//!
//! For now it only reports its version. The server itself arrives in P02.

fn main() {
    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
}

//! `microscope-server`: serves the microscope's Things and the web app over
//! HTTP. See the library's documentation for the command line.

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    microscope_server::cli::main().await
}

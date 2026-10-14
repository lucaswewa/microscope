//! `microscope-launcher`: starts the microscope server, keeps it running and
//! opens the app. See the library's documentation.

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    microscope_launcher::main().await
}

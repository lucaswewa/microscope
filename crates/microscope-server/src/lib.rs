//! The microscope server.
//!
//! It serves the Things in [`microscope_things::registry`] from a
//! configuration file, with `teta-wot`. Rather than `teta-wot`'s own serve
//! loop, which only serves `teta-wot`'s router, it runs the lifecycle itself
//! ([`lifecycle`], ADR-0006), so that routes of its own ([`routes`]) can sit
//! in front of the Web of Things API.
//!
//! - [`cli`]: the command line and exit codes, as `teta-wot`'s;
//! - [`config`]: reading the configuration file;
//! - [`lifecycle`]: start, serve and shut down;
//! - [`logging`]: the console, the server log and the log files;
//! - [`routes`]: the application's own routes, such as `/api/v1/health`;
//! - [`webapp`]: the web app, at `/`.

pub mod cli;
pub mod config;
pub mod lifecycle;
pub mod logging;
pub mod routes;
pub mod webapp;

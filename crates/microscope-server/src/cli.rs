//! The command line.
//!
//! ```text
//! microscope-server -c config.json [--host 127.0.0.1] [--port 5000] [--fallback] [--debug] [--webapp-dir web/dist]
//! microscope-server -j '{"things": {…}}'
//! microscope-server -c config.json --print-openapi
//! ```
//!
//! The options, messages and exit codes are `teta-wot`'s (and LabThings'),
//! plus `--webapp-dir`, which serves the web app from a folder instead of the
//! build in the binary (ADR-0016), and `--print-openapi`, which writes the
//! configured Things' OpenAPI document instead of serving them (ADR-0021):
//!
//! | Code | When |
//! |---|---|
//! | 0 | The server stopped normally, or `--help` or `--version` was given |
//! | 1 | Anything else: no configuration, a missing file, a Thing that can't be built, a port in use |
//! | 2 | The command line is wrong |
//! | 3 | The configuration is invalid, a Thing failed to start, or the fallback server was served |

use std::ffi::OsString;
use std::future::Future;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use clap::Parser;
use teta_wot::http::axum;
use teta_wot::server::cli::CliArgs;
use teta_wot::server::{FallbackPage, ServeError, ThingServer, fallback_router, shutdown_signal};
use tokio::net::TcpListener;
use tokio::sync::watch;
use tower::ServiceExt;

use crate::config::{self, ConfigError};
use crate::logging::{self, Logs};
use crate::webapp::{self, WebApp};
use crate::{lifecycle, routes};

/// The command-line options: `teta-wot`'s, and where the web app comes from.
#[derive(Debug, Clone, Parser)]
#[command(
    name = "microscope-server",
    version,
    about = "Serve the microscope's Things over HTTP, from a configuration file."
)]
pub struct Args {
    /// The options every `teta-wot` server takes.
    #[command(flatten)]
    pub server: CliArgs,
    /// Serve the web app from this folder, such as `web/dist`, instead of
    /// the build in the binary.
    #[arg(long, value_name = "PATH")]
    pub webapp_dir: Option<PathBuf>,
    /// Write the OpenAPI document of the configured Things to standard
    /// output, without starting them, and exit.
    #[arg(long)]
    pub print_openapi: bool,
}

/// Why serving failed, and so the exit code.
#[derive(Debug)]
enum Failure {
    /// The configuration is invalid (exit code 3).
    Config(String),
    /// A Thing failed to start (exit code 3).
    Startup {
        thing: String,
        error: String,
        things: Vec<String>,
    },
    /// Anything else (exit code 1).
    Other(String),
}

impl From<ConfigError> for Failure {
    fn from(error: ConfigError) -> Self {
        match error {
            ConfigError::Missing(message) => Failure::Other(message),
            ConfigError::Invalid(message) => Failure::Config(message),
        }
    }
}

impl Failure {
    fn message(&self) -> String {
        match self {
            Failure::Config(message) | Failure::Other(message) => message.clone(),
            Failure::Startup { thing, error, .. } => {
                format!("Failed to enter '{thing}' Thing: {error}")
            }
        }
    }
}

/// Parses the process's arguments and serves until Ctrl-C (or the other
/// ways Windows asks a console program to stop), returning the exit code.
pub async fn main() -> ExitCode {
    run(std::env::args_os(), shutdown_signal()).await
}

/// Parses `args` (the first is the program name) and serves until
/// `shutdown` completes, returning the exit code.
pub async fn run<I, T>(args: I, shutdown: impl Future<Output = ()> + Send + 'static) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let (stop, stopped) = watch::channel(false);
    let _shutdown = tokio::spawn(async move {
        shutdown.await;
        let _ = stop.send(true);
    });
    let stop = Stop(stopped);

    let Args {
        server: args,
        webapp_dir,
        print_openapi,
    } = match Args::try_parse_from(args) {
        Ok(args) => args,
        Err(error) => {
            // --help and --version are "errors" with exit code 0.
            let _ = error.print();
            return ExitCode::from(u8::try_from(error.exit_code()).unwrap_or(2));
        }
    };

    // Logging starts once the configuration says where log files go.
    let mut config_text = None;
    let mut logs = None;
    let source = webapp_dir.map_or(WebApp::Embedded, WebApp::Dir);
    let outcome = if print_openapi {
        write_openapi(&args).await
    } else {
        serve(&args, source, &mut config_text, &mut logs, &stop).await
    };
    let Err(failure) = outcome else {
        return ExitCode::SUCCESS;
    };

    if args.fallback && !print_openapi {
        println!("Error: {}", failure.message());
        println!("Starting fallback server.");
        let page = FallbackPage {
            error_message: failure.message(),
            things: match &failure {
                Failure::Startup { things, .. } => things.clone(),
                _ => Vec::new(),
            },
            config: config_text.unwrap_or_default(),
            traceback: match &failure {
                Failure::Startup { error, .. } => error.clone(),
                other => other.message(),
            },
            logging: logs
                .as_ref()
                .map(Logs::text)
                .filter(|text| !text.is_empty()),
        };
        if let Err(error) = serve_fallback(&args, page, &stop).await {
            eprintln!("Error: the fallback server couldn't start: {error}");
        }
        return ExitCode::from(3);
    }
    match failure {
        Failure::Config(message) => {
            println!("Error reading configuration:\n{message}");
            ExitCode::from(3)
        }
        startup @ Failure::Startup { .. } => {
            eprintln!("Error: {}", startup.message());
            ExitCode::from(3)
        }
        Failure::Other(message) => {
            eprintln!("Error: {message}");
            ExitCode::from(1)
        }
    }
}

async fn serve(
    args: &CliArgs,
    webapp: WebApp,
    config_text: &mut Option<String>,
    logs: &mut Option<Logs>,
    stop: &Stop,
) -> Result<(), Failure> {
    let text = config::text(args).map_err(|e| Failure::Other(e.message().to_owned()))?;
    *config_text = Some(text.clone());
    let config = config::parse(&text)?;
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
        *config_text = serde_json::to_string_pretty(&value).ok();
    }
    let server_log = logs.insert(logging::init(args.debug, &config.app.log_folder));
    if config.server.mdns {
        tracing::warn!(
            "`mdns` isn't supported by microscope-server yet, and is ignored (ADR-0006)"
        );
    }
    tracing::info!(
        data_folder = %config.app.data_folder.display(),
        log_folder = %config.app.log_folder.display(),
        "application configuration"
    );

    let server = ThingServer::from_config(&config.server, &microscope_things::registry())
        .map_err(|e| Failure::Config(e.to_string()))?
        .build()
        .map_err(|e| Failure::Other(e.to_string()))?;
    let things = server
        .runtime()
        .things()
        .map(|t| t.name().to_owned())
        .collect();
    server_log.attach(Arc::clone(server.runtime().invocations()));

    let listener = bind(args).await.map_err(Failure::Other)?;
    match listener.local_addr() {
        Ok(addr) => println!("listening on http://{addr}"),
        Err(_) => println!("listening"),
    }
    let app_routes = routes::app_routes(&config.server.api_prefix, server_log.clone())
        .merge(webapp::webapp_routes(webapp));
    match lifecycle::serve(server, app_routes, listener, stop.wait()).await {
        Ok(()) => Ok(()),
        Err(ServeError::Startup(failure)) => Err(Failure::Startup {
            thing: failure.thing,
            error: format!("{:#}", failure.error),
            things,
        }),
        Err(other) => Err(Failure::Other(other.to_string())),
    }
}

/// Writes the OpenAPI document of the configured Things to standard output,
/// as the server would serve it, without starting them or listening.
async fn write_openapi(args: &CliArgs) -> Result<(), Failure> {
    let text = config::text(args).map_err(|e| Failure::Other(e.message().to_owned()))?;
    let config = config::parse(&text)?;
    let server = ThingServer::from_config(&config.server, &microscope_things::registry())
        .map_err(|e| Failure::Config(e.to_string()))?
        .build()
        .map_err(|e| Failure::Other(e.to_string()))?;
    // Served at the root, as in FastAPI, whatever the API prefix.
    let request = axum::http::Request::get("/openapi.json")
        .body(axum::body::Body::empty())
        .map_err(|e| Failure::Other(e.to_string()))?;
    let Ok(response) = server.router().oneshot(request).await;
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .map_err(|e| Failure::Other(e.to_string()))?;
    let document: serde_json::Value =
        serde_json::from_slice(&body).map_err(|e| Failure::Other(e.to_string()))?;
    println!("{document:#}");
    Ok(())
}

async fn bind(args: &CliArgs) -> Result<TcpListener, String> {
    let address = format!("{}:{}", args.host, args.port);
    let address: SocketAddr = match address.parse() {
        Ok(address) => address,
        Err(_) => tokio::net::lookup_host(&address)
            .await
            .map_err(|e| format!("can't resolve {address}: {e}"))?
            .next()
            .ok_or_else(|| format!("can't resolve {address}"))?,
    };
    TcpListener::bind(address)
        .await
        .map_err(|e| format!("can't listen on {address}: {e}"))
}

async fn serve_fallback(args: &CliArgs, page: FallbackPage, stop: &Stop) -> Result<(), String> {
    let listener = bind(args).await?;
    if let Ok(addr) = listener.local_addr() {
        println!("listening on http://{addr}");
    }
    axum::serve(listener, fallback_router(page))
        .with_graceful_shutdown(stop.wait())
        .await
        .map_err(|e| e.to_string())
}

/// The shutdown request, shared by the server and the fallback server.
struct Stop(watch::Receiver<bool>);

impl Stop {
    fn wait(&self) -> impl Future<Output = ()> + Send + 'static {
        let mut stopped = self.0.clone();
        async move {
            let _ = stopped.wait_for(|stop| *stop).await;
        }
    }
}

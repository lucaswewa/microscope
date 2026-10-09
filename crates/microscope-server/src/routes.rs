//! The application's own routes, served in front of `teta-wot`'s.
//!
//! They sit under the same API prefix as the Things (`/api/v1`) and allow
//! cross-origin requests as `teta-wot`'s routes do, so a web app served from
//! elsewhere can use them:
//!
//! | Route | What |
//! |---|---|
//! | `GET {prefix}/health` | The server's status, version and uptime |
//! | `GET {prefix}/log/` | The server log as JSON, oldest first; `?level=WARNING` keeps that level and above |
//! | `GET {prefix}/logfile/` | Today's log file, as a text download |
//!
//! Errors answer `{"detail": …}`, as `teta-wot`'s do.

use std::sync::Arc;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::json;
use teta_wot::http::axum::extract::{Query, State};
use teta_wot::http::axum::http::{StatusCode, header};
use teta_wot::http::axum::response::{IntoResponse, Response};
use teta_wot::http::axum::routing::get;
use teta_wot::http::axum::{Json, Router};
use tower_http::cors::CorsLayer;

use crate::logging::{self, Logs};

/// What `GET {prefix}/health` answers.
#[derive(Debug, Clone, Serialize)]
pub struct Health {
    /// Always `"ok"`: the route is only served once every Thing has started.
    pub status: &'static str,
    /// The server's version number.
    pub version: &'static str,
    /// Seconds since the routes were built, which is when serving began.
    pub uptime_seconds: f64,
}

struct AppState {
    started: Instant,
    logs: Logs,
}

/// The application's routes, under `api_prefix` (such as `/api/v1`), with
/// `logs` behind the log routes.
pub fn app_routes(api_prefix: &str, logs: Logs) -> Router {
    let state = Arc::new(AppState {
        started: Instant::now(),
        logs,
    });
    Router::new()
        .route(&format!("{api_prefix}/health"), get(health))
        .route(&format!("{api_prefix}/log"), get(log))
        .route(&format!("{api_prefix}/log/"), get(log))
        .route(&format!("{api_prefix}/logfile"), get(log_file))
        .route(&format!("{api_prefix}/logfile/"), get(log_file))
        .with_state(state)
        .layer(cors())
}

async fn health(State(state): State<Arc<AppState>>) -> Json<Health> {
    Json(Health {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        uptime_seconds: state.started.elapsed().as_secs_f64(),
    })
}

#[derive(Debug, Deserialize)]
struct LogQuery {
    level: Option<String>,
}

async fn log(State(state): State<Arc<AppState>>, Query(query): Query<LogQuery>) -> Response {
    let min_levelno = match query.level.as_deref() {
        None => 0,
        Some(level) => match logging::parse_level(level) {
            Some(levelno) => levelno,
            None => {
                return detail(
                    StatusCode::BAD_REQUEST,
                    format!(
                        "Unknown level '{level}': use DEBUG, INFO, WARNING, ERROR, CRITICAL or a number."
                    ),
                );
            }
        },
    };
    Json(state.logs.records(min_levelno)).into_response()
}

async fn log_file(State(state): State<Arc<AppState>>) -> Response {
    let Some(path) = state.logs.current_file() else {
        return detail(
            StatusCode::NOT_FOUND,
            "No log file has been written.".to_owned(),
        );
    };
    match tokio::fs::read(&path).await {
        Ok(contents) => {
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            (
                [
                    (header::CONTENT_TYPE, "text/plain; charset=utf-8".to_owned()),
                    (
                        header::CONTENT_DISPOSITION,
                        format!("attachment; filename=\"{name}\""),
                    ),
                ],
                contents,
            )
                .into_response()
        }
        Err(error) => detail(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("The log file can't be read: {error}"),
        ),
    }
}

fn detail(status: StatusCode, message: String) -> Response {
    (status, Json(json!({ "detail": message }))).into_response()
}

/// The same policy as `teta-wot`'s routes: every origin, method and header,
/// with credentials, including Private Network Access preflights.
fn cors() -> CorsLayer {
    CorsLayer::very_permissive().allow_private_network(true)
}

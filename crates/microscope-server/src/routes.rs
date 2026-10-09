//! The application's own routes, served in front of `teta-wot`'s.
//!
//! They sit under the same API prefix as the Things (`/api/v1`) and allow
//! cross-origin requests as `teta-wot`'s routes do, so a web app served from
//! elsewhere can use them.

use std::sync::Arc;
use std::time::Instant;

use serde::Serialize;
use teta_wot::http::axum::extract::State;
use teta_wot::http::axum::routing::get;
use teta_wot::http::axum::{Json, Router};
use tower_http::cors::CorsLayer;

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
}

/// The application's routes, under `api_prefix` (such as `/api/v1`).
pub fn app_routes(api_prefix: &str) -> Router {
    let state = Arc::new(AppState {
        started: Instant::now(),
    });
    Router::new()
        .route(&format!("{api_prefix}/health"), get(health))
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

/// The same policy as `teta-wot`'s routes: every origin, method and header,
/// with credentials, including Private Network Access preflights.
fn cors() -> CorsLayer {
    CorsLayer::very_permissive().allow_private_network(true)
}

//! The web app, served by the same server as the API (ADR-0016).
//!
//! | Route | What | Cached |
//! |---|---|---|
//! | `GET /`, `GET /index.html` | The app's page, or a placeholder when there's no build | never (`no-store`) |
//! | `GET /assets/{*path}` | Its scripts, styles and fonts, whose names carry a content hash | for a year (`immutable`) |
//! | `GET /favicon.svg` | Its icon | revalidated (`no-cache`) |
//!
//! The files come from `web/dist`: embedded in release builds, read from
//! disk in debug builds, or read from the folder given with `--webapp-dir`.
//! Build the web app before the server (`npm run build` in `web/`, then
//! `cargo build --release`) so a release build has it.

use std::borrow::Cow;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use rust_embed::RustEmbed;
use serde_json::json;
use teta_wot::http::axum::extract::{Path as PathParam, State};
use teta_wot::http::axum::http::{HeaderValue, StatusCode, header};
use teta_wot::http::axum::response::{Html, IntoResponse, Response};
use teta_wot::http::axum::routing::get;
use teta_wot::http::axum::{Json, Router};

/// `web/dist`, the web app's production build: embedded in release builds,
/// read from disk in debug builds. It may be missing, when the web app
/// hasn't been built.
#[derive(RustEmbed)]
#[folder = "../../web/dist"]
#[allow_missing = true]
struct Embedded;

/// Where the web app's files come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebApp {
    /// The build in the binary (`web/dist` when it was compiled).
    Embedded,
    /// A build on disk, such as `web/dist` (`--webapp-dir`).
    Dir(PathBuf),
}

impl WebApp {
    /// The contents of the file at `path`, relative to the build's root.
    /// Paths that could leave the build's folder are refused.
    async fn file(&self, path: &str) -> Option<Cow<'static, [u8]>> {
        if !Path::new(path)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
        {
            return None;
        }
        match self {
            WebApp::Embedded => Embedded::get(path).map(|file| file.data),
            WebApp::Dir(dir) => tokio::fs::read(dir.join(path)).await.ok().map(Cow::Owned),
        }
    }
}

const NEVER: &str = "no-store";
const REVALIDATE: &str = "no-cache";
const FOREVER: &str = "public, max-age=31536000, immutable";

/// The web app's routes, from `source`.
pub fn webapp_routes(source: WebApp) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/index.html", get(index))
        .route("/assets/{*path}", get(asset))
        .route("/favicon.svg", get(favicon))
        .with_state(Arc::new(source))
}

async fn index(State(source): State<Arc<WebApp>>) -> Response {
    match source.file("index.html").await {
        Some(page) => file_response(page, "text/html; charset=utf-8", NEVER),
        None => with_cache(Html(PLACEHOLDER).into_response(), NEVER),
    }
}

async fn asset(State(source): State<Arc<WebApp>>, PathParam(path): PathParam<String>) -> Response {
    match source.file(&format!("assets/{path}")).await {
        Some(data) => file_response(data, content_type(&path), FOREVER),
        None => not_found(),
    }
}

async fn favicon(State(source): State<Arc<WebApp>>) -> Response {
    match source.file("favicon.svg").await {
        Some(data) => file_response(data, "image/svg+xml", REVALIDATE),
        None => not_found(),
    }
}

fn file_response(
    data: Cow<'static, [u8]>,
    content_type: &'static str,
    cache: &'static str,
) -> Response {
    let response = ([(header::CONTENT_TYPE, content_type)], data.into_owned()).into_response();
    with_cache(response, cache)
}

fn with_cache(mut response: Response, cache: &'static str) -> Response {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static(cache));
    response
}

/// As `teta-wot` answers a path it doesn't know.
fn not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(json!({ "detail": "Not Found" })),
    )
        .into_response()
}

/// The media type of a built file, by its extension.
fn content_type(path: &str) -> &'static str {
    match Path::new(path).extension().and_then(|ext| ext.to_str()) {
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("woff2") => "font/woff2",
        Some("json" | "map") => "application/json",
        _ => "application/octet-stream",
    }
}

/// The page at `/` when the server has no build of the web app.
const PLACEHOLDER: &str = r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Microscope</title>
<style>
  body { margin: 0; font-family: 'Segoe UI', system-ui, sans-serif; line-height: 1.5; color: #1a1a1a; }
  main { max-width: 40rem; margin: 4rem auto; padding: 0 1.5rem; }
  h1 { font-weight: 300; color: #b41d73; }
  code { font-family: 'Cascadia Mono', Consolas, monospace; }
</style>
</head>
<body>
<main>
<h1>The microscope server is running</h1>
<p>It has no web app to show, because the web app wasn't built when this server was.</p>
<ul>
<li>Build the web app (<code>npm run build</code> in <code>web/</code>), then the server.</li>
<li>Or start the server with <code>--webapp-dir web/dist</code>.</li>
</ul>
<p>The API is at <a href="/api/v1/thing_descriptions/">/api/v1/</a>, and its documentation at <a href="/docs">/docs</a>.</p>
</main>
</body>
</html>
"#;

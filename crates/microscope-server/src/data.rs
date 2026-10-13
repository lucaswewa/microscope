//! The data route (ADR-0026): `GET {prefix}/data/{*path}` serves a file
//! from the data folder, such as a capture or its thumbnail, with a content
//! type for its extension. A path that would leave the folder, through `..`,
//! a root, a drive or a link, is answered as if there were no such file.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use teta_wot::http::axum::Router;
use teta_wot::http::axum::extract::{self, State};
use teta_wot::http::axum::http::{StatusCode, header};
use teta_wot::http::axum::response::{IntoResponse, Response};
use teta_wot::http::axum::routing::get;

use crate::routes::{cors, detail};

/// The data route, under `api_prefix`, serving `folder`.
pub fn data_routes(api_prefix: &str, folder: PathBuf) -> Router {
    Router::new()
        .route(&format!("{api_prefix}/data/{{*path}}"), get(file))
        .with_state(Arc::new(folder))
        .layer(cors())
}

async fn file(
    State(folder): State<Arc<PathBuf>>,
    extract::Path(path): extract::Path<String>,
) -> Response {
    let target = microscope_things::data::resolve(&folder, &path).ok();
    // Following links mustn't lead outside the folder either.
    let inside = target.as_deref().and_then(|target| {
        let real = std::fs::canonicalize(target).ok()?;
        (real.is_file() && real.starts_with(std::fs::canonicalize(&*folder).ok()?)).then_some(real)
    });
    let Some(real) = inside else {
        return no_file(&path);
    };
    match tokio::fs::read(&real).await {
        Ok(bytes) => {
            let headers = [
                (header::CONTENT_TYPE, content_type(&real)),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            ];
            (headers, bytes).into_response()
        }
        Err(_) => no_file(&path),
    }
}

fn no_file(path: &str) -> Response {
    detail(
        StatusCode::NOT_FOUND,
        format!("There's no file `{path}` in the data folder."),
    )
}

/// The content type of the files the microscope saves, by extension.
fn content_type(path: &Path) -> &'static str {
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match extension.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "tif" | "tiff" => "image/tiff",
        "json" => "application/json",
        "dzi" | "xml" => "application/xml",
        "csv" => "text/csv; charset=utf-8",
        "txt" | "log" => "text/plain; charset=utf-8",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
}

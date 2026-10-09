//! The web app's routes (ADR-0016), from a build on disk: each file with its
//! type and caching, the placeholder without a build, and refusing paths that
//! leave the build's folder. Also `--webapp-dir`, through the binary.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Command, Stdio};

use http_body_util::BodyExt;
use microscope_server::logging::Logs;
use microscope_server::webapp::{WebApp, webapp_routes};
use microscope_server::{lifecycle, routes};
use microscope_things::system::MicroscopeSystem;
use teta_wot::http::axum::Router;
use teta_wot::http::axum::body::Body;
use teta_wot::http::axum::http::{HeaderMap, Request, StatusCode, header};
use teta_wot::server::ThingServer;
use tower::ServiceExt;

/// A build in `<folder>/dist`, with a secret beside it that must stay unreachable.
fn build(folder: &Path) -> std::path::PathBuf {
    let dist = folder.join("dist");
    std::fs::create_dir_all(dist.join("assets")).expect("folders");
    std::fs::write(dist.join("index.html"), "<!doctype html><title>App</title>").expect("index");
    std::fs::write(dist.join("assets/index-abc123.js"), "console.log(1)").expect("script");
    std::fs::write(dist.join("assets/index-abc123.css"), "body{}").expect("styles");
    std::fs::write(dist.join("favicon.svg"), "<svg/>").expect("icon");
    std::fs::write(folder.join("secret.txt"), "secret").expect("secret");
    dist
}

async fn get(router: &Router, path: &str) -> (StatusCode, HeaderMap, String) {
    let request = Request::get(path).body(Body::empty()).expect("a request");
    let response = router
        .clone()
        .oneshot(request)
        .await
        .expect("routers don't fail");
    let (parts, body) = response.into_parts();
    let body = body.collect().await.expect("a body").to_bytes();
    (
        parts.status,
        parts.headers,
        String::from_utf8_lossy(&body).into_owned(),
    )
}

fn header(headers: &HeaderMap, name: header::HeaderName) -> &str {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
}

#[tokio::test]
async fn serves_the_page_uncached_and_the_assets_cached_for_good() {
    let folder = tempfile::tempdir().expect("a temporary folder");
    let router = webapp_routes(WebApp::Dir(build(folder.path())));

    for path in ["/", "/index.html"] {
        let (status, headers, body) = get(&router, path).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert_eq!(
            header(&headers, header::CONTENT_TYPE),
            "text/html; charset=utf-8"
        );
        assert_eq!(header(&headers, header::CACHE_CONTROL), "no-store");
        assert!(body.contains("<title>App</title>"), "{body}");
    }

    for (path, content_type) in [
        ("/assets/index-abc123.js", "text/javascript; charset=utf-8"),
        ("/assets/index-abc123.css", "text/css; charset=utf-8"),
    ] {
        let (status, headers, _) = get(&router, path).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert_eq!(header(&headers, header::CONTENT_TYPE), content_type);
        assert_eq!(
            header(&headers, header::CACHE_CONTROL),
            "public, max-age=31536000, immutable"
        );
    }

    let (status, headers, _) = get(&router, "/favicon.svg").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(header(&headers, header::CONTENT_TYPE), "image/svg+xml");
    assert_eq!(header(&headers, header::CACHE_CONTROL), "no-cache");
}

#[tokio::test]
async fn answers_a_missing_asset_as_teta_wot_answers_a_missing_path() {
    let folder = tempfile::tempdir().expect("a temporary folder");
    let router = webapp_routes(WebApp::Dir(build(folder.path())));
    let (status, _, body) = get(&router, "/assets/nothing.js").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, r#"{"detail":"Not Found"}"#);
}

#[tokio::test]
async fn refuses_paths_that_leave_the_build() {
    let folder = tempfile::tempdir().expect("a temporary folder");
    let router = webapp_routes(WebApp::Dir(build(folder.path())));
    for path in [
        "/assets/..%2F..%2Fsecret.txt",
        "/assets/%2E%2E%2F%2E%2E%2Fsecret.txt",
        "/assets/..%5C..%5Csecret.txt",
    ] {
        let (status, _, body) = get(&router, path).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
        assert!(!body.contains("secret"), "{path}: {body}");
    }
}

#[tokio::test]
async fn shows_a_placeholder_without_a_build() {
    let folder = tempfile::tempdir().expect("a temporary folder");
    let router = webapp_routes(WebApp::Dir(folder.path().join("no-build")));
    let (status, headers, body) = get(&router, "/").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(header(&headers, header::CACHE_CONTROL), "no-store");
    assert!(body.contains("The microscope server is running"), "{body}");
    assert!(body.contains("--webapp-dir"), "{body}");
}

#[tokio::test]
async fn sits_in_front_of_the_api() {
    let folder = tempfile::tempdir().expect("a temporary folder");
    let server = ThingServer::builder()
        .thing("system", MicroscopeSystem::default())
        .api_prefix("/api/v1")
        .build()
        .expect("the server builds");
    server.runtime().start().await.expect("the Things start");
    let app = routes::app_routes("/api/v1", Logs::new(None))
        .merge(webapp_routes(WebApp::Dir(build(folder.path()))));
    let router = lifecycle::compose(app, &server);

    let (status, _, body) = get(&router, "/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<title>App</title>"));
    let (status, _, body) = get(&router, "/api/v1/thing_descriptions/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("MicroscopeSystem"), "{body}");
    server.runtime().stop().await;
}

#[test]
fn webapp_dir_serves_the_app_from_a_folder() {
    let folder = tempfile::tempdir().expect("a temporary folder");
    let dist = build(folder.path());
    let mut child = Command::new(env!("CARGO_BIN_EXE_microscope-server"))
        .current_dir(folder.path())
        .args(["--port", "0", "-j", r#"{"things": {}}"#, "--webapp-dir"])
        .arg(&dist)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("the server binary runs");
    let stdout = child.stdout.take().expect("stdout is piped");
    let address = BufReader::new(stdout)
        .lines()
        .map_while(Result::ok)
        .find_map(|line| line.strip_prefix("listening on http://").map(str::to_owned));
    let page = address.map(|address| {
        let mut stream = TcpStream::connect(&address).expect("connects");
        stream
            .write_all(b"GET / HTTP/1.1\r\nHost: test\r\nConnection: close\r\n\r\n")
            .expect("writes");
        let mut page = String::new();
        stream.read_to_string(&mut page).expect("reads");
        page
    });
    let _ = child.kill();
    let _ = child.wait();
    let page = page.expect("the server printed its address");
    assert!(page.contains("<title>App</title>"), "{page}");
}

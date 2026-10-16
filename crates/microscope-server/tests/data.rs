//! The data route (ADR-0026): the data folder's files, with their content
//! types, and nothing outside the folder, however the path is written.

use std::path::Path;

use http_body_util::BodyExt;
use microscope_server::data::data_routes;
use teta_wot::http::axum::Router;
use teta_wot::http::axum::body::Body;
use teta_wot::http::axum::http::{Request, StatusCode, header};
use tower::ServiceExt;

/// What `GET /api/v1/data/{path}` answers: the status, the content type and
/// the body.
async fn get(router: &Router, path: &str) -> (StatusCode, String, Vec<u8>) {
    let request = Request::get(format!("/api/v1/data/{path}"))
        .body(Body::empty())
        .expect("a request");
    let response = router.clone().oneshot(request).await.expect("no failure");
    let status = response.status();
    let content_type = response.headers().get(header::CONTENT_TYPE);
    let content_type = content_type.map(|t| t.to_str().expect("text").to_owned());
    let body = response.into_body().collect().await.expect("a body");
    (
        status,
        content_type.unwrap_or_default(),
        body.to_bytes().to_vec(),
    )
}

fn write(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().expect("a folder")).expect("created");
    std::fs::write(path, contents).expect("written");
}

#[tokio::test]
async fn serves_the_data_folders_files_with_their_types() {
    let root = tempfile::tempdir().expect("a folder");
    let data = root.path().join("data");
    write(&data.join("captures/a b.jpeg"), "jpeg");
    write(&data.join("captures/a b.json"), "{}");
    write(&data.join("scans/s/scan.log"), "log");
    write(&data.join("other.bin"), "bin");
    let router = data_routes("/api/v1", data);

    for (path, content_type, body) in [
        ("captures/a%20b.jpeg", "image/jpeg", "jpeg"),
        ("captures/a%20b.json", "application/json", "{}"),
        ("scans/s/scan.log", "text/plain; charset=utf-8", "log"),
        ("other.bin", "application/octet-stream", "bin"),
    ] {
        let answer = get(&router, path).await;
        assert_eq!(
            answer,
            (StatusCode::OK, content_type.to_owned(), body.into())
        );
    }
    for missing in ["captures/c.jpeg", "captures", "captures/"] {
        let (status, _, _) = get(&router, missing).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{missing}");
    }
}

#[tokio::test]
async fn nothing_outside_the_data_folder_is_served() {
    let root = tempfile::tempdir().expect("a folder");
    let data = root.path().join("data");
    write(&data.join("inside.txt"), "inside");
    write(&root.path().join("secret.txt"), "secret");
    // A link inside the folder to the folder above it: a junction on
    // Windows, which needs no privilege, and a symbolic link elsewhere.
    #[cfg(windows)]
    {
        let junction = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(data.join("up"))
            .arg(root.path())
            .output()
            .expect("mklink runs");
        assert!(junction.status.success(), "{junction:?}");
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(root.path(), data.join("up")).expect("linked");
    let router = data_routes("/api/v1", data);

    let absolute = root.path().join("secret.txt");
    let absolute = absolute.to_str().expect("text").replace('\\', "/");
    for path in [
        "../secret.txt",
        "%2e%2e/secret.txt",
        "..%2fsecret.txt",
        "..%5csecret.txt",
        "inside.txt/../../secret.txt",
        "./../secret.txt",
        "up/secret.txt",
        &absolute,
        &format!("/{absolute}"),
    ] {
        let (status, _, body) = get(&router, path).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
        assert_ne!(body, b"secret", "{path}");
    }
    assert_eq!(get(&router, "inside.txt").await.2, b"inside");
}

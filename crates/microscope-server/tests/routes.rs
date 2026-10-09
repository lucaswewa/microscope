//! The composed router: the application's routes first, and `teta-wot`'s for
//! every request they don't match.

use http_body_util::BodyExt;
use microscope_server::{lifecycle, routes};
use microscope_things::system::MicroscopeSystem;
use serde_json::Value;
use teta_wot::http::axum::Router;
use teta_wot::http::axum::body::Body;
use teta_wot::http::axum::http::{HeaderMap, Method, Request, StatusCode, header};
use teta_wot::server::ThingServer;
use tower::ServiceExt;

const PREFIX: &str = "/api/v1";
const ORIGIN: &str = "http://elsewhere.test";

/// A started server with the `system` Thing, and its composed router.
async fn composed() -> (ThingServer, Router) {
    let server = ThingServer::builder()
        .thing("system", MicroscopeSystem::default())
        .api_prefix(PREFIX)
        .build()
        .expect("the server builds");
    server.runtime().start().await.expect("the Things start");
    let router = lifecycle::compose(routes::app_routes(PREFIX), &server);
    (server, router)
}

struct Answer {
    status: StatusCode,
    headers: HeaderMap,
    body: Vec<u8>,
}

impl Answer {
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).expect("a JSON body")
    }
}

async fn send(router: &Router, request: Request<Body>) -> Answer {
    let response = router
        .clone()
        .oneshot(request)
        .await
        .expect("routers don't fail");
    let status = response.status();
    let headers = response.headers().clone();
    let body = response
        .into_body()
        .collect()
        .await
        .expect("a body")
        .to_bytes()
        .to_vec();
    Answer {
        status,
        headers,
        body,
    }
}

fn get(uri: &str) -> Request<Body> {
    Request::get(uri).body(Body::empty()).expect("a request")
}

fn get_from(uri: &str, origin: &str) -> Request<Body> {
    Request::get(uri)
        .header(header::ORIGIN, origin)
        .body(Body::empty())
        .expect("a request")
}

#[tokio::test]
async fn health_is_served_under_the_api_prefix() {
    let (_server, router) = composed().await;
    let answer = send(&router, get("/api/v1/health")).await;
    assert_eq!(answer.status, StatusCode::OK);
    let health = answer.json();
    assert_eq!(health["status"], "ok");
    assert_eq!(health["version"], env!("CARGO_PKG_VERSION"));
    assert!(health["uptime_seconds"].as_f64().is_some());
}

#[tokio::test]
async fn things_are_served_behind_the_app_routes() {
    let (_server, router) = composed().await;
    let td = send(&router, get("/api/v1/system/")).await;
    assert_eq!(td.status, StatusCode::OK);
    assert_eq!(td.json()["title"], "MicroscopeSystem");

    let version = send(&router, get("/api/v1/system/version_data")).await;
    assert_eq!(version.status, StatusCode::OK);
    assert_eq!(version.json()["version"], env!("CARGO_PKG_VERSION"));
}

#[tokio::test]
async fn unmatched_requests_get_teta_wots_answer() {
    let (server, router) = composed().await;
    for uri in [
        "/api/v1/nothing-here",
        "/elsewhere",
        "/api/v1/system/no_such_property",
    ] {
        let composed = send(&router, get(uri)).await;
        let teta_wot = send(&server.router(), get(uri)).await;
        assert_eq!(composed.status, StatusCode::NOT_FOUND, "{uri}");
        assert_eq!(composed.status, teta_wot.status, "{uri}");
        assert_eq!(composed.body, teta_wot.body, "{uri}");
    }
}

#[tokio::test]
async fn app_routes_allow_cross_origin_requests() {
    let (_server, router) = composed().await;
    let answer = send(&router, get_from("/api/v1/health", ORIGIN)).await;
    assert_eq!(answer.headers[header::ACCESS_CONTROL_ALLOW_ORIGIN], ORIGIN);
    assert_eq!(
        answer.headers[header::ACCESS_CONTROL_ALLOW_CREDENTIALS],
        "true"
    );

    let preflight = Request::builder()
        .method(Method::OPTIONS)
        .uri("/api/v1/health")
        .header(header::ORIGIN, ORIGIN)
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
        .body(Body::empty())
        .expect("a request");
    let answer = send(&router, preflight).await;
    assert!(answer.status.is_success(), "{}", answer.status);
    assert_eq!(answer.headers[header::ACCESS_CONTROL_ALLOW_ORIGIN], ORIGIN);
}

#[tokio::test]
async fn teta_wot_routes_keep_their_own_cors_headers_once() {
    let (_server, router) = composed().await;
    let answer = send(&router, get_from("/api/v1/system/hostname", ORIGIN)).await;
    assert_eq!(answer.status, StatusCode::OK);
    let origins: Vec<_> = answer
        .headers
        .get_all(header::ACCESS_CONTROL_ALLOW_ORIGIN)
        .iter()
        .collect();
    assert_eq!(origins, [ORIGIN]);
}

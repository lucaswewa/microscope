//! The server log end to end: an action's log lines reach its invocation's
//! log, the server log (credited to its Thing and action) and the log file.
//!
//! The test installs the process's global `tracing` subscriber, so it is the
//! only test in this file (each file is its own process).

use std::sync::Arc;

use http_body_util::BodyExt;
use microscope_server::{lifecycle, logging, routes};
use serde_json::Value;
use teta_wot::http::axum::Router;
use teta_wot::http::axum::body::Body;
use teta_wot::http::axum::http::{HeaderMap, Request, StatusCode, header};
use teta_wot::prelude::*;
use tower::ServiceExt;
use uuid::Uuid;

const PREFIX: &str = "/api/v1";

/// A Thing whose action logs.
#[derive(Thing)]
struct Chatty {}

#[thing_impl]
impl Chatty {
    /// Say something, then warn about something.
    #[action]
    async fn speak(&self) {
        tracing::info!("hello from the action");
        tracing::warn!("a warning from the action");
    }
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

    fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
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

#[tokio::test(flavor = "multi_thread")]
async fn action_logs_reach_the_invocation_the_server_log_and_the_file() {
    let folder = tempfile::tempdir().expect("a temporary folder");
    let logs = logging::init(false, folder.path());
    let server = ThingServer::builder()
        .thing("chatty", Chatty {})
        .api_prefix(PREFIX)
        .build()
        .expect("the server builds");
    server.runtime().start().await.expect("the Things start");
    logs.attach(Arc::clone(server.runtime().invocations()));
    let router = lifecycle::compose(routes::app_routes(PREFIX, logs.clone()), &server);

    // Invoke the action, and wait for it.
    let invoked = send(
        &router,
        Request::post("/api/v1/chatty/speak")
            .body(Body::empty())
            .expect("a request"),
    )
    .await;
    assert_eq!(invoked.status, StatusCode::CREATED);
    let id: Uuid = invoked.json()["id"]
        .as_str()
        .and_then(|id| id.parse().ok())
        .expect("the invocation's ID");
    let invocation = server
        .runtime()
        .invocations()
        .get(id)
        .expect("the invocation");
    assert_eq!(invocation.wait().await, InvocationStatus::Completed);

    // The invocation's own log.
    let invocation_log: Vec<String> = invocation.logs().into_iter().map(|r| r.message).collect();
    assert!(
        invocation_log.contains(&"hello from the action".to_owned()),
        "{invocation_log:?}"
    );

    // The server log, credited to the Thing and action.
    let records = send(&router, get("/api/v1/log/")).await.json();
    let record = records
        .as_array()
        .expect("a list")
        .iter()
        .find(|r| r["message"] == "hello from the action")
        .expect("the action's record is in the server log");
    assert_eq!(record["invocation_id"], id.to_string());
    assert_eq!(record["thing"], "chatty");
    assert_eq!(record["action"], "speak");
    assert_eq!(record["levelname"], "INFO");

    // Filtering by level, and a level that doesn't exist.
    let warnings = send(&router, get("/api/v1/log/?level=WARNING"))
        .await
        .json();
    let warnings = warnings.as_array().expect("a list");
    assert!(warnings.iter().all(|r| r["levelno"].as_u64() >= Some(30)));
    assert!(
        warnings
            .iter()
            .any(|r| r["message"] == "a warning from the action")
    );
    let refused = send(&router, get("/api/v1/log/?level=loud")).await;
    assert_eq!(refused.status, StatusCode::BAD_REQUEST);
    assert!(
        refused.json()["detail"]
            .as_str()
            .is_some_and(|d| d.contains("Unknown level"))
    );

    // The log file.
    let file = send(&router, get("/api/v1/logfile/")).await;
    assert_eq!(file.status, StatusCode::OK);
    assert_eq!(
        file.headers[header::CONTENT_TYPE],
        "text/plain; charset=utf-8"
    );
    let disposition = file.headers[header::CONTENT_DISPOSITION]
        .to_str()
        .expect("text");
    assert!(
        disposition.starts_with("attachment; filename=\"microscope."),
        "{disposition}"
    );
    assert!(file.text().contains("hello from the action"));

    server.runtime().stop().await;
}

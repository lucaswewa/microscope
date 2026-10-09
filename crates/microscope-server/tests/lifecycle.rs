//! Starting, serving and shutting down, with test Things.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use http_body_util::BodyExt;
use microscope_server::logging::Logs;
use microscope_server::{lifecycle, routes};
use teta_wot::http::axum::body::Body;
use teta_wot::http::axum::http::{Request, StatusCode};
use teta_wot::prelude::*;
use teta_wot::server::ServeError;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tower::ServiceExt;

const PREFIX: &str = "/api/v1";

/// A Thing with an action that takes a minute unless it's cancelled.
#[derive(Thing)]
struct Slow {
    cancelled: Arc<AtomicBool>,
    stopped: Arc<AtomicBool>,
}

#[thing_impl]
impl Slow {
    /// Wait a minute.
    #[action]
    async fn wait(&self, ctx: ActionCtx) -> Result<(), ActionError> {
        let waited = ctx.sleep(Duration::from_secs(60)).await;
        if waited.is_err() {
            self.cancelled.store(true, Ordering::SeqCst);
        }
        waited?;
        Ok(())
    }

    /// Record that the Thing stopped.
    #[on_stop]
    async fn record_stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
    }
}

/// A Thing that can't start.
#[derive(Thing)]
struct Broken {}

#[thing_impl]
impl Broken {
    /// Fail to connect.
    #[on_start]
    async fn connect(&self) -> anyhow::Result<()> {
        anyhow::bail!("the instrument isn't plugged in")
    }
}

/// Sends `GET path` over the network and returns the answer's first line,
/// once the server is serving (connections wait in the listener's backlog
/// until then).
async fn status_line(addr: std::net::SocketAddr, path: &str) -> String {
    let mut stream = TcpStream::connect(addr).await.expect("connects");
    let request = format!("GET {path} HTTP/1.1\r\nHost: test\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).await.expect("writes");
    let mut answer = String::new();
    stream.read_to_string(&mut answer).await.expect("reads");
    answer.lines().next().unwrap_or_default().to_owned()
}

#[tokio::test(flavor = "multi_thread")]
async fn shutdown_cancels_invocations_and_stops_the_things() {
    let cancelled = Arc::new(AtomicBool::new(false));
    let stopped = Arc::new(AtomicBool::new(false));
    let server = ThingServer::builder()
        .thing(
            "slow",
            Slow {
                cancelled: Arc::clone(&cancelled),
                stopped: Arc::clone(&stopped),
            },
        )
        .api_prefix(PREFIX)
        .build()
        .expect("the server builds");
    // The same runtime, reached in-process, to start an invocation.
    let in_process = lifecycle::compose(routes::app_routes(PREFIX, Logs::new(None)), &server);
    let grace = server.shutdown_grace();

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("binds");
    let addr = listener.local_addr().expect("an address");
    let (stop, stop_requested) = oneshot::channel::<()>();
    let serving = tokio::spawn(lifecycle::serve(
        server,
        routes::app_routes(PREFIX, Logs::new(None)),
        listener,
        async move {
            let _ = stop_requested.await;
        },
    ));

    assert_eq!(status_line(addr, "/api/v1/health").await, "HTTP/1.1 200 OK");
    let invoked = in_process
        .oneshot(
            Request::post("/api/v1/slow/wait")
                .body(Body::empty())
                .expect("a request"),
        )
        .await
        .expect("routers don't fail");
    assert_eq!(invoked.status(), StatusCode::CREATED);
    let _ = invoked.into_body().collect().await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    let asked = Instant::now();
    stop.send(()).expect("the server is waiting");
    let result = serving.await.expect("the serve task doesn't panic");
    let took = asked.elapsed();

    assert!(result.is_ok(), "{result:?}");
    assert!(
        took < grace / 2,
        "shutdown took {took:?}: the invocation wasn't cancelled"
    );
    assert!(
        cancelled.load(Ordering::SeqCst),
        "the action saw the cancellation"
    );
    assert!(stopped.load(Ordering::SeqCst), "the Thing was stopped");
}

#[tokio::test]
async fn a_thing_that_cant_start_is_a_startup_error() {
    let server = ThingServer::builder()
        .thing("broken", Broken {})
        .api_prefix(PREFIX)
        .build()
        .expect("the server builds");
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("binds");
    let result = lifecycle::serve(
        server,
        routes::app_routes(PREFIX, Logs::new(None)),
        listener,
        std::future::pending(),
    )
    .await;
    match result {
        Err(ServeError::Startup(failure)) => {
            assert_eq!(failure.thing, "broken");
            assert!(format!("{:#}", failure.error).contains("isn't plugged in"));
        }
        other => panic!("expected a startup error, got {other:?}"),
    }
}

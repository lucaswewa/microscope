//! Starting, serving and stopping the server.
//!
//! `teta-wot`'s `ThingServer::serve_with` serves only `teta-wot`'s router.
//! [`serve`] runs the same lifecycle with the application's routes in front
//! of that router (ADR-0006):
//!
//! 1. start the Things, each after the Things in its slots;
//! 2. serve until the shutdown signal;
//! 3. stop accepting connections, end observation streams (SSE and
//!    WebSocket) and cancel unfinished invocations;
//! 4. wait up to the grace period for requests and invocations to finish;
//! 5. stop the Things in reverse order.
//!
//! These steps copy `ThingServer::serve_with` at `teta-wot` v0.1.0. Compare
//! them again whenever `teta-wot` is upgraded (ADR-0004).

use std::future::{Future, IntoFuture};
use std::sync::Arc;

use teta_wot::http::axum::{self, Router};
use teta_wot::server::{ServeError, ThingServer};
use tokio::net::TcpListener;
use tokio::sync::watch;

/// The router to serve: `app_routes` first, and `teta-wot`'s router for
/// every request they don't match.
pub fn compose(app_routes: Router, server: &ThingServer) -> Router {
    app_routes.fallback_service(server.router())
}

/// Starts `server`'s Things, serves them with `app_routes` in front on
/// `listener` until `shutdown` completes, then shuts down gracefully.
///
/// Fails with [`ServeError::Startup`] if a Thing can't start (the Things
/// already started are stopped), or [`ServeError::Io`] if serving fails.
pub async fn serve(
    server: ThingServer,
    app_routes: Router,
    listener: TcpListener,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<(), ServeError> {
    let runtime = Arc::clone(server.runtime());
    let grace = server.shutdown_grace();
    let router = compose(app_routes, &server);
    drop(server);

    if let Err(failure) = runtime.start().await {
        tracing::error!(thing = %failure.thing, "{failure}");
        return Err(failure.into());
    }
    if let Ok(addr) = listener.local_addr() {
        tracing::info!(
            "serving {} Things on http://{addr}",
            runtime.things().count()
        );
    }

    let (stop, stopped) = watch::channel(false);
    let serving = axum::serve(listener, router).with_graceful_shutdown(async move {
        let mut stopped = stopped;
        let _ = stopped.wait_for(|stop| *stop).await;
    });
    let mut serving = tokio::spawn(serving.into_future());

    tokio::select! {
        result = &mut serving => {
            // The server stopped by itself, which means an I/O error.
            runtime.shutdown(grace).await;
            return match result {
                Ok(served) => served.map_err(ServeError::Io),
                Err(join) => Err(ServeError::Io(std::io::Error::other(join))),
            };
        }
        () = shutdown => {}
    }

    tracing::info!("shutting down");
    let _ = stop.send(true);
    // End WebSocket and SSE streams, which would otherwise hold their
    // connections open for the whole grace period.
    runtime.broker().close();
    runtime.invocations().cancel_all();
    let drained = tokio::time::timeout(grace, async {
        let _ = (&mut serving).await;
        runtime.invocations().wait_idle().await;
    })
    .await;
    if drained.is_err() {
        tracing::warn!("requests or invocations were still running after {grace:?}");
        serving.abort();
    }
    runtime.stop().await;
    tracing::info!("stopped");
    Ok(())
}

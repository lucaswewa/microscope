//! The `system` Thing: the server and the computer it runs on, and stopping
//! or restarting the server (ADR-0032).

use std::sync::{Arc, OnceLock};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use teta_wot::prelude::*;

/// The server's version, and where the build came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct VersionData {
    /// The server's version number, such as `0.1.0`.
    pub version: String,
    /// Where the build came from: `git describe` when the server was built,
    /// or `unknown`.
    pub version_source: String,
}

impl VersionData {
    /// The version of this build.
    pub fn current() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_owned(),
            version_source: env!("MICROSCOPE_GIT_DESCRIBE").to_owned(),
        }
    }
}

/// How the server is asked to stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    /// Stop, with exit code 0.
    Stop,
    /// Stop so that the supervisor starts it again, with exit code 75.
    Restart,
}

/// What the server gives the `system` Thing, so it can stop the server.
#[derive(Clone)]
pub struct Control {
    /// Whether a supervisor started the server (`--managed`), and so
    /// restarts it on exit code 75.
    pub managed: bool,
    /// Asks the server to stop. The first request wins.
    pub stop: Arc<dyn Fn(Exit) + Send + Sync>,
}

/// The microscope server and the computer it runs on.
#[derive(Thing)]
pub struct MicroscopeSystem {
    /// Set by the server once it's built ([`MicroscopeSystem::attach`]).
    control: OnceLock<Control>,
}

impl MicroscopeSystem {
    /// Lets this Thing stop the server. Later calls are ignored.
    pub fn attach(&self, control: Control) {
        let _ = self.control.set(control);
    }

    fn control(&self) -> Result<&Control, ActionError> {
        (self.control.get())
            .ok_or_else(|| ActionError::handled("this server can't be stopped from here"))
    }
}

#[thing_impl]
impl MicroscopeSystem {
    /// Whether a supervisor, such as the launcher, started the server, and
    /// so can restart it.
    #[property]
    async fn managed(&self) -> bool {
        self.control.get().is_some_and(|control| control.managed)
    }

    /// Stops the server, ending what's running.
    #[action(global_lock = false)]
    async fn shutdown(&self) -> Result<(), ActionError> {
        (self.control()?.stop)(Exit::Stop);
        Ok(())
    }

    /// Restarts the server, ending what's running. Only a server that a
    /// supervisor started can restart.
    #[action(global_lock = false)]
    async fn restart(&self) -> Result<(), ActionError> {
        let control = self.control()?;
        if !control.managed {
            return Err(ActionError::handled(
                "the server can only restart when a supervisor, such as the launcher, started it",
            ));
        }
        (control.stop)(Exit::Restart);
        Ok(())
    }

    /// The computer's host name.
    #[property]
    async fn hostname(&self) -> String {
        gethostname::gethostname().to_string_lossy().into_owned()
    }

    /// The server's version.
    ///
    /// `version` is the version number; `version_source` is where the build
    /// came from (`git describe` when it was built).
    #[property]
    async fn version_data(&self) -> VersionData {
        VersionData::current()
    }

    /// The operating system and its version.
    #[property]
    async fn os_version(&self) -> String {
        os_info::get().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use teta_wot::testing::{Harness, TestClient};

    #[test]
    fn version_data_reports_the_crate_version_and_a_source() {
        let data = VersionData::current();
        assert_eq!(data.version, env!("CARGO_PKG_VERSION"));
        assert!(!data.version_source.is_empty());
    }

    #[tokio::test]
    async fn properties_are_served() {
        let server = ThingServer::builder()
            .thing("system", MicroscopeSystem::default())
            .build()
            .expect("the server builds");
        let client = TestClient::start(server).await.expect("the server starts");

        let hostname = client.get("/system/hostname").await;
        assert_eq!(hostname.status, 200);
        assert!(!hostname.json().as_str().unwrap_or_default().is_empty());

        let version = client.get("/system/version_data").await.json();
        assert_eq!(version["version"], env!("CARGO_PKG_VERSION"));

        let os = client.get("/system/os_version").await;
        assert_eq!(os.status, 200);

        client.stop().await;
    }

    #[tokio::test]
    async fn it_stops_and_restarts_the_server_only_as_its_control_allows() {
        use std::sync::Mutex;
        let asked = Arc::new(Mutex::new(Vec::new()));
        for managed in [None, Some(false), Some(true)] {
            let harness = Harness::builder("system", MicroscopeSystem::default())
                .start()
                .await
                .expect("the harness starts");
            if let Some(managed) = managed {
                let asked = Arc::clone(&asked);
                let stop = Arc::new(move |exit| asked.lock().expect("not poisoned").push(exit));
                ThingRef::thing(harness.thing()).attach(Control { managed, stop });
            }
            let client = harness.client();
            let shown = client.get("/system/managed").await.json();
            assert_eq!(shown, managed == Some(true), "{managed:?}");
            for action in ["shutdown", "restart"] {
                let invoked = client.post_json(&format!("/system/{action}"), None).await;
                let id = invoked.json()["id"].as_str().expect("an id").to_owned();
                let done = loop {
                    let record = client
                        .get(&format!("/action_invocations/{id}"))
                        .await
                        .json();
                    if !matches!(record["status"].as_str(), Some("pending" | "running")) {
                        break record;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                };
                let ran = managed.is_some() && (action == "shutdown" || managed == Some(true));
                assert_eq!(
                    done["status"] == "completed",
                    ran,
                    "{managed:?} {action}: {done}"
                );
            }
            harness.stop().await;
        }
        let asked = asked.lock().expect("not poisoned").clone();
        assert_eq!(asked, [Exit::Stop, Exit::Stop, Exit::Restart]);
    }

    #[tokio::test]
    async fn the_harness_starts_the_thing_alone() {
        let harness = Harness::builder("system", MicroscopeSystem::default())
            .start()
            .await
            .expect("the harness starts");
        let td = harness.client().get("/system/").await.json();
        assert_eq!(td["title"], "MicroscopeSystem");
        harness.stop().await;
    }
}

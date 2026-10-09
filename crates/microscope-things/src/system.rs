//! The `system` Thing: the server and the computer it runs on.

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

/// The microscope server and the computer it runs on.
#[derive(Thing)]
pub struct MicroscopeSystem {}

#[thing_impl]
impl MicroscopeSystem {
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

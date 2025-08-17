use std::sync::Arc;

use chrono::{DateTime, Utc};
use kube::{
    Client,
    runtime::events::{Recorder, Reporter},
};
use serde::Serialize;
use tokio::sync::RwLock;

use crate::metrics::Metrics;

mod metrics;
pub mod telemetry;
pub mod worker_group;

#[derive(Debug, Serialize, Clone)]
pub struct Diagnostics {
    #[serde(deserialize_with = "from_ts")]
    pub last_event: DateTime<Utc>,
    #[serde(skip)]
    pub reporter: Reporter,
}

impl Default for Diagnostics {
    fn default() -> Self {
        Self {
            last_event: Utc::now(),
            reporter: "probelet-operator".into(),
        }
    }
}

impl Diagnostics {
    fn recorder(&self, client: Client) -> Recorder {
        Recorder::new(client, self.reporter.clone())
    }
}

/// State shared between the controller and the web server.
#[derive(Clone)]
pub struct AppState {
    diagnostics: Arc<RwLock<Diagnostics>>,
    metrics: Arc<Metrics>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            diagnostics: Arc::new(RwLock::new(Diagnostics::default())),
            metrics: Arc::new(Metrics::default()),
        }
    }
}

impl AppState {
    /// Get the metrics as a string.
    pub fn metrics(&self) -> String {
        let mut buffer = String::new();
        let registry = &*self.metrics.registry;
        prometheus_client::encoding::text::encode(&mut buffer, registry).unwrap();
        buffer
    }

    /// Get the diagnostics.
    pub async fn diagnostics(&self) -> Diagnostics {
        self.diagnostics.read().await.clone()
    }

    /// Create a controller context that can update the state.
    pub async fn controller_context(&self, client: Client) -> Arc<Context> {
        Arc::new(Context {
            client: client.clone(),
            recorder: self.diagnostics.read().await.recorder(client),
            metrics: self.metrics.clone(),
            diagnostics: self.diagnostics.clone(),
        })
    }
}

/// The context for the operator.
///
/// This is passed to all the components of the operator.
#[derive(Clone)]
pub struct Context {
    pub client: Client,
    pub recorder: Recorder,
    pub diagnostics: Arc<RwLock<Diagnostics>>,
    pub metrics: Arc<Metrics>,
}

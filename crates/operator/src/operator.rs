use crate::{Error, Result};
use futures::StreamExt;
use kube::{
    CustomResource,
    api::{Api, ListParams},
    client::Client,
    runtime::{Controller, controller::Action, watcher::Config},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::Duration;
use tracing::*;

pub static DOCUMENT_FINALIZER: &str = "probelet.dev";

/// A probe that can be used to check the status of a resource
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct HttpProbe {
    /// The URL to monitor
    pub url: String,
    /// The HTTP method to use
    pub method: String,
}

/// The kind of probe to use
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub enum ProbeKind {
    /// A HTTP probe
    Http(HttpProbe),
}

/// Generate the Kubernetes wrapper struct `Probe` from our Spec and Status struct
#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(kind = "Probe", group = "probelet.dev", version = "v0", namespaced)]
#[kube(status = "ProbeStatus", shortname = "probe")]
pub struct ProbeSpec {
    pub kind: ProbeKind,
}

/// The status object of `Probe`
#[derive(Deserialize, Serialize, Clone, Default, Debug, JsonSchema)]
pub struct ProbeStatus {}

impl Probe {}

// Context for our reconciler
#[derive(Clone)]
pub struct Context {
    /// Kubernetes client
    pub client: Client,
}

async fn reconcile(_probe: Arc<Probe>, _ctx: Arc<Context>) -> Result<Action> {
    Ok(Action::requeue(Duration::from_secs(5 * 60)))
}

fn error_policy(_probe: Arc<Probe>, _error: &Error, _ctx: Arc<Context>) -> Action {
    warn!("reconcile failed: {:?}", _error);
    Action::requeue(Duration::from_secs(5 * 60))
}

impl Probe {
    // Reconcile (for non-finalizer related changes)
    async fn _reconcile(&self, _ctx: Arc<Context>) -> Result<Action> {
        todo!()
    }

    // Finalizer cleanup (the object was deleted, ensure nothing is orphaned)
    async fn _cleanup(&self, _ctx: Arc<Context>) -> Result<Action> {
        todo!()
    }
}

/// State shared between the operator and the web server
#[derive(Clone, Default)]
pub struct State {}

impl State {
    pub async fn to_context(&self, client: Client) -> Arc<Context> {
        Arc::new(Context { client })
    }
}

/// Initialize the operator and shared state (given the crd is installed)
pub async fn run(state: State) {
    let client = Client::try_default()
        .await
        .expect("failed to create kube Client");
    let probes = Api::<Probe>::all(client.clone());
    if let Err(e) = probes.list(&ListParams::default().limit(1)).await {
        error!("CRD is not queryable; {e:?}. Is the CRD installed?");
        std::process::exit(1);
    }
    Controller::new(probes, Config::default().any_semantic())
        .shutdown_on_signal()
        .run(reconcile, error_policy, state.to_context(client).await)
        .filter_map(|x| async move { std::result::Result::ok(x) })
        .for_each(|_| futures::future::ready(()))
        .await;
}

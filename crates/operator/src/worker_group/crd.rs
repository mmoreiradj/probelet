use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
    time::Duration,
};

use kube::{CustomResource, ResourceExt, runtime::controller::Action};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::Result;
use crate::{Context, metrics::MetricLabel, worker_group::reconcile::ReconcileWorkerGroupTask};

/// The `WorkerGroup` is a resource that manages a group of `Worker` instances (Pods).
/// `Workers` are where the probes are going to be executed.
#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(
    kind = "WorkerGroup",
    group = "probelet.dev",
    version = "v0",
    namespaced
)]
#[kube(status = "WorkerGroupStatus", shortname = "workergroup")]
pub struct WorkerGroupSpec {
    /// The number of replicas to create
    /// The max is `1` for the first version of the CRD
    pub replicas: i32,
    /// The image to use for the `WorkerGroup`
    pub image: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, PartialEq, Eq, Hash)]
pub struct WorkerInstanceName(pub String);

impl TryFrom<String> for WorkerInstanceName {
    type Error = String;

    /// Try to convert a string to a `WorkerInstanceName`
    /// `WorkerInstanceName` has to be a valid kubernetes object name
    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        if value.is_empty() {
            return Err("Worker instance name cannot be empty".to_string());
        }

        if value.len() > 63 {
            return Err("Worker instance name cannot be longer than 63 characters".to_string());
        }

        if value.contains('.') || value.contains('/') || value.contains('%') {
            return Err("Worker instance name cannot contain . or / or %".to_string());
        }

        Ok(WorkerInstanceName(value))
    }
}

impl WorkerInstanceName {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_string(&self) -> String {
        self.0.clone()
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, PartialEq, Eq, Hash)]
pub enum WorkerGroupInstanceStatus {
    /// The instance is ready
    Ready,
    /// The instance is not ready
    NotReady,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, PartialEq, Eq, Hash)]
pub struct WorkerGroupReportedInstanceState {
    /// The state of the instance, can be `Ready` or `NotReady`
    pub status: WorkerGroupInstanceStatus,
    /// The last updated time of the instance
    pub last_updated: String,
}

/// The status object of `WorkerGroup`
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct WorkerGroupStatus {
    /// Runnning instances names
    pub instance_names: Vec<WorkerInstanceName>,
    /// Number of instances
    pub instances: i32,
    /// State of instances
    pub instances_reported_state: HashMap<WorkerInstanceName, WorkerGroupReportedInstanceState>,
    /// Ready instances
    pub ready_instances: i32,
}

impl MetricLabel for WorkerGroup {
    fn metric_label(&self) -> String {
        format!("worker_group__{}", self.name_any())
    }
}

impl WorkerGroup {
    pub(crate) async fn reconcile(&self, context: Arc<Context>) -> Result<Action> {
        match ReconcileWorkerGroupTask::from_worker_group(self.clone(), context)? {
            Some(task) => task.run().await,
            None => Ok(Action::requeue(Duration::from_secs(5 * 60))),
        }
    }

    pub(crate) async fn cleanup(&self, _context: Arc<Context>) -> Result<Action> {
        Ok(Action::requeue(Duration::from_secs(5 * 60)))
    }

    pub fn default_annotations(&self) -> BTreeMap<String, String> {
        let mut annotations = BTreeMap::new();
        annotations.insert(
            "probelet.dev/operatorVersion".to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
        );
        annotations
    }

    pub fn default_labels(&self) -> BTreeMap<String, String> {
        let mut labels = BTreeMap::new();
        labels.insert("probelet.dev/workerGroupName".to_string(), self.name_any());
        labels
    }
}

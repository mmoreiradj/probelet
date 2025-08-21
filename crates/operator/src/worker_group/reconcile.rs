use std::{fmt::Display, sync::Arc, time::Duration};

use kube::{ResourceExt, runtime::controller::Action};

use super::Result;
use crate::{
    Context,
    worker_group::{crd::WorkerGroup, worker::Worker},
};

#[derive(Debug, Clone)]
pub enum EventReason {
    /// The worker was created
    WorkerCreated,
}

impl Display for EventReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Tasks to be run to reconcile the `WorkerGroup`
#[derive(Debug, Clone)]
pub enum Tasks {
    /// If the number of instances is less than the desired number of instances,
    /// we need to create a new worker
    CreateWorker,
}

#[derive(Clone)]
pub struct ReconcileWorkerGroupTask {
    worker_group: WorkerGroup,
    task: Tasks,
    context: Arc<Context>,
}

impl ReconcileWorkerGroupTask {
    /// Determines wether a task should be run based on the state of the `WorkerGroup`
    pub fn from_worker_group(
        worker_group: WorkerGroup,
        context: Arc<Context>,
    ) -> Result<Option<Self>> {
        if worker_group.spec.replicas
            > worker_group
                .status
                .as_ref()
                .map(|status| status.instances)
                .unwrap_or(-1)
        {
            return Ok(Some(Self::new(worker_group, context, Tasks::CreateWorker)));
        }

        Ok(None)
    }

    pub fn new(worker_group: WorkerGroup, context: Arc<Context>, task: Tasks) -> Self {
        Self {
            worker_group,
            context,
            task,
        }
    }
}

impl ReconcileWorkerGroupTask {
    async fn create_worker(&self) -> Result<Action> {
        let worker = Worker::new(
            self.worker_group.name_any(),
            self.worker_group.spec.image.clone(),
            Arc::new(self.worker_group.clone()),
        );

        worker.create(self.context.clone()).await?;

        Ok(Action::requeue(Duration::from_secs(5 * 60)))
    }

    pub async fn run(&self) -> Result<Action> {
        match self.task {
            Tasks::CreateWorker => self.create_worker().await,
        }
    }
}

use std::{sync::Arc, time::Duration};

use k8s_openapi::api::core::v1::{Container, Pod, PodSpec};
use kube::{
    Api, Resource, ResourceExt,
    api::{ObjectMeta, PostParams},
    runtime::{
        controller::Action,
        events::{Event, EventType},
    },
};
use snafu::ResultExt;

use crate::{
    Context,
    worker_group::{WorkerGroup, error::KubeSnafu, reconcile::EventReason},
};

use super::Result;

const WORKER_GROUP_DEFAULT_DELETION_GRACE_PERIOD_SECONDS: i64 = 30; // 30 seconds

#[derive(Debug, Clone)]
pub struct Worker {
    pub name: String,
    pub image: String,
    pub worker_group: Arc<WorkerGroup>,
}

impl Worker {
    pub fn new(name: String, image: String, worker_group: Arc<WorkerGroup>) -> Self {
        Self {
            name,
            image,
            worker_group,
        }
    }

    pub async fn create(&self, context: Arc<Context>) -> Result<Action> {
        let pod = self.pod();
        let client = context.client.clone();
        let api = Api::<Pod>::namespaced(client, &self.worker_group.namespace().unwrap());
        api.create(&PostParams::default(), &pod)
            .await
            .context(KubeSnafu {
                message: format!("Failed to create worker pod {}", self.name),
            })?;

        let event = Event {
            type_: EventType::Normal,
            reason: EventReason::WorkerCreated.to_string(),
            note: Some("Worker Created".to_string()),
            secondary: Some(self.worker_group.object_ref(&())),
            action: EventReason::WorkerCreated.to_string(),
        };

        let recorder = context.recorder.clone();
        recorder
            .publish(&event, &pod.object_ref(&()))
            .await
            .context(KubeSnafu {
                message: format!("Failed to publish event for worker {}", self.name),
            })?;

        Ok(Action::requeue(Duration::from_secs(5 * 60)))
    }

    pub fn pod(&self) -> Pod {
        let spec = PodSpec {
            containers: vec![Container {
                name: "worker".to_string(),
                image: Some(self.image.clone()),
                command: Some(vec!["/bin/sh".to_string(), "-c".to_string()]),
                args: Some(vec!["sleep infinity".to_string()]),
                ..Default::default()
            }],
            restart_policy: Some("Always".to_string()),
            ..Default::default()
        };

        let mut annotations = self.worker_group.default_annotations();
        annotations.insert(
            "probelet.dev/podSpec".to_string(),
            serde_json::to_string(&spec).unwrap(),
        );
        let mut labels = self.worker_group.default_labels();
        labels.insert("probelet.dev/workerName".to_string(), self.name.clone());

        Pod {
            metadata: ObjectMeta {
                name: Some(self.name.clone()),
                owner_references: Some(vec![self.worker_group.owner_ref(&()).unwrap()]),
                annotations: Some(annotations),
                deletion_grace_period_seconds: Some(
                    WORKER_GROUP_DEFAULT_DELETION_GRACE_PERIOD_SECONDS,
                ),
                labels: Some(labels),
                ..Default::default()
            },
            spec: Some(spec),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use crate::worker_group::crd::WorkerGroupSpec;

    use super::*;

    #[test_log::test(tokio::test)]
    async fn test_pod() {
        let worker = Worker::new(
            "test".to_string(),
            "test".to_string(),
            Arc::new(WorkerGroup {
                metadata: ObjectMeta {
                    name: Some("test".to_string()),
                    uid: Some("test".to_string()),
                    ..Default::default()
                },
                spec: WorkerGroupSpec {
                    replicas: 1,
                    image: "test".to_string(),
                },
                status: None,
            }),
        );

        let pod = worker.pod();
        let pod_json = serde_json::to_string_pretty(&pod).unwrap();

        assert_snapshot!(pod_json);
    }
}

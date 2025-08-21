mod crd;
mod error;
mod reconcile;
mod worker;

use std::{sync::Arc, time::Duration};

use chrono::Utc;
pub use crd::WorkerGroup;
use error::Result;
use futures::StreamExt;
use kube::{
    Api, Client, ResourceExt,
    api::ListParams,
    runtime::{Controller, controller::Action, finalizer, watcher::Config},
};
use snafu::ResultExt;
use tracing::{Span, instrument, warn};

use crate::{
    AppState, Context, telemetry,
    worker_group::error::{FinalizerSnafu, WorkerGroupError},
};

const WORKER_GROUP_FINALIZER: &str = "probelet.io/worker-group";

#[instrument(skip(worker_group, context), fields(trace_id))]
async fn reconcile(worker_group: Arc<WorkerGroup>, context: Arc<Context>) -> Result<Action> {
    let trace_id = telemetry::get_trace_id();
    if trace_id != opentelemetry::trace::TraceId::INVALID {
        Span::current().record("trace_id", tracing::field::display(trace_id));
    }
    let _timer = context.metrics.reconcile.count_and_measure(&trace_id);
    context.diagnostics.write().await.last_event = Utc::now();
    // we can unwrap because the worker_group is namespace scoped
    let ns = worker_group.namespace().unwrap();
    let worker_groups = Api::<WorkerGroup>::namespaced(context.client.clone(), &ns);

    tracing::info!(
        "reconciling worker group \"{}\" in ns \"{}\"",
        worker_group.name_any(),
        ns
    );
    finalizer(
        &worker_groups,
        WORKER_GROUP_FINALIZER,
        worker_group,
        |event| async {
            match event {
                finalizer::Event::Apply(wg) => wg.reconcile(context.clone()).await,
                finalizer::Event::Cleanup(wg) => wg.cleanup(context.clone()).await,
            }
        },
    )
    .await
    .context(FinalizerSnafu)
}

fn error_policy(
    worker_group: Arc<WorkerGroup>,
    error: &WorkerGroupError,
    context: Arc<Context>,
) -> Action {
    warn!(
        "reconcile failed for worker group \"{}\" in ns \"{}\": {error:?}",
        worker_group.name_any(),
        worker_group.namespace().unwrap()
    );
    context.metrics.reconcile.set_failure(&*worker_group, error);
    Action::requeue(Duration::from_secs(5 * 60))
}

/// Runs the `WorkerGroup` controller
pub async fn run(client: Client, watcher_config: Config, state: AppState) {
    let worker_groups = Api::<WorkerGroup>::all(client.clone());

    if let Err(e) = worker_groups.list(&ListParams::default().limit(1)).await {
        tracing::error!("CRD is not queryable; {e:?}. Is the CRD installed?");
        std::process::exit(1);
    }
    Controller::new(worker_groups, watcher_config)
        .shutdown_on_signal()
        .run(
            reconcile,
            error_policy,
            state.controller_context(client).await,
        )
        .filter_map(|x| async move { std::result::Result::ok(x) })
        .for_each(|_| futures::future::ready(()))
        .await;
}

use snafu::Snafu;

use crate::metrics::MetricLabel;

#[derive(Snafu, Debug)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum WorkerGroupError {
    #[snafu(display("Finalizer error: {source}"))]
    Finalizer {
        #[snafu(source(from(kube::runtime::finalizer::Error<WorkerGroupError>, Box::new)))]
        source: Box<kube::runtime::finalizer::Error<WorkerGroupError>>,
    },
}

impl MetricLabel for WorkerGroupError {
    fn metric_label(&self) -> String {
        "worker_group_error".to_string()
    }
}

pub type Result<T> = std::result::Result<T, WorkerGroupError>;

use kube::CustomResourceExt;
use operator::worker_group::WorkerGroup;
use std::io::{self, Write};

fn main() {
    let worker_crd = serde_yaml::to_string(&WorkerGroup::crd()).unwrap();

    io::stdout().write_all(b"---\n").unwrap();
    io::stdout().write_all(worker_crd.as_bytes()).unwrap();
    io::stdout().flush().unwrap();
}

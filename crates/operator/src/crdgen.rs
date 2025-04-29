use kube::CustomResourceExt;
use operator::Probe;

fn main() {
    print!("{}", serde_yaml::to_string(&Probe::crd()).unwrap())
}

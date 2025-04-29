[private]
default:
  @just --list --unsorted

compile bin:
  #!/usr/bin/env bash
  cargo build --release --bin {{bin}} --target x86_64-unknown-linux-musl
  cp target/x86_64-unknown-linux-musl/release/{{bin}} crates/{{bin}}
  if [ "{{bin}}" = "operator" ]; then
    cargo run --bin crdgen > charts/operator/templates/crds.yaml
  fi

build bin="":
  docker build -t probelet/{{bin}} crates/{{bin}}

generate-crd:
  cargo run --bin crdgen > charts/operator/templates/crds.yaml

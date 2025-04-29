use axum::{Json, Router};
use axum_extra::routing::RouterExt;
use axum_extra::routing::TypedPath;
use operator::State;
use operator::operator;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::signal;
use tracing::info;

#[derive(Clone, Debug, Deserialize, Serialize, TypedPath)]
#[typed_path("/healthz")]
pub struct HealthRoute;

async fn health(_: HealthRoute) -> Json<&'static str> {
    Json("healthy")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let state = State::default();

    info!("Starting operator");
    let controller = operator::run(state.clone());

    let app = Router::new().typed_get(health).with_state(state);

    info!("Starting server on port 8080");
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let listener = TcpListener::bind(addr).await?;
    let server =
        axum::serve(listener, app.into_make_service()).with_graceful_shutdown(shutdown_signal());

    tokio::join!(controller, server).1?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

use axum::{Router, routing::get};
use clap::{Parser, ValueEnum, crate_version};
use serde::Serialize;
use tokio::signal;
use std::{net::SocketAddr, sync::Arc};

mod healthcheck;
use crate::healthcheck::*;

mod routes;
use crate::routes::*;

#[derive(Clone, ValueEnum, Debug, Serialize)]
enum Environment {
    Development,
    Test,
    Production,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
   #[arg(short, long, default_value_t = 4000)]
   port: u16,

   #[arg(short, long, default_value_t = Environment::Development, value_enum)]
   env: Environment,
}

#[derive(Clone)]
pub struct AppState {
    version: &'static str,
    env: Environment,
}

#[tokio::main]
async fn main() {
    let cli = Args::parse();

    let shared_state = Arc::new(AppState {
        version: crate_version!(),
        env: cli.env,
    });

    let app = Router::new()
        .merge(healthcheck_routes(shared_state.clone()));

    let addr = SocketAddr::from(([0, 0, 0, 0], cli.port));
    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("signal received, starting graceful shutdown");
}

use axum::Router;
use clap::{crate_version, Parser, ValueEnum};
use serde::Serialize;

use sqlx::{postgres::PgPoolOptions, pool::PoolConnection, Postgres};
use std::{net::SocketAddr, sync::Arc};
use tokio::signal;

mod healthcheck;

mod routes;
use crate::{routes::*};

mod heroes;

mod data;

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

    #[arg(short, long, default_value = "", env = "DATABASE_URL")]
    database_url: String,
}

#[derive(Clone)]
pub struct AppState {
    version: &'static str,
    env: Environment
}

#[tokio::main]
async fn main() {
    let cli = Args::parse();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&cli.database_url)
        .await
        .expect("can connect to database");

    let shared_state = Arc::new(AppState {
        version: crate_version!(),
        env: cli.env,
    });

    let app = Router::new()
        .merge(healthcheck_routes(shared_state.clone()))
        .nest("/heroes", heroes_routes(pool));

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
        signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
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

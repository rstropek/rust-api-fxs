use axum::Router;
use clap::{crate_version, Parser, ValueEnum};
use serde::Serialize;

use sqlx::postgres::PgPoolOptions;
use tower_http::{trace::TraceLayer, catch_panic::CatchPanicLayer};
use std::{net::SocketAddr, sync::Arc};
use tokio::signal;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tower::ServiceBuilder;

mod healthcheck;
mod heroes;
mod data;
mod axum_helpers;
mod error;
mod model;

use error::Error;

use crate::{data::HeroesRepository, heroes::DynHeroesRepository};
type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Clone, ValueEnum, Debug, Serialize, PartialEq, Eq)]
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

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "hero_manager_axum=debug,tower_http=debug,sqlx=info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let repo = Arc::new(HeroesRepository(pool)) as DynHeroesRepository;

    let app = Router::new()
        .merge(healthcheck::healthcheck_routes(shared_state.clone()))
        .nest("/heroes", heroes::heroes_routes(repo))
        .layer(
            ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .layer(CatchPanicLayer::custom(error::handle_panic))
                    .into_inner(),
            );

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

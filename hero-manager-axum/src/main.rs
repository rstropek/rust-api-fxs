use crate::{
    data::HeroesRepository,
    heroes::DynHeroesRepository,
    model::{AppConfiguration, Environment},
};
use anyhow::Context;
use axum::{BoxError, Router, error_handling::HandleErrorLayer, http};
use clap::{Parser, crate_version};
use sqlx::postgres::PgPoolOptions;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::{net::TcpListener, signal};
use tower::ServiceBuilder;
use tower_http::{catch_panic::CatchPanicLayer, trace::TraceLayer};
use tracing_subscriber::EnvFilter;

mod data;
mod error;
mod healthcheck;
mod heroes;
mod model;

/// Arguments for clap
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = 4000)]
    port: u16,

    #[arg(short, long, default_value_t = Environment::Development, value_enum)]
    env: Environment,

    #[arg(short, long, env = "DATABASE_URL", hide_env_values = true)]
    database_url: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse command-line args
    let cli = Args::parse();

    // Configure tracing. Use the RUST_LOG environment variable to override the filter.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "hero_manager_axum=debug,tower_http=debug,sqlx=debug".into()),
        )
        .init();

    // Setup connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&cli.database_url)
        .await
        .context("failed to connect to database")?;

    // Build app configuration object
    let app_config = Arc::new(AppConfiguration {
        version: crate_version!(),
        env: cli.env,
    });

    let repo = Arc::new(HeroesRepository(pool)) as DynHeroesRepository;

    // Setup top-level router
    let app = Router::new()
        // Add healthcheck routes
        .merge(healthcheck::healthcheck_routes(app_config))
        // Add heroes routes under /heroes
        .nest("/heroes", heroes::heroes_routes(repo))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(HandleErrorLayer::new(|_: BoxError| async {
                    http::StatusCode::REQUEST_TIMEOUT
                }))
                .timeout(Duration::from_secs(2))
                .layer(CatchPanicLayer::custom(error::handle_panic)),
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], cli.port));
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("listening on {addr}");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
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

    tracing::info!("signal received, starting graceful shutdown");
}

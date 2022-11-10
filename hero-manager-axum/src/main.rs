use crate::{data::HeroesRepository, heroes::DynHeroesRepository, model::AppConfiguration};
use axum::Router;
use clap::{crate_version, Parser};
use model::Environment;
use sqlx::postgres::PgPoolOptions;
use std::{net::SocketAddr, sync::Arc};
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::{catch_panic::CatchPanicLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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

    #[arg(short, long, default_value = "", env = "DATABASE_URL")]
    database_url: String,
}

#[tokio::main]
async fn main() {
    // Parse command-line args
    let cli = Args::parse();

    // Setup connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&cli.database_url)
        .await
        .expect("can connect to database");

    // Build app configuration object
    let app_config = Arc::new(AppConfiguration {
        version: crate_version!(),
        env: cli.env,
    });

    // Configure tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "hero_manager_axum=debug,tower_http=debug,sqlx=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let repo = Arc::new(HeroesRepository(pool)) as DynHeroesRepository;

    // Setup top-level router
    let app = Router::new()
        // Add healthcheck routes
        .merge(healthcheck::healthcheck_routes(app_config.clone()))
        // Add heroes routes under /heroes
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

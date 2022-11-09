use std::sync::Arc;

use axum::{Router, routing::{get, post}, body::Body};
use sqlx::{PgPool, Postgres, Pool};

use crate::{AppState, healthcheck::*, heroes::{get_all_heroes, insert_hero}, data::{DatabaseConnection}};

pub fn healthcheck_routes(shared_state: Arc<AppState>) -> Router<AppState, Body> {
    Router::with_state_arc(shared_state)
        .route("/health_1", get(healthcheck_handler_1))
        .route("/health_2", get(healthcheck_handler_2))
        .route("/health_3", get(healthcheck_handler_3))
        .route("/health_4", get(healthcheck_handler_4))
}

pub fn heroes_routes(pool: PgPool) -> Router<Pool<Postgres>, Body> {
    Router::with_state(pool)
        .route("/", post(insert_hero))
}

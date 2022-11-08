use std::sync::Arc;

use axum::{Router, routing::get, body::Body};

use crate::{AppState, healthcheck::*};

pub fn healthcheck_routes(shared_state: Arc<AppState>) -> Router<AppState, Body> {
    Router::with_state_arc(shared_state)
        .route("/health_1", get(healthcheck_handler_1))
        .route("/health_2", get(healthcheck_handler_2))
        .route("/health_3", get(healthcheck_handler_3))
        .route("/health_4", get(healthcheck_handler_4))
}

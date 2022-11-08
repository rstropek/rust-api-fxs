use axum::{response::{IntoResponse}, extract::State};
use sqlx::PgPool;

/// Healthcheck handler
///
/// This implementation demonstrates how to manually build a response.
/// For more details see https://docs.rs/axum/0.6.0-rc.2/axum/response/index.html#building-responses
pub async fn get_all_heroes(State(pool): State<PgPool>) -> impl IntoResponse {
    todo!("get_all_heroes")
}

use axum::{http::request::Parts, async_trait, extract::{FromRequestParts, FromRef}};
use sqlx::PgPool;
use tracing::error;

use crate::data::DatabaseConnection;

#[async_trait]
impl<S> FromRequestParts<S> for DatabaseConnection
where
    PgPool: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = crate::Error;

    async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let pool = PgPool::from_ref(state);

        let conn = pool.acquire().await.map_err(|e| {
            error!("Failed to acquire connection from pool: {}", e);
            e
        })?;

        Ok(Self(conn))
    }
}

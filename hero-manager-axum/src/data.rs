use axum::{async_trait, extract::{FromRequestParts, FromRef}, http::{StatusCode, request::Parts}};
use sqlx::{PgPool, postgres::PgQueryResult, types::chrono::{DateTime, Utc}};

pub struct Hero {
    pub id: i32,
    pub first_seen: DateTime<Utc>,
    pub name: String,
    pub can_fly: bool,
    pub realname: Option<String>,
    pub abilities: Option<Vec<String>>,
    pub version: i32,
}

pub struct DatabaseConnection(pub sqlx::pool::PoolConnection<sqlx::Postgres>);

#[async_trait]
impl<S> FromRequestParts<S> for DatabaseConnection
where
    PgPool: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let pool = PgPool::from_ref(state);

        let conn = pool.acquire().await.map_err(internal_error)?;

        Ok(Self(conn))
    }
}

fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

pub async fn insert(DatabaseConnection(conn): DatabaseConnection, hero: &Hero) -> anyhow::Result<()> {
    let mut conn = conn;
    sqlx::query(r#"INSERT INTO heroes (id, first_seen, name, can_fly, realname, abilities, version) VALUES ($1, $2, $3, $4, $5, $6, $7)"#)
        .bind(hero.id)
        .bind(hero.first_seen)
        .bind(&hero.name)
        .bind(hero.can_fly)
        .bind(&hero.realname)
        .bind(&hero.abilities)
        .bind(hero.version)
        .execute(&mut conn)
        .await?;
    Ok(())
}

use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, StatusCode},
};
use sqlx::{
    types::chrono::{DateTime, Utc},
    PgPool,
};

pub struct Hero {
    pub id: i64,
    pub first_seen: DateTime<Utc>,
    pub name: String,
    pub can_fly: bool,
    pub realname: Option<String>,
    pub abilities: Option<Vec<String>>,
    pub version: i32,
}

pub struct NewHero {
    pub first_seen: DateTime<Utc>,
    pub name: String,
    pub can_fly: bool,
    pub realname: Option<String>,
    pub abilities: Option<Vec<String>>,
}

pub struct HeroPkVersion {
    pub id: i64,
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

pub async fn insert(DatabaseConnection(conn): DatabaseConnection, hero: &NewHero) -> anyhow::Result<HeroPkVersion> {
    let mut conn = conn;
    let pk: (i64, i32) = sqlx::query_as(
        r#"
        INSERT INTO heroes (first_seen, name, can_fly, realname, abilities)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, version"#,
    )
    .bind(hero.first_seen)
    .bind(&hero.name)
    .bind(hero.can_fly)
    .bind(&hero.realname)
    .bind(&hero.abilities)
    .fetch_one(&mut conn)
    .await?;
    Ok(HeroPkVersion {
        id: pk.0,
        version: pk.1,
    })
}

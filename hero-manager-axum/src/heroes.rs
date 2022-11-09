use crate::{
    data::{self, DatabaseConnection},
    model::{Hero, IdentifyableHero},
};

use axum::{
    body::Body,
    extract::Query,
    http::{header::LOCATION, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use serde::Deserialize;
use sqlx::{PgPool, Pool, Postgres};
use tracing::error;
use validator::Validate;

fn log(e: sqlx::Error) -> sqlx::Error {
    error!("Failed to execute SQL statement: {:?}", e);
    e
}

pub fn heroes_routes(pool: PgPool) -> Router<Pool<Postgres>, Body> {
    Router::with_state(pool)
        .route("/", post(insert_hero).get(get_heroes))
        .route("/cleanup", post(cleanup_heroes))
}

#[derive(Deserialize)]
pub struct GetHeroFilter {
    #[serde(rename = "name")]
    name_filter: Option<String>,
    // In practice, add additional query parameters here
}

pub async fn get_heroes(
    conn: DatabaseConnection,
    filter: Query<GetHeroFilter>,
) -> crate::Result<Json<Vec<IdentifyableHero>>> {
    let heroes = data::get_by_name(conn, filter.name_filter.as_deref().unwrap_or("%"))
        .await
        .map_err(log)?;
    Ok(Json(heroes))
}

pub async fn cleanup_heroes(conn: DatabaseConnection) -> crate::Result<impl IntoResponse> {
    data::cleanup(conn).await.map_err(log)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn insert_hero(conn: DatabaseConnection, Json(hero): Json<Hero>) -> crate::Result<impl IntoResponse> {
    hero.validate()?;

    let hero_pk = data::insert(conn, &hero).await.map_err(log)?;

    let mut headers = HeaderMap::new();
    headers.insert(
        LOCATION,
        format!("/heroes/{}", hero_pk.id)
            .parse()
            .expect("Parsing location header should never fail"),
    );
    Ok((
        StatusCode::OK,
        headers,
        Json(IdentifyableHero {
            id: hero_pk.id,
            inner_hero: hero,
            version: hero_pk.version,
        }),
    )
        .into_response())
}

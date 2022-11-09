use axum::{
    http::{
        header::LOCATION,
        HeaderMap, StatusCode,
    },
    response::IntoResponse,
    Json,
};

use tracing::error;
use validator::Validate;

use crate::{data::{self, DatabaseConnection}, model::{IdentifyableHero, Hero}};

use axum::{Router, routing::post, body::Body};
use sqlx::{PgPool, Postgres, Pool};

pub fn heroes_routes(pool: PgPool) -> Router<Pool<Postgres>, Body> {
    Router::with_state(pool)
        .route("/", post(insert_hero).get(get_all_heroes))
        .route("/cleanup", post(cleanup_heroes))
}

pub async fn get_all_heroes(conn: DatabaseConnection) -> crate::Result<Json<Vec<IdentifyableHero>>> {
    let heroes = data::get_by_name(conn, "%").await.map_err(|e| {
        error!("Failed to execute SELECT {:?}", e);
        e
    })?;
    Ok(Json(heroes))
}

pub async fn cleanup_heroes(conn: DatabaseConnection) -> crate::Result<impl IntoResponse> {
    data::cleanup(conn).await.map_err(|e| {
        error!("Failed to execute DELETE {:?}", e);
        e
    })?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn insert_hero(conn: DatabaseConnection, Json(hero): Json<Hero>) -> crate::Result<impl IntoResponse> {
    hero.validate()?;

    let hero_pk = data::insert(conn, &hero).await.map_err(|e| {
        error!("Failed to execute INSERT {:?}", e);
        e
    })?;

    let mut headers = HeaderMap::new();
    headers.insert(
        LOCATION,
        format!("/heroes/{}", hero_pk.id).parse().expect("Parsing location header should never fail"),
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

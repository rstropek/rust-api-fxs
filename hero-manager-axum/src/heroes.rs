use std::sync::Arc;

use crate::{
    data::{DatabaseConnection, HeroesRepositoryTrait},
    model::{Hero, IdentifyableHero},
};

use axum::{
    body::Body,
    extract::{Query, State},
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

pub type DynHeroesRepository = Arc<dyn HeroesRepositoryTrait + Send + Sync>;

//pub fn heroes_routes(pool: PgPool) -> Router<Pool<Postgres>, Body> {
pub fn heroes_routes(repo: DynHeroesRepository) -> Router<DynHeroesRepository, Body> {
    Router::with_state(repo)
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
    State(repo): State<DynHeroesRepository>,
    filter: Query<GetHeroFilter>,
) -> crate::Result<Json<Vec<IdentifyableHero>>> {
    let heroes = repo
        .get_by_name(filter.name_filter.as_deref().unwrap_or("%"))
        .await
        .map_err(log)?;
    Ok(Json(heroes))
}

pub async fn cleanup_heroes(State(repo): State<DynHeroesRepository>) -> crate::Result<impl IntoResponse> {
    repo.cleanup().await.map_err(log)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn insert_hero(
    State(repo): State<DynHeroesRepository>,
    Json(hero): Json<Hero>,
) -> crate::Result<impl IntoResponse> {
    hero.validate()?;

    let hero_pk = repo.insert(&hero).await.map_err(log)?;

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

#[cfg(test)]
mod tests {
    use crate::data::MockHeroesRepositoryTrait;

    use super::*;
    use axum::http::Request;
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt;

    #[tokio::test]
    async fn hello_world() {
        let mut repo_mock = MockHeroesRepositoryTrait::new();
        repo_mock.expect_cleanup().returning(|| Ok(()));

        let mut repo = Arc::new(repo_mock) as DynHeroesRepository;

        let app = heroes_routes(repo).into_service();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/cleanup")
                    .method("POST")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }
}

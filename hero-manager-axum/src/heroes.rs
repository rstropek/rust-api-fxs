/// API handler for hero management
///
/// The most important aspect of this part of the sample is dependency
/// injection with a trait. Our goal is to unit-test our handlers using
/// mocked versions of our data access layer.
use crate::{
    data::{log_error, HeroesRepositoryTrait},
    model::{Hero, IdentifyableHero}, error,
};
use axum::{
    extract::{Query, State},
    http::{header::LOCATION, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;
use validator::Validate;

/// Type alias for our shared state
///
/// Note that we use a dyn state so that we can easily replace it
/// with a mock object.
pub type DynHeroesRepository = Arc<dyn HeroesRepositoryTrait + Send + Sync>;

/// Setup hero management API routes
pub fn heroes_routes(repo: DynHeroesRepository) -> Router {
    Router::new()
        .route("/", post(insert_hero).get(get_heroes))
        .route("/cleanup", post(cleanup_heroes))
        .with_state(repo)
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
) -> error::Result<Json<Vec<IdentifyableHero>>> {
    let heroes = repo
        .get_by_name(filter.name_filter.as_deref().unwrap_or("%"))
        .await
        .map_err(log_error)?;
    Ok(Json(heroes))
}

pub async fn cleanup_heroes(State(repo): State<DynHeroesRepository>) -> error::Result<impl IntoResponse> {
    repo.cleanup().await.map_err(log_error)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn insert_hero(
    State(repo): State<DynHeroesRepository>,
    Json(hero): Json<Hero>,
) -> error::Result<impl IntoResponse> {
    hero.validate()?;

    let hero_pk = repo.insert(&hero).await.map_err(log_error)?;

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
    use hyper::Body;
    use mockall::predicate::*;
    use rstest::rstest;
    use serde_json::Value;
    use sqlx::Error;
    use tower::ServiceExt;

    #[rstest]
    #[case(Ok(()), StatusCode::NO_CONTENT)]
    #[case(Err(Error::WorkerCrashed), StatusCode::INTERNAL_SERVER_ERROR)]
    #[tokio::test]
    async fn cleanup(#[case] result: Result<(), sqlx::error::Error>, #[case] status_code: StatusCode) {
        let mut repo_mock = MockHeroesRepositoryTrait::new();
        repo_mock.expect_cleanup().return_once(|| result);

        let repo = Arc::new(repo_mock) as DynHeroesRepository;

        let app = heroes_routes(repo);//.into_service();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/cleanup")
                    .method("POST")
                    .body(hyper::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), status_code);
    }

    #[tokio::test]
    async fn get_heroes() {
        let mut repo_mock = MockHeroesRepositoryTrait::new();
        repo_mock.expect_get_by_name()
            .with(eq("Super%"))
            .returning(|_| Ok(vec![Default::default()]));

        let repo = Arc::new(repo_mock) as DynHeroesRepository;

        let app = heroes_routes(repo);//.into_service();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/?name=Super%")
                    .method("GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body: Value = serde_json::from_slice(&body).unwrap();

        assert!(matches!(body, Value::Array { .. }));
    }
}

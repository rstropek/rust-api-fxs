//! Healthcheck routes and handlers
//!
//! This part of the sample demonstrates various ways for how to
//! build web responses based on a healthcheck endpoint.
//! We also use the healthcheck endpoints to demonstrate some
//! principles about testing handlers.

use axum::{
    Json, Router,
    body::Body,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::{convert::Infallible, sync::Arc};

use crate::{
    error,
    model::{AppConfiguration, Environment},
};

/// Setup healthcheck API routes
pub fn healthcheck_routes(shared_state: Arc<AppConfiguration>) -> Router {
    Router::new()
        .route("/health_1", get(healthcheck_handler_1))
        .route("/health_2", get(healthcheck_handler_2))
        .route("/health_3", get(healthcheck_handler_3))
        .route("/health_4", get(healthcheck_handler_4))
        .route("/health_failing_1", get(failing_healthcheck_1))
        .route("/health_failing_2", get(failing_healthcheck_2))
        .with_state(shared_state)
}

/// Healthcheck handler
///
/// This implementation demonstrates how to manually build a response.
/// For more details see https://docs.rs/axum/latest/axum/response/index.html#building-responses
pub async fn healthcheck_handler_1(State(state): State<Arc<AppConfiguration>>) -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        format!(r#"{{"version":"{0}","env":"{1:?}"}}"#, state.version, state.env),
    )
}

/// Healthcheck handler
///
/// This implementation demonstrates how to build a response with low-level builder.
/// For more details see https://docs.rs/axum/latest/axum/response/index.html#building-responses
pub async fn healthcheck_handler_2(State(state): State<Arc<AppConfiguration>>) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(format!(
            r#"{{"version":"{0}","env":"{1:?}"}}"#,
            state.version, state.env
        )))
        .unwrap()
}

/// Healthcheck handler
///
/// This implementation demonstrates how to build a JSON response with Json.
/// For more details see https://docs.rs/axum/latest/axum/struct.Json.html
pub async fn healthcheck_handler_3(State(state): State<Arc<AppConfiguration>>) -> Json<Value> {
    let value = json!({
        "version": state.version,
        "env": format!("{:?}", state.env),
    });
    Json(value)
}

#[derive(Serialize)]
pub struct HealthcheckResponseDto {
    version: String,
    env: Environment,
}

/// Healthcheck handler
///
/// This implementation demonstrates how to build a JSON response with Axum's Json responder.
/// For more details see https://docs.rs/axum/latest/axum/struct.Json.html
pub async fn healthcheck_handler_4(State(state): State<Arc<AppConfiguration>>) -> Json<HealthcheckResponseDto> {
    Json(HealthcheckResponseDto {
        version: state.version.to_string(),
        env: state.env.clone(),
    })
}

pub async fn failing_healthcheck_1() -> error::Result<()> {
    Err(error::Error::Anyhow(anyhow::anyhow!("Something bad happened")))
}

pub async fn failing_healthcheck_2() -> Infallible {
    panic!("Something very bad happened");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::to_bytes, http::Request};
    use hyper_util::{client::legacy::Client, rt::TokioExecutor};
    use rstest::rstest;
    use tokio::net::TcpListener;
    use tower::ServiceExt;

    fn test_app() -> Router {
        healthcheck_routes(Arc::new(AppConfiguration {
            env: Environment::Development,
            version: "1.0.0",
        }))
    }

    #[rstest]
    #[case("/health_1")]
    #[case("/health_2")]
    #[case("/health_3")]
    #[case("/health_4")]
    #[tokio::test]
    async fn healthchecks(#[case] uri: &str) {
        // `Router` implements `tower::Service<Request<Body>>` so we can
        // call it like any tower service, no need to run an HTTP server.
        let response = test_app()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(body, json!({ "version": "1.0.0", "env": "Development" }));
    }

    #[rstest]
    #[case("/health_1")]
    #[case("/health_2")]
    #[case("/health_3")]
    #[case("/health_4")]
    #[tokio::test]
    async fn healthchecks_real(#[case] url: &str) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            axum::serve(listener, test_app()).await.unwrap();
        });

        let client = Client::builder(TokioExecutor::new()).build_http::<Body>();

        let response = client
            .request(
                Request::builder()
                    .uri(format!("http://{addr}{url}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(Body::new(response.into_body()), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(body, json!({ "version": "1.0.0", "env": "Development" }));
    }
}

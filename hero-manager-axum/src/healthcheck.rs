/// Healtheck routes and handlers
///
/// This part of the sample demonstrates various ways for how to
/// build web responses based on a healthcheck endpoint.
/// We also use the healthcheck endpoints to demonstrate some
/// principles about testing handlers.

use axum::{
    body::{Bytes, Full},
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use serde_json::{json, Value};
use std::{convert::Infallible, sync::Arc};

use crate::{AppConfiguration, Environment, error};

/// Setup healthcheck API routes
pub fn healthcheck_routes(shared_state: Arc<AppConfiguration>) -> Router {
    // Note that we are using the new state sharing API of the latest RC of Axum here.
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
/// For more details see https://docs.rs/axum/0.6.0-rc.4/axum/response/index.html#building-responses
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
/// For more details see https://docs.rs/axum/0.6.0-rc.4/axum/response/index.html#building-responses
pub async fn healthcheck_handler_2(State(state): State<Arc<AppConfiguration>>) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Full::from(format!(
            r#"{{"version":"{0}","env":"{1:?}"}}"#,
            state.version, state.env
        )))
        .unwrap()
}

/// Healthcheck handler
///
/// This implementation demonstrates how to build a JSON response with Json.
/// For more details see https://docs.rs/axum/0.6.0-rc.4/axum/struct.Json.html
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
/// For more details see https://docs.rs/axum/0.6.0-rc.4/axum/struct.Json.html
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
    use std::net::{SocketAddr, TcpListener};

    use super::*;
    use axum::http::Request;
    use rstest::rstest;
    use tower::ServiceExt;

    #[rstest]
    #[case("/health_1")]
    #[case("/health_2")]
    #[case("/health_3")]
    #[case("/health_4")]
    #[tokio::test]
    async fn healthchecks(#[case] uri: &str) {
        let app = healthcheck_routes(Arc::new(AppConfiguration {
            env: Environment::Development,
            version: "1.0.0",
        }))
        ;//.into_make_service();

        // `Router` implements `tower::Service<Request<Body>>` so we can
        // call it like any tower service, no need to run an HTTP server.
        let response = app
            .oneshot(Request::builder().uri(uri).body(hyper::Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
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
        let listener = TcpListener::bind("0.0.0.0:0".parse::<SocketAddr>().unwrap()).unwrap();
        let addr = listener.local_addr().unwrap();

        let app = healthcheck_routes(Arc::new(AppConfiguration {
            env: Environment::Development,
            version: "1.0.0",
        }))
        .into_make_service();

        tokio::spawn(async move {
            axum::Server::from_tcp(listener)
                .unwrap()
                .serve(app)
                .await
                .unwrap();
        });

        let client = hyper::Client::new();

        let response = client
            .request(
                Request::builder()
                    .uri(format!("http://{addr}{url}"))
                    .body(hyper::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(body, json!({ "version": "1.0.0", "env": "Development" }));
    }
}

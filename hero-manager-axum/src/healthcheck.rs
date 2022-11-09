use std::{convert::Infallible, sync::Arc};

use axum::{response::{IntoResponse, Response}, extract::State, http::{StatusCode, header}, body::{Full, Bytes, Body}, Json, Router, routing::get};
use serde::Serialize;
use serde_json::{Value, json};

use crate::{AppState, Environment};

pub fn healthcheck_routes(shared_state: Arc<AppState>) -> Router<AppState, Body> {
    Router::with_state_arc(shared_state)
        .route("/health_1", get(healthcheck_handler_1))
        .route("/health_2", get(healthcheck_handler_2))
        .route("/health_3", get(healthcheck_handler_3))
        .route("/health_4", get(healthcheck_handler_4))
        .route("/health_failing_1", get(failing_healthcheck_1))
        .route("/health_failing_2", get(failing_healthcheck_2))
}

/// Healthcheck handler
///
/// This implementation demonstrates how to manually build a response.
/// For more details see https://docs.rs/axum/0.6.0-rc.2/axum/response/index.html#building-responses
pub async fn healthcheck_handler_1(State(state): State<AppState>) -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        format!(r#"{{"version":"{0}","env":"{1:?}"}}"#, state.version, state.env),
    )
}

/// Healthcheck handler
///
/// This implementation demonstrates how to build a response with low-level builder.
/// For more details see https://docs.rs/axum/0.6.0-rc.2/axum/response/index.html#building-responses
pub async fn healthcheck_handler_2(State(state): State<AppState>) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Full::from(format!(r#"{{"version":"{0}","env":"{1:?}"}}"#, state.version, state.env)))
        .unwrap()
}

/// Healthcheck handler
///
/// This implementation demonstrates how to build a JSON response with Json.
/// For more details see https://docs.rs/axum/0.6.0-rc.2/axum/struct.Json.html
pub async fn healthcheck_handler_3(State(state): State<AppState>) -> Json<Value> {
    let value = json!({
        "version": state.version,
        "env": format!("{:?}", state.env),
    });
    Json(value)
}

#[derive(Serialize)]
pub struct HealthcheckResponseDto {
    version: &'static str,
    env: Environment,
}

/// Healthcheck handler
///
/// This implementation demonstrates how to build a JSON response with serde::Serialize.
/// For more details see https://docs.rs/axum/0.6.0-rc.2/axum/struct.Json.html
pub async fn healthcheck_handler_4(State(state): State<AppState>) -> Json<HealthcheckResponseDto> {
    Json(HealthcheckResponseDto {
        version: state.version,
        env: state.env.clone(),
    })
}

pub async fn failing_healthcheck_1() -> crate::Result<()> {
    Err(crate::Error::Anyhow(anyhow::anyhow!("Something bad happened")))
}

pub async fn failing_healthcheck_2() -> Infallible {
    panic!("Something very bad happened");
}


#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn hello_world() {
        let app = healthcheck_routes(Arc::new(AppState{env: Environment::Development, version: "1.0.0"}));

        // `Router` implements `tower::Service<Request<Body>>` so we can
        // call it like any tower service, no need to run an HTTP server.
        let response = app
            .oneshot(Request::builder().uri("/health_1").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let v: serde_json::Value = serde_json::from_str(String::from_utf8(body.to_vec()).unwrap().as_str()).unwrap();

        assert_eq!(v.get("version").unwrap(), "1.0.0");
        assert_eq!(*v.get("env").unwrap(), serde_json::to_value(Environment::Development).unwrap());
    }
}

use axum::{response::{IntoResponse, Response}, extract::State, http::{StatusCode, header}, body::{Full, Bytes}, Json};
use serde::Serialize;
use serde_json::{Value, json};

use crate::{AppState, Environment};

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

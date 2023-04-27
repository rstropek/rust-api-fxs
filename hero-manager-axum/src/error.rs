// Helpers for error handling

use axum::{
    body::Body,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use http_api_problem::HttpApiProblem;
use std::any::Any;
use validator::ValidationErrors;

/// Represents an application-level error
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("an internal database error occurred")]
    Sqlx(#[from] sqlx::Error),

    #[error("an internal server error occurred")]
    Anyhow(#[from] anyhow::Error),

    #[error("validation error in request body")]
    InvalidEntity(#[from] ValidationErrors),
}

/// Type alias for Results that use our application-level error enum
pub type Result<T, E = Error> = std::result::Result<T, E>;

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let payload = match self {
            Self::InvalidEntity(errors) => HttpApiProblem::new(StatusCode::UNPROCESSABLE_ENTITY)
                .type_url("https://example.com/errors/unprocessable-entity")
                .title("Unprocessable entity in request body")
                .detail(errors.to_string()),
            _ => HttpApiProblem::new(StatusCode::INTERNAL_SERVER_ERROR)
                .type_url("https://example.com/errors/internal-error")
                .title("Internal Server Error"),
        };
        (payload.status.unwrap(), Json(payload)).into_response()
    }
}

pub fn handle_panic(err: Box<dyn Any + Send + 'static>) -> Response<Body> {
    let mut problem = HttpApiProblem::new(StatusCode::INTERNAL_SERVER_ERROR)
        .type_url("https://example.com/errors/internal-error")
        .title("Internal Server Error");

    if let Some(s) = err.downcast_ref::<String>() {
        tracing::error!("Panic: {}", s);
        problem = problem.detail(s)
    } else if let Some(s) = err.downcast_ref::<&str>() {
        tracing::error!("Panic: {}", s);
        problem = problem.detail(s.to_string())
    }

    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&problem).unwrap()))
        .unwrap()
}

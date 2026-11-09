//! Helpers for error handling

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
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
        match self {
            Self::InvalidEntity(errors) => HttpApiProblem::new(StatusCode::UNPROCESSABLE_ENTITY)
                .type_url("https://example.com/errors/unprocessable-entity")
                .title("Unprocessable entity in request body")
                .detail(errors.to_string()),
            Self::Sqlx(sqlx::Error::Database(e)) if e.is_unique_violation() => {
                HttpApiProblem::new(StatusCode::CONFLICT)
                    .type_url("https://example.com/errors/conflict")
                    .title("A hero with this name already exists")
            },
            _ => internal_error(),
        }
        .into_response()
    }
}

/// Builds the problem description for internal errors
///
/// Details about internal errors are logged, but not sent to the client.
fn internal_error() -> HttpApiProblem {
    HttpApiProblem::new(StatusCode::INTERNAL_SERVER_ERROR)
        .type_url("https://example.com/errors/internal-error")
        .title("Internal Server Error")
}

pub fn handle_panic(err: Box<dyn Any + Send + 'static>) -> Response {
    let message = err
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| err.downcast_ref::<&str>().copied())
        .unwrap_or("unknown panic payload");
    tracing::error!("Panic: {message}");

    internal_error().into_response()
}

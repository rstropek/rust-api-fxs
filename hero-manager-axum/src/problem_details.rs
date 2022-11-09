use axum::response::IntoResponse;
use http_api_problem::{HttpApiProblem, StatusCode, into_axum_response};

const UNPROCESSABLE_ENTITY_TYPE: &str = "https://example.com/errors/unprocessable-entity";
const UNPROCESSABLE_ENTITY_TITLE: &str = "Unprocessable entity in request body";

pub enum ProblemDetail<'a> {
    UnprocessableEntity(&'a str),
}

impl<'a> IntoResponse for ProblemDetail<'a> {
    fn into_response(self) -> axum::response::Response {
        match self {
            ProblemDetail::UnprocessableEntity(detail) => {
                let problem = HttpApiProblem::new(StatusCode::UNPROCESSABLE_ENTITY)
                    .type_url(UNPROCESSABLE_ENTITY_TYPE)
                    .title(UNPROCESSABLE_ENTITY_TITLE)
                    .detail(detail);
                into_axum_response(problem).into_response()
            }
        }
    }
}

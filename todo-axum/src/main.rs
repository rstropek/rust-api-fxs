use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use serde_json::json;
use std::{net::SocketAddr, sync::Arc};
use todo_logic::{Pagination, TodoItem, TodoStore, TodoStoreError, UpdateTodoItem};
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

type Db = Arc<RwLock<TodoStore>>;

#[tokio::main]
async fn main() {
    // Enable tracing using Tokio's https://tokio.rs/#tk-lib-tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "todo_axum=debug,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // In-memory store of todos
    let db = Db::default();

    let app = Router::with_state(db)
        .route("/todos", get(get_todos).post(add_todo))
        .route("/todos/persist", post(persist))
        .route("/todos/:id", delete(delete_todo).patch(update_todo).get(get_todo))
        // Using tower to add tracing layer
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()).into_inner());

    // In practice: Use graceful shutdown.
    // Note that Axum has great examples for a log of practical scenarios,
    // including graceful shutdown (https://github.com/tokio-rs/axum/tree/main/examples)
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("listening on {}", addr);
    axum::Server::bind(&addr).serve(app.into_make_service()).await.unwrap();
}

async fn get_todos(pagination: Option<Query<Pagination>>, State(db): State<Db>) -> impl IntoResponse {
    // Note how the Query extractor is used to get query parameters
    // Note how the State extractor is used to get the database (changes in Axum 0.6 RC)

    let todos = db.read().await;
    let Query(pagination) = pagination.unwrap_or_default();
    Json(todos.get_todos(pagination))
}

async fn get_todo(Path(id): Path<usize>, State(db): State<Db>) -> impl IntoResponse {
    // Note how the Path extractor is used to get query parameters

    let todos = db.read().await;
    if let Some(item) = todos.get_todo(id) {
        // Note how to return Json
        Json(item).into_response()
    } else {
        // Note how a tuple can be turned into a response
        (StatusCode::NOT_FOUND, "Not found").into_response()
    }
}

async fn add_todo(State(db): State<Db>, Json(todo): Json<TodoItem>) -> impl IntoResponse {
    let mut todos = db.write().await;
    let todo = todos.add_todo(todo);
    (StatusCode::CREATED, Json(todo))
}

async fn delete_todo(Path(id): Path<usize>, State(db): State<Db>) -> impl IntoResponse {
    if db.write().await.remove_todo(id).is_some() {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn update_todo(
    Path(id): Path<usize>,
    State(db): State<Db>,
    Json(input): Json<UpdateTodoItem>,
) -> Result<impl IntoResponse, StatusCode> {
    let mut todos = db.write().await;
    let res = todos.update_todo(&id, input);
    match res {
        Some(todo) => Ok(Json(todo.clone())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

enum AppError {
    UserRepo(TodoStoreError),
}

impl From<TodoStoreError> for AppError {
    fn from(inner: TodoStoreError) -> Self {
        AppError::UserRepo(inner)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::UserRepo(TodoStoreError::FileAccessError(_)) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Error while writing to file")
            },
            AppError::UserRepo(TodoStoreError::SerializationError(_)) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Error during serialization")
            },
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

async fn persist(State(db): State<Db>) -> Result<(), AppError> {
    tracing::debug!("Persisting todos");
    let todos = db.read().await;
    todos.persist().await?;
    Ok(())
}

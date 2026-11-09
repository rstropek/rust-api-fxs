use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{delete, get, post},
};
use serde_json::json;
use std::sync::Arc;
use todo_logic::{IdentifiableTodoItem, Pagination, TodoItem, TodoStore, TodoStoreError, UpdateTodoItem};
use tokio::{net::TcpListener, sync::RwLock};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

/// Type for our shared state
///
/// In our sample application, we store the todo list in memory. As the state is shared
/// between concurrently running web requests, we need to make it thread-safe.
type Db = Arc<RwLock<TodoStore>>;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Enable logging with tracing. Use the RUST_LOG environment variable to override the filter.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| "todo_axum=debug,tower_http=debug".into()),
        )
        .init();

    // Create shared data store
    let db = Db::default();

    // We register our shared state so that handlers can get it using the State extractor.
    let app = Router::new()
        // Here we setup the routes. Note: No macros
        .route("/", get(say_hello))
        .route("/todos", get(get_todos).post(add_todo))
        .route("/todos/{id}", delete(delete_todo).patch(update_todo).get(get_todo))
        .route("/todos/persist", post(persist))
        .with_state(db)
        // Using tower to add tracing layer
        .layer(TraceLayer::new_for_http());

    // In practice: Use graceful shutdown. The hero-manager-axum sample in this repository
    // shows how to do that. Axum also has great examples for a lot of practical scenarios
    // (https://github.com/tokio-rs/axum/tree/main/examples).
    let listener = TcpListener::bind("0.0.0.0:3000").await?;
    tracing::debug!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await
}

/// Say hello
async fn say_hello() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}

/// Get list of todo items
///
/// Note how the Query extractor is used to get query parameters and how the State
/// extractor is used to get the database. Extractors are types that implement
/// FromRequest or FromRequestParts. You can create your own extractors or use the
/// ones provided by Axum.
async fn get_todos(Query(pagination): Query<Pagination>, State(db): State<Db>) -> Json<Vec<IdentifiableTodoItem>> {
    let todos = db.read().await;
    // Json is an extractor and a response.
    Json(todos.get_todos(pagination))
}

/// Get a single todo item
///
/// Note how the Path extractor is used to get path parameters.
async fn get_todo(Path(id): Path<usize>, State(db): State<Db>) -> Response {
    let todos = db.read().await;
    if let Some(item) = todos.get_todo(id) {
        // Note how to return Json
        Json(item).into_response()
    } else {
        // Note how a tuple can be turned into a response
        (StatusCode::NOT_FOUND, "Not found").into_response()
    }
}

/// Add a new todo item
///
/// Note that this time, Json is used as an extractor. This means that the request body
/// will be deserialized into a TodoItem.
async fn add_todo(State(db): State<Db>, Json(todo): Json<TodoItem>) -> impl IntoResponse {
    let mut todos = db.write().await;
    let todo = todos.add_todo(todo);
    (StatusCode::CREATED, Json(todo))
}

/// Delete a todo item
async fn delete_todo(Path(id): Path<usize>, State(db): State<Db>) -> StatusCode {
    if db.write().await.remove_todo(id).is_some() {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

/// Update a todo item
async fn update_todo(
    Path(id): Path<usize>,
    State(db): State<Db>,
    Json(input): Json<UpdateTodoItem>,
) -> Result<Json<IdentifiableTodoItem>, StatusCode> {
    let mut todos = db.write().await;
    match todos.update_todo(id, input) {
        Some(todo) => Ok(Json(todo.clone())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Application-level error object
enum AppError {
    TodoStore(TodoStoreError),
}

impl From<TodoStoreError> for AppError {
    fn from(inner: TodoStoreError) -> Self {
        AppError::TodoStore(inner)
    }
}

/// Logic for turning an error into a response.
///
/// By providing this trait, handlers can return AppError and Axum will automatically
/// convert it into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::TodoStore(TodoStoreError::FileAccess(_)) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Error while writing to file")
            },
            AppError::TodoStore(TodoStoreError::Serialization(_)) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Error during serialization")
            },
        };

        (status, Json(json!({ "error": error_message }))).into_response()
    }
}

/// Persist the todo store to disk
async fn persist(State(db): State<Db>) -> Result<(), AppError> {
    tracing::debug!("Persisting todos");
    let todos = db.read().await;
    todos.persist().await?;
    Ok(())
}

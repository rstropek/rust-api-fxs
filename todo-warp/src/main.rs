use std::{convert::Infallible, sync::Arc};

use todo_logic::{Pagination, TodoItem, TodoStore, TodoStoreError, UpdateTodoItem};
use tokio::sync::RwLock;
use tracing::debug;
use tracing_subscriber::EnvFilter;
use warp::{Filter, Rejection, Reply, http::StatusCode, reject, reply};

/// Type for our shared state
type Db = Arc<RwLock<TodoStore>>;

#[tokio::main]
async fn main() {
    // Initialize logging. Warp's log filter emits its messages with the log crate.
    // tracing-subscriber forwards them to tracing, so we get them together with our own
    // events. Use the RUST_LOG environment variable to override the filter.
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "todo_warp=debug".into()))
        .init();

    // Create shared data store
    let db = Db::default();

    // This filter injects our shared state into the handler functions. Filters
    // are cheap to clone, so we can reuse it for all of our routes.
    let with_db = warp::any().map(move || db.clone());

    // Note that you would probably create dedicated functions for each filter.
    // However, to make Warp's approach more obvious, we'll inline the filters.
    // Note that Warp makes less use of macros than e.g. Rocket. Only the route
    // is defined using a kind of DSL. Everything else is specified using filters.
    // The filters also ensure that the handler function has the correct signature.
    let get = warp::path!("todos")
        .and(warp::get())
        // The query filter is used to extract the query parameters.
        .and(warp::query::<Pagination>())
        .and(with_db.clone())
        // ...and finally we connect the handler.
        .and_then(get_todos);

    let add = warp::path!("todos")
        .and(warp::post())
        // The body filter is used to extract the request body (JSON).
        .and(warp::body::json())
        .and(with_db.clone())
        .and_then(add_todo);

    let get_single = warp::path!("todos" / usize)
        .and(warp::get())
        .and(with_db.clone())
        .and_then(get_todo);

    let delete = warp::path!("todos" / usize)
        .and(warp::delete())
        .and(with_db.clone())
        .and_then(delete_todo);

    let update = warp::path!("todos" / usize)
        .and(warp::patch())
        .and(warp::body::json())
        .and(with_db.clone())
        .and_then(update_todo);

    let persist = warp::path!("todos" / "persist")
        .and(warp::post())
        .and(with_db)
        .and_then(persist);

    // The final API consists of all the filters we defined above
    // connected with the `or` combinator.
    let api = get
        .or(add)
        .or(get_single)
        .or(delete)
        .or(update)
        .or(persist)
        // Handlers can return a Rejection in case of an error. Rejections are
        // handled by the `recover` filter. It turns the error object into a response.
        // It has to be applied to the whole API so that it sees the final rejection
        // after all routes have been tried.
        .recover(handle_rejection);

    // For logging, we wrap the API with a wrapping filter (similar to a middleware
    // in other frameworks).
    let routes = api.with(warp::log("todo_warp"));
    warp::serve(routes).run(([0, 0, 0, 0], 3000)).await;
}

/// Get list of todo items
///
/// Note that we do not need any special handling of the parameters.
/// The previously defined filters already extracted query parameters,
/// body, path parameters, etc.
async fn get_todos(pagination: Pagination, db: Db) -> Result<impl Reply, Infallible> {
    let todos = db.read().await;
    Ok(reply::json(&todos.get_todos(pagination)))
}

/// Get a single todo item
///
/// Note that this method returns different return types.
/// into_response converts the result into a reply.
async fn get_todo(id: usize, db: Db) -> Result<impl Reply, Infallible> {
    let todos = db.read().await;
    if let Some(item) = todos.get_todo(id) {
        Ok(reply::json(item).into_response())
    } else {
        Ok(reply::with_status("Not found", StatusCode::NOT_FOUND).into_response())
    }
}

/// Add a new todo item
async fn add_todo(todo: TodoItem, db: Db) -> Result<impl Reply, Infallible> {
    let mut todos = db.write().await;
    let todo = todos.add_todo(todo);
    Ok(reply::with_status(reply::json(&todo), StatusCode::CREATED))
}

/// Delete a todo item
async fn delete_todo(id: usize, db: Db) -> Result<impl Reply, Infallible> {
    if db.write().await.remove_todo(id).is_some() {
        Ok(reply::with_status("", StatusCode::NO_CONTENT))
    } else {
        Ok(reply::with_status("", StatusCode::NOT_FOUND))
    }
}

/// Update a todo item
async fn update_todo(id: usize, input: UpdateTodoItem, db: Db) -> Result<impl Reply, Infallible> {
    let mut todos = db.write().await;
    match todos.update_todo(id, input) {
        Some(todo) => Ok(reply::json(todo).into_response()),
        None => Ok(reply::with_status("", StatusCode::NOT_FOUND).into_response()),
    }
}

/// Application-level error object
#[derive(Debug)]
enum AppError {
    TodoStore(TodoStoreError),
}

impl From<TodoStoreError> for AppError {
    fn from(inner: TodoStoreError) -> Self {
        AppError::TodoStore(inner)
    }
}

/// Add marker trait to AppError for custom rejections
impl reject::Reject for AppError {}

async fn persist(db: Db) -> Result<impl Reply, Rejection> {
    // Write a log message
    debug!("Persisting todos");

    let todos = db.read().await;
    // In case of an error, we return a custom rejection. It will be handled
    // by the `recover` filter.
    todos.persist().await.map_err(|e| reject::custom(AppError::from(e)))?;
    Ok(StatusCode::OK)
}

/// Handles custom rejections and turns them into a response.
///
/// All other rejections (e.g. not found, invalid body) are passed on so
/// that Warp can generate its default response for them.
async fn handle_rejection(err: Rejection) -> Result<impl Reply, Rejection> {
    match err.find::<AppError>() {
        Some(AppError::TodoStore(e)) => Ok(reply::with_status(
            match e {
                TodoStoreError::FileAccess(_) => "Error while writing to file",
                TodoStoreError::Serialization(_) => "Error during serialization",
            },
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
        None => Err(err),
    }
}

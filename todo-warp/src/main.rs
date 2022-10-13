use std::{convert::Infallible, sync::Arc};

use log::{debug, LevelFilter};
use simplelog::{Config, SimpleLogger};
use todo_logic::{Pagination, TodoItem, TodoStore, TodoStoreError, UpdateTodoItem};
use tokio::sync::RwLock;
use warp::http::StatusCode;
use warp::{reject, reply};
use warp::{Filter, Rejection, Reply};

/// Type for our shared state
type Db = Arc<RwLock<TodoStore>>;

#[tokio::main]
async fn main() {
    // Initialize logging.
    // Warp uses the log crate (https://crates.io/crates/log) to log requests. You can use any
    // compatible logger, but for this example we'll use simplelog.
    SimpleLogger::init(LevelFilter::Debug, Config::default()).unwrap();

    // Create shared data store
    let db = Db::default();

    // Note that you would probably create dedicated functions for each filter.
    // However, to make Warp's approach more obvious, we'll inline the filters.
    // Note that Warp makes less use of macros than e.g. Rocket. Only the route
    // is defined using a kind of DSL. Everything else is specified using filters.
    // The filters also ensure that the handler function has the correct signature.
    let get_db = db.clone();
    let get = warp::path!("todos")
        .and(warp::get())
        // The query filter is used to extract the query parameters.
        .and(warp::query::<Pagination>())
        // Here we inject our shared state into the handler function.
        .and(warp::any().map(move || get_db.clone()))
        // ...and finally we connect the handler.
        .and_then(get_todos);

    let add_db = db.clone();
    let add = warp::path!("todos")
        .and(warp::post())
        // The body filter is used to extract the request body (JSON).
        .and(warp::body::json())
        .and(warp::any().map(move || add_db.clone()))
        .and_then(add_todo);

    let get_single_db = db.clone();
    let get_single = warp::path!("todos" / usize)
        .and(warp::get())
        .and(warp::any().map(move || get_single_db.clone()))
        .and_then(get_todo);

    let delete_db = db.clone();
    let delete = warp::path!("todos" / usize)
        .and(warp::delete())
        .and(warp::any().map(move || delete_db.clone()))
        .and_then(delete_todo);

    let update_db = db.clone();
    let update = warp::path!("todos" / usize)
        .and(warp::patch())
        .and(warp::body::json())
        .and(warp::any().map(move || update_db.clone()))
        .and_then(update_todo);

    let persist_db = db.clone();
    let persist = warp::path!("todos" / "persist")
        .and(warp::post())
        .and(warp::any().map(move || persist_db.clone()))
        .and_then(persist)
        // The persist can handler can return a Rejection in case of an error.
        // Rejections are handled by the `recover` filter. It turns the error
        // object into a response.
        .recover(handle_rejection);

    // The final API consists of all the filters we defined above
    // connected with the `or` combinator.
    let api = get.or(add).or(get_single).or(delete).or(update).or(persist);

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
async fn get_todos(pagination: Pagination, db: Db) -> Result<impl warp::Reply, Infallible> {
    let todos = db.read().await;
    Ok(reply::json(&todos.get_todos(pagination)))
}

/// Get a single todo item
///
/// Note that this method returns different return types.
/// into_response converts the result into a reply.
async fn get_todo(id: usize, db: Db) -> Result<impl warp::Reply, Infallible> {
    let todos = db.read().await;
    if let Some(item) = todos.get_todo(id) {
        Ok(reply::json(item).into_response())
    } else {
        Ok(reply::with_status("Not found", StatusCode::NOT_FOUND).into_response())
    }
}

/// Add a new todo item
async fn add_todo(todo: TodoItem, db: Db) -> Result<impl warp::Reply, Infallible> {
    let mut todos = db.write().await;
    let todo = todos.add_todo(todo.clone());
    Ok(reply::json(&todo))
}

/// Delete a todo item
async fn delete_todo(id: usize, db: Db) -> Result<impl warp::Reply, Infallible> {
    if db.write().await.remove_todo(id).is_some() {
        Ok(reply::with_status("", StatusCode::NO_CONTENT))
    } else {
        Ok(reply::with_status("", StatusCode::NOT_FOUND))
    }
}

/// Update a todo item
async fn update_todo(id: usize, input: UpdateTodoItem, db: Db) -> Result<impl warp::Reply, Infallible> {
    let mut todos = db.write().await;
    let res = todos.update_todo(&id, input);
    match res {
        Some(todo) => Ok(reply::json(todo).into_response()),
        None => Ok(reply::with_status("", StatusCode::NOT_FOUND).into_response()),
    }
}

/// Application-level error object
#[derive(Debug)]
enum AppError {
    UserRepo(TodoStoreError),
}
impl From<TodoStoreError> for AppError {
    fn from(inner: TodoStoreError) -> Self {
        AppError::UserRepo(inner)
    }
}

/// Add marker trait to AppError for custom rejections
impl reject::Reject for AppError {}

async fn persist(db: Db) -> Result<impl warp::Reply, Rejection> {
    // Write a log message
    debug!("Persisting todos");

    let todos = db.read().await;
    todos
        .persist()
        .await
        // In case of an error, we return a custom rejection. It will be handled
        // by teh `recover` filter.
        .map_err(|e| warp::reject::custom::<AppError>(e.into()))?;
    Ok::<_, Rejection>(reply::with_status("", StatusCode::OK).into_response())
}

/// Handles custom rejection and turns it into a response.
async fn handle_rejection(err: Rejection) -> Result<impl Reply, std::convert::Infallible> {
    if let Some(e) = err.find::<AppError>() {
        return match e {
            AppError::UserRepo(e) => Ok(reply::with_status(
                match e {
                    TodoStoreError::FileAccessError(_) => "Error while writing to file",
                    TodoStoreError::SerializationError(_) => "Error during serialization",
                },
                StatusCode::INTERNAL_SERVER_ERROR,
            )),
        };
    }
    Ok(reply::with_status(
        "INTERNAL_SERVER_ERROR",
        StatusCode::INTERNAL_SERVER_ERROR,
    ))
}

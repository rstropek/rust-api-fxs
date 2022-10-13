use std::{convert::Infallible, sync::Arc};

use log::{debug, LevelFilter};
use simplelog::{Config, SimpleLogger};
use todo_logic::{Pagination, TodoItem, TodoStore, TodoStoreError, UpdateTodoItem};
use tokio::sync::RwLock;
use warp::http::StatusCode;
use warp::reject;
use warp::{Filter, Rejection, Reply};

type Db = Arc<RwLock<TodoStore>>;

#[tokio::main]
async fn main() {
    let db = Db::default();
    SimpleLogger::init(LevelFilter::Debug, Config::default()).unwrap();

    let get_db = db.clone();
    let get = warp::path!("todos")
        .and(warp::get())
        .and(warp::query::<Pagination>())
        .and(warp::any().map(move || get_db.clone()))
        .and_then(get_todos);

    let add_db = db.clone();
    let add = warp::path!("todos")
        .and(warp::post())
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
        .recover(handle_rejection);

    let api = get.or(add).or(get_single).or(delete).or(update).or(persist);

    let routes = api.with(warp::log("todo_warp"));
    warp::serve(routes).run(([0, 0, 0, 0], 3000)).await;
}

async fn get_todos(pagination: Pagination, db: Db) -> Result<impl warp::Reply, Infallible> {
    let todos = db.read().await;
    Ok(warp::reply::json(&todos.get_todos(pagination)))
}

async fn get_todo(id: usize, db: Db) -> Result<impl warp::Reply, Infallible> {
    let todos = db.read().await;
    if let Some(item) = todos.get_todo(id) {
        // Note how to return Json
        Ok(warp::reply::json(item).into_response())
    } else {
        // Note how a tuple can be turned into a response
        Ok(warp::reply::with_status("Not found", StatusCode::NOT_FOUND).into_response())
    }
}

async fn add_todo(todo: TodoItem, db: Db) -> Result<impl warp::Reply, Infallible> {
    let mut todos = db.write().await;
    let todo = todos.add_todo(todo.clone());
    Ok(warp::reply::json(&todo))
}

async fn delete_todo(id: usize, db: Db) -> Result<impl warp::Reply, Infallible> {
    if db.write().await.remove_todo(id).is_some() {
        Ok(warp::reply::with_status("", StatusCode::NO_CONTENT))
    } else {
        Ok(warp::reply::with_status("", StatusCode::NOT_FOUND))
    }
}

async fn update_todo(id: usize, input: UpdateTodoItem, db: Db) -> Result<impl warp::Reply, Infallible> {
    let mut todos = db.write().await;
    let res = todos.update_todo(&id, input);
    match res {
        Some(todo) => Ok(warp::reply::json(todo).into_response()),
        None => Ok(warp::reply::with_status("", StatusCode::NOT_FOUND).into_response()),
    }
}

#[derive(Debug)]
enum AppError {
    UserRepo(TodoStoreError),
}

impl From<TodoStoreError> for AppError {
    fn from(inner: TodoStoreError) -> Self {
        AppError::UserRepo(inner)
    }
}

impl reject::Reject for AppError {}

async fn persist(db: Db) -> Result<impl warp::Reply, Rejection> {
    debug!("Persisting todos");
    let todos = db.read().await;
    todos.persist().await.map_err(|e| warp::reject::custom::<AppError>(e.into()))?;
    Ok::<_, Rejection>(warp::reply::with_status("", StatusCode::OK).into_response())
}

async fn handle_rejection(err: Rejection) -> Result<impl Reply, std::convert::Infallible> {
    if let Some(e) = err.find::<AppError>() {
        return match e {
            AppError::UserRepo(e) =>
                Ok(warp::reply::with_status(
                    match e {
                        TodoStoreError::FileAccessError(_) => "Error while writing to file",
                        TodoStoreError::SerializationError(_) => "Error during serialization",
                    },
                    StatusCode::INTERNAL_SERVER_ERROR,
                ))
        };
    }
    Ok(warp::reply::with_status(
        "INTERNAL_SERVER_ERROR",
        StatusCode::INTERNAL_SERVER_ERROR,
    ))
}

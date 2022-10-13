#[macro_use]
extern crate rocket;

use log::{debug, LevelFilter};
use rocket::http::Status;
use rocket::response::status::Created;
use rocket::serde::json::Json;
use rocket::tokio::sync::RwLock;
use rocket::{uri, State};
use simplelog::{Config, SimpleLogger};
use std::sync::Arc;
use todo_logic::{IdentifyableTodoItem, Pagination, TodoItem, TodoStore, TodoStoreError, UpdateTodoItem};

type Db = Arc<RwLock<TodoStore>>;

#[launch]
fn rocket() -> _ {
    let db = Db::default();

    SimpleLogger::init(LevelFilter::Debug, Config::default()).unwrap();

    rocket::build()
        .mount(
            "/",
            routes![get_todos, get_todo, add_todo, update_todo, delete_todo, persist],
        )
        .manage(db)
}

#[get("/todos?<offset>&<limit>")]
async fn get_todos(offset: Option<usize>, limit: Option<usize>, db: &State<Db>) -> Json<Vec<IdentifyableTodoItem>> {
    let todos = db.read().await;
    let pagination = Pagination::new(offset, limit);
    Json(todos.get_todos(pagination))
}

#[get("/todos/<id>")]
async fn get_todo(id: usize, db: &State<Db>) -> Option<Json<IdentifyableTodoItem>> {
    let todos = db.read().await;
    todos.get_todo(id).map(|item| Json(item.clone()))
}

#[post("/todos", format = "json", data = "<todo>")]
async fn add_todo(todo: Json<TodoItem>, db: &State<Db>) -> Created<Json<IdentifyableTodoItem>> {
    let mut todos = db.write().await;
    let todo = todos.add_todo(todo.0);
    let location = uri!("/", get_todo(todo.id));
    Created::new(location.to_string()).body(Json(todo))
}

#[delete("/todos/<id>")]
async fn delete_todo(id: usize, db: &State<Db>) -> Status {
    if db.write().await.remove_todo(id).is_some() {
        Status::NoContent
    } else {
        Status::NotFound
    }
}

#[patch("/todos/<id>", format = "json", data = "<input>")]
async fn update_todo(id: usize, input: Json<UpdateTodoItem>, db: &State<Db>) -> Option<Json<IdentifyableTodoItem>> {
    let mut todos = db.write().await;
    let res = todos.update_todo(&id, input.0);
    res.map(|todo| Json(todo.clone()))
}

#[derive(Responder)]
enum AppError {
    #[response(status = 500)]
    InternalError(String),
}

impl From<TodoStoreError> for AppError {
    fn from(inner: TodoStoreError) -> Self {
        AppError::InternalError(Json(inner).to_string())
    }
}

#[post("/todos/persist")]
async fn persist(db: &State<Db>) -> Result<(), AppError> {
    debug!("Persisting todos");
    let todos = db.read().await;
    todos.persist().await?;
    Ok(())
}

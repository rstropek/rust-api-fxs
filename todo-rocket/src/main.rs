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

/// Type for our shared state
///
/// In our sample application, we store the todo list in memory. As the state is shared
/// between concurrently running web requests, we need to make it thread-safe.
type Db = Arc<RwLock<TodoStore>>;

/// Rocket relies heavily on macros. The launch macro will generate a
/// tokio main function for us.
#[launch]
fn rocket() -> _ {
    // Initialize logging.
    // Rocket uses the log crate (https://crates.io/crates/log) to log requests. You can use any
    // compatible logger, but for this example we'll use simplelog. Enhancements in terms
    // of more flexible logging are planned for future releases
    // (https://github.com/SergioBenitez/Rocket/issues/21).
    SimpleLogger::init(LevelFilter::Debug, Config::default()).unwrap();

    // Create shared data store
    let db = Db::default();

    rocket::build()
        // Here we mount our routes. More details about route mounting
        // at https://rocket.rs/v0.5-rc/guide/overview/#mounting.
        .mount(
            "/",
            routes![get_todos, get_todo, add_todo, update_todo, delete_todo, persist],
        )
        // Register our shared state.
        // More about using shared state at https://rocket.rs/v0.5-rc/guide/state/.
        .manage(db)
}

/// Get list of todo items
///
/// Rocket implements the FromParam trait for typically used data types so they
/// can be used to extract data from the query string. Of course you can implement
/// FromParam for custom data types, too.
/// More about it at https://rocket.rs/v0.5-rc/guide/requests/#query-strings.
///
/// Also note the Responder trait (https://rocket.rs/v0.5-rc/guide/responses/#custom-responders).
/// Rocket comes with a lot of built-in responders, but you can also
/// implement the trait for your own custom types.
#[get("/todos?<offset>&<limit>")]
async fn get_todos(offset: Option<usize>, limit: Option<usize>, db: &State<Db>) -> Json<Vec<IdentifyableTodoItem>> {
    let todos = db.read().await;
    let pagination = Pagination::new(offset, limit);
    Json(todos.get_todos(pagination))
}

/// Get a single todo item
///
/// Note that Option<T> implements the Responder trait, too. This makes it really
/// simple to return a 404 if the requested item does not exist.
#[get("/todos/<id>")]
async fn get_todo(id: usize, db: &State<Db>) -> Option<Json<IdentifyableTodoItem>> {
    let todos = db.read().await;
    todos.get_todo(id).map(|item| Json(item.clone()))
}

/// Add a new todo item
///
/// Note the use of a "Request Guard" (FromRequest trait) here. Here it is used
/// to extract the JSON body of the request. You can implement your own guards, too
/// (https://rocket.rs/v0.5-rc/guide/requests/#custom-guards). Many things that you
/// would do with middlewares in other frameworks are done with request guards in Rocket.
#[post("/todos", format = "json", data = "<todo>")]
async fn add_todo(todo: Json<TodoItem>, db: &State<Db>) -> Created<Json<IdentifyableTodoItem>> {
    let mut todos = db.write().await;
    let todo = todos.add_todo(todo.0);

    // Nice detail here: The uri macro helps you to generate URIs for your routes.
    // Very useful for building the location header.
    let location = uri!("/", get_todo(todo.id));
    Created::new(location.to_string()).body(Json(todo))
}

/// Delete a todo item
///
/// Note the extraction of the id from the path.
#[delete("/todos/<id>")]
async fn delete_todo(id: usize, db: &State<Db>) -> Status {
    match db.write().await.remove_todo(id) {
        // Note that Status represents the HTTP status code
        Some(_) => Status::NoContent,
        None => Status::NotFound,
    }
}

/// Update a todo item
#[patch("/todos/<id>", format = "json", data = "<input>")]
async fn update_todo(id: usize, input: Json<UpdateTodoItem>, db: &State<Db>) -> Option<Json<IdentifyableTodoItem>> {
    let mut todos = db.write().await;
    let res = todos.update_todo(&id, input.0);
    res.map(|todo| Json(todo.clone()))
}

/// Application-level error object
///
/// Note how easy it is to implement Rocket's Responder trait with
/// the macros that Rocket provides.
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

/// Persist the todo store to disk
#[post("/todos/persist")]
async fn persist(db: &State<Db>) -> Result<(), AppError> {
    debug!("Persisting todos");
    let todos = db.read().await;
    todos.persist().await?;
    Ok(())
}

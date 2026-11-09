use rocket::{
    Responder, State, delete, get, http::Status, launch, patch, post, response::status::Created, routes,
    serde::json::Json, tokio::sync::RwLock, uri,
};
use todo_logic::{IdentifiableTodoItem, Pagination, TodoItem, TodoStore, TodoStoreError, UpdateTodoItem};
use tracing::debug;
use tracing_subscriber::EnvFilter;

/// Type for our shared state
///
/// In our sample application, we store the todo list in memory. As the state is shared
/// between concurrently running web requests, we need to make it thread-safe. Note that we
/// do not need an Arc here because Rocket hands out references to its managed state.
type Db = RwLock<TodoStore>;

/// Rocket relies heavily on macros. The launch macro will generate a
/// tokio main function for us.
#[launch]
fn rocket() -> _ {
    // Initialize logging. Rocket 0.5 emits its messages with the log crate. tracing-subscriber
    // forwards them to tracing, so we get them together with our own events. Use the RUST_LOG
    // environment variable to override the filter.
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "todo_rocket=debug,rocket=info".into()))
        .init();

    rocket::build()
        // Here we mount our routes. More details about route mounting
        // at https://rocket.rs/guide/v0.5/overview/#mounting.
        .mount(
            "/",
            routes![get_todos, get_todo, add_todo, update_todo, delete_todo, persist],
        )
        // Register our shared state.
        // More about using shared state at https://rocket.rs/guide/v0.5/state/.
        .manage(Db::default())
}

/// Get list of todo items
///
/// Rocket implements the FromForm trait for typically used data types so they
/// can be used to extract data from the query string. Of course you can implement
/// FromForm for custom data types, too.
/// More about it at https://rocket.rs/guide/v0.5/requests/#query-strings.
///
/// Also note the Responder trait (https://rocket.rs/guide/v0.5/responses/#custom-responders).
/// Rocket comes with a lot of built-in responders, but you can also
/// implement the trait for your own custom types.
#[get("/todos?<offset>&<limit>")]
async fn get_todos(offset: Option<usize>, limit: Option<usize>, db: &State<Db>) -> Json<Vec<IdentifiableTodoItem>> {
    let todos = db.read().await;
    Json(todos.get_todos(Pagination::new(offset, limit)))
}

/// Get a single todo item
///
/// Note that Option<T> implements the Responder trait, too. This makes it really
/// simple to return a 404 if the requested item does not exist.
#[get("/todos/<id>")]
async fn get_todo(id: usize, db: &State<Db>) -> Option<Json<IdentifiableTodoItem>> {
    let todos = db.read().await;
    todos.get_todo(id).cloned().map(Json)
}

/// Add a new todo item
///
/// Note the use of a data guard (FromData trait) here. It is used to extract the JSON
/// body of the request. You can implement your own guards, too
/// (https://rocket.rs/guide/v0.5/requests/#custom-guards). Many things that you
/// would do with middlewares in other frameworks are done with request guards in Rocket.
#[post("/todos", format = "json", data = "<todo>")]
async fn add_todo(todo: Json<TodoItem>, db: &State<Db>) -> Created<Json<IdentifiableTodoItem>> {
    let mut todos = db.write().await;
    let todo = todos.add_todo(todo.into_inner());

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
async fn update_todo(id: usize, input: Json<UpdateTodoItem>, db: &State<Db>) -> Option<Json<IdentifiableTodoItem>> {
    let mut todos = db.write().await;
    todos.update_todo(id, input.into_inner()).cloned().map(Json)
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
        AppError::InternalError(inner.to_string())
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

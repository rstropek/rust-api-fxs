use actix_web::{
    App, Either, HttpResponse, HttpServer, Responder, ResponseError, delete, get,
    http::StatusCode,
    middleware::Logger,
    patch, post, web,
    web::{Data, Json, Path, Query},
};
use std::fmt::{self, Display};
use todo_logic::{IdentifiableTodoItem, Pagination, TodoItem, TodoStore, TodoStoreError, UpdateTodoItem};
use tokio::sync::RwLock;
use tracing::debug;
use tracing_subscriber::EnvFilter;

/// Type for our shared state
///
/// Note that we do not need an additional Arc here because Actix's Data
/// is already reference-counted.
type Db = RwLock<TodoStore>;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logging. Actix's Logger middleware
    // (https://docs.rs/actix-web/latest/actix_web/middleware/struct.Logger.html) emits its
    // messages with the log crate. tracing-subscriber forwards them to tracing, so we get
    // them together with our own events. Use the RUST_LOG environment variable to override
    // the filter.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| "todo_actix_web=debug,actix_web=info".into()),
        )
        .init();

    // Create shared data store
    let state = Data::new(Db::default());

    HttpServer::new(move || {
        App::new()
            // Register a middleware to log requests.
            // More about writing custom middleware at https://actix.rs/docs/middleware/
            .wrap(Logger::default())
            // Register our shared state.
            // More about using shared state at https://actix.rs/docs/application/
            .app_data(state.clone())
            // Register our routes. Actix supports working with (service)
            // and without macros (route).
            .service(get_todos)
            .service(add_todo)
            .service(delete_todo)
            .service(update_todo)
            .service(persist)
            .route("/todos/{id}", web::get().to(get_todo))
    })
    // Start the server.
    // More about server at https://actix.rs/docs/server/
    .bind(("0.0.0.0", 3000))?
    .run()
    .await
}

/// Get list of todo items
///
/// Note the use of Extractors to extract data from the query
/// and the shared state. More about extractors at
/// https://actix.rs/docs/extractors/.
///
/// Also note the Responder trait (https://actix.rs/docs/handlers/).
/// Actix comes with a lot of built-in responders, but you can also
/// implement your own.
#[get("/todos")]
async fn get_todos(Query(pagination): Query<Pagination>, db: Data<Db>) -> impl Responder {
    let todos = db.read().await;
    Json(todos.get_todos(pagination))
}

/// If a method returns different return types, Actix offers
/// the Either enum (https://actix.rs/docs/handlers/).
type ItemOrStatus = Either<Json<IdentifiableTodoItem>, HttpResponse>;

/// Get a single todo item
async fn get_todo(id: Path<usize>, db: Data<Db>) -> ItemOrStatus {
    let todos = db.read().await;
    if let Some(item) = todos.get_todo(*id) {
        Either::Left(Json(item.clone()))
    } else {
        // Use HttpResponse to build responses with status code,
        // body, headers, etc.
        Either::Right(HttpResponse::NotFound().body("Not found"))
    }
}

/// Add a new todo item
///
/// Note the use of the Json extractor to extract the body.
#[post("/todos")]
async fn add_todo(db: Data<Db>, Json(todo): Json<TodoItem>) -> impl Responder {
    let mut todos = db.write().await;
    let todo = todos.add_todo(todo);
    HttpResponse::Created().json(todo)
}

/// Delete a todo item
///
/// Note the use of another Extractor, Path, to extract the id.
#[delete("/todos/{id}")]
async fn delete_todo(id: Path<usize>, db: Data<Db>) -> impl Responder {
    match db.write().await.remove_todo(*id) {
        Some(_) => HttpResponse::NoContent(),
        None => HttpResponse::NotFound(),
    }
}

/// Update a todo item
#[patch("/todos/{id}")]
async fn update_todo(id: Path<usize>, db: Data<Db>, Json(input): Json<UpdateTodoItem>) -> ItemOrStatus {
    let mut todos = db.write().await;
    match todos.update_todo(*id, input) {
        Some(todo) => Either::Left(Json(todo.clone())),
        None => Either::Right(HttpResponse::NotFound().finish()),
    }
}

/// Application-level error object
#[derive(Debug)]
enum AppError {
    TodoStore(TodoStoreError),
    // In practice, we would have more error types here.
}

impl From<TodoStoreError> for AppError {
    fn from(inner: TodoStoreError) -> Self {
        AppError::TodoStore(inner)
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::TodoStore(e) => write!(f, "Todo store related error: {e}"),
            // In practice, we would have more error types here.
        }
    }
}

/// Implement a custom error response.
///
/// More about error handling at https://actix.rs/docs/errors/.
impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).json(match self {
            AppError::TodoStore(TodoStoreError::FileAccess(_)) => "Error while writing to file",
            AppError::TodoStore(TodoStoreError::Serialization(_)) => "Error during serialization",
        })
    }
}

/// Persist the todo store to disk
///
/// Note the return type here. We can return our custom error type
/// AppError as it implements ResponseError.
#[post("/todos/persist")]
async fn persist(db: Data<Db>) -> Result<&'static str, AppError> {
    // Write a log message
    debug!("Persisting todos");

    let todos = db.read().await;
    todos.persist().await?;
    Ok("")
}

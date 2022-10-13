use actix_web::{
    delete, get,
    http::StatusCode,
    middleware::Logger,
    patch, post, web,
    web::{Data, Json, Path, Query},
    App, Either, HttpResponse, HttpServer, Responder, ResponseError,
};
use log::debug;
use simplelog::{Config, LevelFilter, SimpleLogger};
use std::{fmt::Display, sync::Arc};
use todo_logic::{IdentifyableTodoItem, Pagination, TodoItem, TodoStore, TodoStoreError, UpdateTodoItem};
use tokio::sync::RwLock;

/// Type for our shared state
type Db = Arc<RwLock<TodoStore>>;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logging.
    // Actix's Logger middleware (https://actix.rs/actix-web/actix_web/middleware/struct.Logger.html)
    // uses the log crate (https://crates.io/crates/log) to log requests. You can use any
    // compatible logger, but for this example we'll use simplelog.
    SimpleLogger::init(LevelFilter::Debug, Config::default()).unwrap();

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
/// Also note the Responder trait (https://actix.rs/docs/extractors/).
/// Actix comes with a lot of built-in responders, but you can also
/// implement your own.
#[get("/todos")]
async fn get_todos(pagination: Query<Pagination>, db: Data<Db>) -> impl Responder {
    let todos = db.read().await;
    let Query(pagination) = pagination;
    Json(todos.get_todos(pagination))
}

/// If a method returns different return types, Actix offers
/// the Either enum (https://actix.rs/docs/handlers/).
type ItemOrStatus = Either<Json<IdentifyableTodoItem>, HttpResponse>;

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
async fn add_todo(db: Data<Db>, todo: Json<TodoItem>) -> impl Responder {
    let mut todos = db.write().await;
    let todo = todos.add_todo(todo.clone());
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
async fn update_todo(id: Path<usize>, db: Data<Db>, input: Json<UpdateTodoItem>) -> ItemOrStatus {
    let mut todos = db.write().await;
    let res = todos.update_todo(&id, input.into_inner());
    match res {
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
    fn status_code(&self) -> actix_web::http::StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::build(self.status_code()).json(match self {
            AppError::TodoStore(e) => match e {
                TodoStoreError::FileAccessError(_) => "Error while writing to file",
                TodoStoreError::SerializationError(_) => "Error during serialization",
            },
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

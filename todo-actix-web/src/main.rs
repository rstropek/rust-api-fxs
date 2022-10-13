use std::fmt::Display;

use actix_web::{get, web::Query, HttpServer, App, web, Responder, HttpResponse, Either, post, delete, patch, error, http::StatusCode, middleware::Logger};
use simplelog::{SimpleLogger, LevelFilter, Config};
use todo_logic::{TodoStore, Pagination, IdentifyableTodoItem, TodoItem, UpdateTodoItem, TodoStoreError};
use tokio::sync::RwLock;
use log::{debug};

type Db = RwLock<TodoStore>;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let state = web::Data::new(Db::default());
    SimpleLogger::init(LevelFilter::Debug, Config::default()).unwrap();

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(state.clone())
            .service(get_todos)
            .service(add_todo)
            .service(delete_todo)
            .service(update_todo)
            .service(persist)
            .route("/todos/{id}", web::get().to(get_todo))
    })
    .bind(("0.0.0.0", 3000))?
    .run()
    .await
}

#[get("/todos")]
async fn get_todos(pagination: Query<Pagination>, db: web::Data<Db>) -> impl Responder {
    let todos = db.read().await;
    let Query(pagination) = pagination;
    web::Json(todos.get_todos(pagination))
}

type ItemOrStatus = Either<web::Json<IdentifyableTodoItem>, HttpResponse>;

async fn get_todo(id: web::Path<usize>, db: web::Data<Db>) -> ItemOrStatus {
    let todos = db.read().await;
    if let Some(item) = todos.get_todo(*id) {
        // Note how to return Json
        Either::Left(web::Json(item.clone()))
    } else {
        // Note how a tuple can be turned into a response
        Either::Right(HttpResponse::NotFound().body("Not found"))
    }
}

#[post("/todos")]
async fn add_todo(db: web::Data<Db>, todo: web::Json<TodoItem>) -> impl Responder {
    let mut todos = db.write().await;
    let todo = todos.add_todo(todo.clone());
    HttpResponse::Created().json(todo)
}

#[delete("/todos/{id}")]
async fn delete_todo(id: web::Path<usize>, db: web::Data<Db>) -> impl Responder {
    if db.write().await.remove_todo(*id).is_some() {
        HttpResponse::NoContent()
    } else {
        HttpResponse::NotFound()
    }
}

#[patch("/todos/{id}")]
async fn update_todo(
    id: web::Path<usize>,
    db: web::Data<Db>,
    input: web::Json<UpdateTodoItem>,
) -> ItemOrStatus {
    let mut todos = db.write().await;
    let res = todos.update_todo(&id, input.into_inner());
    match res {
        Some(todo) => Either::Left(web::Json(todo.clone())),
        None => Either::Right(HttpResponse::NotFound().finish()),
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

impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::UserRepo(e) => write!(f, "{}", e),
        }
    }
}

impl error::ResponseError for AppError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::build(self.status_code())
            .json(match self {
                AppError::UserRepo(e) => match e {
                    TodoStoreError::FileAccessError(_) => "Error while writing to file",
                    TodoStoreError::SerializationError(_) => "Error during serialization",
                },
            })
    }
}

#[post("/todos/persist")]
async fn persist(db: web::Data<Db>) -> Result<&'static str, AppError> {
    debug!("Persisting todos");
    let todos = db.read().await;
    todos.persist().await?;
    Ok("")
}

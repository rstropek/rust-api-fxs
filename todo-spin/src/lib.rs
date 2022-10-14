use anyhow::Result;
use http::{Method, StatusCode};
use spin_sdk::{
    http::{Request, Response},
    http_component,
};
use todo_logic::{IdentifyableTodoItem, Pagination, TodoItem, TodoStore};

mod extractors;
mod responders;
use crate::{extractors::{extract_db, extract_pagination, extract_todo_item, extract_id}, responders::to_response};

#[http_component]
fn todo_manager(req: Request) -> Result<Response> {
    let path = req.uri().path().to_string();

    // In Spin, we cannot store data in memory. We have to persist or round-trip it anywhere.
    // In this simple example, we use cookies to store the todos. We use a hand-written
    // "extractor" to get the store from Spin's session cookie.
    let mut db = extract_db(&req);

    // In Spin, we don't have a fancy router yet. We have to manually match the path.
    if path.ends_with("/todos") || path.ends_with("/todos/") {
        match *req.method() {
            Method::GET => {
                // In Spin, there are no "extractors" yet. We have to manually get the
                // pagination data out of the query string.
                let pagination = extract_pagination(&req);
                let result = get_todos(pagination, &db);

                // In Spin, there are no "responders" yet. We have to manually turn
                // our result into a HTTP response.
                to_response(StatusCode::OK, Some(result), None)
            },
            Method::POST => {
                let todo = extract_todo_item(&req);
                let result = add_todo(todo, &mut db);
                to_response(StatusCode::OK, Some(result), Some(db))
            },
            _ => to_response(StatusCode::METHOD_NOT_ALLOWED, None::<IdentifyableTodoItem>, None),
        }
    } else if path.starts_with("/todos/") {
        let id = extract_id(&req);
        match *req.method() {
            Method::GET => {
                let result = get_todo(id, &db);
                to_response(match result {
                    Some(_) => StatusCode::OK,
                    None => StatusCode::NOT_FOUND,
                }, result, None)
            },
            Method::DELETE => {
                let res = delete_todo(id, &mut db);
                to_response(
                    match res {
                        Some(_) => StatusCode::NO_CONTENT,
                        None => StatusCode::NOT_FOUND,
                    },
                    None::<IdentifyableTodoItem>,
                    Some(db),
                )
            },
            _ => to_response(StatusCode::METHOD_NOT_ALLOWED, None::<IdentifyableTodoItem>, None),
        }
    } else {
        to_response(StatusCode::NOT_FOUND, None::<IdentifyableTodoItem>, None)
    }
}

fn get_todos(pagination: Pagination, todos: &TodoStore) -> Vec<IdentifyableTodoItem> {
    todos.get_todos(pagination)
}

fn add_todo(todo: TodoItem, todos: &mut TodoStore) -> IdentifyableTodoItem {
    todos.add_todo(todo)
}

fn delete_todo(id: usize, todos: &mut TodoStore) -> Option<IdentifyableTodoItem> {
    todos.remove_todo(id)
}

fn get_todo(id: usize, todos: &TodoStore) -> Option<&IdentifyableTodoItem> {
    todos.get_todo(id)
}

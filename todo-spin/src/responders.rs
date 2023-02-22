use std::collections::HashMap;

use anyhow::Result;
use base64::{engine::general_purpose, Engine};
use http::StatusCode;
use serde::Serialize;
use spin_sdk::http::Response;
use todo_logic::{IdentifyableTodoItem, TodoStore};

// Rather naive, manual responders. Anybody wants to write a framework for that? 😉

pub fn to_response<T>(status: StatusCode, result: Option<T>, todos: Option<TodoStore>) -> Result<Response>
where
    T: Serialize,
{
    let mut builder = http::Response::builder();
    let mut body = None;

    if let Some(result) = result {
        let response = serde_json::to_string_pretty(&result)?.as_bytes().to_vec();
        builder = builder.header("Content-Type", "application/json");
        body = Some(response);
    }

    if let Some(todos) = todos {
        let db = serde_json::to_string(&Into::<HashMap<usize, IdentifyableTodoItem>>::into(todos))?;
        let db = format!("db={}", general_purpose::STANDARD_NO_PAD.encode(db));
        builder = builder.header("Set-Cookie", format!("{}; SameSite=Strict; Path=/", db));
    }

    Ok(builder.status(status).body(body.map(|body| body.into()))?)
}

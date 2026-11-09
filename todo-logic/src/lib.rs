use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tokio::fs;

/// Represents a single todo item
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TodoItem {
    pub title: String,
    pub notes: String,
    pub assigned_to: String,
    pub completed: bool,
}

/// DTO for patching a todo item
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateTodoItem {
    pub title: Option<String>,
    pub notes: Option<String>,
    pub assigned_to: Option<String>,
    pub completed: Option<bool>,
}

/// Represents a todo item with an id
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IdentifiableTodoItem {
    pub id: usize,

    #[serde(flatten)]
    pub item: TodoItem,
}

impl IdentifiableTodoItem {
    pub fn new(id: usize, item: TodoItem) -> Self {
        Self { id, item }
    }
}

/// Parameters for pagination
///
/// Used to demonstrate handling of query parameters.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Pagination {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

impl Pagination {
    pub fn new(offset: Option<usize>, limit: Option<usize>) -> Self {
        Self { offset, limit }
    }
}

/// Error type for the todo items store
#[derive(thiserror::Error, Debug)]
pub enum TodoStoreError {
    #[error("persistent data store error")]
    FileAccess(#[from] std::io::Error),
    #[error("serialization error")]
    Serialization(#[from] serde_json::Error),
}

/// Todo items store
///
/// Items are kept in a `BTreeMap` so that they are always ordered by id. This is what makes
/// offset-based pagination stable.
#[derive(Default)]
pub struct TodoStore {
    store: BTreeMap<usize, IdentifiableTodoItem>,
    next_id: usize,
}

impl TodoStore {
    /// Get list of todo items
    ///
    /// Supports pagination.
    pub fn get_todos(&self, pagination: Pagination) -> Vec<IdentifiableTodoItem> {
        self.store
            .values()
            .skip(pagination.offset.unwrap_or(0))
            .take(pagination.limit.unwrap_or(usize::MAX))
            .cloned()
            .collect()
    }

    /// Get a single todo item by id
    pub fn get_todo(&self, id: usize) -> Option<&IdentifiableTodoItem> {
        self.store.get(&id)
    }

    /// Create a new todo item
    pub fn add_todo(&mut self, todo: TodoItem) -> IdentifiableTodoItem {
        let id = self.next_id;
        self.next_id += 1;
        let new_item = IdentifiableTodoItem::new(id, todo);
        self.store.insert(id, new_item.clone());
        new_item
    }

    /// Remove a todo item by id
    pub fn remove_todo(&mut self, id: usize) -> Option<IdentifiableTodoItem> {
        self.store.remove(&id)
    }

    /// Patch a todo item by id
    pub fn update_todo(&mut self, id: usize, todo: UpdateTodoItem) -> Option<&IdentifiableTodoItem> {
        let existing = self.store.get_mut(&id)?;
        if let Some(title) = todo.title {
            existing.item.title = title;
        }
        if let Some(notes) = todo.notes {
            existing.item.notes = notes;
        }
        if let Some(assigned_to) = todo.assigned_to {
            existing.item.assigned_to = assigned_to;
        }
        if let Some(completed) = todo.completed {
            existing.item.completed = completed;
        }
        Some(existing)
    }

    /// Store todo items to disk
    ///
    /// Used to demonstrate error handling.
    pub async fn persist(&self) -> Result<(), TodoStoreError> {
        const FILENAME: &str = "todo_store.json";

        let items: Vec<_> = self.store.values().collect();
        let json = serde_json::to_string_pretty(&items)?;
        fs::write(FILENAME, json).await?;
        Ok(())
    }
}

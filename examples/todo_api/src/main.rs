//! # Todo API — Full Axoas Example
//!
//! A complete Todo REST API demonstrating axoas features:
//! - `#[openapi]` macro with tags, summaries, descriptions
//! - `route!` macro for handler+docs bundling
//! - `DocHandler::with()` for inline customization
//! - DocRouter nesting, merging, with_state
//! - `serve_openapi` for the JSON spec endpoint
//!
//! Run: `cargo run --example todo_api`

use axoas::{openapi, route, DocRouter};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ============================================================
// Types
// ============================================================

#[derive(Debug, Clone, JsonSchema, Serialize, Deserialize)]
struct Todo {
    id: u64,
    title: String,
    completed: bool,
}

#[derive(Debug, Clone, JsonSchema, Deserialize)]
struct CreateTodo {
    title: String,
}

#[derive(Debug, Clone, JsonSchema, Deserialize)]
struct UpdateTodo {
    title: Option<String>,
    completed: Option<bool>,
}

#[derive(Debug, Clone, JsonSchema, Serialize)]
struct ErrorResponse {
    error: String,
}

// ============================================================
// App State
// ============================================================

#[derive(Clone)]
struct AppState {
    todos: Arc<Mutex<HashMap<u64, Todo>>>,
    next_id: Arc<Mutex<u64>>,
}

impl AppState {
    fn new() -> Self {
        Self {
            todos: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
        }
    }
}

// ============================================================
// Handlers — /api/todos
// ============================================================

#[openapi(
    tag = "todos",
    summary = "List all todos",
    description = "Returns the complete list of todos"
)]
async fn list_todos(State(state): State<AppState>) -> axum::Json<Vec<Todo>> {
    let todos = state.todos.lock().unwrap();
    let mut list: Vec<Todo> = todos.values().cloned().collect();
    list.sort_by_key(|t| t.id);
    axum::Json(list)
}

#[openapi(
    tag = "todos",
    summary = "Create a new todo",
    description = "Creates a todo and returns it with an assigned ID"
)]
async fn create_todo(
    State(state): State<AppState>,
    axum::Json(payload): axum::Json<CreateTodo>,
) -> (StatusCode, axum::Json<Todo>) {
    if payload.title.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(Todo { id: 0, title: String::new(), completed: false }),
        );
    }

    let mut next_id = state.next_id.lock().unwrap();
    let id = *next_id;
    *next_id += 1;

    let todo = Todo { id, title: payload.title, completed: false };
    state.todos.lock().unwrap().insert(id, todo.clone());

    (StatusCode::CREATED, axum::Json(todo))
}

// ============================================================
// Handlers — /api/todos/{id}
// ============================================================

#[openapi(tag = "todos", summary = "Get a todo by ID")]
async fn get_todo(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> axum::Json<Todo> {
    let todos = state.todos.lock().unwrap();
    let todo = todos.get(&id).cloned().unwrap_or(Todo { id: 0, title: String::new(), completed: false });
    axum::Json(todo)
}

#[openapi(tag = "todos", summary = "Update a todo")]
async fn update_todo(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    axum::Json(payload): axum::Json<UpdateTodo>,
) -> axum::Json<Todo> {
    let mut todos = state.todos.lock().unwrap();
    if let Some(todo) = todos.get_mut(&id) {
        if let Some(title) = payload.title { todo.title = title; }
        if let Some(completed) = payload.completed { todo.completed = completed; }
        axum::Json(todo.clone())
    } else {
        axum::Json(Todo { id: 0, title: String::new(), completed: false })
    }
}

#[openapi(
    tag = "todos",
    summary = "Delete a todo",
    description = "Deletes a todo by ID. Returns 204 on success."
)]
async fn delete_todo(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> StatusCode {
    if state.todos.lock().unwrap().remove(&id).is_some() {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

// ============================================================
// Health check
// ============================================================

#[openapi(tag = "health", summary = "Health check")]
async fn health_check() -> axum::Json<ErrorResponse> {
    axum::Json(ErrorResponse { error: "ok".into() })
}

// ============================================================
// Main — assemble the router
// ============================================================

#[tokio::main]
async fn main() {
    let state = AppState::new();

    // Build the todos sub-router
    let todos_router = DocRouter::new()
        .route("/todos", axoas::routing::get(route!(list_todos))
            .post(route!(create_todo)))
        .route("/todos/{id}", axoas::routing::get(route!(get_todo))
            .put(route!(update_todo))
            .delete(route!(delete_todo)));

    // Build the health sub-router
    let health_router = DocRouter::new()
        .route("/health", axoas::routing::get(route!(health_check)));

    // Merge and nest under /api
    let api_router = health_router.merge(todos_router);

    // Build the full app with customized metadata
    let app = DocRouter::new()
        .nest("/api", api_router)
        .serve_openapi("/openapi.json")
        .with_info(openapi3_rs::Info {
            title: "Todo API".into(),
            version: "1.0.0".into(),
            description: Some("A complete Todo REST API documented with axoas".into()),
            summary: None,
            terms_of_service: None,
            contact: None,
            license: None,
        })
        .into_axum_router()
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Todo API at http://localhost:3000");
    println!("  GET    /openapi.json   — OpenAPI spec");
    println!("  GET    /api/todos       — List todos");
    println!("  POST   /api/todos       — Create todo");
    println!("  GET    /api/todos/{{id}}  — Get todo");
    println!("  PUT    /api/todos/{{id}}  — Update todo");
    println!("  DELETE /api/todos/{{id}}  — Delete todo");
    println!("  GET    /api/health      — Health check");

    axum::serve(listener, app).await.unwrap();
}

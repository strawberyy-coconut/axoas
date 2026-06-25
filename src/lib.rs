//! # Axoas — Auto OpenAPI Documentation for Axum
//!
//! `axoas` extends axum's routing API with automatic OpenAPI 3.2 spec generation.
//! It introduces `DocRouter` (wraps `axum::Router` + collects OpenAPI metadata),
//! a `#[openapi]` proc macro on handlers that generates companion doc functions
//! with schemars-powered schema inference, and a `route!` macro for ergonomic
//! handler+docs bundling.
//!
//! The API mirrors axum's native style: `get(route!(handler))`.
//!
//! ## Quick Start
//!
//! ```ignore
//! use axoas::{DocRouter, route, openapi, routing::get};
//! use schemars::JsonSchema;
//!
//! #[derive(JsonSchema, serde::Serialize)]
//! struct User { id: Uuid, name: String }
//!
//! #[openapi(tag = "users", summary = "List all users")]
//! async fn list_users() -> axum::Json<Vec<User>> { todo!() }
//!
//! let app = DocRouter::new()
//!     .route("/users", get(route!(list_users)))
//!     .serve_openapi("/openapi.json")
//!     .into_axum_router();
//! ```

pub mod context;
pub mod extract;
pub mod handler;
pub mod method;
pub mod openapi;
pub mod router;
pub mod routing;

// Re-exports
pub use axoas_macros::{openapi, route};
pub use context::GenContext;
pub use extract::OpenApiExtractor;
pub use extract::OpenApiOutput;
pub use handler::DocHandler;
pub use method::DocMethodRouter;
pub use openapi3_rs;
pub use router::DocRouter;
pub use schemars;
pub use indexmap;

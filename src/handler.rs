//! `DocHandler` — bundles an axum handler with its OpenAPI documentation.

use openapi3_rs::Operation;

/// Bundles an axum handler function with its pre-built OpenAPI `Operation`.
///
/// This is what `route!(handler_name)` expands to.
/// The handler is the original async function; the operation is produced
/// by the `#[openapi]`-generated companion function `__axoas_doc_{hash}()`.
///
/// # Customization
///
/// Use `.with()` to mutate the operation inline:
///
/// ```ignore
/// post(route!(create_user).with(|op| {
///     op.description = Some("Custom description".into());
/// }))
/// ```
#[derive(Debug, Clone)]
pub struct DocHandler<H> {
    /// The axum handler function.
    pub handler: H,
    /// The pre-built OpenAPI Operation describing this handler.
    pub operation: Operation,
}

impl<H> DocHandler<H> {
    /// Create a new `DocHandler` from a handler and its operation.
    pub fn new(handler: H, operation: Operation) -> Self {
        Self { handler, operation }
    }

    /// Apply a mutation function to the operation, returning `self`.
    ///
    /// This enables inline customization without `_with` variants:
    ///
    /// ```ignore
    /// post(route!(create_user).with(|op| {
    ///     op.description = Some("Custom".into());
    /// }))
    /// ```
    pub fn with(mut self, f: impl FnOnce(&mut Operation)) -> Self {
        f(&mut self.operation);
        self
    }
}

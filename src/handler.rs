//! `DocHandler` — bundles an axum handler with its OpenAPI documentation.

use openapi3_rs::{Components, Operation};

/// Bundles an axum handler function with its pre-built OpenAPI `Operation`
/// and any `Components` contributed by extractors (e.g. security schemes).
///
/// This is what `route!(handler_name)` expands to.
/// The handler is the original async function; the operation is produced
/// by the `#[openapi]`-generated companion function `__axoas_doc_{hash}()`.
///
/// # Customization
///
/// Use `.with()` to mutate the operation inline, and `.with_components()`
/// to mutate the components:
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
    /// Reusable components contributed by extractors (security schemes, schemas, etc.).
    pub components: Components,
}

impl<H> DocHandler<H> {
    /// Create a new `DocHandler` from a handler and its operation.
    /// Components default to empty.
    pub fn new(handler: H, operation: Operation) -> Self {
        Self { handler, operation, components: Components::default() }
    }

    /// Create a new `DocHandler` with an operation and pre-populated components.
    /// This is the primary constructor used by the `route!` macro.
    pub fn new_with_components(
        handler: H,
        operation: Operation,
        components: Components,
    ) -> Self {
        Self { handler, operation, components }
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

    /// Apply a mutation function to the components, returning `self`.
    ///
    /// Use this to override or extend the components contributed by
    /// extractors (e.g., change a security scheme definition).
    ///
    /// ```ignore
    /// post(route!(create_user).with_components(|c| {
    ///     c.security_schemes = None; // strip all auto-registered schemes
    /// }))
    /// ```
    pub fn with_components(mut self, f: impl FnOnce(&mut Components)) -> Self {
        f(&mut self.components);
        self
    }
}

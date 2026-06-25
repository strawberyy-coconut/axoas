//! `DocMethodRouter<S, E>` — wraps `axum::routing::MethodRouter<S, E>` and
//! stores OpenAPI `Operation` entries per HTTP method.

use std::convert::Infallible;

use axum::extract::Request as AxumRequest;
use axum::handler::Handler;
use axum::routing::{MethodRouter, Route};
use axum::{body::Body, response::IntoResponse};
use openapi3_rs::{Components, Operation, Parameter, Server};
use tower_layer::Layer;
use tower_service::Service;

use crate::handler::DocHandler;

/// A wrapper over `axum::routing::MethodRouter` that adds API
/// documentation-specific features.
#[derive(Debug, Clone)]
#[must_use]
pub struct DocMethodRouter<S = (), E = Infallible> {
    pub(crate) method_router: MethodRouter<S, E>,
    pub(crate) get: Option<Operation>,
    pub(crate) post: Option<Operation>,
    pub(crate) put: Option<Operation>,
    pub(crate) delete: Option<Operation>,
    pub(crate) patch: Option<Operation>,
    pub(crate) head: Option<Operation>,
    pub(crate) options: Option<Operation>,
    pub(crate) trace: Option<Operation>,
    pub(crate) connect: Option<Operation>,
    pub(crate) query: Option<Operation>,
    /// Accumulated reusable components from all chained handlers.
    pub(crate) components: Components,
    /// Path-level summary (applies to all operations on this path).
    pub(crate) path_summary: Option<String>,
    /// Path-level description (applies to all operations on this path).
    pub(crate) path_description: Option<String>,
    /// Path-level servers (overrides top-level servers).
    pub(crate) path_servers: Option<Vec<Server>>,
    /// Path-level parameters (applicable to all operations on this path).
    pub(crate) path_parameters: Option<Vec<Parameter>>,
}

impl<S, E> DocMethodRouter<S, E> {
    /// Create a new `DocMethodRouter` with the given method router.
    pub fn new(method_router: MethodRouter<S, E>) -> Self {
        Self {
            method_router,
            get: None,
            post: None,
            put: None,
            delete: None,
            patch: None,
            head: None,
            options: None,
            trace: None,
            connect: None,
            query: None,
            components: Components::default(),
            path_summary: None,
            path_description: None,
            path_servers: None,
            path_parameters: None,
        }
    }

    /// Split into the inner MethodRouter, accumulated PathItem, and Components.
    pub fn split(self) -> (MethodRouter<S, E>, openapi3_rs::PathItem, Components) {
        let path_item = self.build_path_item();
        (self.method_router, path_item, self.components)
    }

    fn build_path_item(&self) -> openapi3_rs::PathItem {
        let mut path_item = openapi3_rs::PathItem::new();
        path_item.summary = self.path_summary.clone();
        path_item.description = self.path_description.clone();
        path_item.servers = self.path_servers.clone();
        path_item.parameters = self.path_parameters.clone().map(|params| {
            params.into_iter().map(openapi3_rs::RefOr::Item).collect()
        });
        path_item.get = self.get.clone();
        path_item.post = self.post.clone();
        path_item.put = self.put.clone();
        path_item.delete = self.delete.clone();
        path_item.patch = self.patch.clone();
        path_item.head = self.head.clone();
        path_item.options = self.options.clone();
        path_item.trace = self.trace.clone();
        path_item.query = self.query.clone();
        if let Some(op) = &self.connect {
            let map = path_item.additional_operations.get_or_insert_with(indexmap::IndexMap::new);
            map.insert("CONNECT".to_string(), op.clone());
        }
        path_item
    }

    /// Merge another `DocMethodRouter` into this one.
    pub fn merge(mut self, other: DocMethodRouter<S, E>) -> Self
    where
        S: Clone + Send + Sync + 'static,
    {
        self.method_router = self.method_router.merge(other.method_router);
        if other.get.is_some() { self.get = other.get; }
        if other.post.is_some() { self.post = other.post; }
        if other.put.is_some() { self.put = other.put; }
        if other.delete.is_some() { self.delete = other.delete; }
        if other.patch.is_some() { self.patch = other.patch; }
        if other.head.is_some() { self.head = other.head; }
        if other.options.is_some() { self.options = other.options; }
        if other.trace.is_some() { self.trace = other.trace; }
        if other.connect.is_some() { self.connect = other.connect; }
        if other.query.is_some() { self.query = other.query; }
        if other.path_summary.is_some() { self.path_summary = other.path_summary; }
        if other.path_description.is_some() { self.path_description = other.path_description; }
        if other.path_servers.is_some() { self.path_servers = other.path_servers; }
        if other.path_parameters.is_some() { self.path_parameters = other.path_parameters; }
        merge_components(&mut self.components, other.components);
        self
    }

    /// Provide state to the inner method router.
    pub fn with_state<S2>(self, state: S) -> DocMethodRouter<S2, E>
    where
        S: Clone + Send + Sync + 'static,
    {
        DocMethodRouter {
            method_router: self.method_router.with_state(state),
            get: self.get,
            post: self.post,
            put: self.put,
            delete: self.delete,
            patch: self.patch,
            head: self.head,
            options: self.options,
            trace: self.trace,
            connect: self.connect,
            query: self.query,
            components: self.components,
            path_summary: self.path_summary,
            path_description: self.path_description,
            path_servers: self.path_servers,
            path_parameters: self.path_parameters,
        }
    }
}

// Chaining methods — only available when E = Infallible
impl<S> DocMethodRouter<S, Infallible>
where
    S: Clone + Send + Sync + 'static,
{
    /// Create an empty `DocMethodRouter` that responds with 405.
    pub fn empty() -> Self {
        Self::new(MethodRouter::new())
    }

    /// Chain a `GET` handler.
    pub fn get<H, T>(self, doc: DocHandler<H>) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        let Self { method_router, get: _, post, put, delete, patch, head, options, trace, connect, query, components: mut comps, path_summary, path_description, path_servers, path_parameters } = self;
        merge_components(&mut comps, doc.components);
        Self { method_router: method_router.get(doc.handler), get: Some(doc.operation), post, put, delete, patch, head, options, trace, connect, query, components: comps, path_summary, path_description, path_servers, path_parameters }
    }

    /// Chain a `POST` handler.
    pub fn post<H, T>(self, doc: DocHandler<H>) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        let Self { method_router, post: _, get, put, delete, patch, head, options, trace, connect, query, components: mut comps, path_summary, path_description, path_servers, path_parameters } = self;
        merge_components(&mut comps, doc.components);
        Self { method_router: method_router.post(doc.handler), post: Some(doc.operation), get, put, delete, patch, head, options, trace, connect, query, components: comps, path_summary, path_description, path_servers, path_parameters }
    }

    /// Chain a `PUT` handler.
    pub fn put<H, T>(self, doc: DocHandler<H>) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        let Self { method_router, put: _, get, post, delete, patch, head, options, trace, connect, query, components: mut comps, path_summary, path_description, path_servers, path_parameters } = self;
        merge_components(&mut comps, doc.components);
        Self { method_router: method_router.put(doc.handler), put: Some(doc.operation), get, post, delete, patch, head, options, trace, connect, query, components: comps, path_summary, path_description, path_servers, path_parameters }
    }

    /// Chain a `DELETE` handler.
    pub fn delete<H, T>(self, doc: DocHandler<H>) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        let Self { method_router, delete: _, get, post, put, patch, head, options, trace, connect, query, components: mut comps, path_summary, path_description, path_servers, path_parameters } = self;
        merge_components(&mut comps, doc.components);
        Self { method_router: method_router.delete(doc.handler), delete: Some(doc.operation), get, post, put, patch, head, options, trace, connect, query, components: comps, path_summary, path_description, path_servers, path_parameters }
    }

    /// Chain a `PATCH` handler.
    pub fn patch<H, T>(self, doc: DocHandler<H>) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        let Self { method_router, patch: _, get, post, put, delete, head, options, trace, connect, query, components: mut comps, path_summary, path_description, path_servers, path_parameters } = self;
        merge_components(&mut comps, doc.components);
        Self { method_router: method_router.patch(doc.handler), patch: Some(doc.operation), get, post, put, delete, head, options, trace, connect, query, components: comps, path_summary, path_description, path_servers, path_parameters }
    }

    /// Chain a `HEAD` handler.
    pub fn head<H, T>(self, doc: DocHandler<H>) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        let Self { method_router, head: _, get, post, put, delete, patch, options, trace, connect, query, components: mut comps, path_summary, path_description, path_servers, path_parameters } = self;
        merge_components(&mut comps, doc.components);
        Self { method_router: method_router.head(doc.handler), head: Some(doc.operation), get, post, put, delete, patch, options, trace, connect, query, components: comps, path_summary, path_description, path_servers, path_parameters }
    }

    /// Chain an `OPTIONS` handler.
    pub fn options<H, T>(self, doc: DocHandler<H>) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        let Self { method_router, options: _, get, post, put, delete, patch, head, trace, connect, query, components: mut comps, path_summary, path_description, path_servers, path_parameters } = self;
        merge_components(&mut comps, doc.components);
        Self { method_router: method_router.options(doc.handler), options: Some(doc.operation), get, post, put, delete, patch, head, trace, connect, query, components: comps, path_summary, path_description, path_servers, path_parameters }
    }

    /// Chain a `TRACE` handler.
    pub fn trace<H, T>(self, doc: DocHandler<H>) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        let Self { method_router, trace: _, get, post, put, delete, patch, head, options, connect, query, components: mut comps, path_summary, path_description, path_servers, path_parameters } = self;
        merge_components(&mut comps, doc.components);
        Self { method_router: method_router.trace(doc.handler), trace: Some(doc.operation), get, post, put, delete, patch, head, options, connect, query, components: comps, path_summary, path_description, path_servers, path_parameters }
    }

    /// Chain a `CONNECT` handler.
    pub fn connect<H, T>(self, doc: DocHandler<H>) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        let Self { method_router, connect: _, get, post, put, delete, patch, head, options, trace, query, components: mut comps, path_summary, path_description, path_servers, path_parameters } = self;
        merge_components(&mut comps, doc.components);
        Self { method_router: method_router.connect(doc.handler), connect: Some(doc.operation), get, post, put, delete, patch, head, options, trace, query, components: comps, path_summary, path_description, path_servers, path_parameters }
    }

    /// Chain a `QUERY` handler (OAS 3.2).
    ///
    /// **Important:** Axum 0.8 does not support the QUERY HTTP method natively.
    /// This method stores the operation for the OpenAPI spec on
    /// `PathItem.query`, but the handler is routed via axum's fallback.
    /// For proper QUERY routing, use a custom `axum::routing::on` with a
    /// `MethodRouter` that handles the QUERY method directly, and construct
    /// the `DocMethodRouter` manually with `DocMethodRouter::new()`.
    pub fn query<H, T>(self, doc: DocHandler<H>) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        let Self { method_router, query: _, get, post, put, delete, patch, head, options, trace, connect, components: mut comps, path_summary, path_description, path_servers, path_parameters } = self;
        merge_components(&mut comps, doc.components);
        Self { method_router: method_router.fallback(doc.handler), query: Some(doc.operation), get, post, put, delete, patch, head, options, trace, connect, components: comps, path_summary, path_description, path_servers, path_parameters }
    }

    /// Add a fallback handler.
    pub fn fallback<H, T>(mut self, handler: H) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        self.method_router = self.method_router.fallback(handler);
        self
    }

    /// Apply a tower `Layer` to the inner method router.
    pub fn layer<L, NewError>(self, layer: L) -> DocMethodRouter<S, NewError>
    where
        L: Layer<Route> + Clone + Send + Sync + 'static,
        L::Service: Service<AxumRequest<Body>> + Clone + Send + Sync + 'static,
        <L::Service as Service<AxumRequest<Body>>>::Response: IntoResponse + 'static,
        <L::Service as Service<AxumRequest<Body>>>::Error: Into<NewError> + 'static,
        <L::Service as Service<AxumRequest<Body>>>::Future: Send + 'static,
        NewError: 'static,
    {
        DocMethodRouter {
            method_router: self.method_router.layer(layer),
            get: self.get, post: self.post, put: self.put,
            delete: self.delete, patch: self.patch, head: self.head,
            options: self.options, trace: self.trace, connect: self.connect,
            query: self.query,
            components: self.components,
            path_summary: self.path_summary,
            path_description: self.path_description,
            path_servers: self.path_servers,
            path_parameters: self.path_parameters,
        }
    }

    /// Set the path-level summary (OAS 3.2 PathItem.summary).
    /// Applies to all operations on this path.
    pub fn path_summary(mut self, summary: &str) -> Self {
        self.path_summary = Some(summary.to_string());
        self
    }

    /// Set the path-level description (OAS 3.2 PathItem.description).
    /// Applies to all operations on this path. Supports CommonMark.
    pub fn path_description(mut self, description: &str) -> Self {
        self.path_description = Some(description.to_string());
        self
    }

    /// Set path-level servers (OAS 3.2 PathItem.servers).
    /// Overrides top-level servers for all operations on this path.
    pub fn path_servers(mut self, servers: Vec<Server>) -> Self {
        self.path_servers = Some(servers);
        self
    }

    /// Set path-level parameters (OAS 3.2 PathItem.parameters).
    /// Applicable to all operations on this path; can be overridden
    /// at the operation level but cannot be removed there.
    pub fn path_parameters(mut self, parameters: Vec<Parameter>) -> Self {
        self.path_parameters = Some(parameters);
        self
    }
}

/// Merge reusable components from `source` into `target`.
///
/// Uses first-write-wins semantics: if a key already exists in `target`,
/// the `source` value is ignored. This ensures that explicit user
/// configuration (via `with_security_scheme` etc.) takes precedence
/// over auto-registered entries from extractors.
pub(crate) fn merge_components(target: &mut Components, source: Components) {
    if let Some(src) = source.schemas {
        let dst = target.schemas.get_or_insert_with(indexmap::IndexMap::new);
        for (k, v) in src { dst.entry(k).or_insert(v); }
    }
    if let Some(src) = source.responses {
        let dst = target.responses.get_or_insert_with(indexmap::IndexMap::new);
        for (k, v) in src { dst.entry(k).or_insert(v); }
    }
    if let Some(src) = source.parameters {
        let dst = target.parameters.get_or_insert_with(indexmap::IndexMap::new);
        for (k, v) in src { dst.entry(k).or_insert(v); }
    }
    if let Some(src) = source.examples {
        let dst = target.examples.get_or_insert_with(indexmap::IndexMap::new);
        for (k, v) in src { dst.entry(k).or_insert(v); }
    }
    if let Some(src) = source.request_bodies {
        let dst = target.request_bodies.get_or_insert_with(indexmap::IndexMap::new);
        for (k, v) in src { dst.entry(k).or_insert(v); }
    }
    if let Some(src) = source.headers {
        let dst = target.headers.get_or_insert_with(indexmap::IndexMap::new);
        for (k, v) in src { dst.entry(k).or_insert(v); }
    }
    if let Some(src) = source.security_schemes {
        let dst = target.security_schemes.get_or_insert_with(indexmap::IndexMap::new);
        for (k, v) in src { dst.entry(k).or_insert(v); }
    }
    if let Some(src) = source.links {
        let dst = target.links.get_or_insert_with(indexmap::IndexMap::new);
        for (k, v) in src { dst.entry(k).or_insert(v); }
    }
    if let Some(src) = source.callbacks {
        let dst = target.callbacks.get_or_insert_with(indexmap::IndexMap::new);
        for (k, v) in src { dst.entry(k).or_insert(v); }
    }
    if let Some(src) = source.path_items {
        let dst = target.path_items.get_or_insert_with(indexmap::IndexMap::new);
        for (k, v) in src { dst.entry(k).or_insert(v); }
    }
    if let Some(src) = source.media_types {
        let dst = target.media_types.get_or_insert_with(indexmap::IndexMap::new);
        for (k, v) in src { dst.entry(k).or_insert(v); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    fn op(summary: &str) -> Operation { Operation { summary: Some(summary.to_string()), ..Default::default() } }
    async fn a() -> StatusCode { StatusCode::OK }
    async fn b() -> StatusCode { StatusCode::CREATED }

    #[test]
    fn build_path_item_from_get() {
        let dmr: DocMethodRouter = crate::routing::get(DocHandler::new(a, op("a")));
        let pi = dmr.build_path_item();
        assert!(pi.get.is_some());
        assert_eq!(pi.get.unwrap().summary.as_deref(), Some("a"));
        assert!(pi.post.is_none());
    }

    #[test]
    fn chaining_get_post() {
        let dmr: DocMethodRouter = crate::routing::get(DocHandler::new(a, op("a")))
            .post(DocHandler::new(b, op("b")));
        let pi = dmr.build_path_item();
        assert!(pi.get.is_some());
        assert!(pi.post.is_some());
    }

    #[test]
    fn empty_method_router() {
        let dmr = DocMethodRouter::<(), Infallible>::empty();
        let pi = dmr.build_path_item();
        assert!(pi.get.is_none());
        assert!(pi.post.is_none());
    }

    #[test]
    fn split_returns_router_and_item() {
        let dmr: DocMethodRouter = crate::routing::get(DocHandler::new(a, op("a")));
        let (router, pi, _components) = dmr.split();
        assert!(pi.get.is_some());
        drop(router);
    }

    #[test]
    fn merge_two_routers() {
        let dmr1: DocMethodRouter = crate::routing::get(DocHandler::new(a, op("a")));
        let dmr2: DocMethodRouter = crate::routing::post(DocHandler::new(b, op("b")));
        let pi = dmr1.merge(dmr2).build_path_item();
        assert!(pi.get.is_some());
        assert!(pi.post.is_some());
    }

    #[test]
    #[should_panic(expected = "Overlapping method route")]
    fn merge_overlapping_panics() {
        let dmr1: DocMethodRouter = crate::routing::get(DocHandler::new(a, op("first")));
        let dmr2: DocMethodRouter = crate::routing::get(DocHandler::new(b, op("second")));
        // Should panic: axum prevents merging overlapping methods
        let _ = dmr1.merge(dmr2);
    }

    #[test]
    fn connect_via_additional_operations() {
        let dmr: DocMethodRouter = crate::routing::connect(DocHandler::new(a, op("tunnel")));
        let pi = dmr.build_path_item();
        let addl = pi.additional_operations.as_ref().unwrap();
        assert!(addl.contains_key("CONNECT"));
    }

    #[test]
    fn any_sets_all_methods() {
        let dmr: DocMethodRouter = crate::routing::any(DocHandler::new(a, op("all")));
        let pi = dmr.build_path_item();
        assert!(pi.get.is_some());
        assert!(pi.post.is_some());
        assert!(pi.put.is_some());
        assert!(pi.delete.is_some());
        assert!(pi.patch.is_some());
        assert!(pi.head.is_some());
        assert!(pi.options.is_some());
        assert!(pi.trace.is_some());
        assert!(pi.additional_operations.as_ref().unwrap().contains_key("CONNECT"));
    }
}

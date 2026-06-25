//! `DocRouter<S>` — wraps `axum::Router<S>` and collects OpenAPI metadata.

use std::convert::Infallible;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::Request;
use axum::handler::Handler;
use axum::response::IntoResponse;
use axum::routing::Route;
use indexmap::IndexMap;
use openapi3_rs::{
    Components, Info, OpenAPI, Operation, PathItem, RefOr, Server, Tag,
};
use tower_layer::Layer;
use tower_service::Service;

use crate::method::DocMethodRouter;

#[derive(Debug, Clone)]
struct DocRouterInner<S> {
    router: axum::Router<S>,
    paths: IndexMap<String, RefOr<PathItem>>,
    info: Info,
    servers: Vec<Server>,
    tags: Vec<Tag>,
    components: Components,
}

#[derive(Debug, Clone)]
#[must_use]
pub struct DocRouter<S = ()> {
    inner: Arc<DocRouterInner<S>>,
}

impl<S> DocRouter<S>
where
    S: Clone + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DocRouterInner {
                router: axum::Router::new(),
                paths: IndexMap::new(),
                info: Info {
                    title: "API".to_string(),
                    version: "0.1.0".to_string(),
                    summary: None,
                    description: None,
                    terms_of_service: None,
                    contact: None,
                    license: None,
                },
                servers: Vec::new(),
                tags: Vec::new(),
                components: Components::default(),
            }),
        }
    }

    pub fn route(
        mut self,
        path: &str,
        doc_method: DocMethodRouter<S, Infallible>,
    ) -> Self {
        let inner = Arc::make_mut(&mut self.inner);
        let (method_router, path_item) = doc_method.split();

        if let Some(existing) = inner.paths.get_mut(path) {
            if let RefOr::Item(existing_item) = existing {
                merge_path_items(existing_item, path_item);
            }
        } else {
            inner.paths.insert(path.to_string(), RefOr::Item(path_item));
        }

        inner.router = std::mem::take(&mut inner.router).route(path, method_router);
        self
    }

    pub fn nest(mut self, prefix: &str, nested: DocRouter<S>) -> Self {
        let inner = Arc::make_mut(&mut self.inner);
        let nested_paths = nested.inner.paths.clone();

        for (nested_path, path_item) in nested_paths {
            let combined = format_nested_path(prefix, &nested_path);
            if let Some(existing) = inner.paths.get_mut(&combined) {
                if let RefOr::Item(existing_item) = existing {
                    if let RefOr::Item(pi) = path_item {
                        merge_path_items(existing_item, pi);
                    }
                }
            } else {
                inner.paths.insert(combined, path_item);
            }
        }

        inner.router = std::mem::take(&mut inner.router).nest(prefix, nested.inner.router.clone());
        self
    }

    pub fn merge(mut self, other: DocRouter<S>) -> Self {
        let inner = Arc::make_mut(&mut self.inner);

        for (path, path_item) in &other.inner.paths {
            if let Some(existing) = inner.paths.get_mut(path) {
                if let (RefOr::Item(existing_item), RefOr::Item(new_item)) = (existing, path_item) {
                    merge_path_items(existing_item, new_item.clone());
                }
            } else {
                inner.paths.insert(path.clone(), path_item.clone());
            }
        }

        inner.router = std::mem::take(&mut inner.router).merge(other.inner.router.clone());
        self
    }

    pub fn with_state<S2>(self, state: S) -> DocRouter<S2>
    where
        S: Clone,
    {
        DocRouter {
            inner: Arc::new(DocRouterInner {
                router: self.inner.router.clone().with_state(state),
                paths: self.inner.paths.clone(),
                info: self.inner.info.clone(),
                servers: self.inner.servers.clone(),
                tags: self.inner.tags.clone(),
                components: self.inner.components.clone(),
            }),
        }
    }

    pub fn layer<L>(mut self, layer: L) -> DocRouter<S>
    where
        L: Layer<Route> + Clone + Send + Sync + 'static,
        L::Service: Service<Request<Body>> + Clone + Send + Sync + 'static,
        <L::Service as Service<Request<Body>>>::Response: IntoResponse + 'static,
        <L::Service as Service<Request<Body>>>::Error: Into<Infallible> + 'static,
        <L::Service as Service<Request<Body>>>::Future: Send + 'static,
    {
        let inner = Arc::make_mut(&mut self.inner);
        inner.router = std::mem::take(&mut inner.router).layer(layer);
        self
    }

    pub fn fallback<H, T>(mut self, handler: H) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        let inner = Arc::make_mut(&mut self.inner);
        inner.router = std::mem::take(&mut inner.router).fallback(handler);
        self
    }

    pub fn with_info(mut self, info: Info) -> Self {
        let inner = Arc::make_mut(&mut self.inner);
        inner.info = info;
        self
    }

    pub fn with_server(mut self, server: Server) -> Self {
        let inner = Arc::make_mut(&mut self.inner);
        inner.servers.push(server);
        self
    }

    pub fn with_tag(mut self, tag: Tag) -> Self {
        let inner = Arc::make_mut(&mut self.inner);
        inner.tags.push(tag);
        self
    }

    pub fn serve_openapi(self, path: &str) -> Self {
        let openapi_doc = self.openapi_doc();
        let json = serde_json::to_string(&openapi_doc).unwrap_or_default();

        let handler = move || {
            let json = json.clone();
            async move {
                axum::response::Json(
                    serde_json::from_str::<serde_json::Value>(&json).unwrap_or_default(),
                )
            }
        };

        let router = axum::routing::get(handler);
        let mut doc_method = DocMethodRouter::new(router);
        doc_method.get = Some(Operation {
            summary: Some("OpenAPI specification".to_string()),
            description: Some("Returns the OpenAPI 3.2 specification for this API".to_string()),
            ..Default::default()
        });

        self.route(path, doc_method)
    }

    pub fn openapi_doc(&self) -> OpenAPI {
        OpenAPI {
            openapi: "3.2.0".to_string(),
            info: self.inner.info.clone(),
            servers: if self.inner.servers.is_empty() {
                None
            } else {
                Some(self.inner.servers.clone())
            },
            paths: Some(self.inner.paths.clone()),
            components: Some(self.inner.components.clone()),
            tags: if self.inner.tags.is_empty() {
                None
            } else {
                Some(self.inner.tags.clone())
            },
            ..Default::default()
        }
    }

    pub fn into_axum_router(self) -> axum::Router<S> {
        self.inner.router.clone()
    }
}

pub(crate) fn merge_path_items(existing: &mut PathItem, new: PathItem) {
    if new.get.is_some() { existing.get = new.get; }
    if new.post.is_some() { existing.post = new.post; }
    if new.put.is_some() { existing.put = new.put; }
    if new.delete.is_some() { existing.delete = new.delete; }
    if new.patch.is_some() { existing.patch = new.patch; }
    if new.head.is_some() { existing.head = new.head; }
    if new.options.is_some() { existing.options = new.options; }
    if new.trace.is_some() { existing.trace = new.trace; }
    if let Some(additional) = new.additional_operations {
        let existing_additional = existing
            .additional_operations
            .get_or_insert_with(IndexMap::new);
        existing_additional.extend(additional);
    }
    if new.summary.is_some() { existing.summary = new.summary; }
    if new.description.is_some() { existing.description = new.description; }
    if new.servers.is_some() { existing.servers = new.servers; }
    if new.parameters.is_some() { existing.parameters = new.parameters; }
}

pub(crate) fn format_nested_path(prefix: &str, nested_path: &str) -> String {
    let prefix = prefix.trim_matches('/');
    let nested = nested_path.trim_start_matches('/');
    if nested.is_empty() {
        if prefix.is_empty() { "/".to_string() }
        else { format!("/{prefix}") }
    } else if prefix.is_empty() {
        format!("/{nested}")
    } else {
        format!("/{prefix}/{nested}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openapi3_rs::{Operation, PathItem};
    use indexmap::indexmap;

    fn make_op(summary: &str) -> Operation {
        Operation { summary: Some(summary.to_string()), ..Default::default() }
    }

    fn make_item(get: Option<Operation>, post: Option<Operation>) -> PathItem {
        PathItem { get, post, ..Default::default() }
    }

    #[test]
    fn merge_non_overlapping() {
        let mut existing = make_item(Some(make_op("get")), None);
        let new = make_item(None, Some(make_op("post")));
        merge_path_items(&mut existing, new);
        assert!(existing.get.is_some());
        assert!(existing.post.is_some());
    }

    #[test]
    fn merge_overlapping_last_wins() {
        let mut existing = make_item(Some(make_op("first")), None);
        let new = make_item(Some(make_op("second")), None);
        merge_path_items(&mut existing, new);
        assert_eq!(existing.get.unwrap().summary.as_deref(), Some("second"));
    }

    #[test]
    fn merge_connect_into_additional() {
        let mut existing = PathItem::default();
        let mut new = PathItem::default();
        new.additional_operations = Some(indexmap! { "CONNECT".into() => make_op("tunnel") });
        merge_path_items(&mut existing, new);
        assert_eq!(existing.additional_operations.unwrap()["CONNECT"].summary.as_deref(), Some("tunnel"));
    }

    #[test]
    fn merge_summary_overwrite() {
        let mut existing = PathItem { summary: Some("old".into()), ..Default::default() };
        let new = PathItem { summary: Some("new".into()), ..Default::default() };
        merge_path_items(&mut existing, new);
        assert_eq!(existing.summary.as_deref(), Some("new"));
    }

    #[test]
    fn nested_path_normal() { assert_eq!(format_nested_path("/api", "/users"), "/api/users"); }
    #[test]
    fn nested_path_trailing_slash() { assert_eq!(format_nested_path("api/", "/users"), "/api/users"); }
    #[test]
    fn nested_path_empty_nested() { assert_eq!(format_nested_path("/api", ""), "/api"); }
    #[test]
    fn nested_path_root_both() { assert_eq!(format_nested_path("", ""), "/"); }
    #[test]
    fn nested_path_empty_prefix() { assert_eq!(format_nested_path("", "/users"), "/users"); }
}

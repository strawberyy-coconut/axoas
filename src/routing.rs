//! Free functions that mirror `axum::routing` while producing `DocMethodRouter`s.

use std::convert::Infallible;

use axum::handler::Handler;

use crate::handler::DocHandler;
use crate::method::DocMethodRouter;

macro_rules! define_routing_fn {
    ($name:ident) => {
        #[doc = concat!("Route `", stringify!($name:upper), "` requests to the given documented handler.")]
        pub fn $name<H, T, S>(doc: DocHandler<H>) -> DocMethodRouter<S, Infallible>
        where
            H: Handler<T, S>,
            T: 'static,
            S: Clone + Send + Sync + 'static,
        {
            let DocHandler { handler, operation, components } = doc;
            let mut dmr = DocMethodRouter::new(axum::routing::$name(handler));
            dmr.$name = Some(operation);
            dmr.components = components;
            dmr
        }
    };
}

define_routing_fn!(get);
define_routing_fn!(post);
define_routing_fn!(put);
define_routing_fn!(delete);
define_routing_fn!(patch);
define_routing_fn!(head);
define_routing_fn!(options);
define_routing_fn!(trace);
define_routing_fn!(connect);

/// Route all HTTP methods to the given handler (uses fallback).
pub fn any<H, T, S>(doc: DocHandler<H>) -> DocMethodRouter<S, Infallible>
where
    H: Handler<T, S>,
    T: 'static,
    S: Clone + Send + Sync + 'static,
{
    let DocHandler { handler, operation, components } = doc;
    DocMethodRouter {
        method_router: axum::routing::any(handler),
        get: Some(operation.clone()),
        post: Some(operation.clone()),
        put: Some(operation.clone()),
        delete: Some(operation.clone()),
        patch: Some(operation.clone()),
        head: Some(operation.clone()),
        options: Some(operation.clone()),
        trace: Some(operation.clone()),
        connect: Some(operation.clone()),
        components,
    }
}

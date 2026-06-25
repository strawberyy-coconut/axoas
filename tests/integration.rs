//! Integration tests for DocRouter.

use axoas::{DocHandler, DocRouter};
use axum::http::StatusCode;
use openapi3_rs::Operation;

fn op(summary: &str) -> Operation { Operation { summary: Some(summary.to_string()), ..Default::default() } }

async fn list_users() -> StatusCode { StatusCode::OK }
async fn create_user() -> StatusCode { StatusCode::CREATED }
async fn get_user() -> StatusCode { StatusCode::OK }

type DR = DocRouter<()>;

#[test]
fn empty_router() {
    let r: DR = DocRouter::new();
    let doc = r.openapi_doc();
    assert_eq!(doc.openapi, "3.2.0");
    assert!(doc.paths.unwrap().is_empty());
}

#[test]
fn single_route() {
    let r: DR = DocRouter::new()
        .route("/users", axoas::routing::get(DocHandler::new(list_users, op("list"))));
    assert!(r.openapi_doc().paths.unwrap().contains_key("/users"));
}

#[test]
fn chained_methods() {
    let d1 = DocHandler::new(list_users, op("list"));
    let d2 = DocHandler::new(create_user, op("create"));
    let r: DR = DocRouter::new().route("/users", axoas::routing::get(d1).post(d2));
    let paths = r.openapi_doc().paths.unwrap();
    let item = match &paths["/users"] { openapi3_rs::RefOr::Item(i) => i, _ => panic!() };
    assert!(item.get.is_some());
    assert!(item.post.is_some());
}

#[test]
fn multiple_paths() {
    let r: DR = DocRouter::new()
        .route("/users", axoas::routing::get(DocHandler::new(list_users, op("list"))))
        .route("/users/{id}", axoas::routing::get(DocHandler::new(get_user, op("get"))));
    assert_eq!(r.openapi_doc().paths.unwrap().len(), 2);
}

#[test]
fn nesting_prefixes_paths() {
    let nested: DR = DocRouter::new()
        .route("/users", axoas::routing::get(DocHandler::new(list_users, op("list"))));
    // axum 0.8 requires non-empty prefix; "/api" is valid
    let r: DR = DocRouter::new().nest("/api", nested);
    assert!(r.openapi_doc().paths.unwrap().contains_key("/api/users"));
}

#[test]
fn merge_routers() {
    let r1: DR = DocRouter::new()
        .route("/users", axoas::routing::get(DocHandler::new(list_users, op("list"))));
    let r2: DR = DocRouter::new()
        .route("/health", axoas::routing::get(DocHandler::new(create_user, op("health"))));
    assert_eq!(r1.merge(r2).openapi_doc().paths.unwrap().len(), 2);
}

#[test]
fn merge_conflict_combines() {
    let r1: DR = DocRouter::new()
        .route("/x", axoas::routing::get(DocHandler::new(list_users, op("get"))));
    let r2: DR = DocRouter::new()
        .route("/x", axoas::routing::post(DocHandler::new(create_user, op("post"))));
    let paths = r1.merge(r2).openapi_doc().paths.unwrap();
    let item = match &paths["/x"] { openapi3_rs::RefOr::Item(i) => i, _ => panic!() };
    assert!(item.get.is_some());
    assert!(item.post.is_some());
}

#[test]
fn openapi_json_valid() {
    let r: DR = DocRouter::new()
        .route("/users", axoas::routing::get(DocHandler::new(list_users, op("list"))));
    let json = serde_json::to_string(&r.openapi_doc()).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["openapi"], "3.2.0");
}

#[test]
fn with_info() {
    let info = openapi3_rs::Info {
        title: "Test".into(), version: "1.0".into(),
        summary: None, description: None, terms_of_service: None, contact: None, license: None,
    };
    let r: DR = DocRouter::new().with_info(info);
    assert_eq!(r.openapi_doc().info.title, "Test");
}

#[test]
fn with_state_same_type() {
    let r: DocRouter<()> = DocRouter::new()
        .route("/users", axoas::routing::get(DocHandler::new(list_users, op("list"))));
    let r2: DocRouter = r.with_state(());
    assert!(r2.openapi_doc().paths.unwrap().contains_key("/users"));
}

#[test]
fn handler_with_mutation() {
    let d = DocHandler::new(list_users, op("list")).with(|op| {
        op.description = Some("desc".into());
    });
    assert_eq!(d.operation.description.as_deref(), Some("desc"));
    assert_eq!(d.operation.summary.as_deref(), Some("list"));
}

#[test]
fn into_axum_router() {
    let r: DR = DocRouter::new()
        .route("/x", axoas::routing::get(DocHandler::new(list_users, op("list"))));
    let _: axum::Router = r.into_axum_router();
}

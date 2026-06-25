//! OpenAPI schema helpers: bridge, responses, bodies.

use indexmap::IndexMap;
use openapi3_rs::{MediaType, RefOr, RequestBody, Response, Schema};

/// Convert schemars Schema to openapi3_rs Schema via JSON round-trip.
pub fn to_openapi_schema(schemars_schema: &schemars::Schema) -> Schema {
    let json = serde_json::to_value(schemars_schema).expect("Failed to serialize schemars::Schema");
    serde_json::from_value(json).expect("Failed to deserialize openapi3_rs::Schema")
}

/// Generate a binary download response.
pub fn binary_response(status: &str, content_type: &str, description: &str) -> (String, RefOr<Response>) {
    let mut content = IndexMap::new();
    content.insert(content_type.to_string(), MediaType { schema: None, ..Default::default() });
    (status.to_string(), RefOr::Item(Response {
        description: description.to_string(), content: Some(content), ..Default::default()
    }))
}

/// Generate a JSON request body from a schemars schema.
pub fn request_body_schema(schema: &schemars::Schema, description: Option<&str>, required: bool) -> RequestBody {
    let openapi_schema = to_openapi_schema(schema);
    let mut content = IndexMap::new();
    content.insert("application/json".to_string(), MediaType {
        schema: Some(RefOr::Item(openapi_schema)), ..Default::default()
    });
    RequestBody { description: description.map(String::from), content, required: Some(required) }
}

/// Generate a JSON response from a schemars schema.
pub fn response_schema(schema: &schemars::Schema, status: &str, description: &str) -> (String, RefOr<Response>) {
    let openapi_schema = to_openapi_schema(schema);
    let mut content = IndexMap::new();
    content.insert("application/json".to_string(), MediaType {
        schema: Some(RefOr::Item(openapi_schema)), ..Default::default()
    });
    (status.to_string(), RefOr::Item(Response {
        description: description.to_string(), content: Some(content), ..Default::default()
    }))
}

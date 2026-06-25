//! `OpenApiExtractor` trait and blanket impls for axum extractors.

use indexmap::IndexMap;
use openapi3_rs::{
    MediaType, Operation, Parameter, ParameterIn, RefOr, RequestBody, Response, Schema, Style,
};
use schemars::JsonSchema;

use crate::context::GenContext;
use crate::openapi::to_openapi_schema;

/// Trait for extractors that contribute to OpenAPI documentation.
pub trait OpenApiExtractor {
    fn operation_input(ctx: &mut GenContext, operation: &mut Operation);
    fn inferred_early_responses(
        _ctx: &mut GenContext,
        _operation: &mut Operation,
    ) -> Vec<(Option<String>, Response)> {
        Vec::new()
    }
}

/// Trait for response types.
pub trait OpenApiOutput {
    type Inner;
    fn operation_response(_ctx: &mut GenContext, _operation: &mut Operation) -> Option<Response> {
        None
    }
    fn inferred_responses(
        _ctx: &mut GenContext,
        _operation: &mut Operation,
    ) -> Vec<(Option<String>, Response)> {
        Vec::new()
    }
}

// --- Ignored types ---

impl<T> OpenApiExtractor for axum::extract::State<T> {
    fn operation_input(_: &mut GenContext, _: &mut Operation) {}
}
impl<T> OpenApiExtractor for axum::Extension<T> {
    fn operation_input(_: &mut GenContext, _: &mut Operation) {}
}
impl OpenApiExtractor for axum::http::HeaderMap {
    fn operation_input(_: &mut GenContext, _: &mut Operation) {}
}
impl OpenApiExtractor for axum::body::Body {
    fn operation_input(_: &mut GenContext, _: &mut Operation) {}
}

// --- Bytes (raw request body) ---

impl OpenApiExtractor for axum::body::Bytes {
    fn operation_input(_ctx: &mut GenContext, operation: &mut Operation) {
        let mut content = IndexMap::new();
        content.insert(
            "application/octet-stream".to_string(),
            MediaType {
                schema: None,
                ..Default::default()
            },
        );
        operation.request_body = Some(RefOr::Item(RequestBody {
            description: Some("Raw binary body".into()),
            content,
            required: Some(true),
        }));
    }
}

// --- RawQuery ---

impl OpenApiExtractor for axum::extract::RawQuery {
    fn operation_input(_ctx: &mut GenContext, operation: &mut Operation) {
        // RawQuery consumes the entire query string; document as a querystring param.
        let mut content = IndexMap::new();
        content.insert(
            "text/plain".to_string(),
            MediaType {
                schema: Some(RefOr::Item(Schema::Object(openapi3_rs::SchemaObject {
                    schema_data: {
                        let mut m = serde_json::Map::new();
                        m.insert("type".into(), "string".into());
                        m
                    },
                    ..Default::default()
                }))),
                ..Default::default()
            },
        );
        operation.parameters.get_or_insert_with(Vec::new).push(RefOr::Item(Parameter {
            name: "query".into(),
            location: ParameterIn::Querystring,
            description: Some("Raw query string".into()),
            required: Some(false),
            content: Some(content),
            ..Default::default()
        }));
    }
}

// --- Path<T> ---

impl<T: JsonSchema> OpenApiExtractor for axum::extract::Path<T> {
    fn operation_input(ctx: &mut GenContext, operation: &mut Operation) {
        let schema = ctx.schema_gen.subschema_for::<T>();
        let openapi_schema = to_openapi_schema(&schema);
        let params = parameters_from_schema(&openapi_schema, ParameterIn::Path);
        let existing = operation.parameters.get_or_insert_with(Vec::new);
        for mut p in params {
            p.required = Some(true);
            existing.push(RefOr::Item(p));
        }
    }
}

// --- Query<T> ---

impl<T: JsonSchema> OpenApiExtractor for axum::extract::Query<T> {
    fn operation_input(ctx: &mut GenContext, operation: &mut Operation) {
        let schema = ctx.schema_gen.subschema_for::<T>();
        let openapi_schema = to_openapi_schema(&schema);
        let params = parameters_from_schema(&openapi_schema, ParameterIn::Query);
        let existing = operation.parameters.get_or_insert_with(Vec::new);
        for p in params {
            existing.push(RefOr::Item(p));
        }
    }
}

// --- Json<T> ---

impl<T: JsonSchema> OpenApiExtractor for axum::Json<T> {
    fn operation_input(ctx: &mut GenContext, operation: &mut Operation) {
        let schema = ctx.schema_gen.subschema_for::<T>();
        let openapi_schema = to_openapi_schema(&schema);
        let mut content = IndexMap::new();
        content.insert(
            "application/json".to_string(),
            MediaType {
                schema: Some(RefOr::Item(openapi_schema)),
                ..Default::default()
            },
        );
        operation.request_body = Some(RefOr::Item(RequestBody {
            description: None,
            content,
            required: Some(true),
        }));
    }

    #[cfg(feature = "opinionated-errors")]
    fn inferred_early_responses(
        _: &mut GenContext,
        _: &mut Operation,
    ) -> Vec<(Option<String>, Response)> {
        vec![
            (
                Some("400".into()),
                Response {
                    description: "Bad Request — invalid JSON".into(),
                    ..Default::default()
                },
            ),
            (
                Some("415".into()),
                Response {
                    description: "Unsupported Media Type".into(),
                    ..Default::default()
                },
            ),
            (
                Some("422".into()),
                Response {
                    description: "Unprocessable Entity — validation failed".into(),
                    ..Default::default()
                },
            ),
        ]
    }
}

// --- Form<T> ---

impl<T: JsonSchema> OpenApiExtractor for axum::Form<T> {
    fn operation_input(ctx: &mut GenContext, operation: &mut Operation) {
        let schema = ctx.schema_gen.subschema_for::<T>();
        let openapi_schema = to_openapi_schema(&schema);
        let mut content = IndexMap::new();
        content.insert(
            "application/x-www-form-urlencoded".to_string(),
            MediaType {
                schema: Some(RefOr::Item(openapi_schema)),
                ..Default::default()
            },
        );
        operation.request_body = Some(RefOr::Item(RequestBody {
            description: None,
            content,
            required: Some(true),
        }));
    }
}

// --- Output impls ---

impl<T: JsonSchema> OpenApiOutput for axum::Json<T> {
    type Inner = T;
    fn operation_response(ctx: &mut GenContext, _operation: &mut Operation) -> Option<Response> {
        let schema = ctx.schema_gen.subschema_for::<T>();
        let openapi_schema = to_openapi_schema(&schema);
        let mut content = IndexMap::new();
        content.insert(
            "application/json".to_string(),
            MediaType {
                schema: Some(RefOr::Item(openapi_schema)),
                ..Default::default()
            },
        );
        Some(Response {
            description: "Successful response".to_string(),
            content: Some(content),
            ..Default::default()
        })
    }
    fn inferred_responses(
        ctx: &mut GenContext,
        operation: &mut Operation,
    ) -> Vec<(Option<String>, Response)> {
        if let Some(resp) = Self::operation_response(ctx, operation) {
            vec![(Some("200".into()), resp)]
        } else {
            Vec::new()
        }
    }
}

impl<T: JsonSchema> OpenApiOutput for (axum::http::StatusCode, axum::Json<T>) {
    type Inner = T;
    fn operation_response(ctx: &mut GenContext, _operation: &mut Operation) -> Option<Response> {
        let schema = ctx.schema_gen.subschema_for::<T>();
        let openapi_schema = to_openapi_schema(&schema);
        let mut content = IndexMap::new();
        content.insert(
            "application/json".to_string(),
            MediaType {
                schema: Some(RefOr::Item(openapi_schema)),
                ..Default::default()
            },
        );
        Some(Response {
            description: "Successful response".to_string(),
            content: Some(content),
            ..Default::default()
        })
    }
}

// --- Plain text response ---

impl OpenApiOutput for String {
    type Inner = String;
    fn operation_response(_ctx: &mut GenContext, _operation: &mut Operation) -> Option<Response> {
        let mut content = IndexMap::new();
        content.insert(
            "text/plain".to_string(),
            MediaType {
                schema: Some(RefOr::Item(Schema::Object(openapi3_rs::SchemaObject {
                    schema_data: {
                        let mut m = serde_json::Map::new();
                        m.insert("type".into(), "string".into());
                        m
                    },
                    ..Default::default()
                }))),
                ..Default::default()
            },
        );
        Some(Response {
            description: "Successful response".to_string(),
            content: Some(content),
            ..Default::default()
        })
    }
}

// --- Binary response ---

impl OpenApiOutput for axum::body::Bytes {
    type Inner = Vec<u8>;
    fn operation_response(_ctx: &mut GenContext, _operation: &mut Operation) -> Option<Response> {
        let mut content = IndexMap::new();
        content.insert(
            "application/octet-stream".to_string(),
            MediaType {
                schema: None,
                ..Default::default()
            },
        );
        Some(Response {
            description: "Binary response".to_string(),
            content: Some(content),
            ..Default::default()
        })
    }
}

// --- HTML response ---

impl<T: Send> OpenApiOutput for axum::response::Html<T> {
    type Inner = T;
    fn operation_response(_ctx: &mut GenContext, _operation: &mut Operation) -> Option<Response> {
        let mut content = IndexMap::new();
        content.insert(
            "text/html".to_string(),
            MediaType {
                schema: Some(RefOr::Item(Schema::Object(openapi3_rs::SchemaObject {
                    schema_data: {
                        let mut m = serde_json::Map::new();
                        m.insert("type".into(), "string".into());
                        m
                    },
                    ..Default::default()
                }))),
                ..Default::default()
            },
        );
        Some(Response {
            description: "HTML response".to_string(),
            content: Some(content),
            ..Default::default()
        })
    }
}

/// Build parameters from a JSON Schema. For object schemas with properties,
/// expands each property into a separate Parameter. For simple (non-object)
/// schemas, creates a single Parameter wrapping the entire schema.
pub(crate) fn parameters_from_schema(schema: &Schema, location: ParameterIn) -> Vec<Parameter> {
    parameters_from_schema_inner(schema, location, "value")
}

fn parameters_from_schema_inner(
    schema: &Schema,
    location: ParameterIn,
    default_name: &str,
) -> Vec<Parameter> {
    let mut params = Vec::new();
    // Try to extract object properties if present
    let has_properties = match schema {
        Schema::Object(obj) => obj
            .schema_data
            .get("properties")
            .and_then(|p| p.as_object())
            .map(|props| {
                let required_set: std::collections::HashSet<String> = obj
                    .schema_data
                    .get("required")
                    .and_then(|r| r.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                for (name, prop_schema) in props {
                    let description = prop_schema
                        .get("description")
                        .and_then(|d| d.as_str())
                        .map(String::from);
                    let prop_required =
                        location == ParameterIn::Path || required_set.contains(name);
                    let style = match location {
                        ParameterIn::Path | ParameterIn::Header => Some(Style::Simple),
                        ParameterIn::Query | ParameterIn::Cookie => Some(Style::Form),
                        _ => None,
                    };
                    let schema_data = if let serde_json::Value::Object(map) = prop_schema.clone() {
                        map
                    } else {
                        serde_json::Map::new()
                    };
                    params.push(Parameter {
                        name: name.clone(),
                        location: location.clone(),
                        description,
                        required: Some(prop_required),
                        style,
                        schema: Some(RefOr::Item(Schema::Object(openapi3_rs::SchemaObject {
                            schema_data,
                            ..Default::default()
                        }))),
                        ..Default::default()
                    });
                }
            })
            .is_some(),
        _ => false,
    };

    // If no properties found (scalar type like u64, String), create single param
    if !has_properties {
        let style = match location {
            ParameterIn::Path | ParameterIn::Header => Some(Style::Simple),
            ParameterIn::Query | ParameterIn::Cookie => Some(Style::Form),
            _ => None,
        };
        params.push(Parameter {
            name: default_name.to_string(),
            location: location.clone(),
            description: None,
            required: Some(location == ParameterIn::Path),
            style,
            schema: Some(RefOr::Item(schema.clone())),
            ..Default::default()
        });
    }
    params
}

#[cfg(test)]
mod tests {
    use super::*;
    use openapi3_rs::{ParameterIn, Schema as OasSchema};
    use schemars::{JsonSchema, schema_for};
    use serde::{Deserialize, Serialize};

    #[derive(JsonSchema, Serialize, Deserialize)]
    struct TestParams {
        name: String,
        count: Option<i32>,
    }

    #[test]
    fn params_from_flat_struct_query() {
        let schema = schema_for!(TestParams);
        let oas = crate::openapi::to_openapi_schema(&schema);
        let params = parameters_from_schema(&oas, ParameterIn::Query);

        assert_eq!(params.len(), 2);
        let name_p = params.iter().find(|p| p.name == "name").unwrap();
        let count_p = params.iter().find(|p| p.name == "count").unwrap();
        assert_eq!(name_p.required, Some(true));
        assert_eq!(count_p.required, Some(false));
        assert_eq!(name_p.location, ParameterIn::Query);
    }

    #[test]
    fn params_from_flat_struct_path_all_required() {
        let schema = schema_for!(TestParams);
        let oas = crate::openapi::to_openapi_schema(&schema);
        let params = parameters_from_schema(&oas, ParameterIn::Path);

        for p in &params {
            assert_eq!(
                p.required,
                Some(true),
                "path param {} should be required",
                p.name
            );
        }
    }

    #[test]
    fn params_from_empty_schema() {
        // Bool(true) schema has no properties → returns a single generic parameter
        let params = parameters_from_schema(&OasSchema::Bool(true), ParameterIn::Query);
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].name, "value");
    }

    #[test]
    fn params_header_style() {
        let schema = schema_for!(TestParams);
        let oas = crate::openapi::to_openapi_schema(&schema);
        let params = parameters_from_schema(&oas, ParameterIn::Header);

        for p in &params {
            assert!(
                matches!(p.style, Some(openapi3_rs::Style::Simple)),
                "header param should have Simple style"
            );
        }
    }
}

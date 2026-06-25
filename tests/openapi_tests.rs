//! Tests for the schema bridge between schemars and openapi3-rs.

use axoas::openapi::{
    binary_response, request_body_schema, response_schema, to_openapi_schema,
};
use openapi3_rs::Schema;
use schemars::{schema_for, JsonSchema};
use serde::Serialize;

#[derive(JsonSchema, Serialize)]
struct SimpleStruct {
    name: String,
    age: i32,
}

#[derive(JsonSchema, Serialize)]
struct OptionalFields {
    required_str: String,
    optional_num: Option<i64>,
    optional_bool: Option<bool>,
}

#[derive(JsonSchema, Serialize)]
struct NestedStruct {
    outer: String,
    inner: SimpleStruct,
}

#[derive(JsonSchema, Serialize)]
enum SimpleEnum {
    A,
    B,
    C,
}

#[derive(JsonSchema, Serialize)]
#[serde(tag = "type")]
enum TaggedEnum {
    #[serde(rename = "alpha")]
    Alpha { value: String },
    #[serde(rename = "beta")]
    Beta { count: i32 },
}

// ---- to_openapi_schema primitives ----

#[test]
fn schema_string() {
    let s = schema_for!(String);
    let oas = to_openapi_schema(&s);
    match &oas {
        Schema::Object(obj) => {
            assert_eq!(
                obj.schema_data.get("type").and_then(|v| v.as_str()),
                Some("string")
            );
        }
        _ => panic!("expected Schema::Object"),
    }
}

#[test]
fn schema_i32() {
    let s = schema_for!(i32);
    let oas = to_openapi_schema(&s);
    match &oas {
        Schema::Object(obj) => {
            assert_eq!(
                obj.schema_data.get("type").and_then(|v| v.as_str()),
                Some("integer")
            );
            assert_eq!(
                obj.schema_data.get("format").and_then(|v| v.as_str()),
                Some("int32")
            );
        }
        _ => panic!("expected Schema::Object"),
    }
}

#[test]
fn schema_bool() {
    let s = schema_for!(bool);
    let oas = to_openapi_schema(&s);
    match &oas {
        Schema::Object(obj) => {
            assert_eq!(
                obj.schema_data.get("type").and_then(|v| v.as_str()),
                Some("boolean")
            );
        }
        _ => panic!("expected Schema::Object"),
    }
}

#[test]
fn schema_f64() {
    let s = schema_for!(f64);
    let oas = to_openapi_schema(&s);
    match &oas {
        Schema::Object(obj) => {
            assert_eq!(
                obj.schema_data.get("type").and_then(|v| v.as_str()),
                Some("number")
            );
            assert_eq!(
                obj.schema_data.get("format").and_then(|v| v.as_str()),
                Some("double")
            );
        }
        _ => panic!("expected Schema::Object"),
    }
}

// ---- to_openapi_schema structs ----

#[test]
fn schema_simple_struct() {
    let s = schema_for!(SimpleStruct);
    let oas = to_openapi_schema(&s);
    match &oas {
        Schema::Object(obj) => {
            assert_eq!(
                obj.schema_data.get("type").and_then(|v| v.as_str()),
                Some("object")
            );
            let props = obj
                .schema_data
                .get("properties")
                .and_then(|v| v.as_object())
                .expect("should have properties");
            assert!(props.contains_key("name"));
            assert!(props.contains_key("age"));
            let required = obj
                .schema_data
                .get("required")
                .and_then(|v| v.as_array())
                .expect("should have required array");
            assert!(required.iter().any(|v| v.as_str() == Some("name")));
            assert!(required.iter().any(|v| v.as_str() == Some("age")));
        }
        _ => panic!("expected Schema::Object"),
    }
}

#[test]
fn schema_optional_fields() {
    let s = schema_for!(OptionalFields);
    let oas = to_openapi_schema(&s);
    match &oas {
        Schema::Object(obj) => {
            let required = obj
                .schema_data
                .get("required")
                .and_then(|v| v.as_array())
                .expect("should have required array");
            // Only required_str is required; Option fields are not
            assert!(required.iter().any(|v| v.as_str() == Some("required_str")));
            assert!(!required.iter().any(|v| v.as_str() == Some("optional_num")));
            assert!(!required.iter().any(|v| v.as_str() == Some("optional_bool")));
        }
        _ => panic!("expected Schema::Object"),
    }
}

#[test]
fn schema_nested_struct() {
    let s = schema_for!(NestedStruct);
    let oas = to_openapi_schema(&s);
    match &oas {
        Schema::Object(obj) => {
            let props = obj
                .schema_data
                .get("properties")
                .and_then(|v| v.as_object())
                .expect("should have properties");
            assert!(props.contains_key("outer"));
            // inner should be a nested object schema
            let inner = props.get("inner").and_then(|v| v.as_object());
            assert!(inner.is_some(), "inner field should be a nested object");
        }
        _ => panic!("expected Schema::Object"),
    }
}

// ---- to_openapi_schema enums ----

#[test]
fn schema_simple_enum() {
    let s = schema_for!(SimpleEnum);
    let oas = to_openapi_schema(&s);
    // Simple enums: schemars handles them — just verify it doesn't panic
    match &oas {
        Schema::Object(_obj) => {
            // OK — schemars may use enumValues or oneOf depending on version
        }
        _ => panic!("expected Schema::Object"),
    }
}

#[test]
fn schema_tagged_enum() {
    let s = schema_for!(TaggedEnum);
    let oas = to_openapi_schema(&s);
    match &oas {
        Schema::Object(obj) => {
            let one_of = obj.schema_data.get("oneOf");
            assert!(one_of.is_some(), "tagged enum should have oneOf");
        }
        _ => panic!("expected Schema::Object"),
    }
}

#[test]
fn schema_roundtrip_is_stable() {
    let s = schema_for!(SimpleStruct);
    let oas1 = to_openapi_schema(&s);
    let oas2 = to_openapi_schema(&s);
    // Two calls with the same input should produce identical output
    assert_eq!(oas1, oas2);
}

// ---- binary_response ----

#[test]
fn binary_response_pdf() {
    let (status, ref_or) = binary_response("200", "application/pdf", "PDF document", None);
    assert_eq!(status, "200");
    match &ref_or {
        openapi3_rs::RefOr::Item(response) => {
            assert_eq!(response.description, "PDF document");
            let content = response.content.as_ref().expect("should have content");
            let media = match content.get("application/pdf").expect("should have pdf media type") {
                openapi3_rs::RefOr::Item(m) => m,
                _ => panic!("expected RefOr::Item"),
            };
            assert!(media.schema.is_none(), "binary response should have no schema");
        }
        _ => panic!("expected RefOr::Item"),
    }
}

#[test]
fn binary_response_octet_stream() {
    let (status, _) = binary_response("200", "application/octet-stream", "Download", None);
    assert_eq!(status, "200");
}

// ---- request_body_schema ----

#[test]
fn request_body_required() {
    let s = schema_for!(SimpleStruct);
    let body = request_body_schema(&s, Some("test body"), true);
    assert_eq!(body.description.as_deref(), Some("test body"));
    assert_eq!(body.required, Some(true));
    let content = match body.content.get("application/json").expect("should have json content") {
        openapi3_rs::RefOr::Item(m) => m,
        _ => panic!("expected RefOr::Item"),
    };
    assert!(content.schema.is_some(), "should have schema");
}

#[test]
fn request_body_not_required() {
    let s = schema_for!(String);
    let body = request_body_schema(&s, None, false);
    assert!(body.description.is_none());
    assert_eq!(body.required, Some(false));
}

// ---- response_schema ----

#[test]
fn response_schema_200() {
    let s = schema_for!(SimpleStruct);
    let (status, ref_or) = response_schema(&s, "200", "Success", None);
    assert_eq!(status, "200");
    match &ref_or {
        openapi3_rs::RefOr::Item(response) => {
            assert_eq!(response.description, "Success");
            let content = response.content.as_ref().expect("should have content");
            assert!(content.contains_key("application/json"));
        }
        _ => panic!("expected RefOr::Item"),
    }
}

#[test]
fn response_schema_custom_status() {
    let s = schema_for!(SimpleStruct);
    let (status, _) = response_schema(&s, "201", "Created", None);
    assert_eq!(status, "201");
}

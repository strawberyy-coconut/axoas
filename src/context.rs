//! Context for schema generation.

use schemars::SchemaGenerator;

/// Context for OpenAPI documentation generation.
#[derive(Debug)]
pub struct GenContext {
    pub schema: SchemaGenerator,
    pub infer_error_responses: bool,
}

impl Default for GenContext {
    fn default() -> Self {
        Self { schema: SchemaGenerator::default(), infer_error_responses: true }
    }
}

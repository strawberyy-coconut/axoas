//! Context for schema generation.

use openapi3_rs::Components;
use schemars::SchemaGenerator;

/// Context for OpenAPI documentation generation.
///
/// Carries state that is threaded through `OpenApiExtractor` and
/// `OpenApiOutput` implementations. The `components` field allows
/// extractors to register reusable objects (security schemes,
/// schemas, responses, etc.) into the global `Components` map.
#[derive(Debug)]
pub struct GenContext {
    pub schema_gen: SchemaGenerator,
    pub components: Components,
    #[cfg(feature = "opinionated-errors")]
    pub infer_error_responses: bool,
}

impl Default for GenContext {
    fn default() -> Self {
        Self {
            schema_gen: SchemaGenerator::new(schemars::generate::SchemaSettings::openapi3()),
            components: Components::default(),
            #[cfg(feature = "opinionated-errors")]
            infer_error_responses: true,
        }
    }
}

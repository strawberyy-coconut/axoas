//! # Auth Example — Custom Security Extractor in Axoas
//!
//! Demonstrates how to write a custom auth extractor that:
//!
//! - Auto-registers its `SecurityScheme` into the OpenAPI `Components`
//! - Marks operations as requiring authentication (`Operation.security`)
//! - Auto-documents 401/403 error responses
//!
//! The extractor works with **zero router-level configuration** —
//! just use it in a handler signature and everything is generated.
//!
//! Run: `cargo run` then visit http://localhost:3000/openapi.json

use axoas::{openapi, route, DocRouter};
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use std::convert::Infallible;

// ============================================================
// 1. Define the custom BearerAuth extractor
// ============================================================

/// A custom extractor for Bearer token authentication.
///
/// Implements `OpenApiExtractor` so the OpenAPI spec is generated
/// automatically — no need to call `with_security_scheme()` or
/// `with_security_requirement()` on the router.
///
/// In a real app you'd implement `axum::extract::FromRequestParts`
/// to actually validate the token from the request headers.
struct BearerAuth;

// In a real app you'd validate the JWT here. For this demo
// we just pass through — the extractor exists solely for
// OpenAPI documentation.
impl<S: Sync> FromRequestParts<S> for BearerAuth {
    type Rejection = Infallible;
    async fn from_request_parts(_parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self)
    }
}

impl axoas::OpenApiExtractor for BearerAuth {
    fn operation_input(
        ctx: &mut axoas::GenContext,
        operation: &mut axoas::openapi3_rs::Operation,
    ) {
        // --- Register the security scheme definition ---
        // This goes into `components.securitySchemes` in the final spec.
        // Uses `or_insert_with` so a router-level
        // `with_security_scheme("bearerAuth", ...)` always wins.
        ctx.components
            .security_schemes
            .get_or_insert_with(axoas::indexmap::IndexMap::new)
            .entry("bearerAuth".to_string())
            .or_insert_with(|| {
                axoas::openapi3_rs::RefOr::Item(axoas::openapi3_rs::SecurityScheme {
                    scheme_type: axoas::openapi3_rs::SecuritySchemeType::Http,
                    scheme: Some("bearer".to_string()),
                    bearer_format: Some("JWT".to_string()),
                    description: Some(
                        "JWT Bearer token — obtain via POST /auth/login"
                            .to_string(),
                    ),
                    ..Default::default()
                })
            });

        // --- Mark this operation as requiring bearerAuth ---
        let mut req = axoas::indexmap::IndexMap::new();
        req.insert("bearerAuth".to_string(), Vec::new());
        operation
            .security
            .get_or_insert_with(Vec::new)
            .push(req);
    }

    fn inferred_early_responses(
        _ctx: &mut axoas::GenContext,
        _operation: &mut axoas::openapi3_rs::Operation,
    ) -> Vec<(Option<String>, axoas::openapi3_rs::Response)> {
        vec![
            (
                Some("401".into()),
                axoas::openapi3_rs::Response {
                    description:
                        "Unauthorized — missing or invalid Bearer token"
                            .into(),
                    ..Default::default()
                },
            ),
            (
                Some("403".into()),
                axoas::openapi3_rs::Response {
                    description:
                        "Forbidden — valid token but insufficient permissions"
                            .into(),
                    ..Default::default()
                },
            ),
        ]
    }
}

// ============================================================
// 2. Define a second extractor: ApiKeyAuth
// ============================================================

/// API key authentication via the `X-API-Key` header.
///
/// Shows how to use a different scheme type (`apiKey` instead of `http`)
/// and a descriptive name in the OpenAPI spec.
struct ApiKeyAuth;

// In a real app you'd validate the API key here.
impl<S: Sync> FromRequestParts<S> for ApiKeyAuth {
    type Rejection = Infallible;
    async fn from_request_parts(_parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self)
    }
}

impl axoas::OpenApiExtractor for ApiKeyAuth {
    fn operation_input(
        ctx: &mut axoas::GenContext,
        operation: &mut axoas::openapi3_rs::Operation,
    ) {
        ctx.components
            .security_schemes
            .get_or_insert_with(axoas::indexmap::IndexMap::new)
            .entry("apiKey".to_string())
            .or_insert_with(|| {
                axoas::openapi3_rs::RefOr::Item(axoas::openapi3_rs::SecurityScheme {
                    scheme_type: axoas::openapi3_rs::SecuritySchemeType::ApiKey,
                    name: Some("X-API-Key".to_string()),
                    location: Some(axoas::openapi3_rs::ApiKeyLocation::Header),
                    description: Some(
                        "API key passed in the X-API-Key header".to_string(),
                    ),
                    ..Default::default()
                })
            });

        let mut req = axoas::indexmap::IndexMap::new();
        req.insert("apiKey".to_string(), Vec::new());
        operation
            .security
            .get_or_insert_with(Vec::new)
            .push(req);
    }

    fn inferred_early_responses(
        _ctx: &mut axoas::GenContext,
        _operation: &mut axoas::openapi3_rs::Operation,
    ) -> Vec<(Option<String>, axoas::openapi3_rs::Response)> {
        vec![(
            Some("401".into()),
            axoas::openapi3_rs::Response {
                description: "Unauthorized — missing or invalid API key".into(),
                ..Default::default()
            },
        )]
    }
}

// ============================================================
// 3. Handlers
// ============================================================

#[openapi(tag = "public", summary = "Public endpoint — no auth")]
async fn public_hello() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({ "message": "Hello, world!" }))
}

#[openapi(
    tag = "protected",
    summary = "Protected by Bearer token",
    description = "Requires a valid JWT Bearer token in the Authorization header."
)]
async fn bearer_protected(_auth: BearerAuth) -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "message": "You have Bearer access!",
        "auth_type": "bearer"
    }))
}

#[openapi(
    tag = "protected",
    summary = "Protected by API key",
    description = "Requires a valid X-API-Key header."
)]
async fn api_key_protected(_auth: ApiKeyAuth) -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "message": "You have API key access!",
        "auth_type": "api_key"
    }))
}

// ============================================================
// 4. Login route — uses `.with()` to customize inline
// ============================================================

#[openapi(
    tag = "auth",
    summary = "Login",
    description = "Returns a JWT token for the given credentials."
)]
async fn login_doc() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({ "token": "eyJhbGciOi...", "type": "bearer" }))
}

// ============================================================
// 5. Assemble the router
// ============================================================

#[tokio::main]
async fn main() {
    let app = DocRouter::new()
        // --- Public endpoints (no security) ---
        .route("/public/hello", axoas::routing::get(route!(public_hello)))

        // --- Protected by BearerAuth ---
        // The extractor auto-registers "bearerAuth" in components
        // and documents 401/403 responses.
        .route(
            "/protected/bearer",
            axoas::routing::get(route!(bearer_protected)),
        )

        // --- Protected by ApiKeyAuth ---
        .route(
            "/protected/apikey",
            axoas::routing::get(route!(api_key_protected)),
        )

        // --- Login (no auth, doesn't consume the extractor) ---
        // Use `.with()` to add a 200 response description inline.
        .route(
            "/auth/login",
            axoas::routing::post(route!(login_doc).with(|op| {
                op.description = Some(
                    "Returns a JWT Bearer token. Use the token to access \
                     /protected/bearer via `Authorization: Bearer <token>`."
                        .into(),
                );
            })),
        )

        // --- Serve the OpenAPI spec ---
        .serve_openapi("/openapi.json")

        // --- Router-level overrides ---
        // These are OPTIONAL — the extractors already registered
        // their schemes.  Uncomment to override an extractor's
        // auto-definition:
        //
        // .with_security_scheme("bearerAuth",
        //     openapi3_rs::SecurityScheme {
        //         scheme_type: "http".into(),
        //         scheme: Some("bearer".into()),
        //         bearer_format: Some("JWT".into()),
        //         description: Some("OVERRIDDEN — custom description".into()),
        //         ..Default::default()
        //     })
        // .with_security_requirement("bearerAuth")  // global default

        .with_info(openapi3_rs::Info {
            title: "Auth Example API".into(),
            version: "1.0.0".into(),
            description: Some(
                "Demonstrates custom security extractors in axoas.\n\n\
                 - `BearerAuth` — HTTP Bearer JWT\n\
                 - `ApiKeyAuth` — X-API-Key header\n\n\
                 Both extractors auto-register their security schemes \
                 and document 401/403 responses without any router config."
                    .into(),
            ),
            summary: None,
            terms_of_service: None,
            contact: None,
            license: None,
            ..Default::default()
        })
        .into_axum_router();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    println!("Auth Example at http://localhost:3000");
    println!("  GET  /openapi.json         — OpenAPI spec");
    println!("  GET  /public/hello         — No auth");
    println!("  GET  /protected/bearer     — Bearer JWT required");
    println!("  GET  /protected/apikey     — X-API-Key required");
    println!("  POST /auth/login           — Get token");

    axum::serve(listener, app).await.unwrap();
}

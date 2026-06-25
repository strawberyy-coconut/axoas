//! Proc macros for axoas: `#[openapi]` attribute and `route!` macro.

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    FnArg, ItemFn, Pat, PatType, ReturnType, Type, TypePath,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

struct OpenApiArgs {
    tag: Option<String>,
    summary: Option<String>,
    description: Option<String>,
    operation_id: Option<String>,
    deprecated: Option<bool>,
    responses: Vec<ResponseArg>,
}

struct ResponseArg {
    status: String,
    ty: Option<Type>,
    content_type: Option<String>,
    description: String,
}

impl Parse for OpenApiArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut tag = None;
        let mut summary = None;
        let mut description = None;
        let mut operation_id = None;
        let mut deprecated = None;
        let mut responses = Vec::new();
        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            match key.to_string().as_str() {
                "tag" => {
                    let _: syn::Token![=] = input.parse()?;
                    tag = Some(input.parse::<syn::LitStr>()?.value());
                }
                "summary" => {
                    let _: syn::Token![=] = input.parse()?;
                    summary = Some(input.parse::<syn::LitStr>()?.value());
                }
                "description" => {
                    let _: syn::Token![=] = input.parse()?;
                    description = Some(input.parse::<syn::LitStr>()?.value());
                }
                "operation_id" => {
                    let _: syn::Token![=] = input.parse()?;
                    operation_id = Some(input.parse::<syn::LitStr>()?.value());
                }
                "deprecated" => {
                    let _: syn::Token![=] = input.parse()?;
                    deprecated = Some(input.parse::<syn::LitBool>()?.value);
                }
                "response" => {
                    let content;
                    syn::parenthesized!(content in input);
                    let mut status = String::new();
                    let mut ty = None;
                    let mut content_type = None;
                    let mut resp_desc = None;
                    while !content.is_empty() {
                        // `type` is a Rust keyword, can't be parsed as Ident — peek for it
                        let key_str: String = if content.peek(syn::Token![type]) {
                            let _: syn::Token![type] = content.parse()?;
                            "type".into()
                        } else {
                            content.parse::<syn::Ident>()?.to_string()
                        };
                        let _: syn::Token![=] = content.parse()?;
                        match key_str.as_str() {
                            "status" => status = content.parse::<syn::LitStr>()?.value(),
                            "type" | "schema" => ty = Some(content.parse()?),
                            "content_type" => {
                                content_type = Some(content.parse::<syn::LitStr>()?.value())
                            }
                            "description" => {
                                resp_desc = Some(content.parse::<syn::LitStr>()?.value())
                            }
                            _ => {
                                return Err(syn::Error::new(
                                    content.span(),
                                    format!("unknown response field: {key_str}"),
                                ));
                            }
                        }
                        if content.peek(syn::Token![,]) {
                            let _: syn::Token![,] = content.parse()?;
                        }
                    }
                    // At the end of the "response" => { ... } block, before .push():
                    if resp_desc.is_none() {
                        return Err(syn::Error::new(
                            content.span(),
                            "response() requires a `description` field",
                        ));
                    }
                    responses.push(ResponseArg {
                        status,
                        ty,
                        content_type,
                        description: resp_desc
                            .expect("description is required; already validated above"),
                    });
                }
                _ => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("unknown attribute: {key}"),
                    ));
                }
            }
            if input.peek(syn::Token![,]) {
                let _: syn::Token![,] = input.parse()?;
            }
        }
        Ok(Self {
            tag,
            summary,

            operation_id,
            deprecated,
            responses,
            description: description,
        })
    }
}

fn classify_param(ty: &Type) -> &'static str {
    if let Type::Path(TypePath { path, .. }) = ty {
        if let Some(seg) = path.segments.last() {
            return match seg.ident.to_string().as_str() {
                "Path" => "path",
                "Query" => "query",
                "Json" => "body_json",
                "Form" => "body_form",
                "State" => "ignore",
                "Extension" => "ignore",
                "HeaderMap" => "headers",
                "TypedHeader" => "header",
                "TypedMultipart" | "BaseMultipart" => "multipart",
                _ => "custom",
            };
        }
    }
    "custom"
}

fn extract_inner_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(TypePath { path, .. }) = ty {
        if let Some(seg) = path.segments.last() {
            if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                    return Some(inner);
                }
            }
        }
    }
    None
}

fn is_json_return(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        path.segments
            .last()
            .map(|s| s.ident == "Json")
            .unwrap_or(false)
    } else {
        false
    }
}

fn is_tuple_with_json(ty: &Type) -> bool {
    if let Type::Tuple(tuple) = ty {
        tuple.elems.len() == 2 && is_json_return(&tuple.elems[1])
    } else {
        false
    }
}

fn is_unit(ty: &Type) -> bool {
    matches!(ty, Type::Tuple(t) if t.elems.is_empty())
}

/// Extract the binding name from a pattern (e.g., `Path(id)` → Some("id"), `_` → None).
fn pat_name(pat: &Pat) -> Option<String> {
    match pat {
        Pat::Ident(pi) => Some(pi.ident.to_string()),
        // Destructured patterns: `Path(id)`, `Query(params)`, `State(state)`, `Json(body)`
        Pat::TupleStruct(pts) => {
            for elem in &pts.elems {
                if let Pat::Ident(pi) = elem {
                    return Some(pi.ident.to_string());
                }
            }
            None
        }
        _ => None,
    }
}

#[proc_macro_attribute]
pub fn openapi(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let args = parse_macro_input!(attr as OpenApiArgs);
    let fn_name = &input.sig.ident;
    let vis = &input.vis;

    let hash = fxhash::hash64(&fn_name.to_string());
    let doc_fn_name = format_ident!("__axoas_doc_{hash:x}");

    let mut input_calls = Vec::new();
    let mut early_calls = Vec::new();

    for arg in &input.sig.inputs {
        if let FnArg::Typed(pat_type) = arg {
            let ty = &pat_type.ty;
            let name = pat_name(&pat_type.pat);
            match classify_param(ty) {
                "path" | "query" | "body_json" | "body_form" | "header" | "multipart" => {
                    input_calls.push(quote! { <#ty as ::axoas::OpenApiExtractor>::operation_input(&mut ctx, &mut op); });
                    // Override parameter names with binding names for path/query params
                    if matches!(classify_param(ty), "path" | "query") {
                        if let Some(n) = &name {
                            input_calls.push(quote! {
                                if let Some(params) = &mut op.parameters {
                                    for p in params.iter_mut().rev().take(1) {
                                        if let ::axoas::openapi3_rs::RefOr::Item(param) = p {
                                            param.name = #n.to_string();
                                        }
                                    }
                                }
                            });
                        }
                    }
                    early_calls.push(quote! {
                        for (code, resp) in <#ty as ::axoas::OpenApiExtractor>::inferred_early_responses(&mut ctx, &mut op) {
                            if let Some(c) = code { rm.entry(c.to_string()).or_insert(::axoas::openapi3_rs::RefOr::Item(resp)); }
                            else { responses.default = Some(::axoas::openapi3_rs::RefOr::Item(resp)); }
                        }
                    });
                }
                "custom" => {
                    input_calls.push(quote! { <#ty as ::axoas::OpenApiExtractor>::operation_input(&mut ctx, &mut op); });
                    early_calls.push(quote! {
                        for (code, resp) in <#ty as ::axoas::OpenApiExtractor>::inferred_early_responses(&mut ctx, &mut op) {
                            if let Some(c) = code { rm.entry(c.to_string()).or_insert(::axoas::openapi3_rs::RefOr::Item(resp)); }
                            else { responses.default = Some(::axoas::openapi3_rs::RefOr::Item(resp)); }
                        }
                    });
                }
                _ => {}
            }
        }
    }

    let output_call = match &input.sig.output {
        ReturnType::Type(_, ty) => {
            if is_json_return(ty) {
                if !args.responses.is_empty() {
                    build_response_entries(&args.responses)
                } else {
                    quote! {
                        if let Some(resp) = <#ty as ::axoas::OpenApiOutput>::operation_response(&mut ctx, &mut op) {
                            rm.insert("200".to_string(), ::axoas::openapi3_rs::RefOr::Item(resp));
                        }
                        for (code, resp) in <#ty as ::axoas::OpenApiOutput>::inferred_responses(&mut ctx, &mut op) {
                            if let Some(c) = code { rm.entry(c.to_string()).or_insert(::axoas::openapi3_rs::RefOr::Item(resp)); }
                            else { responses.default = Some(::axoas::openapi3_rs::RefOr::Item(resp)); }
                        }
                    }
                }
            } else if is_tuple_with_json(ty) {
                if !args.responses.is_empty() {
                    build_response_entries(&args.responses)
                } else {
                    quote! {
                        if let Some(resp) = <#ty as ::axoas::OpenApiOutput>::operation_response(&mut ctx, &mut op) {
                            rm.insert("200".to_string(), ::axoas::openapi3_rs::RefOr::Item(resp));
                        }
                        for (code, resp) in <#ty as ::axoas::OpenApiOutput>::inferred_responses(&mut ctx, &mut op) {
                            if let Some(c) = code { rm.entry(c.to_string()).or_insert(::axoas::openapi3_rs::RefOr::Item(resp)); }
                            else { responses.default = Some(::axoas::openapi3_rs::RefOr::Item(resp)); }
                        }
                    }
                }
            } else if is_unit(ty) {
                if !args.responses.is_empty() {
                    build_response_entries(&args.responses)
                } else {
                    quote! { rm.insert("200".to_string(), ::axoas::openapi3_rs::RefOr::Item(
                    ::axoas::openapi3_rs::Response { description: "OK".to_string(), ..Default::default() })); }
                }
            } else {
                // StatusCode and other non-JSON non-tuple types
                if !args.responses.is_empty() {
                    build_response_entries(&args.responses)
                } else {
                    quote! { rm.insert("200".to_string(), ::axoas::openapi3_rs::RefOr::Item(
                    ::axoas::openapi3_rs::Response { description: "OK".to_string(), ..Default::default() })); }
                }
            }
        }
        ReturnType::Default => {
            quote! { rm.insert("200".to_string(), ::axoas::openapi3_rs::RefOr::Item(
            ::axoas::openapi3_rs::Response { description: "OK".to_string(), ..Default::default() })); }
        }
    };

    let tag_val = args
        .tag
        .map(|t| quote! { op.tags = Some(vec![#t.to_string()]); });
    let summary_val = args
        .summary
        .map(|s| quote! { op.summary = Some(#s.to_string()); });
    let desc_val = args
        .description
        .map(|d| quote! { op.description = Some(#d.to_string()); });
    let opid_val = args
        .operation_id
        .map(|id| quote! { op.operation_id = Some(#id.to_string()); });
    let dep_val = args
        .deprecated
        .map(|d| quote! { op.deprecated = Some(#d); });

    let output = quote! {
        #input

        #[doc(hidden)] #[allow(non_snake_case)]
        #vis fn #doc_fn_name() -> (::axoas::openapi3_rs::Operation, ::axoas::openapi3_rs::Components) {
            let mut ctx = ::axoas::GenContext::default();
            let mut op = ::axoas::openapi3_rs::Operation::default();
            #tag_val #summary_val #desc_val #opid_val #dep_val
            #(#input_calls)*
            let mut rm = ::axoas::indexmap::IndexMap::new();
            let mut responses = ::axoas::openapi3_rs::Responses::default();
            #output_call
            #(#early_calls)*
            responses.responses = rm;
            op.responses = responses;

            for (name, schema) in ctx.schema.take_definitions(true) {
                let schemas_map = ctx.components.schemas
                    .get_or_insert_with(::axoas::indexmap::IndexMap::new);
                let oas_schema = ::axoas::openapi::definition_to_openapi_schema(&schema);
                schemas_map.insert(name, oas_schema);
            }

            (op, ctx.components)
        }
    };
    output.into()
}

/// Build the token stream for inserting explicit response entries into `rm`.
fn build_response_entries(responses: &[ResponseArg]) -> proc_macro2::TokenStream {
    let mut entries = Vec::new();
    for r in responses {
        let status = &r.status;

        let desc = &r.description;
        if let Some(ref_t) = &r.ty {
            // response_schema already returns (String, RefOr<Response>) — no extra wrap
            entries.push(quote! {
                rm.insert(#status.to_string(),
                    ::axoas::openapi::typed_response_schema::<#ref_t>(
                        &mut ctx, #status, #desc
                    ).1
                );
            });
        } else if let Some(ct) = &r.content_type {
            // binary_response already returns (String, RefOr<Response>) — no extra wrap
            let ct_s = ct.clone();
            entries.push(quote! {
                rm.insert(#status.to_string(),
                    ::axoas::openapi::binary_response(#status, #ct_s, #desc).1
                );
            });
        } else {
            // Bare Response — needs RefOr::Item
            entries.push(quote! {
                rm.insert(#status.to_string(), ::axoas::openapi3_rs::RefOr::Item(
                    ::axoas::openapi3_rs::Response {
                        description: #desc.to_string(),
                        ..Default::default()
                    }
                ));
            });
        }
    }
    quote! { #(#entries)* }
}

#[proc_macro]
pub fn route(input: TokenStream) -> TokenStream {
    let handler_str = input.to_string().trim().to_string();
    let handler_ident = syn::parse_str::<syn::Ident>(&handler_str)
        .unwrap_or_else(|_| panic!("route! requires a valid identifier, got: {handler_str}"));
    let hash = fxhash::hash64(&handler_str);
    let doc_fn_name = format_ident!("__axoas_doc_{hash:x}");
    quote! {
        {
            let (__axoas_op, __axoas_comp) = #doc_fn_name();
            axoas::DocHandler::new_with_components(#handler_ident, __axoas_op, __axoas_comp)
        }
    }
    .into()
}

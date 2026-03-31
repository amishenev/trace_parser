//! Code generation for trace_event macros.

use crate::attrs::{DefineTemplateAttr, FieldAttr, TraceEventAttr, TraceMarkersAttr};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

/// Generate `impl EventType` for the struct
pub fn generate_event_type_impl(
    struct_name: &Ident,
    event_attr: &TraceEventAttr,
) -> TokenStream {
    let name = &event_attr.name;
    let aliases = &event_attr.aliases;

    quote! {
        impl ::trace_parser::common::EventType for #struct_name {
            const EVENT_NAME: &'static str = #name;
            const EVENT_ALIASES: &'static [&'static str] = &[#(#aliases),*];
        }
    }
}

/// Generate `impl FastMatch` for the struct
pub fn generate_fast_match_impl(
    struct_name: &Ident,
    markers_attr: Option<&TraceMarkersAttr>,
) -> TokenStream {
    if let Some(attr) = markers_attr {
        let markers = &attr.0;
        // Convert string markers to byte slices
        let marker_bytes: Vec<TokenStream> = markers
            .iter()
            .map(|m| {
                let bytes = m.as_bytes();
                quote! { &[#(#bytes),*] }
            })
            .collect();

        quote! {
            impl ::trace_parser::common::FastMatch for #struct_name {
                const PAYLOAD_MARKERS: &'static [&'static [u8]] = &[#(#marker_bytes),*];
            }
        }
    } else {
        // No markers - use default empty implementation
        quote! {
            impl ::trace_parser::common::FastMatch for #struct_name {}
        }
    }
}

/// Generate `impl TemplateEvent` for the struct
pub fn generate_template_event_impl(
    struct_name: &Ident,
    templates: &[DefineTemplateAttr],
    fields: &[(&Ident, &FieldAttr)],
) -> TokenStream {
    if templates.is_empty() {
        return quote! {};
    }

    // Generate format registry with all templates
    let format_specs: Vec<TokenStream> = templates
        .iter()
        .enumerate()
        .map(|(id, template)| {
            let template_str = &template.0;
            let id = id as u8;
            quote! {
                ::trace_parser::format_registry::FormatSpec {
                    kind: #id,
                    template: &::trace_parser::payload_template::PayloadTemplate::new(
                        #template_str,
                        &[]
                    )
                }
            }
        })
        .collect();

    // Generate field specs for the template (used in parse_payload)
    let _field_specs: Vec<TokenStream> = fields
        .iter()
        .map(|(field_name, field_attr)| {
            let name_str = field_attr.name.as_ref()
                .map(|s| s.clone())
                .unwrap_or_else(|| field_name.to_string());
            let ty = &field_attr.ty;
            
            let field_spec = match ty.as_str() {
                "string" => quote! { ::trace_parser::payload_template::FieldSpec::string() },
                "u32" => quote! { ::trace_parser::payload_template::FieldSpec::u32() },
                "i32" => quote! { ::trace_parser::payload_template::FieldSpec::i32() },
                "f64" => quote! { ::trace_parser::payload_template::FieldSpec::f64() },
                "bool_int" => quote! { ::trace_parser::payload_template::FieldSpec::bool_int() },
                _ => quote! { ::trace_parser::payload_template::FieldSpec::custom(r".+") },
            };

            quote! {
                (#name_str, #field_spec)
            }
        })
        .collect();

    quote! {
        impl ::trace_parser::common::TemplateEvent for #struct_name {
            fn formats() -> &'static ::trace_parser::format_registry::FormatRegistry {
                static FORMATS: ::std::sync::LazyLock<::trace_parser::format_registry::FormatRegistry> = 
                    ::std::sync::LazyLock::new(|| {
                        ::trace_parser::format_registry::FormatRegistry::new(vec![
                            #(#format_specs),*
                        ])
                    });
                &FORMATS
            }

            fn detect_format(_payload: &str) -> u8 {
                0
            }

            fn parse_payload(
                parts: ::trace_parser::common::BaseTraceParts,
                captures: &::regex::Captures<'_>,
                _format_id: u8,
            ) -> ::std::option::Option<Self> {
                // TODO: implement parsing based on field specs
                None
            }

            fn render_payload(&self) -> ::pyo3::PyResult<::std::string::String> {
                // TODO: implement rendering based on field specs
                Ok(String::new())
            }
        }
    }
}

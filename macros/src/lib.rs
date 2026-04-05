//! Proc-macros for trace_parser
//!
//! Provides derive macros for automatic generation of trace event implementations.
//!
//! # Available macros
//!
//! - `#[derive(TraceEvent)]` - for regular trace events
//! - `#[derive(TracingMarkEvent)]` - for tracing_mark_write subtypes
//!
//! # Attributes
//!
//! - `#[trace_event(name = "...", aliases = ["..."])]` - event name and aliases
//! - `#[trace_markers("...", "...")]` - payload markers for FastMatch
//! - `#[fast_match(contains_any = ["...", ...])]` - optional `payload_quick_check` substrings
//! - `#[define_template("...")]` - payload template (can be multiple)
//! - `#[field(name = "...", regex = "...", choice = [...], format = "...", readonly, private)]` - field attributes

#[cfg(test)]
pub mod attrs;

#[cfg(not(test))]
mod attrs;

mod enum_gen;
mod generator;
mod pymethods;
mod stub_gen;

use attrs::{
    MarkType, find_define_template_attrs, find_fast_match_attr, find_field_attr,
    find_trace_event_attr, find_trace_markers_attr,
};
use enum_gen::{generate_trace_enum, parse_variants};
use generator::{
    generate_event_type_impl, generate_fast_match_impl, generate_registration,
    generate_template_event_impl,
};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use pymethods::generate_pymethods_block;
use quote::quote;
use syn::{DeriveInput, Fields, ItemStruct, parse_macro_input};

fn build_pyclass_attr(attr_tokens: TokenStream2) -> TokenStream2 {
    if attr_tokens.is_empty() {
        return quote! { #[pyo3::pyclass(skip_from_py_object)] };
    }

    let attr_text = attr_tokens.to_string();
    if attr_text.contains("skip_from_py_object") || attr_text.contains("from_py_object") {
        quote! { #[pyo3::pyclass(#attr_tokens)] }
    } else {
        quote! { #[pyo3::pyclass(skip_from_py_object, #attr_tokens)] }
    }
}

fn expand_event_class(
    item: ItemStruct,
    pyclass_attr: TokenStream2,
    derive_macro: TokenStream2,
) -> TokenStream {
    let expanded = quote! {
        #pyclass_attr
        #[derive(Clone, Debug, PartialEq)]
        #[derive(#derive_macro)]
        #item
    };
    expanded.into()
}

/// Derive macro for regular trace events.
///
/// # Example
///
/// ```rust,ignore
/// #[trace_event(name = "sched_switch", aliases = ["sched_sw"])]
/// #[define_template("prev_comm={prev_comm} prev_pid={prev_pid} ...")]
/// #[derive(TraceEvent)]
/// struct TraceSchedSwitch {
///     #[field(ty = "string")]
///     prev_comm: String,
/// }
/// ```
#[proc_macro_derive(
    TraceEvent,
    attributes(trace_event, trace_markers, fast_match, define_template, field)
)]
pub fn derive_trace_event(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Parse attributes
    let event_attr = match find_trace_event_attr(&input.attrs) {
        Some(attr) => attr,
        None => {
            return syn::Error::new(input.ident.span(), "missing #[trace_event] attribute")
                .to_compile_error()
                .into();
        }
    };

    let markers_attr = find_trace_markers_attr(&input.attrs);
    let fast_match_attr = find_fast_match_attr(&input.attrs);
    let contains_any: &[String] = fast_match_attr
        .as_ref()
        .map(|a| a.contains_any.as_slice())
        .unwrap_or(&[]);
    let templates = find_define_template_attrs(&input.attrs);

    // Parse fields - only named fields with identifiers
    // Collect (ident, field_type, field_attr)
    let fields = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => fields
                .named
                .iter()
                .filter_map(|f| {
                    f.ident.as_ref().and_then(|ident| {
                        find_field_attr(&f.attrs).map(|attr| (ident.clone(), f.ty.clone(), attr))
                    })
                })
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        },
        _ => Vec::new(),
    };

    // Generate code
    let event_type_impl = generate_event_type_impl(&input.ident, &event_attr);
    let fast_match_impl =
        generate_fast_match_impl(&input.ident, markers_attr.as_ref(), contains_any);
    let template_event_impl = generate_template_event_impl(&input.ident, &templates, &fields);
    let registration = generate_registration(&input.ident, &event_attr, false);

    // Generate pymethods only if requested
    let pymethods = if event_attr.generate_pymethods {
        stub_gen::write_stub_file(&input.ident, &fields, &event_attr);
        generate_pymethods_block(&input.ident, &fields)
    } else {
        quote! {}
    };

    let expanded = quote! {
        #event_type_impl
        #fast_match_impl
        #template_event_impl
        #pymethods
        #registration
    };

    expanded.into()
}

/// Attribute macro wrapper for regular trace events.
/// Adds `#[pyclass(skip_from_py_object)]`, `#[derive(Clone, Debug, PartialEq)]`,
/// and `#[derive(TraceEvent)]` to reduce boilerplate.
#[proc_macro_attribute]
pub fn trace_event_class(attr: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemStruct);
    let pyclass_attr = build_pyclass_attr(attr.into());
    expand_event_class(
        item,
        pyclass_attr,
        quote! { trace_parser_macros::TraceEvent },
    )
}

/// Derive macro for tracing_mark_write subtypes.
///
/// # Example
///
/// ```rust,ignore
/// #[trace_event(name = "tracing_mark_write")]
/// #[trace_markers("B|")]
/// #[define_template("B|{trace_mark_tgid}|{message}")]
/// #[derive(TracingMarkEvent)]
/// struct TraceMarkBegin {
///     #[field(ty = "u32")]
///     trace_mark_tgid: u32,
/// }
/// ```
#[proc_macro_derive(
    TracingMarkEvent,
    attributes(trace_event, trace_markers, fast_match, define_template, field)
)]
pub fn derive_tracing_mark_event(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Parse attributes
    let event_attr = match find_trace_event_attr(&input.attrs) {
        Some(attr) => attr,
        None => {
            return syn::Error::new(input.ident.span(), "missing #[trace_event] attribute")
                .to_compile_error()
                .into();
        }
    };

    let fast_match_attr = find_fast_match_attr(&input.attrs);
    let contains_any: &[String] = fast_match_attr
        .as_ref()
        .map(|a| a.contains_any.as_slice())
        .unwrap_or(&[]);

    // Determine markers and template prefix from mark_type
    let (markers_attr, templates) = if let Some(mark_type) = event_attr.mark_type {
        let (prefix, marker) = match mark_type {
            MarkType::Begin => ("B|{trace_mark_tgid}|", "B|"),
            MarkType::End => ("E|{trace_mark_tgid}|", "E|"),
        };

        // Wrap templates with prefix
        let wrapped_templates: Vec<_> = find_define_template_attrs(&input.attrs)
            .into_iter()
            .map(|t| {
                let wrapped_template = format!("{}{}", prefix, t.template);
                crate::attrs::DefineTemplateAttr {
                    template: wrapped_template,
                    id: t.id,
                    detect: t.detect.clone(),
                    extra_fields: t.extra_fields,
                }
            })
            .collect();

        // Merge B|/E| with user-provided trace_markers (if any)
        let mut all_markers = vec![marker.to_string()];
        if let Some(user_markers) = find_trace_markers_attr(&input.attrs) {
            all_markers.extend(user_markers.0);
        }
        let markers = crate::attrs::TraceMarkersAttr(all_markers);
        (Some(markers), wrapped_templates)
    } else {
        (
            find_trace_markers_attr(&input.attrs),
            find_define_template_attrs(&input.attrs),
        )
    };

    // Parse fields - only named fields with identifiers
    // Collect (ident, field_type, field_attr)
    let fields = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => fields
                .named
                .iter()
                .filter_map(|f| {
                    f.ident.as_ref().and_then(|ident| {
                        find_field_attr(&f.attrs).map(|attr| (ident.clone(), f.ty.clone(), attr))
                    })
                })
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        },
        _ => Vec::new(),
    };

    // Generate code
    let event_type_impl = generate_event_type_impl(&input.ident, &event_attr);
    let fast_match_impl =
        generate_fast_match_impl(&input.ident, markers_attr.as_ref(), contains_any);
    let template_event_impl = generate_template_event_impl(&input.ident, &templates, &fields);
    let registration = generate_registration(&input.ident, &event_attr, true);

    // Generate pymethods only if requested
    let pymethods = if event_attr.generate_pymethods {
        stub_gen::write_stub_file(&input.ident, &fields, &event_attr);
        generate_pymethods_block(&input.ident, &fields)
    } else {
        quote! {}
    };

    let expanded = quote! {
        #event_type_impl
        #fast_match_impl
        #template_event_impl
        #pymethods
        #registration
    };

    expanded.into()
}

/// Attribute macro wrapper for tracing_mark events.
/// Adds `#[pyclass(skip_from_py_object)]`, `#[derive(Clone, Debug, PartialEq)]`,
/// and `#[derive(TracingMarkEvent)]` to reduce boilerplate.
#[proc_macro_attribute]
pub fn tracing_mark_event_class(attr: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemStruct);
    let pyclass_attr = build_pyclass_attr(attr.into());
    expand_event_class(
        item,
        pyclass_attr,
        quote! { trace_parser_macros::TracingMarkEvent },
    )
}

/// Derive macro for payload enum types.
/// Generates Display, FromStr, and TraceEnum implementations.
/// Use `#[value("...")]` to specify the string representation.
#[proc_macro_derive(TraceEnum, attributes(value))]
pub fn derive_trace_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let variants = match &input.data {
        syn::Data::Enum(data) => parse_variants(&data.variants),
        _ => {
            return syn::Error::new(input.ident.span(), "TraceEnum only works on enums")
                .to_compile_error()
                .into();
        }
    };
    let generated = generate_trace_enum(&input.ident, &variants);
    generated.into()
}

#[cfg(test)]
mod tests {
    use super::build_pyclass_attr;
    use quote::quote;

    #[test]
    fn build_pyclass_attr_default_skip_from_py_object() {
        let attr = build_pyclass_attr(quote! {});
        let code = attr.to_string();
        assert!(code.contains("pyclass"));
        assert!(code.contains("skip_from_py_object"));
    }

    #[test]
    fn build_pyclass_attr_preserves_explicit_skip() {
        let attr =
            build_pyclass_attr(quote! { skip_from_py_object, module = "trace_parser._native" });
        let code = attr.to_string();
        assert!(code.contains("skip_from_py_object"));
        assert!(code.contains("module"));
        assert_eq!(code.matches("skip_from_py_object").count(), 1);
    }

    #[test]
    fn build_pyclass_attr_preserves_explicit_from_py_object() {
        let attr = build_pyclass_attr(quote! { from_py_object, module = "trace_parser._native" });
        let code = attr.to_string();
        assert!(code.contains("from_py_object"));
        assert!(code.contains("module"));
        assert!(!code.contains("skip_from_py_object"));
    }

    #[test]
    fn build_pyclass_attr_adds_skip_for_other_options() {
        let attr = build_pyclass_attr(quote! { module = "trace_parser._native" });
        let code = attr.to_string();
        assert!(code.contains("skip_from_py_object"));
        assert!(code.contains("module"));
    }
}

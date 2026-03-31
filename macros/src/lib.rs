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
//! - `#[define_template("...")]` - payload template (can be multiple)
//! - `#[field(ty = "...", name = "...", optional)]` - field attributes

mod attrs;
mod generator;
mod pymethods;

use attrs::{
    find_trace_event_attr, find_trace_markers_attr, find_define_template_attrs,
    find_field_attr,
};
use generator::{generate_event_type_impl, generate_fast_match_impl, generate_template_event_impl};
use pymethods::generate_pymethods_block;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Fields};

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
#[proc_macro_derive(TraceEvent, attributes(trace_event, trace_markers, define_template, field))]
pub fn derive_trace_event(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    
    // Parse attributes
    let event_attr = match find_trace_event_attr(&input.attrs) {
        Some(attr) => attr,
        None => return syn::Error::new(input.ident.span(), "missing #[trace_event] attribute")
            .to_compile_error().into(),
    };
    
    let markers_attr = find_trace_markers_attr(&input.attrs);
    let templates = find_define_template_attrs(&input.attrs);
    
    // Parse fields - only named fields with identifiers
    let fields = match &input.data {
        syn::Data::Struct(data) => {
            match &data.fields {
                Fields::Named(fields) => {
                    fields.named.iter()
                        .filter_map(|f| {
                            f.ident.as_ref().and_then(|ident| {
                                find_field_attr(&f.attrs).map(|attr| (ident.clone(), attr))
                            })
                        })
                        .collect::<Vec<_>>()
                }
                _ => Vec::new(),
            }
        }
        _ => Vec::new(),
    };
    
    // Generate code
    let event_type_impl = generate_event_type_impl(&input.ident, &event_attr);
    let fast_match_impl = generate_fast_match_impl(&input.ident, markers_attr.as_ref());
    let template_event_impl = generate_template_event_impl(&input.ident, &templates, &fields);
    let pymethods = generate_pymethods_block(&input.ident, &fields);
    
    let expanded = quote! {
        #event_type_impl
        #fast_match_impl
        #template_event_impl
        #pymethods
    };
    
    expanded.into()
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
#[proc_macro_derive(TracingMarkEvent, attributes(trace_event, trace_markers, define_template, field))]
pub fn derive_tracing_mark_event(input: TokenStream) -> TokenStream {
    unimplemented!("TODO: implement TracingMarkEvent derive macro")
}

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

use proc_macro::TokenStream;

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
    unimplemented!("TODO: implement TraceEvent derive macro")
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

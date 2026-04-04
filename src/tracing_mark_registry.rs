//! Registry for tracing_mark_write subtypes.
//!
//! Handles parsing of tracing_mark_write events with multiple subtypes:
//! 1. Specific subtypes (ReceiveVsync, RequestVsync, SubmitVsync, etc.)
//! 2. Begin/End markers (hardcoded)
//! 3. Base TracingMark (fallback)

use pyo3::prelude::*;

/// A registered tracing_mark parser entry
pub struct TracingMarkEntry {
    pub parser: fn(Python<'_>, &str) -> Option<Py<PyAny>>,
}

// Collect all registered tracing_mark parsers at compile time
inventory::collect!(TracingMarkEntry);

/// Parse tracing_mark_write events.
///
/// Order of parsing:
/// 1. Registered specific subtypes (ReceiveVsync, RequestVsync, etc.)
/// 2. TraceMarkBegin (hardcoded)
/// 3. TraceMarkEnd (hardcoded)
/// 4. TracingMark (fallback)
pub fn parse_tracing_mark(py: Python<'_>, line: &str) -> Option<Py<PyAny>> {
    // 1. Try registered specific subtypes first
    for entry in inventory::iter::<TracingMarkEntry> {
        if let Some(event) = (entry.parser)(py, line) {
            return Some(event);
        }
    }

    // 2. Try Begin marker (hardcoded)
    if let Some(event) = crate::parse_and_wrap(py, line, crate::TraceMarkBegin::parse) {
        return Some(event);
    }

    // 3. Try End marker (hardcoded)
    if let Some(event) = crate::parse_and_wrap(py, line, crate::TraceMarkEnd::parse) {
        return Some(event);
    }

    // 4. Fallback to base TracingMark
    crate::parse_and_wrap(py, line, crate::TracingMark::parse)
}

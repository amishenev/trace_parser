//! Registry for auto-registering trace event parsers.
//!
//! Uses the `inventory` crate for compile-time registration without runtime overhead.

use pyo3::prelude::*;

/// A registered parser for a specific event name
pub struct RegisteredParser {
    pub event_name: &'static str,
    pub parser: fn(Python<'_>, &str) -> Option<Py<PyAny>>,
}

// Collect all registered parsers at compile time
inventory::collect!(RegisteredParser);

/// Dispatch parsing to the appropriate parser based on event name
///
/// Iterates through all registered parsers and uses the first one that matches.
/// For tracing_mark_write, delegates to tracing_mark_registry.
/// Falls back to the base Trace parser if no match is found.
pub fn dispatch_parse(py: Python<'_>, line: &str) -> PyResult<Option<Py<PyAny>>> {
    let Some(event_name) = crate::common::extract_event_name(line) else {
        return Ok(None);
    };

    // Special handling for tracing_mark_write (multiple subtypes)
    if event_name == "tracing_mark_write" {
        return Ok(crate::tracing_mark_registry::parse_tracing_mark(py, line));
    }

    // Iterate through all registered parsers
    for registered in inventory::iter::<RegisteredParser> {
        if registered.event_name == event_name {
            return Ok((registered.parser)(py, line));
        }
    }

    // Fallback to base Trace parser for unknown events
    Ok(crate::parse_and_wrap(py, line, crate::Trace::parse))
}

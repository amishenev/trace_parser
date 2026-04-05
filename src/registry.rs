//! Registry for auto-registering trace event parsers.
//!
//! Uses the `inventory` crate for compile-time registration without runtime overhead.

use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;

/// A registered parser for a specific event name
pub struct RegisteredParser {
    pub event_name: &'static str,
    pub parser: fn(Python<'_>, &str) -> Option<Py<PyAny>>,
    pub quick_check: fn(&str) -> bool,
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
        if let Some(event) = crate::tracing_mark_registry::parse_tracing_mark(py, line) {
            return Ok(Some(event));
        }
        return Err(PyValueError::new_err(format!(
            "Unsupported tracing_mark_write format: {}",
            line
        )));
    }

    // Iterate through all registered parsers.
    // If quick_check fails, try the next registered parser for the same event name.
    // If quick_check passes but the parser returns None, return an error (broken format).
    for registered in inventory::iter::<RegisteredParser> {
        if registered.event_name == event_name {
            if !(registered.quick_check)(line) {
                continue;
            }
            if let Some(event) = (registered.parser)(py, line) {
                return Ok(Some(event));
            }
            return Err(PyValueError::new_err(format!(
                "Unsupported format for event '{}': {}",
                event_name, line
            )));
        }
    }

    // Fallback to base Trace parser for unknown events
    Ok(crate::parse_and_wrap(py, line, crate::Trace::parse))
}

mod common;
mod format_registry;
mod frequency;
mod payload_template;
mod registry;
mod sched_process_exit;
mod sched_switch;
mod sched_wakeup;
mod trace;
mod trace_exit;
mod tracing_mark;
mod tracing_mark_registry;

// Make the crate visible under its own name for proc-macros
extern crate self as trace_parser;

pub use format_registry::{FormatRegistry, FormatSpec};
pub use frequency::{TraceCpuFrequency, TraceDevFrequency};
pub use sched_process_exit::TraceSchedProcessExit;
pub use sched_switch::TraceSchedSwitch;
pub use sched_wakeup::{TraceSchedWakeup, TraceSchedWakeupNew};
pub use trace::Trace;
pub use trace_exit::TraceExit;
pub use tracing_mark::{TraceMarkBegin, TraceMarkEnd, TraceReceiveVsync, TracingMark};

use pyo3::BoundObject;
use pyo3::prelude::*;
use std::fs::File;
use std::io::{BufRead, BufReader};

/// Хелпер для парсинга и создания Python объекта из Rust события
pub fn parse_and_wrap<'py, T>(
    py: Python<'py>,
    line: &str,
    parser: fn(&str) -> Option<T>,
) -> Option<Py<PyAny>>
where
    T: IntoPyObject<'py>,
{
    parser(line)
        .and_then(|e| e.into_pyobject(py).ok())
        .map(|bound| bound.into_any().unbind())
}

#[pyfunction]
fn parse_trace(py: Python<'_>, line: &str) -> PyResult<Option<Py<PyAny>>> {
    registry::dispatch_parse(py, line)
}

#[pyfunction]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Parse an entire trace file efficiently.
///
/// This function reads and parses a trace file line-by-line in Rust,
/// which is much faster than calling parse_trace() for each line in Python.
///
/// Args:
///     path: Path to the trace file
///     filter_event: Optional event name to filter by (e.g., "sched_switch")
///
/// Returns:
///     List of parsed trace events
#[pyfunction(signature = (path, filter_event=None))]
fn parse_trace_file(
    py: Python<'_>,
    path: &str,
    filter_event: Option<&str>,
) -> PyResult<Vec<Py<PyAny>>> {
    let file = File::open(path).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Failed to open file: {}", e))
    })?;
    let reader = BufReader::new(file);
    let mut results = Vec::new();

    for line_result in reader.lines() {
        let line = line_result.map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Failed to read line: {}", e))
        })?;

        // Quick filter by event_name
        if let Some(event_name) = filter_event
            && !line.contains(event_name)
        {
            continue;
        }

        if let Some(event) = parse_trace(py, &line)? {
            results.push(event);
        }
    }

    Ok(results)
}

#[pymodule]
fn _native(_py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Trace>()?;
    module.add_class::<TraceSchedSwitch>()?;
    module.add_class::<TraceCpuFrequency>()?;
    module.add_class::<TraceDevFrequency>()?;
    module.add_class::<TraceSchedWakeup>()?;
    module.add_class::<TraceSchedWakeupNew>()?;
    module.add_class::<TraceSchedProcessExit>()?;
    module.add_class::<TraceExit>()?;
    module.add_class::<TracingMark>()?;
    module.add_class::<TraceMarkBegin>()?;
    module.add_class::<TraceMarkEnd>()?;
    module.add_class::<TraceReceiveVsync>()?;
    module.add_function(wrap_pyfunction!(parse_trace, module)?)?;
    module.add_function(wrap_pyfunction!(parse_trace_file, module)?)?;
    module.add_function(wrap_pyfunction!(version, module)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!version().is_empty());
    }
}

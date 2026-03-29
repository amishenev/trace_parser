mod common;
mod format_registry;
mod frequency;
mod payload_template;
mod sched_process_exit;
mod sched_switch;
mod sched_wakeup;
mod trace;
mod tracing_mark;

pub use format_registry::{FormatRegistry, FormatSpec};
pub use frequency::{TraceCpuFrequency, TraceDevFrequency};
pub use sched_process_exit::TraceSchedProcessExit;
pub use sched_switch::TraceSchedSwitch;
pub use sched_wakeup::{TraceSchedWakeup, TraceSchedWakeupNew};
pub use trace::Trace;
pub use tracing_mark::{TraceMarkBegin, TraceMarkEnd, TraceReceiveVsync, TracingMark};

use pyo3::prelude::*;
use pyo3::BoundObject;
use memchr::memmem;
use std::fs::File;
use std::io::{BufRead, BufReader};

/// Хелпер для парсинга и создания Python объекта из Rust события
fn parse_and_wrap<'py, T>(
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

/// Парсинг tracing_mark_write с приоритетом событий
fn parse_tracing_mark(py: Python<'_>, line: &str) -> Option<Py<PyAny>> {
    parse_and_wrap(py, line, TraceReceiveVsync::parse)
        .or_else(|| parse_and_wrap(py, line, TraceMarkBegin::parse))
        .or_else(|| parse_and_wrap(py, line, TraceMarkEnd::parse))
        .or_else(|| parse_and_wrap(py, line, TracingMark::parse))
}

/// Извлечь event_name из строки трассировки
fn extract_event_name(line: &str) -> Option<&str> {
    // Используем SIMD поиск через memchr
    let colon_pos = memmem::find(line.as_bytes(), b": ")? + 2;
    let rest = &line[colon_pos..];
    let end_pos = memmem::find(rest.as_bytes(), b": ")?;
    Some(rest[..end_pos].trim())
}

#[pyfunction]
fn parse_trace(py: Python<'_>, line: &str) -> PyResult<Option<Py<PyAny>>> {
    let Some(event_name) = extract_event_name(line) else {
        return Ok(None);
    };

    let result = match event_name {
        // Tracing mark события
        "tracing_mark_write" => parse_tracing_mark(py, line),

        // Частотные события
        "clock_set_rate" => parse_and_wrap(py, line, TraceDevFrequency::parse),
        "cpu_frequency" => parse_and_wrap(py, line, TraceCpuFrequency::parse),

        // Sched события
        "sched_switch" => parse_and_wrap(py, line, TraceSchedSwitch::parse),
        "sched_wakeup" => parse_and_wrap(py, line, TraceSchedWakeup::parse),
        "sched_wakeup_new" => parse_and_wrap(py, line, TraceSchedWakeupNew::parse),
        "sched_process_exit" => parse_and_wrap(py, line, TraceSchedProcessExit::parse),

        // Неизвестное событие — fallback на базовый Trace
        _ => parse_and_wrap(py, line, Trace::parse),
    };

    Ok(result)
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
        if let Some(event_name) = filter_event {
            if !line.contains(event_name) {
                continue;
            }
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
    module.add_class::<TracingMark>()?;
    module.add_class::<TraceMarkBegin>()?;
    module.add_class::<TraceMarkEnd>()?;
    module.add_class::<TraceReceiveVsync>()?;
    module.add_function(wrap_pyfunction!(parse_trace, module)?)?;
    module.add_function(wrap_pyfunction!(parse_trace_file, module)?)?;
    module.add_function(wrap_pyfunction!(version, module)?)?;
    Ok(())
}

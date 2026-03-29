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

/// Хелпер для парсинга и создания Python объекта из Rust события
fn parse_and_wrap<T: IntoPy<PyObject>>(py: Python<'_>, line: &str, parser: fn(&str) -> Option<T>) -> Option<PyObject> {
    parser(line).map(|e| e.into_py(py))
}

/// Парсинг tracing_mark_write с приоритетом событий
fn parse_tracing_mark(py: Python<'_>, line: &str) -> Option<PyObject> {
    parse_and_wrap(py, line, TraceReceiveVsync::parse)
        .or_else(|| parse_and_wrap(py, line, TraceMarkBegin::parse))
        .or_else(|| parse_and_wrap(py, line, TraceMarkEnd::parse))
        .or_else(|| parse_and_wrap(py, line, TracingMark::parse))
}

/// Извлечь event_name из строки трассировки
fn extract_event_name(line: &str) -> Option<&str> {
    let colon_pos = line.find(": ")? + 2;
    let rest = &line[colon_pos..];
    let end_pos = rest.find(": ")?;
    Some(rest[..end_pos].trim())
}

#[pyfunction]
fn parse_trace(py: Python<'_>, line: &str) -> PyResult<Option<PyObject>> {
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
    module.add_function(wrap_pyfunction!(version, module)?)?;
    Ok(())
}

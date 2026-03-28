mod common;
mod frequency;
mod payload_template;
mod sched_process_exit;
mod sched_switch;
mod sched_wakeup;
mod trace;
mod tracing_mark;

use pyo3::prelude::*;

pub use frequency::{TraceCpuFrequency, TraceDevFrequency};
pub use sched_process_exit::TraceSchedProcessExit;
pub use sched_switch::TraceSchedSwitch;
pub use sched_wakeup::{TraceSchedWakeup, TraceSchedWakeupNew};
pub use trace::Trace;
pub use tracing_mark::{TraceMarkBegin, TraceMarkEnd, TraceReceiveVsync, TracingMark};

#[pyfunction]
fn parse_trace(py: Python<'_>, line: &str) -> PyResult<Option<PyObject>> {
    if TraceReceiveVsync::can_be_parsed(line) {
        if let Some(event) = TraceReceiveVsync::parse(line) {
            return Ok(Some(Py::new(py, event)?.to_object(py)));
        }
    }
    if TraceMarkBegin::can_be_parsed(line) {
        if let Some(event) = TraceMarkBegin::parse(line) {
            return Ok(Some(Py::new(py, event)?.to_object(py)));
        }
    }
    if TraceMarkEnd::can_be_parsed(line) {
        if let Some(event) = TraceMarkEnd::parse(line) {
            return Ok(Some(Py::new(py, event)?.to_object(py)));
        }
    }
    if TracingMark::can_be_parsed(line) {
        if let Some(event) = TracingMark::parse(line) {
            return Ok(Some(Py::new(py, event)?.to_object(py)));
        }
    }
    if TraceDevFrequency::can_be_parsed(line) {
        if let Some(event) = TraceDevFrequency::parse(line) {
            return Ok(Some(Py::new(py, event)?.to_object(py)));
        }
    }
    if TraceCpuFrequency::can_be_parsed(line) {
        if let Some(event) = TraceCpuFrequency::parse(line) {
            return Ok(Some(Py::new(py, event)?.to_object(py)));
        }
    }
    if TraceSchedWakeupNew::can_be_parsed(line) {
        if let Some(event) = TraceSchedWakeupNew::parse(line) {
            return Ok(Some(Py::new(py, event)?.to_object(py)));
        }
    }
    if TraceSchedWakeup::can_be_parsed(line) {
        if let Some(event) = TraceSchedWakeup::parse(line) {
            return Ok(Some(Py::new(py, event)?.to_object(py)));
        }
    }
    if TraceSchedProcessExit::can_be_parsed(line) {
        if let Some(event) = TraceSchedProcessExit::parse(line) {
            return Ok(Some(Py::new(py, event)?.to_object(py)));
        }
    }
    if TraceSchedSwitch::can_be_parsed(line) {
        if let Some(event) = TraceSchedSwitch::parse(line) {
            return Ok(Some(Py::new(py, event)?.to_object(py)));
        }
    }
    if let Some(trace) = Trace::parse(line) {
        return Ok(Some(Py::new(py, trace)?.to_object(py)));
    }
    Ok(None)
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

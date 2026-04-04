use pyo3::prelude::*;

use crate::common::{validate_timestamp, parse_event, EventType, FastMatch};
use crate::trace::{extract_base_fields, format_trace_header};
use std::sync::LazyLock;
use crate::payload_template::{FieldSpec, PayloadTemplate};

/// Shared template for Begin markers (used by TraceMarkBegin and TraceReceiveVsync)
pub(crate) static BEGIN_TEMPLATE: LazyLock<PayloadTemplate> = LazyLock::new(|| {
    PayloadTemplate::new(
        "B|{trace_mark_tgid}|{message}",
        &[
            ("trace_mark_tgid", FieldSpec::u32()),
            ("message", FieldSpec::custom(r".*")),
        ],
    )
});

#[pyclass(skip_from_py_object)]
#[derive(Clone, Debug, PartialEq)]
pub struct TracingMark {
    #[pyo3(get, set)]
    pub thread_name: String,
    #[pyo3(get, set)]
    pub thread_tid: u32,
    #[pyo3(get, set)]
    pub thread_tgid: u32,
    #[pyo3(get, set)]
    pub cpu: u32,
    #[pyo3(get, set)]
    pub flags: String,
    #[pyo3(get, set)]
    pub timestamp: f64,
    #[pyo3(get)]
    pub event_name: String,
    payload_raw: String,
}

impl EventType for TracingMark {
    const EVENT_NAME: &'static str = "tracing_mark_write";
}

impl FastMatch for TracingMark {}

#[pymethods]
impl TracingMark {
    #[new]
    #[pyo3(signature = (thread_name, thread_tid, thread_tgid, cpu, flags, timestamp, event_name, payload_raw))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        thread_name: String,
        thread_tid: u32,
        thread_tgid: u32,
        cpu: u32,
        flags: String,
        timestamp: f64,
        event_name: String,
        payload_raw: String,
    ) -> PyResult<Self> {
        validate_timestamp(timestamp)?;
        Ok(Self {
            thread_name,
            thread_tid,
            thread_tgid,
            cpu,
            flags,
            timestamp,
            event_name,
            payload_raw,
        })
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "TracingMark(thread_name='{}', thread_tid={}, timestamp={:.6}, event_name='{}')",
            self.thread_name, self.thread_tid, self.timestamp, self.event_name
        ))
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.thread_name == other.thread_name
            && self.thread_tid == other.thread_tid
            && self.thread_tgid == other.thread_tgid
            && self.cpu == other.cpu
            && self.flags == other.flags
            && self.timestamp == other.timestamp
            && self.event_name == other.event_name
            && self.payload_raw == other.payload_raw
    }

    fn __str__(&self) -> PyResult<String> {
        self.to_string()
    }

    fn __copy__(slf: PyRef<'_, Self>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        Ok(slf.into_pyobject(py).map(|o| o.into_any().unbind())?)
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(self.clone())
    }

    #[staticmethod]
    pub fn can_be_parsed(line: &str) -> bool {
        Self::quick_check(line)
    }

    #[staticmethod]
    pub fn parse(line: &str) -> Option<Self> {
        if !Self::quick_check(line) {
            return None;
        }
        let parts = parse_event::<Self>(line)?;
        let (thread_name, thread_tid, thread_tgid, cpu, flags, timestamp, event_name, payload_raw) = extract_base_fields(&parts);
        Some(Self {
            thread_name,
            thread_tid,
            thread_tgid,
            cpu,
            flags,
            timestamp,
            event_name,
            payload_raw,
        })
    }

    #[getter]
    pub fn payload(&self) -> &str {
        &self.payload_raw
    }

    #[getter]
    pub fn template(&self) -> &'static str {
        "{payload}"
    }

    pub fn to_string(&self) -> PyResult<String> {
        validate_timestamp(self.timestamp)?;
        Ok(format_trace_header(
            &self.thread_name, self.thread_tid, self.thread_tgid, self.cpu,
            &self.flags, self.timestamp, &self.event_name,
            self.payload()
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::TracingMark;

    #[test]
    fn tracing_mark_accepts_any_payload() {
        let line = "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: anything at all";
        let mark = TracingMark::parse(line).expect("tracing mark must parse");
        assert_eq!(mark.event_name, "tracing_mark_write");
        assert_eq!(mark.payload_raw, "anything at all");
        assert_eq!(mark.payload(), "anything at all");
    }

    #[test]
    fn tracing_mark_custom_payload() {
        let line = "task-100 (100) [000] .... 1.0: tracing_mark_write: custom_payload_here";
        let mark = TracingMark::parse(line).expect("must parse");
        assert_eq!(mark.payload_raw, "custom_payload_here");
        assert_eq!(mark.payload(), "custom_payload_here");
        assert_eq!(mark.template(), "{payload}");
    }
}

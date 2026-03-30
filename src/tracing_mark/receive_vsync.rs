use pyo3::prelude::*;
use std::sync::LazyLock;
use lexical_core::parse;

use crate::common::{validate_timestamp, BaseTraceParts};
use crate::payload_template::{FieldSpec, PayloadTemplate, TemplateValue};
use crate::trace::extract_base_fields;
use super::base::{contains_begin_marker, BEGIN_TEMPLATE};

static TEMPLATE: LazyLock<PayloadTemplate> = LazyLock::new(|| {
    PayloadTemplate::new(
        "{?ignore:extra_info}ReceiveVsync {frame_number}",
        &[
            ("extra_info", FieldSpec::custom(r"\[[^]]+\]")),
            ("frame_number", FieldSpec::u32()),
        ],
    )
});

#[pyclass(skip_from_py_object)]
#[derive(Clone, Debug, PartialEq)]
pub struct TraceReceiveVsync {
    #[pyo3(get, set)]
    pub thread_name: String,
    #[pyo3(get, set)]
    pub tid: u32,
    #[pyo3(get, set)]
    pub tgid: u32,
    #[pyo3(get, set)]
    pub cpu: u32,
    #[pyo3(get, set)]
    pub flags: String,
    #[pyo3(get, set)]
    pub timestamp: f64,
    #[pyo3(get)]
    pub event_name: String,
    #[pyo3(get, set)]
    pub payload_raw: String,
    #[pyo3(get, set)]
    pub trace_mark_tgid: u32,
    #[pyo3(get, set)]
    pub payload: String,
    #[pyo3(get, set)]
    pub frame_number: u32,
}

#[pymethods]
impl TraceReceiveVsync {
    #[new]
    #[pyo3(signature = (thread_name, tid, tgid, cpu, flags, timestamp, event_name, payload_raw, trace_mark_tgid, payload, frame_number))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        thread_name: String,
        tid: u32,
        tgid: u32,
        cpu: u32,
        flags: String,
        timestamp: f64,
        event_name: String,
        payload_raw: String,
        trace_mark_tgid: u32,
        payload: String,
        frame_number: u32,
    ) -> PyResult<Self> {
        validate_timestamp(timestamp)?;
        Ok(Self {
            thread_name,
            tid,
            tgid,
            cpu,
            flags,
            timestamp,
            event_name,
            payload_raw,
            trace_mark_tgid,
            payload,
            frame_number,
        })
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "TraceReceiveVsync(thread_name='{}', tid={}, timestamp={:.6}, event_name='{}', frame_number={})",
            self.thread_name, self.tid, self.timestamp, self.event_name, self.frame_number
        ))
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.thread_name == other.thread_name
            && self.tid == other.tid
            && self.tgid == other.tgid
            && self.cpu == other.cpu
            && self.flags == other.flags
            && self.timestamp == other.timestamp
            && self.event_name == other.event_name
            && self.payload_raw == other.payload_raw
            && self.trace_mark_tgid == other.trace_mark_tgid
            && self.payload == other.payload
            && self.frame_number == other.frame_number
    }

    fn __str__(&self) -> PyResult<String> {
        self.to_string()
    }

    fn __copy__(slf: PyRef<'_, Self>, py: Python<'_>) -> PyResult<Py<Self>> {
        Ok(slf.clone().into_pyobject(py)?.unbind())
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> PyResult<Py<Self>> {
        unsafe {
            Ok(self.clone().into_pyobject(Python::assume_attached())?.unbind())
        }
    }

    #[getter]
    pub fn timestamp_ms(&self) -> f64 {
        self.timestamp * 1_000.0
    }

    #[setter]
    pub fn set_timestamp_ms(&mut self, value: f64) -> PyResult<()> {
        self.timestamp = validate_timestamp(value / 1_000.0)?;
        Ok(())
    }

    #[getter]
    pub fn timestamp_ns(&self) -> u64 {
        (self.timestamp * 1_000_000_000.0).round() as u64
    }

    #[setter]
    pub fn set_timestamp_ns(&mut self, value: u64) -> PyResult<()> {
        self.timestamp = (value as f64) / 1_000_000_000.0;
        Ok(())
    }

    #[staticmethod]
    pub fn can_be_parsed(line: &str) -> bool {
        contains_begin_marker(line) && line.contains("ReceiveVsync ")
    }

    #[staticmethod]
    pub fn parse(line: &str) -> Option<Self> {
        if !Self::can_be_parsed(line) {
            return None;
        }
        let parts = BaseTraceParts::parse(line)?;
        let (thread_name, tid, tgid, cpu, flags, timestamp, event_name, payload_raw) = extract_base_fields(&parts);
        
        let begin_captures = BEGIN_TEMPLATE.captures(&parts.payload_raw)?;
        let trace_mark_tgid: u32 = parse(begin_captures.name("trace_mark_tgid")?.as_str().as_bytes()).ok()?;
        let payload = begin_captures.name("payload")?.as_str().to_string();
        
        let captures = TEMPLATE.captures(&payload)?;
        let frame_number = parse(captures.name("frame_number")?.as_str().as_bytes()).ok()?;
        
        Some(Self {
            thread_name,
            tid,
            tgid,
            cpu,
            flags,
            timestamp,
            event_name,
            payload_raw,
            trace_mark_tgid,
            payload,
            frame_number,
        })
    }

    pub fn payload_to_string(&self) -> PyResult<String> {
        let payload_values = [("frame_number", Some(TemplateValue::U32(self.frame_number)))];
        Ok(TEMPLATE
            .format(&payload_values)
            .expect("receive vsync template must render"))
    }

    pub fn to_string(&self) -> PyResult<String> {
        validate_timestamp(self.timestamp)?;
        let inner_payload = self.payload_to_string()?;
        let begin_values = [
            ("trace_mark_tgid", Some(TemplateValue::U32(self.trace_mark_tgid))),
            ("payload", Some(TemplateValue::Str(&inner_payload))),
        ];
        let full_payload = BEGIN_TEMPLATE
            .format(&begin_values)
            .expect("trace mark begin template must render");

        Ok(format!(
            "{}-{} ({}) [{:03}] {} {:.6}: {}: {}",
            self.thread_name,
            self.tid,
            self.tgid,
            self.cpu,
            self.flags,
            self.timestamp,
            self.event_name,
            full_payload,
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::TraceReceiveVsync;

    #[test]
    fn receive_vsync_parses_specific_begin_payload() {
        let line =
            "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: B|10|ReceiveVsync 42";
        let mark = TraceReceiveVsync::parse(line)
            .expect("receive vsync begin mark must parse");
        assert_eq!(mark.trace_mark_tgid, 10);
        assert_eq!(mark.frame_number, 42);
        assert_eq!(
            mark.payload_to_string().expect("payload_to_string must work"),
            "ReceiveVsync 42"
        );
        assert_eq!(
            mark.to_string().expect("to_string must work"),
            "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: B|10|ReceiveVsync 42"
        );
    }

    #[test]
    fn receive_vsync_ignores_service_prefix() {
        let line =
            "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: B|10|[ExtraInfo]ReceiveVsync 42";
        let mark = TraceReceiveVsync::parse(line)
            .expect("receive vsync begin mark must parse");
        assert_eq!(mark.frame_number, 42);
    }
}

use pyo3::prelude::*;
use regex::Captures;
use std::sync::LazyLock;

use crate::common::{
    cap_parse, cap_str, parse_template_event, validate_timestamp, BaseTraceParts, EventType,
    FastMatch, TemplateEvent,
};
use crate::format_registry::{FormatRegistry, FormatSpec};
use crate::payload_template::{FieldSpec, PayloadTemplate, TemplateValue};
use crate::trace::{extract_base_fields, format_trace_header};

pub(crate) static BEGIN_TEMPLATE: LazyLock<PayloadTemplate> = LazyLock::new(|| {
    PayloadTemplate::new(
        "B|{trace_mark_tgid}|{message}",
        &[
            ("trace_mark_tgid", FieldSpec::u32()),
            ("message", FieldSpec::custom(r".*")),
        ],
    )
});

pub(crate) static BEGIN_FORMATS: LazyLock<FormatRegistry> = LazyLock::new(|| {
    FormatRegistry::new(vec![
        FormatSpec {
            kind: 0,
            template: &BEGIN_TEMPLATE,
        },
    ])
});

pub(crate) static END_TEMPLATE: LazyLock<PayloadTemplate> = LazyLock::new(|| {
    PayloadTemplate::new(
        "E|{trace_mark_tgid}|{message}",
        &[
            ("trace_mark_tgid", FieldSpec::u32()),
            ("message", FieldSpec::custom(r".*")),
        ],
    )
});

pub(crate) static END_FORMATS: LazyLock<FormatRegistry> = LazyLock::new(|| {
    FormatRegistry::new(vec![
        FormatSpec {
            kind: 0,
            template: &END_TEMPLATE,
        },
    ])
});

use crate::common::parse_event;

pub(crate) fn contains_begin_marker(line: &str) -> bool {
    line.contains(" B|") || line.contains(": B|") || line.contains("tracing_mark_write: B|")
}

pub(crate) fn contains_end_marker(line: &str) -> bool {
    line.contains(" E|") || line.contains(": E|") || line.contains("tracing_mark_write: E|")
}

#[pyclass(skip_from_py_object)]
#[derive(Clone, Debug, PartialEq)]
pub struct TracingMark {
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
    payload_raw: String,
}

#[pyclass(from_py_object)]
#[derive(Clone, Debug, PartialEq)]
pub struct TraceMarkBegin {
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
    format_id: u8,
    #[pyo3(get, set)]
    pub trace_mark_tgid: u32,
    #[pyo3(get, set)]
    pub message: String,
}

#[pyclass(skip_from_py_object)]
#[derive(Clone, Debug, PartialEq)]
pub struct TraceMarkEnd {
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
    format_id: u8,
    #[pyo3(get, set)]
    pub trace_mark_tgid: u32,
    #[pyo3(get, set)]
    pub message: String,
}

impl EventType for TracingMark {
    const EVENT_NAME: &'static str = "tracing_mark_write";
}

impl FastMatch for TracingMark {}

impl EventType for TraceMarkBegin {
    const EVENT_NAME: &'static str = "tracing_mark_write";
}

impl FastMatch for TraceMarkBegin {
    fn payload_quick_check(line: &str) -> bool {
        contains_begin_marker(line)
    }
}

impl TemplateEvent for TraceMarkBegin {
    fn formats() -> &'static FormatRegistry {
        &BEGIN_FORMATS
    }

    fn parse_payload(
        parts: BaseTraceParts,
        captures: &Captures<'_>,
        _format_id: u8,
    ) -> Option<Self> {
        let (thread_name, tid, tgid, cpu, flags, timestamp, event_name, _payload_raw) = extract_base_fields(&parts);
        Some(Self {
            thread_name,
            tid,
            tgid,
            cpu,
            flags,
            timestamp,
            event_name,
            format_id: 0,
            trace_mark_tgid: cap_parse(captures, "trace_mark_tgid")?,
            message: cap_str(captures, "message")?,
        })
    }

    fn render_payload(&self) -> PyResult<String> {
        let template = Self::formats().template(0).unwrap();
        let values: [(&str, Option<TemplateValue>); 2] = [
            ("trace_mark_tgid", Some(TemplateValue::U32(self.trace_mark_tgid))),
            ("message", Some(TemplateValue::Str(&self.message))),
        ];
        Ok(template
            .format(&values)
            .expect("trace mark begin template must render"))
    }
}

impl EventType for TraceMarkEnd {
    const EVENT_NAME: &'static str = "tracing_mark_write";
}

impl FastMatch for TraceMarkEnd {
    fn payload_quick_check(line: &str) -> bool {
        contains_end_marker(line)
    }
}

impl TemplateEvent for TraceMarkEnd {
    fn formats() -> &'static FormatRegistry {
        &END_FORMATS
    }

    fn parse_payload(
        parts: BaseTraceParts,
        captures: &Captures<'_>,
        _format_id: u8,
    ) -> Option<Self> {
        let (thread_name, tid, tgid, cpu, flags, timestamp, event_name, _payload_raw) = extract_base_fields(&parts);
        Some(Self {
            thread_name,
            tid,
            tgid,
            cpu,
            flags,
            timestamp,
            event_name,
            format_id: 0,
            trace_mark_tgid: cap_parse(captures, "trace_mark_tgid")?,
            message: cap_str(captures, "message")?,
        })
    }

    fn render_payload(&self) -> PyResult<String> {
        let template = Self::formats().template(0).unwrap();
        let values: [(&str, Option<TemplateValue>); 2] = [
            ("trace_mark_tgid", Some(TemplateValue::U32(self.trace_mark_tgid))),
            ("message", Some(TemplateValue::Str(&self.message))),
        ];
        Ok(template
            .format(&values)
            .expect("trace mark end template must render"))
    }
}

#[pymethods]
impl TracingMark {
    #[new]
    #[pyo3(signature = (thread_name, tid, tgid, cpu, flags, timestamp, event_name, payload_raw))]
    fn new(
        thread_name: String,
        tid: u32,
        tgid: u32,
        cpu: u32,
        flags: String,
        timestamp: f64,
        event_name: String,
        payload_raw: String,
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
        })
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "TracingMark(thread_name='{}', tid={}, timestamp={:.6}, event_name='{}')",
            self.thread_name, self.tid, self.timestamp, self.event_name
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
        let (thread_name, tid, tgid, cpu, flags, timestamp, event_name, payload_raw) = extract_base_fields(&parts);
        Some(Self {
            thread_name,
            tid,
            tgid,
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
            &self.thread_name, self.tid, self.tgid, self.cpu,
            &self.flags, self.timestamp, &self.event_name,
            self.payload()
        ))
    }
}

#[pymethods]
impl TraceMarkBegin {
    #[new]
    #[pyo3(signature = (thread_name, tid, tgid, cpu, flags, timestamp, event_name, trace_mark_tgid, message))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        thread_name: String,
        tid: u32,
        tgid: u32,
        cpu: u32,
        flags: String,
        timestamp: f64,
        event_name: String,
        trace_mark_tgid: u32,
        message: String,
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
            format_id: 0,
            trace_mark_tgid,
            message,
        })
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "TraceMarkBegin(thread_name='{}', tid={}, timestamp={:.6}, event_name='{}', trace_mark_tgid={}, message='{}')",
            self.thread_name, self.tid, self.timestamp, self.event_name, self.trace_mark_tgid, self.message
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
            && self.trace_mark_tgid == other.trace_mark_tgid
            && self.message == other.message
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
        if !Self::can_be_parsed(line) {
            return None;
        }
        parse_template_event::<Self>(line)
    }

    #[getter]
    pub fn payload(&self) -> String {
        format!("B|{}|{}", self.trace_mark_tgid, self.message)
    }

    #[getter]
    pub fn message(&self) -> &str {
        &self.message
    }

    #[getter]
    pub fn template(&self) -> &'static str {
        Self::formats().template(0).unwrap().template_str()
    }

    pub fn payload_to_string(&self) -> PyResult<String> {
        self.render_payload()
    }

    pub fn to_string(&self) -> PyResult<String> {
        validate_timestamp(self.timestamp)?;
        let payload = self.payload();
        Ok(format_trace_header(
            &self.thread_name, self.tid, self.tgid, self.cpu,
            &self.flags, self.timestamp, &self.event_name,
            &payload
        ))
    }
}

#[pymethods]
impl TraceMarkEnd {
    #[new]
    #[pyo3(signature = (thread_name, tid, tgid, cpu, flags, timestamp, event_name, trace_mark_tgid, message))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        thread_name: String,
        tid: u32,
        tgid: u32,
        cpu: u32,
        flags: String,
        timestamp: f64,
        event_name: String,
        trace_mark_tgid: u32,
        message: String,
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
            format_id: 0,
            trace_mark_tgid,
            message,
        })
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "TraceMarkEnd(thread_name='{}', tid={}, timestamp={:.6}, event_name='{}', trace_mark_tgid={}, message='{}')",
            self.thread_name, self.tid, self.timestamp, self.event_name, self.trace_mark_tgid, self.message
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
            && self.trace_mark_tgid == other.trace_mark_tgid
            && self.message == other.message
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
        if !Self::can_be_parsed(line) {
            return None;
        }
        parse_template_event::<Self>(line)
    }

    #[getter]
    pub fn payload(&self) -> String {
        format!("E|{}|{}", self.trace_mark_tgid, self.message)
    }

    #[getter]
    pub fn message(&self) -> &str {
        &self.message
    }

    #[getter]
    pub fn template(&self) -> &'static str {
        Self::formats().template(0).unwrap().template_str()
    }

    pub fn payload_to_string(&self) -> PyResult<String> {
        self.render_payload()
    }

    pub fn to_string(&self) -> PyResult<String> {
        validate_timestamp(self.timestamp)?;
        let payload = self.payload();
        Ok(format_trace_header(
            &self.thread_name, self.tid, self.tgid, self.cpu,
            &self.flags, self.timestamp, &self.event_name,
            &payload
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::{TraceMarkBegin, TraceMarkEnd, TracingMark};

    #[test]
    fn tracing_mark_accepts_any_payload() {
        let line = "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: anything at all";
        let mark = TracingMark::parse(line).expect("tracing mark must parse");
        assert_eq!(mark.event_name, "tracing_mark_write");
        assert_eq!(mark.payload_raw, "anything at all");
        assert_eq!(
            mark.payload(),
            "anything at all"
        );
    }

    #[test]
    fn trace_mark_begin_parses_generic_begin_payload() {
        let line =
            "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: B|10|some_custom_message";
        let mark = TraceMarkBegin::parse(line).expect("begin mark must parse");
        assert_eq!(mark.trace_mark_tgid, 10);
        assert_eq!(mark.message, "some_custom_message");
        assert_eq!(
            mark.payload_to_string().expect("payload_to_string must work"),
            "B|10|some_custom_message"
        );
        assert_eq!(
            mark.to_string().expect("to_string must work"),
            "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: B|10|some_custom_message"
        );
    }

    #[test]
    fn trace_mark_end_parses_generic_end_payload() {
        let line = "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: E|10|done";
        let mark = TraceMarkEnd::parse(line).expect("end mark must parse");
        assert_eq!(mark.trace_mark_tgid, 10);
        assert_eq!(mark.message, "done");
    }
}

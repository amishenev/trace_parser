use pyo3::prelude::*;

use crate::common::{BASE_TRACE_RE, BaseTraceParts, parse_base_parts, validate_timestamp};

#[pyclass(skip_from_py_object)]
#[derive(Clone, Debug, PartialEq)]
pub struct Trace {
    #[pyo3(get, set)]
    pub(crate) thread_name: String,
    #[pyo3(get, set)]
    pub(crate) thread_tid: u32,
    #[pyo3(get, set)]
    pub(crate) thread_tgid: Option<u32>,
    #[pyo3(get, set)]
    pub(crate) cpu: u32,
    #[pyo3(get, set)]
    pub(crate) flags: String,
    #[pyo3(get, set)]
    pub(crate) timestamp: f64,
    #[pyo3(get)]
    pub(crate) event_name: String,
    #[pyo3(get, set)]
    pub(crate) payload_raw: String,
}

impl Trace {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        thread_name: String,
        thread_tid: u32,
        thread_tgid: Option<u32>,
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

    pub fn from_parts(parts: BaseTraceParts) -> Self {
        Self {
            thread_name: parts.thread_name,
            thread_tid: parts.thread_tid,
            thread_tgid: parts.thread_tgid,
            cpu: parts.cpu,
            flags: parts.flags,
            timestamp: parts.timestamp,
            event_name: parts.event_name,
            payload_raw: parts.payload_raw,
        }
    }

    pub fn to_string_with_payload(&self, payload: &str) -> String {
        let tgid = self
            .thread_tgid
            .map_or_else(|| "-".to_string(), |v| v.to_string());
        format!(
            "{}-{} ({}) [{:03}] {} {:.6}: {}: {}",
            self.thread_name,
            self.thread_tid,
            tgid,
            self.cpu,
            self.flags,
            self.timestamp,
            self.event_name,
            payload
        )
    }
}

#[inline]
#[allow(clippy::too_many_arguments)]
pub fn format_trace_header(
    thread_name: &str,
    thread_tid: u32,
    thread_tgid: Option<u32>,
    cpu: u32,
    flags: &str,
    timestamp: f64,
    event_name: &str,
    payload: &str,
) -> String {
    let tgid = thread_tgid.map_or_else(|| "-".to_string(), |v| v.to_string());
    format!(
        "{}-{} ({}) [{:03}] {} {:.6}: {}: {}",
        thread_name, thread_tid, tgid, cpu, flags, timestamp, event_name, payload
    )
}

#[pymethods]
impl Trace {
    #[new]
    #[pyo3(signature = (thread_name, thread_tid, thread_tgid, cpu, flags, timestamp, event_name, payload_raw))]
    #[allow(clippy::too_many_arguments)]
    fn py_new(
        thread_name: String,
        thread_tid: u32,
        thread_tgid: Option<u32>,
        cpu: u32,
        flags: String,
        timestamp: f64,
        event_name: String,
        payload_raw: String,
    ) -> PyResult<Self> {
        Self::new(
            thread_name,
            thread_tid,
            thread_tgid,
            cpu,
            flags,
            timestamp,
            event_name,
            payload_raw,
        )
    }

    #[staticmethod]
    pub fn can_be_parsed(line: &str) -> bool {
        BASE_TRACE_RE.is_match(line)
    }

    #[staticmethod]
    pub fn parse(line: &str) -> Option<Self> {
        Some(Self::from_parts(parse_base_parts(line)?))
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "Trace(thread_name='{}', thread_tid={}, timestamp={:.6}, event_name='{}')",
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

    fn __copy__(slf: PyRef<'_, Self>, py: Python<'_>) -> PyResult<Py<Self>> {
        slf.clone().into_pyobject(py).map(|o| o.unbind())
    }

    fn __deepcopy__(&self, py: Python<'_>, _memo: &Bound<'_, PyAny>) -> PyResult<Py<Self>> {
        self.clone().into_pyobject(py).map(|o| o.unbind())
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

    #[getter]
    pub fn payload(&self) -> &str {
        &self.payload_raw
    }

    #[getter]
    pub fn template(&self) -> &'static str {
        "{payload}"
    }

    pub fn has_unknown_thread(&self) -> bool {
        self.thread_name.trim() == "<...>"
    }

    pub fn to_string(&self) -> PyResult<String> {
        validate_timestamp(self.timestamp)?;
        Ok(self.to_string_with_payload(self.payload()))
    }
}

#[cfg(test)]
mod tests {
    use crate::Trace;

    #[test]
    fn base_trace_parses_sched_switch() {
        let line = "bash-1977   (  12) [000] .... 12345.678901: sched_switch: prev_comm=bash prev_pid=1977 ==> next_comm=worker next_pid=123";
        let trace = Trace::parse(line).expect("trace must parse");
        assert_eq!(trace.thread_name, "bash");
        assert_eq!(trace.thread_tid, 1977);
        assert_eq!(trace.thread_tgid, Some(12));
        assert_eq!(trace.cpu, 0);
        assert_eq!(trace.flags, "....");
        assert!((trace.timestamp - 12345.678901).abs() < 1e-9);
        assert!((trace.timestamp_ms() - 12_345_678.901).abs() < 1e-6);
        assert_eq!(trace.timestamp_ns(), 12_345_678_901_000);
        assert_eq!(trace.event_name, "sched_switch");
    }

    #[test]
    fn base_trace_allows_hyphenated_thread_name() {
        let line = "my-thread-name-42 (  12) [010] d..1 12345.678901: tracing_mark_write: B|10|some_custom_message";
        let trace = Trace::parse(line).expect("trace must parse");
        assert_eq!(trace.thread_name, "my-thread-name");
        assert_eq!(trace.thread_tid, 42);
        assert_eq!(trace.cpu, 10);
        assert_eq!(trace.event_name, "tracing_mark_write");
        assert_eq!(trace.payload_raw, "B|10|some_custom_message");
    }

    #[test]
    fn base_trace_parses_dashed_tgid_as_none() {
        let line = "<idle>-0 (-----) [001] d..2 2318.330977: softirq_raise: vec=9 [action=RCU]";
        let trace = Trace::parse(line).expect("trace must parse");
        assert_eq!(trace.thread_name, "<idle>");
        assert_eq!(trace.thread_tid, 0);
        assert_eq!(trace.thread_tgid, None);
        assert!(!trace.has_unknown_thread());
        assert_eq!(
            trace.to_string().expect("to_string must work"),
            "<idle>-0 (-) [001] d..2 2318.330977: softirq_raise: vec=9 [action=RCU]"
        );
    }

    #[test]
    fn base_trace_marks_unknown_thread_name() {
        let line = "<...>-0 (-----) [001] d..2 2318.330977: softirq_raise: vec=9 [action=RCU]";
        let trace = Trace::parse(line).expect("trace must parse");
        assert!(trace.has_unknown_thread());
    }

    #[test]
    fn base_can_be_parsed_rejects_invalid_lines() {
        assert!(!Trace::can_be_parsed("not a trace line"));
    }

    #[test]
    fn base_payload_getter_returns_raw_payload() {
        let line = "bash-1977   (  12) [000] .... 12345.678901: sched_switch: prev_comm=bash prev_pid=1977 ==> next_comm=worker next_pid=123";
        let trace = Trace::parse(line).expect("trace must parse");
        assert_eq!(
            trace.payload(),
            "prev_comm=bash prev_pid=1977 ==> next_comm=worker next_pid=123"
        );
    }

    #[test]
    fn trace_new_and_repr() {
        let trace = Trace::new(
            "bash".into(),
            1234,
            Some(1234),
            0,
            "....".into(),
            12345.678901,
            "test".into(),
            "payload".into(),
        )
        .unwrap();
        assert_eq!(trace.thread_name, "bash");
        assert_eq!(trace.thread_tid, 1234);
        assert_eq!(trace.thread_tgid, Some(1234));
        assert_eq!(trace.cpu, 0);
        assert_eq!(trace.flags, "....");
        assert_eq!(trace.timestamp, 12345.678901);
        assert_eq!(trace.event_name, "test");
        assert_eq!(trace.payload_raw, "payload");
    }

    #[test]
    fn trace_copy_and_clone() {
        let trace = Trace::new(
            "bash".into(),
            1234,
            Some(1234),
            0,
            "....".into(),
            1.0,
            "test".into(),
            "payload".into(),
        )
        .unwrap();
        let copy = trace.clone();
        assert_eq!(trace, copy);
    }

    #[test]
    fn trace_template() {
        let trace = Trace::new(
            "bash".into(),
            1234,
            Some(1234),
            0,
            "....".into(),
            1.0,
            "test".into(),
            "payload".into(),
        )
        .unwrap();
        assert_eq!(trace.template(), "{payload}");
    }
}

use pyo3::prelude::*;

use crate::common::{parse_base_parts, validate_timestamp, BaseTraceParts, BASE_TRACE_RE};

#[pyclass(skip_from_py_object)]
#[derive(Clone, Debug, PartialEq)]
pub struct Trace {
    #[pyo3(get, set)]
    pub(crate) thread_name: String,
    #[pyo3(get, set)]
    pub(crate) tid: u32,
    #[pyo3(get, set)]
    pub(crate) tgid: u32,
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
    pub fn new(
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

    pub fn from_parts(parts: BaseTraceParts) -> Self {
        Self {
            thread_name: parts.thread_name,
            tid: parts.tid,
            tgid: parts.tgid,
            cpu: parts.cpu,
            flags: parts.flags,
            timestamp: parts.timestamp,
            event_name: parts.event_name,
            payload_raw: parts.payload_raw,
        }
    }

    pub fn to_string_with_payload(&self, payload: &str) -> String {
        format!(
            "{}-{} ({}) [{:03}] {} {:.6}: {}: {}",
            self.thread_name,
            self.tid,
            self.tgid,
            self.cpu,
            self.flags,
            self.timestamp,
            self.event_name,
            payload
        )
    }
}

/// Helper для извлечения base полей из BaseTraceParts
/// Возвращает кортеж: (thread_name, tid, tgid, cpu, flags, timestamp, event_name, payload_raw)
#[inline]
pub fn extract_base_fields(parts: &BaseTraceParts) -> (
    String, u32, u32, u32, String, f64, String, String,
) {
    (
        parts.thread_name.clone(),
        parts.tid,
        parts.tgid,
        parts.cpu,
        parts.flags.clone(),
        parts.timestamp,
        parts.event_name.clone(),
        parts.payload_raw.clone(),
    )
}

/// Форматирует базовую часть trace строки
#[inline]
pub fn format_trace_header(
    thread_name: &str,
    tid: u32,
    tgid: u32,
    cpu: u32,
    flags: &str,
    timestamp: f64,
    event_name: &str,
    payload: &str,
) -> String {
    format!(
        "{}-{} ({}) [{:03}] {} {:.6}: {}: {}",
        thread_name, tid, tgid, cpu, flags, timestamp, event_name, payload
    )
}

#[pymethods]
impl Trace {
    #[new]
    #[pyo3(signature = (thread_name, tid, tgid, cpu, flags, timestamp, event_name, payload_raw))]
    fn py_new(
        thread_name: String,
        tid: u32,
        tgid: u32,
        cpu: u32,
        flags: String,
        timestamp: f64,
        event_name: String,
        payload_raw: String,
    ) -> PyResult<Self> {
        Self::new(thread_name, tid, tgid, cpu, flags, timestamp, event_name, payload_raw)
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
            "Trace(thread_name='{}', tid={}, timestamp={:.6}, event_name='{}')",
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

    pub fn payload_to_string(&self) -> PyResult<String> {
        Ok(self.payload_raw.clone())
    }

    pub fn to_string(&self) -> PyResult<String> {
        validate_timestamp(self.timestamp)?;
        Ok(self.to_string_with_payload(&self.payload_to_string()?))
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
        assert_eq!(trace.tid, 1977);
        assert_eq!(trace.tgid, 12);
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
        assert_eq!(trace.tid, 42);
        assert_eq!(trace.cpu, 10);
        assert_eq!(trace.event_name, "tracing_mark_write");
        assert_eq!(trace.payload_raw, "B|10|some_custom_message");
    }

    #[test]
    fn base_can_be_parsed_rejects_invalid_lines() {
        assert!(!Trace::can_be_parsed("not a trace line"));
    }

    #[test]
    fn base_payload_to_string_returns_raw_payload() {
        let line = "bash-1977   (  12) [000] .... 12345.678901: sched_switch: prev_comm=bash prev_pid=1977 ==> next_comm=worker next_pid=123";
        let trace = Trace::parse(line).expect("trace must parse");
        assert_eq!(
            trace.payload_to_string().expect("payload_to_string must work"),
            "prev_comm=bash prev_pid=1977 ==> next_comm=worker next_pid=123"
        );
    }
}

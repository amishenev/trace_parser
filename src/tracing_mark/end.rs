use pyo3::prelude::*;
use trace_parser_macros::TracingMarkEvent;

use crate::common::{validate_timestamp, FastMatch};
use crate::trace::format_trace_header;

#[pyclass(skip_from_py_object)]
#[derive(Clone, Debug, PartialEq)]
#[derive(TracingMarkEvent)]
#[trace_event(name = "tracing_mark_write", skip_registration, generate_pymethods = false)]
#[trace_markers("E|")]
#[define_template("E|{trace_mark_tgid}|{message}")]
pub struct TraceMarkEnd {
    #[field]
    format_id: u8,
    #[pyo3(get, set)]
    #[field]
    pub thread_name: String,
    #[pyo3(get, set)]
    #[field]
    pub thread_tid: u32,
    #[pyo3(get, set)]
    #[field]
    pub thread_tgid: u32,
    #[pyo3(get, set)]
    #[field]
    pub cpu: u32,
    #[pyo3(get, set)]
    #[field]
    pub flags: String,
    #[pyo3(get, set)]
    #[field]
    pub timestamp: f64,
    #[pyo3(get)]
    #[field]
    pub event_name: String,
    #[pyo3(get, set)]
    #[field]
    pub trace_mark_tgid: u32,
    #[pyo3(get, set)]
    #[field(regex = r".*")]
    pub message: String,
}

#[pymethods]
impl TraceMarkEnd {
    #[new]
    #[pyo3(signature = (thread_name, thread_tid, thread_tgid, cpu, flags, timestamp, event_name, trace_mark_tgid, message))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        thread_name: String,
        thread_tid: u32,
        thread_tgid: u32,
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
            thread_tid,
            thread_tgid,
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
            "TraceMarkEnd(thread_name='{}', thread_tid={}, timestamp={:.6}, event_name='{}', trace_mark_tgid={}, message='{}')",
            self.thread_name, self.thread_tid, self.timestamp, self.event_name, self.trace_mark_tgid, self.message
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
        ::trace_parser::common::parse_template_event::<Self>(line)
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
        <Self as ::trace_parser::common::TemplateEvent>::formats().template(self.format_id).unwrap().template_str()
    }

    pub fn payload_to_string(&self) -> PyResult<String> {
        <Self as ::trace_parser::common::TemplateEvent>::render_payload(self)
    }

    pub fn to_string(&self) -> PyResult<String> {
        validate_timestamp(self.timestamp)?;
        let payload = <Self as ::trace_parser::common::TemplateEvent>::render_payload(self)?;
        Ok(format_trace_header(
            &self.thread_name, self.thread_tid, self.thread_tgid, self.cpu,
            &self.flags, self.timestamp, &self.event_name,
            &payload
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::TraceMarkEnd;

    #[test]
    fn trace_mark_end_parses() {
        let line = "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: E|10|done";
        let mark = TraceMarkEnd::parse(line).expect("end mark must parse");
        assert_eq!(mark.trace_mark_tgid, 10);
        assert_eq!(mark.message, "done");
    }

    #[test]
    fn trace_mark_end_to_string() {
        let line = "task-100 (100) [000] .... 1.000000: tracing_mark_write: E|100|finished";
        let mark = TraceMarkEnd::parse(line).expect("must parse");
        let result = mark.to_string().expect("to_string must work");
        assert_eq!(result, line);
    }

    #[test]
    fn trace_mark_end_new_and_methods() {
        let mark = TraceMarkEnd::new(
            "task".into(), 100, 100, 0, "....".into(), 1.0, "tracing_mark_write".into(),
            100, "done".into(),
        ).unwrap();
        assert_eq!(mark.thread_name, "task");
        assert_eq!(mark.message, "done");
        assert_eq!(mark.payload(), "E|100|done");
    }
}

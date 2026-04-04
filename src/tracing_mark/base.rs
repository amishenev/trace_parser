use pyo3::prelude::*;
use trace_parser_macros::TraceEvent;

#[pyclass(skip_from_py_object)]
#[derive(Clone, Debug, PartialEq)]
#[derive(TraceEvent)]
#[trace_event(name = "tracing_mark_write")]
#[define_template("{_raw}", _raw = r".*")]
pub struct TracingMark {
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
    #[field(private)]
    _raw: String,
}

#[cfg(test)]
mod tests {
    use crate::TracingMark;

    #[test]
    fn tracing_mark_accepts_any_payload() {
        let line = "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: anything at all";
        let mark = TracingMark::parse(line).expect("tracing mark must parse");
        assert_eq!(mark.event_name, "tracing_mark_write");
        assert_eq!(mark.payload().unwrap(), "anything at all");
    }

    #[test]
    fn tracing_mark_custom_payload() {
        let line = "task-100 (100) [000] .... 1.0: tracing_mark_write: custom_payload_here";
        let mark = TracingMark::parse(line).expect("must parse");
        assert_eq!(mark.payload().unwrap(), "custom_payload_here");
        assert_eq!(mark.template(), "{_raw}");
    }
}

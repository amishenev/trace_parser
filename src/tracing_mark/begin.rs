use pyo3::prelude::*;
use trace_parser_macros::TracingMarkEvent;

#[pyclass(skip_from_py_object)]
#[derive(Clone, Debug, PartialEq)]
#[derive(TracingMarkEvent)]
#[trace_event(name = "tracing_mark_write", begin, skip_registration)]
#[define_template("{message}")]
pub struct TraceMarkBegin {
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
    #[field]
    pub message: String,
}

#[cfg(test)]
mod tests {
    use crate::TraceMarkBegin;

    #[test]
    fn trace_mark_begin_parses() {
        let line =
            "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: B|10|some_custom_message";
        let mark = TraceMarkBegin::parse(line).expect("begin mark must parse");
        assert_eq!(mark.trace_mark_tgid, 10);
        assert_eq!(mark.message, "some_custom_message");
        assert_eq!(
            mark.to_string().expect("to_string must work"),
            "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: B|10|some_custom_message"
        );
    }

    #[test]
    fn trace_mark_begin_new_and_methods() {
        let mark = TraceMarkBegin::new(
            0, "task".into(), 100, 100, 0, "....".into(), 1.0, "tracing_mark_write".into(),
            100, "message".into(),
        ).unwrap();
        assert_eq!(mark.thread_name, "task");
        assert_eq!(mark.thread_tid, 100);
        assert_eq!(mark.trace_mark_tgid, 100);
        assert_eq!(mark.message, "message");
    }
}

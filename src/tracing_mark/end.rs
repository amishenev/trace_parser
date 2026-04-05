use trace_parser_macros::tracing_mark_event_class;

#[tracing_mark_event_class]
#[trace_event(name = "tracing_mark_write", end, skip_registration)]
#[define_template("{message}")]
pub struct TraceMarkEnd {
    #[field(private)]
    format_id: u8,
    #[field]
    pub thread_name: String,
    #[field]
    pub thread_tid: u32,
    #[field]
    pub thread_tgid: u32,
    #[field]
    pub cpu: u32,
    #[field]
    pub flags: String,
    #[field]
    pub timestamp: f64,
    #[field(readonly)]
    pub event_name: String,
    #[field]
    pub trace_mark_tgid: u32,
    #[field]
    pub message: String,
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
            0, "task".into(), 100, 100, 0, "....".into(), 1.0, "tracing_mark_write".into(),
            100, "done".into(),
        ).unwrap();
        assert_eq!(mark.thread_name, "task");
        assert_eq!(mark.message, "done");
    }
}

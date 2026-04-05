use trace_parser_macros::trace_event_class;

#[trace_event_class]
#[trace_event(name = "tracing_mark_write")]
#[define_template("{_raw}", _raw = r".*")]
pub struct TracingMark {
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
    #[field(private)]
    _raw: String,
}

#[cfg(test)]
mod tests {
    use crate::TracingMark;

    #[test]
    fn tracing_mark_accepts_any_payload() {
        let line =
            "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: anything at all";
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

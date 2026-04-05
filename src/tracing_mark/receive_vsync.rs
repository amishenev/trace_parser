use trace_parser_macros::tracing_mark_event_class;

#[tracing_mark_event_class]
#[trace_event(name = "tracing_mark_write", begin, skip_registration)]
#[trace_markers("ReceiveVsync")]
#[define_template(
    "{?ignore:extra_info}ReceiveVsync {frame_number}",
    extra_info = r"\[[^\]]+\]"
)]
pub struct TraceReceiveVsync {
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
    pub frame_number: u32,
}

#[cfg(test)]
mod tests {
    use crate::TraceReceiveVsync;

    #[test]
    fn receive_vsync_parses_specific_begin_payload() {
        let line =
            "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: B|10|ReceiveVsync 42";
        let mark = TraceReceiveVsync::parse(line).expect("receive vsync begin mark must parse");
        assert_eq!(mark.trace_mark_tgid, 10);
        assert_eq!(mark.frame_number, 42);
        assert_eq!(
            mark.to_string().expect("to_string must work"),
            "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: B|10|ReceiveVsync 42"
        );
    }

    #[test]
    fn receive_vsync_ignores_service_prefix() {
        let line = "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: B|10|[ExtraInfo]ReceiveVsync 42";
        let mark = TraceReceiveVsync::parse(line).expect("receive vsync begin mark must parse");
        assert_eq!(mark.frame_number, 42);
    }
}

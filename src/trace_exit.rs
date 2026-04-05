use trace_parser_macros::trace_event_class;

#[trace_event_class]
#[trace_event(name = "exit1", aliases = ["exit2"])]
#[define_template("pid={pid} comm={comm} tgid={tgid}")]
pub struct TraceExit {
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
    pub pid: u32,
    #[field]
    pub comm: String,
    #[field]
    pub tgid: u32,
}

#[cfg(test)]
mod tests {
    use crate::TraceExit;

    #[test]
    fn test_exit1_parse() {
        let line = "task-100 (100) [000] .... 123.456789: exit1: pid=123 comm=test tgid=100";
        let exit = TraceExit::parse(line).expect("exit1 must parse");
        assert_eq!(exit.pid, 123);
        assert_eq!(exit.comm, "test");
        assert_eq!(exit.tgid, 100);
    }

    #[test]
    fn test_exit2_parse() {
        let line = "task-200 (200) [001] .... 456.789012: exit2: pid=456 comm=foo tgid=200";
        let exit = TraceExit::parse(line).expect("exit2 must parse");
        assert_eq!(exit.pid, 456);
        assert_eq!(exit.comm, "foo");
        assert_eq!(exit.tgid, 200);
    }

    #[test]
    fn test_exit_to_string() {
        let line = "task-100 (100) [000] .... 123.456789: exit1: pid=123 comm=test tgid=100";
        let exit = TraceExit::parse(line).expect("exit1 must parse");
        let result = exit.to_string().expect("to_string must work");
        assert_eq!(result, line);
    }
}

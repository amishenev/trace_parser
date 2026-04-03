use pyo3::prelude::*;

use crate::register_parser;
use trace_parser_macros::TraceEvent;

#[pyclass(skip_from_py_object)]
#[derive(Clone, Debug, PartialEq)]
#[derive(TraceEvent)]
#[trace_event(name = "sched_switch")]
#[define_template("prev_comm={prev_comm} prev_pid={prev_pid} prev_prio={prev_prio} prev_state={prev_state} ==> next_comm={next_comm} next_pid={next_pid} next_prio={next_prio}")]
pub struct TraceSchedSwitch {
    #[pyo3(get, set)]
    #[field(ty = "string")]
    pub thread_name: String,
    #[pyo3(get, set)]
    #[field(ty = "u32")]
    pub thread_tid: u32,
    #[pyo3(get, set)]
    #[field(ty = "u32")]
    pub thread_tgid: u32,
    #[pyo3(get, set)]
    #[field(ty = "u32")]
    pub cpu: u32,
    #[pyo3(get, set)]
    #[field(ty = "string")]
    pub flags: String,
    #[pyo3(get, set)]
    #[field(ty = "f64")]
    pub timestamp: f64,
    #[pyo3(get)]
    #[field(ty = "string")]
    pub event_name: String,
    #[field(ty = "u8")]
    pub format_id: u8,
    #[field(ty = "string")]
    pub payload_raw: String,
    #[pyo3(get, set)]
    #[field(ty = "string")]
    pub prev_comm: String,
    #[pyo3(get, set)]
    #[field(ty = "u32")]
    pub prev_pid: u32,
    #[pyo3(get, set)]
    #[field(ty = "i32")]
    pub prev_prio: i32,
    #[pyo3(get, set)]
    #[field(ty = "string")]
    pub prev_state: String,
    #[pyo3(get, set)]
    #[field(ty = "string")]
    pub next_comm: String,
    #[pyo3(get, set)]
    #[field(ty = "u32")]
    pub next_pid: u32,
    #[pyo3(get, set)]
    #[field(ty = "i32")]
    pub next_prio: i32,
}

#[cfg(test)]
mod tests {
    use crate::TraceSchedSwitch;

    #[test]
    fn sched_switch_can_be_parsed_matches_only_sched_switch() {
        let line = "bash-1977   (  12) [000] .... 12345.678901: sched_switch: prev_comm=bash prev_pid=1977 prev_prio=120 prev_state=S ==> next_comm=worker next_pid=123 next_prio=120";
        assert!(TraceSchedSwitch::can_be_parsed(line));

        let wrong = "kworker-123 ( 123) [000] .... 12345.679001: sched_wakeup: comm=bash pid=1977 prio=120 target_cpu=000";
        assert!(!TraceSchedSwitch::can_be_parsed(wrong));
    }

    #[test]
    fn sched_switch_parse_extracts_payload_fields() {
        let line = "bash-1977   (  12) [000] .... 12345.678901: sched_switch: prev_comm=bash prev_pid=1977 prev_prio=120 prev_state=S ==> next_comm=worker next_pid=123 next_prio=120";
        let trace = TraceSchedSwitch::parse(line).expect("sched_switch must parse");
        assert_eq!(trace.thread_name, "bash");
        assert_eq!(trace.thread_tid, 1977);
        assert_eq!(trace.thread_tgid, 12);
        assert_eq!(trace.cpu, 0);
        assert_eq!(trace.flags, "....");
        assert!((trace.timestamp - 12345.678901).abs() < 1e-9);
        assert_eq!(trace.prev_comm, "bash");
        assert_eq!(trace.prev_pid, 1977);
        assert_eq!(trace.prev_prio, 120);
        assert_eq!(trace.prev_state, "S");
        assert_eq!(trace.next_comm, "worker");
        assert_eq!(trace.next_pid, 123);
        assert_eq!(trace.next_prio, 120);
        assert_eq!(
            trace.payload().expect("payload must work"),
            "prev_comm=bash prev_pid=1977 prev_prio=120 prev_state=S ==> next_comm=worker next_pid=123 next_prio=120"
        );
        assert_eq!(
            trace.to_string().expect("to_string must work"),
            "bash-1977 (12) [000] .... 12345.678901: sched_switch: prev_comm=bash prev_pid=1977 prev_prio=120 prev_state=S ==> next_comm=worker next_pid=123 next_prio=120"
        );
    }

    #[test]
    fn timestamp_setters_update_canonical_output() {
        let line = "bash-1977   (  12) [000] .... 12345.678901: sched_switch: prev_comm=bash prev_pid=1977 prev_prio=120 prev_state=S ==> next_comm=worker next_pid=123 next_prio=120";
        let mut trace = TraceSchedSwitch::parse(line).expect("sched_switch must parse");
        trace
            .set_timestamp_ms(1_500.25)
            .expect("timestamp_ms setter must work");
        assert!((trace.timestamp - 1.50025).abs() < 1e-9);
        assert_eq!(trace.timestamp_ns(), 1_500_250_000);
        assert_eq!(
            trace.to_string().expect("to_string must work"),
            "bash-1977 (12) [000] .... 1.500250: sched_switch: prev_comm=bash prev_pid=1977 prev_prio=120 prev_state=S ==> next_comm=worker next_pid=123 next_prio=120"
        );
    }
}

// Register parser at compile time
register_parser!("sched_switch", TraceSchedSwitch);

use pyo3::prelude::*;
use trace_parser_macros::TraceEvent;

#[pyclass(skip_from_py_object)]
#[derive(Clone, Debug, PartialEq)]
#[derive(TraceEvent)]
#[trace_event(name = "sched_process_exit")]
#[define_template("comm={comm} pid={pid} prio={prio} group_dead={group_dead}")]
pub struct TraceSchedProcessExit {
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
    pub comm: String,
    #[pyo3(get, set)]
    #[field]
    pub pid: u32,
    #[pyo3(get, set)]
    #[field]
    pub prio: i32,
    #[pyo3(get, set)]
    #[field]
    pub group_dead: bool,
}

#[cfg(test)]
mod tests {
    use crate::TraceSchedProcessExit;

    #[test]
    fn sched_process_exit_parses() {
        let line = "bash-1977 (12) [000] .... 12345.678901: sched_process_exit: comm=bash pid=1977 prio=120 group_dead=1";
        let trace = TraceSchedProcessExit::parse(line).expect("sched_process_exit must parse");
        assert_eq!(trace.comm, "bash");
        assert_eq!(trace.pid, 1977);
        assert_eq!(trace.prio, 120);
        assert!(trace.group_dead);
        assert_eq!(trace.thread_name, "bash");
        assert_eq!(trace.thread_tid, 1977);
        assert_eq!(trace.thread_tgid, 12);
        assert_eq!(trace.cpu, 0);
        assert_eq!(
            trace.payload().expect("payload must work"),
            "comm=bash pid=1977 prio=120 group_dead=1"
        );
        assert_eq!(
            trace.to_string().expect("to_string must work"),
            "bash-1977 (12) [000] .... 12345.678901: sched_process_exit: comm=bash pid=1977 prio=120 group_dead=1"
        );
    }
}

use pyo3::prelude::*;
use trace_parser_macros::TraceEvent;

#[pyclass(skip_from_py_object)]
#[derive(Clone, Debug, PartialEq)]
#[derive(TraceEvent)]
#[trace_event(name = "sched_wakeup")]
#[define_template("comm={comm} pid={pid} prio={prio} target_cpu={target_cpu}")]
#[define_template("comm={comm} pid={pid} prio={prio} target_cpu={target_cpu} reason={reason}", detect = ["reason="])]
pub struct TraceSchedWakeup {
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
    #[field(regex = r"\d{3}", format = "{:03}")]
    pub target_cpu: u32,
    #[pyo3(get, set)]
    #[field(optional)]
    pub reason: Option<u32>,
}

#[pyclass(skip_from_py_object)]
#[derive(Clone, Debug, PartialEq)]
#[derive(TraceEvent)]
#[trace_event(name = "sched_wakeup_new")]
#[define_template("comm={comm} pid={pid} prio={prio} target_cpu={target_cpu}")]
pub struct TraceSchedWakeupNew {
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
    #[field(regex = r"\d{3}", format = "{:03}")]
    pub target_cpu: u32,
}

#[cfg(test)]
mod tests {
    use crate::{TraceSchedWakeup, TraceSchedWakeupNew};

    #[test]
    fn sched_wakeup_default_format_parses() {
        let line = "kworker-123 (123) [000] .... 12345.679001: sched_wakeup: comm=bash pid=1977 prio=120 target_cpu=000";
        let trace = TraceSchedWakeup::parse(line).expect("sched_wakeup must parse");
        assert_eq!(trace.comm, "bash");
        assert_eq!(trace.pid, 1977);
        assert_eq!(trace.prio, 120);
        assert_eq!(trace.target_cpu, 0);
        assert_eq!(trace.reason, None);
        assert_eq!(trace.payload().unwrap(), "comm=bash pid=1977 prio=120 target_cpu=000");
    }

    #[test]
    fn sched_wakeup_with_reason_format_parses() {
        let line = "kworker-123 (123) [000] .... 12345.679001: sched_wakeup: comm=bash pid=1977 prio=120 target_cpu=000 reason=3";
        let trace = TraceSchedWakeup::parse(line).expect("sched_wakeup with reason must parse");
        assert_eq!(trace.comm, "bash");
        assert_eq!(trace.pid, 1977);
        assert_eq!(trace.prio, 120);
        assert_eq!(trace.target_cpu, 0);
        assert_eq!(trace.reason, Some(3));
        assert_eq!(trace.payload().unwrap(), "comm=bash pid=1977 prio=120 target_cpu=000 reason=3");
    }

    #[test]
    fn sched_wakeup_new_parses() {
        let line = "kworker-123 (123) [000] .... 12345.679001: sched_wakeup_new: comm=bash pid=1977 prio=120 target_cpu=000";
        let trace = TraceSchedWakeupNew::parse(line).expect("sched_wakeup_new must parse");
        assert_eq!(trace.comm, "bash");
        assert_eq!(trace.pid, 1977);
        assert_eq!(trace.prio, 120);
        assert_eq!(trace.target_cpu, 0);
        assert_eq!(trace.payload().unwrap(), "comm=bash pid=1977 prio=120 target_cpu=000");
    }

    #[test]
    fn sched_wakeup_to_string() {
        let line = "kworker-123 (123) [000] .... 12345.679001: sched_wakeup: comm=bash pid=1977 prio=120 target_cpu=000";
        let trace = TraceSchedWakeup::parse(line).expect("sched_wakeup must parse");
        assert_eq!(trace.to_string().unwrap(), line);

        let line2 = "kworker-123 (123) [000] .... 12345.679001: sched_wakeup: comm=bash pid=1977 prio=120 target_cpu=000 reason=3";
        let trace2 = TraceSchedWakeup::parse(line2).expect("sched_wakeup with reason must parse");
        assert_eq!(trace2.to_string().unwrap(), line2);
    }
}

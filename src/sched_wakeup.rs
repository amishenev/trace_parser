use trace_parser_macros::trace_event_class;

#[trace_event_class]
#[trace_event(name = "sched_wakeup")]
#[define_template("comm={comm} pid={pid} prio={prio} target_cpu={target_cpu}")]
#[define_template("comm={comm} pid={pid} prio={prio} target_cpu={target_cpu} reason={reason}", detect = ["reason="])]
#[define_template("comm={comm} pid={pid} prio={prio} success={success} target_cpu={target_cpu}", detect = ["success="])]
pub struct TraceSchedWakeup {
    #[field(private)]
    format_id: u8,
    #[field]
    pub thread_name: String,
    #[field]
    pub thread_tid: u32,
    #[field]
    pub thread_tgid: Option<u32>,
    #[field]
    pub cpu: u32,
    #[field]
    pub flags: String,
    #[field]
    pub timestamp: f64,
    #[field(readonly)]
    pub event_name: String,
    #[field]
    pub comm: String,
    #[field]
    pub pid: u32,
    #[field]
    pub prio: i32,
    #[field(regex = r"\d{3}", format = "{:03}")]
    pub target_cpu: u32,
    #[field]
    pub success: Option<bool>,
    #[field]
    pub reason: Option<u32>,
}

#[trace_event_class]
#[trace_event(name = "sched_wakeup_new")]
#[define_template("comm={comm} pid={pid} prio={prio} target_cpu={target_cpu}")]
pub struct TraceSchedWakeupNew {
    #[field(private)]
    format_id: u8,
    #[field]
    pub thread_name: String,
    #[field]
    pub thread_tid: u32,
    #[field]
    pub thread_tgid: Option<u32>,
    #[field]
    pub cpu: u32,
    #[field]
    pub flags: String,
    #[field]
    pub timestamp: f64,
    #[field(readonly)]
    pub event_name: String,
    #[field]
    pub comm: String,
    #[field]
    pub pid: u32,
    #[field]
    pub prio: i32,
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
        assert_eq!(
            trace.payload().unwrap(),
            "comm=bash pid=1977 prio=120 target_cpu=000"
        );
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
        assert_eq!(
            trace.payload().unwrap(),
            "comm=bash pid=1977 prio=120 target_cpu=000 reason=3"
        );
    }

    #[test]
    fn sched_wakeup_new_parses() {
        let line = "kworker-123 (123) [000] .... 12345.679001: sched_wakeup_new: comm=bash pid=1977 prio=120 target_cpu=000";
        let trace = TraceSchedWakeupNew::parse(line).expect("sched_wakeup_new must parse");
        assert_eq!(trace.comm, "bash");
        assert_eq!(trace.pid, 1977);
        assert_eq!(trace.prio, 120);
        assert_eq!(trace.target_cpu, 0);
        assert_eq!(
            trace.payload().unwrap(),
            "comm=bash pid=1977 prio=120 target_cpu=000"
        );
    }

    #[test]
    fn sched_wakeup_to_string() {
        let line = "kworker-123 (123) [000] .... 12345.679001: sched_wakeup: comm=bash pid=1977 prio=120 target_cpu=000";
        let trace = TraceSchedWakeup::parse(line).expect("sched_wakeup must parse");
        assert_eq!(trace.to_string().unwrap(), line);

        let line2 = "kworker-123 (123) [000] .... 12345.679001: sched_wakeup: comm=bash pid=1977 prio=120 target_cpu=000 reason=3";
        let trace2 = TraceSchedWakeup::parse(line2).expect("sched_wakeup with reason must parse");
        assert_eq!(trace2.to_string().unwrap(), line2);

        let line3 = "kworker-123 (123) [000] .... 12345.679001: sched_wakeup: comm=bash pid=1977 prio=120 success=1 target_cpu=001";
        let trace3 = TraceSchedWakeup::parse(line3).expect("sched_wakeup with success must parse");
        assert_eq!(trace3.to_string().unwrap(), line3);
    }

    #[test]
    fn sched_wakeup_parses_dashed_tgid_as_none() {
        let line = "<idle>-0 (-----) [001] d..2 2318.330977: sched_wakeup: comm=bash pid=1977 prio=120 target_cpu=001";
        let trace = TraceSchedWakeup::parse(line).expect("sched_wakeup must parse");
        assert_eq!(trace.thread_tid, 0);
        assert_eq!(trace.thread_tgid, None);
        assert!(!trace.has_unknown_thread());
        assert_eq!(
            trace.to_string().unwrap(),
            "<idle>-0 (-) [001] d..2 2318.330977: sched_wakeup: comm=bash pid=1977 prio=120 target_cpu=001"
        );
    }

    #[test]
    fn sched_wakeup_marks_unknown_thread_name() {
        let line =
            "<...>-0 (-----) [001] d..2 2318.330977: sched_wakeup: comm=bash pid=1977 prio=120 target_cpu=001";
        let trace = TraceSchedWakeup::parse(line).expect("sched_wakeup must parse");
        assert!(trace.has_unknown_thread());
    }

    #[test]
    fn sched_wakeup_with_success_format_parses() {
        let line = "<idle>-0     (-----) [001] dn.4  2318.331005: sched_wakeup: comm=ksoftirqd/1 pid=12 prio=120 success=1 target_cpu=001";
        let trace =
            TraceSchedWakeup::parse(line).expect("sched_wakeup with success must parse");
        assert_eq!(trace.comm, "ksoftirqd/1");
        assert_eq!(trace.pid, 12);
        assert_eq!(trace.prio, 120);
        assert_eq!(trace.target_cpu, 1);
        assert_eq!(trace.success, Some(true));
        assert_eq!(trace.reason, None);
        assert_eq!(
            trace.payload().unwrap(),
            "comm=ksoftirqd/1 pid=12 prio=120 success=1 target_cpu=001"
        );
    }

    #[test]
    fn sched_wakeup_success_is_none_in_other_formats() {
        // Без success и reason
        let line = "kworker-123 (123) [000] .... 12345.679001: sched_wakeup: comm=bash pid=1977 prio=120 target_cpu=000";
        let trace = TraceSchedWakeup::parse(line).expect("sched_wakeup must parse");
        assert_eq!(trace.success, None);
        assert_eq!(trace.reason, None);

        // Только reason
        let line2 = "kworker-123 (123) [000] .... 12345.679001: sched_wakeup: comm=bash pid=1977 prio=120 target_cpu=000 reason=3";
        let trace2 = TraceSchedWakeup::parse(line2).expect("sched_wakeup with reason must parse");
        assert_eq!(trace2.success, None);
        assert_eq!(trace2.reason, Some(3));
    }
}

use pyo3::prelude::*;
use regex::Captures;
use std::sync::LazyLock;

use crate::common::{
    cap_parse, cap_str, parse_template_event, validate_timestamp, BaseTraceParts, EventType,
    FastMatch, TemplateEvent,
};
use crate::format_registry::{FormatRegistry, FormatSpec};
use crate::payload_template::{FieldSpec, PayloadTemplate, TemplateValue};
use crate::trace::{extract_base_fields, format_trace_header};

static TEMPLATE_DEFAULT: LazyLock<PayloadTemplate> = LazyLock::new(|| {
    PayloadTemplate::new(
        "comm={comm} pid={pid} prio={prio} target_cpu={target_cpu}",
        &[
            ("comm", FieldSpec::string()),
            ("pid", FieldSpec::u32()),
            ("prio", FieldSpec::i32()),
            ("target_cpu", FieldSpec::custom(r"\d{3}")),
        ],
    )
});

static TEMPLATE_WITH_REASON: LazyLock<PayloadTemplate> = LazyLock::new(|| {
    PayloadTemplate::new(
        "comm={comm} pid={pid} prio={prio} target_cpu={target_cpu} reason={reason}",
        &[
            ("comm", FieldSpec::string()),
            ("pid", FieldSpec::u32()),
            ("prio", FieldSpec::i32()),
            ("target_cpu", FieldSpec::custom(r"\d{3}")),
            ("reason", FieldSpec::u32()),
        ],
    )
});

static FORMATS: LazyLock<FormatRegistry> = LazyLock::new(|| {
    FormatRegistry::new(vec![
        FormatSpec {
            kind: 0,
            template: &TEMPLATE_DEFAULT,
        },
        FormatSpec {
            kind: 1,
            template: &TEMPLATE_WITH_REASON,
        },
    ])
});

#[pyclass(skip_from_py_object)]
#[derive(Clone, Debug, PartialEq)]
pub struct TraceSchedWakeup {
    #[pyo3(get, set)]
    pub thread_name: String,
    #[pyo3(get, set)]
    pub tid: u32,
    #[pyo3(get, set)]
    pub tgid: u32,
    #[pyo3(get, set)]
    pub cpu: u32,
    #[pyo3(get, set)]
    pub flags: String,
    #[pyo3(get, set)]
    pub timestamp: f64,
    #[pyo3(get)]
    pub event_name: String,
    #[pyo3(get, set)]
    pub payload_raw: String,
    #[pyo3(get, set)]
    pub format_id: u8,
    #[pyo3(get, set)]
    pub comm: String,
    #[pyo3(get, set)]
    pub pid: u32,
    #[pyo3(get, set)]
    pub prio: i32,
    #[pyo3(get, set)]
    pub target_cpu: u32,
    #[pyo3(get, set)]
    pub reason: Option<u32>,
}

#[pyclass(skip_from_py_object)]
#[derive(Clone, Debug, PartialEq)]
pub struct TraceSchedWakeupNew {
    #[pyo3(get, set)]
    pub thread_name: String,
    #[pyo3(get, set)]
    pub tid: u32,
    #[pyo3(get, set)]
    pub tgid: u32,
    #[pyo3(get, set)]
    pub cpu: u32,
    #[pyo3(get, set)]
    pub flags: String,
    #[pyo3(get, set)]
    pub timestamp: f64,
    #[pyo3(get)]
    pub event_name: String,
    #[pyo3(get, set)]
    pub payload_raw: String,
    #[pyo3(get, set)]
    pub format_id: u8,
    #[pyo3(get, set)]
    pub comm: String,
    #[pyo3(get, set)]
    pub pid: u32,
    #[pyo3(get, set)]
    pub prio: i32,
    #[pyo3(get, set)]
    pub target_cpu: u32,
}

impl EventType for TraceSchedWakeup {
    const EVENT_NAME: &'static str = "sched_wakeup";
}

impl FastMatch for TraceSchedWakeup {}

impl TemplateEvent for TraceSchedWakeup {
    fn formats() -> &'static FormatRegistry {
        &FORMATS
    }

    fn detect_format(payload: &str) -> u8 {
        if payload.contains("reason=") { 1 } else { 0 }
    }

    fn parse_payload(
        parts: BaseTraceParts,
        captures: &Captures<'_>,
        format_id: u8,
    ) -> Option<Self> {
        let reason = if format_id == 1 {
            cap_parse(captures, "reason")
        } else {
            None
        };

        let (thread_name, tid, tgid, cpu, flags, timestamp, event_name, payload_raw) =
            extract_base_fields(&parts);

        Some(Self {
            thread_name,
            tid,
            tgid,
            cpu,
            flags,
            timestamp,
            event_name,
            payload_raw,
            format_id,
            comm: cap_str(captures, "comm")?,
            pid: cap_parse(captures, "pid")?,
            prio: cap_parse(captures, "prio")?,
            target_cpu: cap_parse(captures, "target_cpu")?,
            reason,
        })
    }

    fn render_payload(&self) -> PyResult<String> {
        let template = Self::formats().template(self.format_id).unwrap();
        let target_cpu = format!("{:03}", self.target_cpu);

        // Собираем values в массив — без аллокации HashMap
        // reason опционален, поэтому используем Option и фильтруем None
        let values: [(&str, Option<TemplateValue>); 5] = [
            ("comm", Some(TemplateValue::Str(&self.comm))),
            ("pid", Some(TemplateValue::U32(self.pid))),
            ("prio", Some(TemplateValue::I32(self.prio))),
            ("target_cpu", Some(TemplateValue::Str(&target_cpu))),
            ("reason", self.reason.map(TemplateValue::U32)),
        ];

        Ok(template
            .format(&values)
            .expect("sched_wakeup template must render"))
    }
}

impl EventType for TraceSchedWakeupNew {
    const EVENT_NAME: &'static str = "sched_wakeup_new";
}

impl FastMatch for TraceSchedWakeupNew {}

impl TemplateEvent for TraceSchedWakeupNew {
    fn formats() -> &'static FormatRegistry {
        &FORMATS
    }

    fn parse_payload(
        parts: BaseTraceParts,
        captures: &Captures<'_>,
        _format_id: u8,
    ) -> Option<Self> {
        let (thread_name, tid, tgid, cpu, flags, timestamp, event_name, payload_raw) =
            extract_base_fields(&parts);

        Some(Self {
            thread_name,
            tid,
            tgid,
            cpu,
            flags,
            timestamp,
            event_name,
            payload_raw,
            format_id: 0,
            comm: cap_str(captures, "comm")?,
            pid: cap_parse(captures, "pid")?,
            prio: cap_parse(captures, "prio")?,
            target_cpu: cap_parse(captures, "target_cpu")?,
        })
    }

    fn render_payload(&self) -> PyResult<String> {
        let template = Self::formats().template(0).unwrap();
        let target_cpu = format!("{:03}", self.target_cpu);
        let values: [(&str, Option<TemplateValue>); 4] = [
            ("comm", Some(TemplateValue::Str(&self.comm))),
            ("pid", Some(TemplateValue::U32(self.pid))),
            ("prio", Some(TemplateValue::I32(self.prio))),
            ("target_cpu", Some(TemplateValue::Str(&target_cpu))),
        ];
        Ok(template
            .format(&values)
            .expect("sched_wakeup_new template must render"))
    }
}

#[pymethods]
impl TraceSchedWakeup {
    #[new]
    #[pyo3(signature = (thread_name, tid, tgid, cpu, flags, timestamp, event_name, payload_raw, format_id, comm, pid, prio, target_cpu, reason=None))]
    fn new(
        thread_name: String,
        tid: u32,
        tgid: u32,
        cpu: u32,
        flags: String,
        timestamp: f64,
        event_name: String,
        payload_raw: String,
        format_id: u8,
        comm: String,
        pid: u32,
        prio: i32,
        target_cpu: u32,
        reason: Option<u32>,
    ) -> PyResult<Self> {
        validate_timestamp(timestamp)?;
        Ok(Self {
            thread_name,
            tid,
            tgid,
            cpu,
            flags,
            timestamp,
            event_name,
            payload_raw,
            format_id,
            comm,
            pid,
            prio,
            target_cpu,
            reason,
        })
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "TraceSchedWakeup(comm={:?}, pid={}, prio={}, target_cpu={}, reason={:?})",
            self.comm, self.pid, self.prio, self.target_cpu, self.reason
        ))
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.thread_name == other.thread_name
            && self.tid == other.tid
            && self.tgid == other.tgid
            && self.cpu == other.cpu
            && self.flags == other.flags
            && self.timestamp == other.timestamp
            && self.event_name == other.event_name
            && self.payload_raw == other.payload_raw
            && self.format_id == other.format_id
            && self.comm == other.comm
            && self.pid == other.pid
            && self.prio == other.prio
            && self.target_cpu == other.target_cpu
            && self.reason == other.reason
    }

    fn __str__(&self) -> PyResult<String> {
        self.to_string()
    }

    fn __copy__(slf: PyRef<'_, Self>, py: Python<'_>) -> PyResult<Py<Self>> {
        slf.clone().into_pyobject(py).map(|o| o.unbind())
    }

    fn __deepcopy__(&self, py: Python<'_>, _memo: &Bound<'_, PyAny>) -> PyResult<Py<Self>> {
        self.clone().into_pyobject(py).map(|o| o.unbind())
    }

    #[staticmethod]
    pub fn can_be_parsed(line: &str) -> bool {
        Self::quick_check(line)
    }

    #[staticmethod]
    pub fn parse(line: &str) -> Option<Self> {
        if !Self::can_be_parsed(line) {
            return None;
        }
        parse_template_event::<Self>(line)
    }

    pub fn payload_to_string(&self) -> PyResult<String> {
        self.render_payload()
    }

    pub fn to_string(&self) -> PyResult<String> {
        validate_timestamp(self.timestamp)?;
        let payload = self.payload_to_string()?;
        Ok(format_trace_header(
            &self.thread_name, self.tid, self.tgid, self.cpu,
            &self.flags, self.timestamp, &self.event_name,
            &payload
        ))
    }
}

#[pymethods]
impl TraceSchedWakeupNew {
    #[new]
    #[pyo3(signature = (thread_name, tid, tgid, cpu, flags, timestamp, event_name, payload_raw, format_id, comm, pid, prio, target_cpu))]
    fn new(
        thread_name: String,
        tid: u32,
        tgid: u32,
        cpu: u32,
        flags: String,
        timestamp: f64,
        event_name: String,
        payload_raw: String,
        format_id: u8,
        comm: String,
        pid: u32,
        prio: i32,
        target_cpu: u32,
    ) -> PyResult<Self> {
        validate_timestamp(timestamp)?;
        Ok(Self {
            thread_name,
            tid,
            tgid,
            cpu,
            flags,
            timestamp,
            event_name,
            payload_raw,
            format_id,
            comm,
            pid,
            prio,
            target_cpu,
        })
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "TraceSchedWakeupNew(comm={:?}, pid={}, prio={}, target_cpu={})",
            self.comm, self.pid, self.prio, self.target_cpu
        ))
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.thread_name == other.thread_name
            && self.tid == other.tid
            && self.tgid == other.tgid
            && self.cpu == other.cpu
            && self.flags == other.flags
            && self.timestamp == other.timestamp
            && self.event_name == other.event_name
            && self.payload_raw == other.payload_raw
            && self.format_id == other.format_id
            && self.comm == other.comm
            && self.pid == other.pid
            && self.prio == other.prio
            && self.target_cpu == other.target_cpu
    }

    fn __str__(&self) -> PyResult<String> {
        self.to_string()
    }

    fn __copy__(slf: PyRef<'_, Self>, py: Python<'_>) -> PyResult<Py<Self>> {
        slf.clone().into_pyobject(py).map(|o| o.unbind())
    }

    fn __deepcopy__(&self, py: Python<'_>, _memo: &Bound<'_, PyAny>) -> PyResult<Py<Self>> {
        self.clone().into_pyobject(py).map(|o| o.unbind())
    }

    #[staticmethod]
    pub fn can_be_parsed(line: &str) -> bool {
        Self::quick_check(line)
    }

    #[staticmethod]
    pub fn parse(line: &str) -> Option<Self> {
        if !Self::can_be_parsed(line) {
            return None;
        }
        parse_template_event::<Self>(line)
    }

    pub fn payload_to_string(&self) -> PyResult<String> {
        self.render_payload()
    }

    pub fn to_string(&self) -> PyResult<String> {
        validate_timestamp(self.timestamp)?;
        let payload = self.payload_to_string()?;
        Ok(format_trace_header(
            &self.thread_name, self.tid, self.tgid, self.cpu,
            &self.flags, self.timestamp, &self.event_name,
            &payload
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::{TraceSchedWakeup, TraceSchedWakeupNew};

    #[test]
    fn sched_wakeup_default_format_parses() {
        let line = "kworker-123 ( 123) [000] .... 12345.679001: sched_wakeup: comm=bash pid=1977 prio=120 target_cpu=000";
        let trace = TraceSchedWakeup::parse(line).expect("sched_wakeup must parse");
        assert_eq!(trace.thread_name, "kworker");
        assert_eq!(trace.tid, 123);
        assert_eq!(trace.tgid, 123);
        assert_eq!(trace.cpu, 0);
        assert_eq!(trace.flags, "....");
        assert!((trace.timestamp - 12345.679001).abs() < 1e-9);
        assert_eq!(trace.event_name, "sched_wakeup");
        assert_eq!(trace.comm, "bash");
        assert_eq!(trace.pid, 1977);
        assert_eq!(trace.prio, 120);
        assert_eq!(trace.target_cpu, 0);
        assert_eq!(trace.reason, None);
        assert_eq!(trace.format_id, 0);
        assert_eq!(
            trace
                .payload_to_string()
                .expect("payload_to_string must work"),
            "comm=bash pid=1977 prio=120 target_cpu=000"
        );
        assert_eq!(
            trace.to_string().expect("to_string must work"),
            "kworker-123 (123) [000] .... 12345.679001: sched_wakeup: comm=bash pid=1977 prio=120 target_cpu=000"
        );
    }

    #[test]
    fn sched_wakeup_with_reason_format_parses() {
        let line = "kworker-123 ( 123) [000] .... 12345.679001: sched_wakeup: comm=bash pid=1977 prio=120 target_cpu=000 reason=3";
        let trace = TraceSchedWakeup::parse(line).expect("sched_wakeup must parse");
        assert_eq!(trace.thread_name, "kworker");
        assert_eq!(trace.tid, 123);
        assert_eq!(trace.tgid, 123);
        assert_eq!(trace.cpu, 0);
        assert_eq!(trace.flags, "....");
        assert!((trace.timestamp - 12345.679001).abs() < 1e-9);
        assert_eq!(trace.event_name, "sched_wakeup");
        assert_eq!(trace.comm, "bash");
        assert_eq!(trace.pid, 1977);
        assert_eq!(trace.prio, 120);
        assert_eq!(trace.target_cpu, 0);
        assert_eq!(trace.reason, Some(3));
        assert_eq!(trace.format_id, 1);
        assert_eq!(
            trace
                .payload_to_string()
                .expect("payload_to_string must work"),
            "comm=bash pid=1977 prio=120 target_cpu=000 reason=3"
        );
        assert_eq!(
            trace.to_string().expect("to_string must work"),
            "kworker-123 (123) [000] .... 12345.679001: sched_wakeup: comm=bash pid=1977 prio=120 target_cpu=000 reason=3"
        );
    }

    #[test]
    fn sched_wakeup_roundtrip_default() {
        let line = "kworker-123 ( 123) [000] .... 12345.679001: sched_wakeup: comm=bash pid=1977 prio=120 target_cpu=000";
        let trace = TraceSchedWakeup::parse(line).expect("must parse");
        let rendered = trace.to_string().expect("to_string must work");
        let reparsed = TraceSchedWakeup::parse(&rendered).expect("must reparse");
        assert_eq!(reparsed.format_id, 0);
        assert_eq!(reparsed.reason, None);
    }

    #[test]
    fn sched_wakeup_roundtrip_with_reason() {
        let line = "kworker-123 ( 123) [000] .... 12345.679001: sched_wakeup: comm=bash pid=1977 prio=120 target_cpu=000 reason=3";
        let trace = TraceSchedWakeup::parse(line).expect("must parse");
        let rendered = trace.to_string().expect("to_string must work");
        let reparsed = TraceSchedWakeup::parse(&rendered).expect("must reparse");
        assert_eq!(reparsed.format_id, 1);
        assert_eq!(reparsed.reason, Some(3));
    }

    #[test]
    fn sched_wakeup_new_parses() {
        let line = "kworker-123 ( 123) [000] .... 12345.679001: sched_wakeup_new: comm=bash pid=1977 prio=120 target_cpu=000";
        let trace = TraceSchedWakeupNew::parse(line).expect("sched_wakeup_new must parse");
        assert_eq!(trace.thread_name, "kworker");
        assert_eq!(trace.tid, 123);
        assert_eq!(trace.tgid, 123);
        assert_eq!(trace.cpu, 0);
        assert_eq!(trace.flags, "....");
        assert!((trace.timestamp - 12345.679001).abs() < 1e-9);
        assert_eq!(trace.event_name, "sched_wakeup_new");
        assert_eq!(trace.comm, "bash");
        assert_eq!(trace.pid, 1977);
        assert_eq!(trace.prio, 120);
        assert_eq!(trace.target_cpu, 0);
    }
}

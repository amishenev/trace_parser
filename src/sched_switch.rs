use pyo3::prelude::*;
use regex::Captures;
use std::sync::LazyLock;

use crate::common::{
    cap_parse, cap_str, parse_template_event, validate_timestamp, BaseTraceParts, EventType,
    FastMatch, TemplateEvent,
};
use crate::format_registry::{FormatRegistry, FormatSpec};
use crate::payload_template::{FieldSpec, PayloadTemplate, TemplateValue};
use crate::register_parser;
use crate::trace::{extract_base_fields, format_trace_header};

static TEMPLATE: LazyLock<PayloadTemplate> = LazyLock::new(|| {
    PayloadTemplate::new(
        "prev_comm={prev_comm} prev_pid={prev_pid} prev_prio={prev_prio} prev_state={prev_state} ==> next_comm={next_comm} next_pid={next_pid} next_prio={next_prio}",
        &[
            ("prev_comm", FieldSpec::string()),
            ("prev_pid", FieldSpec::u32()),
            ("prev_prio", FieldSpec::i32()),
            ("prev_state", FieldSpec::custom(r"\S+")),
            ("next_comm", FieldSpec::string()),
            ("next_pid", FieldSpec::u32()),
            ("next_prio", FieldSpec::i32()),
        ],
    )
});

static FORMATS: LazyLock<FormatRegistry> = LazyLock::new(|| {
    FormatRegistry::new(vec![
        FormatSpec {
            kind: 0,
            template: &TEMPLATE,
        },
    ])
});

#[pyclass(skip_from_py_object)]
#[derive(Clone, Debug, PartialEq)]
pub struct TraceSchedSwitch {
    #[pyo3(get, set)]
    pub thread_name: String,
    #[pyo3(get, set)]
    pub thread_tid: u32,
    #[pyo3(get, set)]
    pub thread_tgid: u32,
    #[pyo3(get, set)]
    pub cpu: u32,
    #[pyo3(get, set)]
    pub flags: String,
    #[pyo3(get, set)]
    pub timestamp: f64,
    #[pyo3(get)]
    pub event_name: String,
    format_id: u8,
    #[pyo3(get, set)]
    pub prev_comm: String,
    #[pyo3(get, set)]
    pub prev_pid: u32,
    #[pyo3(get, set)]
    pub prev_prio: i32,
    #[pyo3(get, set)]
    pub prev_state: String,
    #[pyo3(get, set)]
    pub next_comm: String,
    #[pyo3(get, set)]
    pub next_pid: u32,
    #[pyo3(get, set)]
    pub next_prio: i32,
}

impl EventType for TraceSchedSwitch {
    const EVENT_NAME: &'static str = "sched_switch";
}

impl FastMatch for TraceSchedSwitch {}

impl TemplateEvent for TraceSchedSwitch {
    fn formats() -> &'static FormatRegistry {
        &FORMATS
    }

    fn parse_payload(
        parts: BaseTraceParts,
        captures: &Captures<'_>,
        _format_id: u8,
    ) -> Option<Self> {
        let (thread_name, thread_tid, thread_tgid, cpu, flags, timestamp, event_name, _) =
            extract_base_fields(&parts);

        Some(Self {
            thread_name,
            thread_tid,
            thread_tgid,
            cpu,
            flags,
            timestamp,
            event_name,
            format_id: 0,
            prev_comm: cap_str(captures, "prev_comm")?,
            prev_pid: cap_parse(captures, "prev_pid")?,
            prev_prio: cap_parse(captures, "prev_prio")?,
            prev_state: cap_str(captures, "prev_state")?,
            next_comm: cap_str(captures, "next_comm")?,
            next_pid: cap_parse(captures, "next_pid")?,
            next_prio: cap_parse(captures, "next_prio")?,
        })
    }

    fn render_payload(&self) -> PyResult<String> {
        let template = Self::formats().template(0).unwrap();
        let values: [(&str, Option<TemplateValue>); 7] = [
            ("prev_comm", Some(TemplateValue::Str(&self.prev_comm))),
            ("prev_pid", Some(TemplateValue::U32(self.prev_pid))),
            ("prev_prio", Some(TemplateValue::I32(self.prev_prio))),
            ("prev_state", Some(TemplateValue::Str(&self.prev_state))),
            ("next_comm", Some(TemplateValue::Str(&self.next_comm))),
            ("next_pid", Some(TemplateValue::U32(self.next_pid))),
            ("next_prio", Some(TemplateValue::I32(self.next_prio))),
        ];
        Ok(template
            .format(&values)
            .expect("sched_switch payload template must render"))
    }
}

#[pymethods]
impl TraceSchedSwitch {
    #[new]
    #[pyo3(signature = (thread_name, thread_tid, thread_tgid, cpu, flags, timestamp, prev_comm, prev_pid, prev_prio, prev_state, next_comm, next_pid, next_prio))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        thread_name: String,
        thread_tid: u32,
        thread_tgid: u32,
        cpu: u32,
        flags: String,
        timestamp: f64,
        prev_comm: String,
        prev_pid: u32,
        prev_prio: i32,
        prev_state: String,
        next_comm: String,
        next_pid: u32,
        next_prio: i32,
    ) -> PyResult<Self> {
        validate_timestamp(timestamp)?;
        Ok(Self {
            thread_name,
            thread_tid,
            thread_tgid,
            cpu,
            flags,
            timestamp,
            event_name: Self::EVENT_NAME.to_string(),
            format_id: 0,
            prev_comm,
            prev_pid,
            prev_prio,
            prev_state,
            next_comm,
            next_pid,
            next_prio,
        })
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

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "TraceSchedSwitch(prev_comm='{}', prev_pid={}, next_comm='{}', next_pid={})",
            self.prev_comm, self.prev_pid, self.next_comm, self.next_pid
        ))
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.thread_name == other.thread_name
            && self.thread_tid == other.thread_tid
            && self.thread_tgid == other.thread_tgid
            && self.cpu == other.cpu
            && self.flags == other.flags
            && self.timestamp == other.timestamp
            && self.event_name == other.event_name
            && self.format_id == other.format_id
            && self.prev_comm == other.prev_comm
            && self.prev_pid == other.prev_pid
            && self.prev_prio == other.prev_prio
            && self.prev_state == other.prev_state
            && self.next_comm == other.next_comm
            && self.next_pid == other.next_pid
            && self.next_prio == other.next_prio
    }

    fn __str__(&self) -> PyResult<String> {
        self.to_string()
    }

    fn __copy__(slf: PyRef<'_, Self>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        Ok(slf.into_pyobject(py).map(|o| o.into_any().unbind())?)
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(self.clone())
    }

    #[getter]
    pub fn payload(&self) -> PyResult<String> {
        self.render_payload()
    }

    #[getter]
    pub fn template(&self) -> &'static str {
        Self::formats().template(self.format_id).unwrap().template_str()
    }

    pub fn to_string(&self) -> PyResult<String> {
        validate_timestamp(self.timestamp)?;
        let payload = self.payload()?;
        Ok(format_trace_header(
            &self.thread_name, self.thread_tid, self.thread_tgid, self.cpu,
            &self.flags, self.timestamp, &self.event_name,
            &payload
        ))
    }

    #[getter]
    pub fn timestamp_ms(&self) -> f64 {
        self.timestamp * 1_000.0
    }

    #[setter]
    pub fn set_timestamp_ms(&mut self, value: f64) -> PyResult<()> {
        validate_timestamp(value / 1_000.0)?;
        self.timestamp = value / 1_000.0;
        Ok(())
    }

    #[getter]
    pub fn timestamp_ns(&self) -> u64 {
        (self.timestamp * 1_000_000_000.0).round() as u64
    }

    #[setter]
    pub fn set_timestamp_ns(&mut self, value: u64) -> PyResult<()> {
        self.timestamp = (value as f64) / 1_000_000_000.0;
        Ok(())
    }
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

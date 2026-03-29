use pyo3::prelude::*;
use regex::Captures;
use std::sync::LazyLock;

use crate::common::{
    cap_parse, cap_str, parse_template_event, validate_timestamp, BaseTraceParts, EventType,
    FastMatch, TemplateEvent,
};
use crate::format_registry::{FormatRegistry, FormatSpec};
use crate::payload_template::{FieldSpec, PayloadTemplate, TemplateValue};
use crate::trace::Trace;

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
#[derive(Clone, Debug)]
pub struct TraceSchedSwitch {
    #[pyo3(get)]
    pub(crate) base: Trace,
    #[pyo3(get, set)]
    pub(crate) format_id: u8,
    #[pyo3(get, set)]
    pub(crate) prev_comm: String,
    #[pyo3(get, set)]
    pub(crate) prev_pid: u32,
    #[pyo3(get, set)]
    pub(crate) prev_prio: i32,
    #[pyo3(get, set)]
    pub(crate) prev_state: String,
    #[pyo3(get, set)]
    pub(crate) next_comm: String,
    #[pyo3(get, set)]
    pub(crate) next_pid: u32,
    #[pyo3(get, set)]
    pub(crate) next_prio: i32,
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
        Some(Self {
            base: Trace::from_parts(parts),
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
    #[pyo3(signature = (thread_name, tid, tgid, cpu, flags, timestamp, event_name, payload_raw, prev_comm, prev_pid, prev_prio, prev_state, next_comm, next_pid, next_prio))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        thread_name: String,
        tid: u32,
        tgid: u32,
        cpu: u32,
        flags: String,
        timestamp: f64,
        event_name: String,
        payload_raw: String,
        prev_comm: String,
        prev_pid: u32,
        prev_prio: i32,
        prev_state: String,
        next_comm: String,
        next_pid: u32,
        next_prio: i32,
    ) -> PyResult<Self> {
        validate_timestamp(timestamp)?;
        let base = Trace {
            thread_name,
            tid,
            tgid,
            cpu,
            flags,
            timestamp,
            event_name,
            payload_raw,
        };
        Ok(Self {
            base,
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
        self.base == other.base
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

    pub fn payload_to_string(&self) -> PyResult<String> {
        self.render_payload()
    }

    pub fn to_string(&self) -> PyResult<String> {
        validate_timestamp(self.base.timestamp)?;
        Ok(self.base.to_string_with_payload(&self.payload_to_string()?))
    }
}

#[cfg(test)]
mod tests {
    use crate::{Trace, TraceSchedSwitch};

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
        assert_eq!(trace.base.thread_name, "bash");
        assert_eq!(trace.prev_comm, "bash");
        assert_eq!(trace.prev_pid, 1977);
        assert_eq!(trace.prev_prio, 120);
        assert_eq!(trace.prev_state, "S");
        assert_eq!(trace.next_comm, "worker");
        assert_eq!(trace.next_pid, 123);
        assert_eq!(trace.next_prio, 120);
        assert_eq!(
            trace.payload_to_string().expect("payload_to_string must work"),
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
        let mut trace = Trace::parse(line).expect("trace must parse");
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

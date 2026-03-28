use std::collections::HashMap;

use once_cell::sync::Lazy;
use pyo3::prelude::*;

use crate::common::{parse_base_parts, validate_timestamp};
use crate::payload_template::{FieldSpec, PayloadTemplate, TemplateValue};
use crate::trace::Trace;

static SCHED_SWITCH_TEMPLATE: Lazy<PayloadTemplate> = Lazy::new(|| {
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

#[pyclass]
#[derive(Clone, Debug)]
pub struct TraceSchedSwitch {
    #[pyo3(get)]
    pub(crate) base: Trace,
    #[pyo3(get, set)]
    pub(crate) format_id: String,
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

#[pymethods]
impl TraceSchedSwitch {
    #[staticmethod]
    pub fn can_be_parsed(line: &str) -> bool {
        let Some(parts) = parse_base_parts(line) else {
            return false;
        };
        parts.event_name == "sched_switch" && SCHED_SWITCH_TEMPLATE.is_match(&parts.payload_raw)
    }

    #[staticmethod]
    pub fn parse(line: &str) -> Option<Self> {
        let parts = parse_base_parts(line)?;
        if parts.event_name != "sched_switch" {
            return None;
        }

        let captures = SCHED_SWITCH_TEMPLATE.captures(&parts.payload_raw)?;
        let prev_comm = captures.name("prev_comm")?.as_str().to_owned();
        let prev_pid = captures.name("prev_pid")?.as_str().parse().ok()?;
        let prev_prio = captures.name("prev_prio")?.as_str().parse().ok()?;
        let prev_state = captures.name("prev_state")?.as_str().to_owned();
        let next_comm = captures.name("next_comm")?.as_str().to_owned();
        let next_pid = captures.name("next_pid")?.as_str().parse().ok()?;
        let next_prio = captures.name("next_prio")?.as_str().parse().ok()?;
        Some(Self {
            base: Trace::from_parts(parts),
            format_id: "default".to_owned(),
            prev_comm,
            prev_pid,
            prev_prio,
            prev_state,
            next_comm,
            next_pid,
            next_prio,
        })
    }

    pub(crate) fn payload_to_string(&self) -> PyResult<String> {
        let values = HashMap::from([
            ("prev_comm", TemplateValue::Str(&self.prev_comm)),
            ("prev_pid", TemplateValue::U32(self.prev_pid)),
            ("prev_prio", TemplateValue::I32(self.prev_prio)),
            ("prev_state", TemplateValue::Str(&self.prev_state)),
            ("next_comm", TemplateValue::Str(&self.next_comm)),
            ("next_pid", TemplateValue::U32(self.next_pid)),
            ("next_prio", TemplateValue::I32(self.next_prio)),
        ]);

        Ok(SCHED_SWITCH_TEMPLATE
            .format(&values)
            .expect("sched_switch payload template must render"))
    }

    pub(crate) fn to_string(&self) -> PyResult<String> {
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
